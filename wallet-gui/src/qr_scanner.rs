//! QR code scanner using the system webcam.
//!
//! Captures frames from the default camera in a background thread,
//! runs QR detection on each frame, and exposes the latest preview
//! frame and any decoded result to the UI.

use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use std::thread;

/// Handle to a running QR scanner background thread.
pub struct QrScannerHandle {
    latest_frame: Arc<Mutex<Option<egui::ColorImage>>>,
    result: Arc<Mutex<Option<String>>>,
    error: Arc<Mutex<Option<String>>>,
    running: Arc<AtomicBool>,
    /// Cached texture handle for the camera preview (avoids recreation flicker).
    texture: Option<egui::TextureHandle>,
}

impl std::fmt::Debug for QrScannerHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QrScannerHandle")
            .field("running", &self.running.load(Ordering::Relaxed))
            .finish()
    }
}

impl QrScannerHandle {
    /// Start scanning from the default camera.
    pub fn start() -> Self {
        let latest_frame = Arc::new(Mutex::new(None));
        let result = Arc::new(Mutex::new(None));
        let error = Arc::new(Mutex::new(None));
        let running = Arc::new(AtomicBool::new(true));

        let frame_ref = latest_frame.clone();
        let result_ref = result.clone();
        let error_ref = error.clone();
        let running_ref = running.clone();

        thread::spawn(move || {
            Self::scan_loop(frame_ref, result_ref, error_ref, running_ref);
        });

        QrScannerHandle {
            latest_frame,
            result,
            error,
            running,
            texture: None,
        }
    }

    fn scan_loop(
        frame_ref: Arc<Mutex<Option<egui::ColorImage>>>,
        result_ref: Arc<Mutex<Option<String>>>,
        error_ref: Arc<Mutex<Option<String>>>,
        running_ref: Arc<AtomicBool>,
    ) {
        use nokhwa::pixel_format::RgbFormat;
        use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};
        use nokhwa::Camera;

        let requested =
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);

        let mut camera = match Camera::new(CameraIndex::Index(0), requested) {
            Ok(c) => c,
            Err(e) => {
                if let Ok(mut g) = error_ref.lock().or_else(|p| Ok::<_, ()>(p.into_inner())) {
                    *g = Some(format!("No camera found: {}", e));
                }
                return;
            }
        };

        if let Err(e) = camera.open_stream() {
            if let Ok(mut g) = error_ref.lock().or_else(|p| Ok::<_, ()>(p.into_inner())) {
                *g = Some(format!("Failed to open camera: {}", e));
            }
            return;
        }

        while running_ref.load(Ordering::Relaxed) {
            let buffer = match camera.frame() {
                Ok(b) => b,
                Err(_) => {
                    thread::sleep(std::time::Duration::from_millis(30));
                    continue;
                }
            };

            let rgb_image = match buffer.decode_image::<RgbFormat>() {
                Ok(img) => img,
                Err(_) => continue,
            };

            // QR detection on grayscale
            let gray = image::imageops::grayscale(&rgb_image);
            let mut prepared = rqrr::PreparedImage::prepare(gray);
            let grids = prepared.detect_grids();
            for grid in grids {
                if let Ok((_, content)) = grid.decode() {
                    // Accept any content — validation happens in the UI
                    let address = content
                        .strip_prefix("time:")
                        .unwrap_or(&content)
                        .to_string();
                    if let Ok(mut g) = result_ref.lock().or_else(|p| Ok::<_, ()>(p.into_inner())) {
                        *g = Some(address);
                    }
                    running_ref.store(false, Ordering::Relaxed);
                    return;
                }
            }

            // Update preview frame for display
            let size = [rgb_image.width() as usize, rgb_image.height() as usize];
            let pixels: Vec<egui::Color32> = rgb_image
                .pixels()
                .map(|p| egui::Color32::from_rgb(p[0], p[1], p[2]))
                .collect();
            let color_image = egui::ColorImage { size, pixels };
            if let Ok(mut g) = frame_ref.lock().or_else(|p| Ok::<_, ()>(p.into_inner())) {
                *g = Some(color_image);
            }
        }
    }

    /// Take the latest camera frame (returns `None` if no new frame).
    pub fn take_frame(&self) -> Option<egui::ColorImage> {
        self.latest_frame.lock().ok()?.take()
    }

    /// Update the cached texture with the latest frame and return a reference.
    /// Reuses the existing texture handle to avoid flicker.
    pub fn update_texture(&mut self, ctx: &egui::Context) -> Option<&egui::TextureHandle> {
        if let Some(frame) = self.take_frame() {
            if let Some(ref mut tex) = self.texture {
                tex.set(frame, egui::TextureOptions::default());
            } else {
                self.texture = Some(ctx.load_texture(
                    "qr_camera_preview",
                    frame,
                    egui::TextureOptions::default(),
                ));
            }
        }
        self.texture.as_ref()
    }

    /// Check if a QR code was decoded (returns the content once).
    pub fn take_result(&self) -> Option<String> {
        self.result.lock().ok()?.take()
    }

    /// Check if there was an error starting the camera.
    pub fn get_error(&self) -> Option<String> {
        self.error.lock().ok()?.clone()
    }

    /// Signal the scanner thread to stop.
    pub fn stop(&self) {
        self.running.store(false, Ordering::Relaxed);
    }
}

impl Drop for QrScannerHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Play a system notification sound to signal successful scan.
pub fn play_scan_sound() {
    std::thread::spawn(|| {
        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            let _ = std::process::Command::new("powershell")
                .args([
                    "-NoProfile",
                    "-Command",
                    "[System.Media.SystemSounds]::Asterisk.Play()",
                ])
                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                .output();
        }
        #[cfg(target_os = "macos")]
        {
            // afplay is available on all macOS versions
            let _ = std::process::Command::new("afplay")
                .arg("/System/Library/Sounds/Tink.aiff")
                .output();
        }
        #[cfg(target_os = "linux")]
        {
            // Try paplay (PulseAudio) first, fall back to aplay (ALSA), then BEL
            let played = std::process::Command::new("paplay")
                .arg("/usr/share/sounds/freedesktop/stereo/complete.oga")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false);
            if !played {
                let _ = std::process::Command::new("aplay")
                    .arg("/usr/share/sounds/freedesktop/stereo/complete.oga")
                    .output();
            }
        }
    });
}
