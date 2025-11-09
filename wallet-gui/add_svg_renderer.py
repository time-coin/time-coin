with open('src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Add imports at the top after the existing use statements
import_section = '''use eframe::egui;
use wallet::NetworkType;

mod wallet_dat;
mod wallet_manager;

use wallet_manager::WalletManager;'''

new_import_section = '''use eframe::egui;
use wallet::NetworkType;
use image::ImageBuffer;

mod wallet_dat;
mod wallet_manager;

use wallet_manager::WalletManager;'''

content = content.replace(import_section, new_import_section)

# Replace the QR code display section
old_qr_display = '''                        // Display QR code
                        if let Ok(qr_code) = manager.get_address_qr_code_svg(&address) {
                            ui.label("📱 Scan QR Code:");
                            ui.add_space(10.0);
                            ui.style_mut().visuals.override_text_color = Some(egui::Color32::BLACK);
                            ui.label(egui::RichText::new(&qr_code).monospace().size(12.0).color(egui::Color32::BLACK));
                            ui.style_mut().visuals.override_text_color = None;
                        }'''

new_qr_display = '''                        // Display QR code
                        if let Ok(svg_string) = manager.get_address_qr_code_svg(&address) {
                            ui.label("📱 Scan QR Code:");
                            ui.add_space(10.0);
                            
                            // Convert SVG to image and display
                            if let Ok(image_data) = Self::svg_to_image(&svg_string) {
                                let texture = ctx.load_texture(
                                    "qr_code",
                                    image_data,
                                    egui::TextureOptions::default()
                                );
                                ui.image(&texture);
                            } else {
                                ui.label("Failed to render QR code");
                            }
                        }'''

content = content.replace(old_qr_display, new_qr_display)

# Add the svg_to_image helper method before the format_amount method
format_amount_start = '    fn format_amount(amount: u64) -> String {'

svg_helper = '''    fn svg_to_image(svg_string: &str) -> Result<egui::ColorImage, String> {
        use resvg::usvg;
        use tiny_skia::Pixmap;
        
        let opt = usvg::Options::default();
        let tree = usvg::Tree::from_str(svg_string, &opt)
            .map_err(|e| format!("Failed to parse SVG: {}", e))?;
        
        let size = tree.size();
        let width = size.width() as u32;
        let height = size.height() as u32;
        
        let mut pixmap = Pixmap::new(width, height)
            .ok_or_else(|| "Failed to create pixmap".to_string())?;
        
        resvg::render(&tree, tiny_skia::Transform::default(), &mut pixmap.as_mut());
        
        let pixels = pixmap.data();
        let mut color_image = egui::ColorImage::new([width as usize, height as usize], egui::Color32::WHITE);
        
        for y in 0..height as usize {
            for x in 0..width as usize {
                let i = (y * width as usize + x) * 4;
                let r = pixels[i];
                let g = pixels[i + 1];
                let b = pixels[i + 2];
                let a = pixels[i + 3];
                color_image.pixels[y * width as usize + x] = egui::Color32::from_rgba_premultiplied(r, g, b, a);
            }
        }
        
        Ok(color_image)
    }

'''

content = content.replace(format_amount_start, svg_helper + format_amount_start)

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Added SVG rendering functionality!')
