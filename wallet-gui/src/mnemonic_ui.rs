// BIP-39 Mnemonic Interface Module
// Provides comprehensive mnemonic phrase management with GUI

use crate::wallet_manager::WalletManager;
use eframe::egui;
use printpdf::*;
use std::fs::File;
use std::io::BufWriter;

#[derive(PartialEq, Clone)]
pub enum MnemonicMode {
    Generate,
    Import,
    Edit,
}

pub struct MnemonicInterface {
    // Word boxes (supports 12 or 24 words)
    pub words: Vec<String>,
    pub use_24_words: bool,

    // Mode selection
    pub mode: MnemonicMode,

    // UI state
    pub show_words: bool,
    pub show_print_dialog: bool,
    pub generated_phrase: Option<String>,

    // Validation
    pub validation_error: Option<String>,
    pub is_valid: bool,

    // Edit mode
    pub edit_enabled: bool,
}

impl Default for MnemonicInterface {
    fn default() -> Self {
        Self {
            words: vec![String::new(); 12],
            use_24_words: false,
            mode: MnemonicMode::Generate,
            show_words: true,
            show_print_dialog: false,
            generated_phrase: None,
            validation_error: None,
            is_valid: false,
            edit_enabled: false,
        }
    }
}

impl MnemonicInterface {
    pub fn new() -> Self {
        Self::default()
    }

    /// Toggle between 12 and 24 words
    pub fn toggle_word_count(&mut self) {
        self.use_24_words = !self.use_24_words;
        let new_count = if self.use_24_words { 24 } else { 12 };
        self.words.resize(new_count, String::new());
    }

    /// Generate new mnemonic phrase
    pub fn generate_mnemonic(&mut self) -> Result<(), String> {
        match WalletManager::generate_mnemonic() {
            Ok(phrase) => {
                self.generated_phrase = Some(phrase.clone());
                self.words = phrase.split_whitespace().map(|s| s.to_string()).collect();

                // Pad to 12 or 24 words if needed
                let target = if self.use_24_words { 24 } else { 12 };
                self.words.resize(target, String::new());

                self.is_valid = true;
                self.validation_error = None;
                Ok(())
            }
            Err(e) => {
                self.validation_error = Some(format!("Failed to generate mnemonic: {}", e));
                Err(e.to_string())
            }
        }
    }

    /// Validate current mnemonic phrase
    pub fn validate(&mut self) -> bool {
        let phrase = self.get_phrase();

        if phrase.is_empty() {
            self.validation_error = Some("Mnemonic phrase is empty".to_string());
            self.is_valid = false;
            return false;
        }

        match WalletManager::validate_mnemonic(&phrase) {
            Ok(_) => {
                self.validation_error = None;
                self.is_valid = true;
                true
            }
            Err(e) => {
                self.validation_error = Some(e.to_string());
                self.is_valid = false;
                false
            }
        }
    }

    /// Get the complete mnemonic phrase
    pub fn get_phrase(&self) -> String {
        self.words
            .iter()
            .filter(|w| !w.is_empty())
            .cloned()
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Clear all words
    pub fn clear(&mut self) {
        for word in &mut self.words {
            word.clear();
        }
        self.validation_error = None;
        self.is_valid = false;
        self.generated_phrase = None;
    }

    /// Render the mnemonic interface
    pub fn render(&mut self, ui: &mut egui::Ui) -> Option<MnemonicAction> {
        let mut action = None;

        // Header
        ui.heading("🔐 BIP-39 Mnemonic Phrase");
        ui.add_space(10.0);

        // Mode selection
        ui.horizontal(|ui| {
            ui.label("Mode:");
            if ui
                .selectable_label(self.mode == MnemonicMode::Generate, "Generate")
                .clicked()
            {
                self.mode = MnemonicMode::Generate;
                self.edit_enabled = false;
            }
            if ui
                .selectable_label(self.mode == MnemonicMode::Import, "Import")
                .clicked()
            {
                self.mode = MnemonicMode::Import;
                self.edit_enabled = true;
            }
            if ui
                .selectable_label(self.mode == MnemonicMode::Edit, "Edit")
                .clicked()
            {
                self.mode = MnemonicMode::Edit;
                self.edit_enabled = true;
            }
        });

        ui.add_space(10.0);

        // Word count toggle
        ui.horizontal(|ui| {
            if ui
                .checkbox(&mut self.use_24_words, "Use 24 words (advanced security)")
                .changed()
            {
                self.toggle_word_count();
            }

            ui.label(format!("({} words)", self.words.len()));
        });

        ui.add_space(10.0);

        // Action buttons
        ui.horizontal(|ui| {
            if self.mode == MnemonicMode::Generate && ui.button("Generate New Phrase").clicked() {
                if let Err(e) = self.generate_mnemonic() {
                    self.validation_error = Some(e);
                }
            }

            if ui.button("Clear All").clicked() {
                self.clear();
            }

            if ui.button("Validate").clicked() {
                self.validate();
            }

            if ui.button("Print").clicked() {
                self.show_print_dialog = true;
            }
        });

        ui.add_space(10.0);

        // Validation status
        if let Some(error) = &self.validation_error {
            ui.colored_label(egui::Color32::RED, format!("Invalid: {}", error));
        } else if self.is_valid {
            ui.colored_label(egui::Color32::GREEN, "Valid mnemonic phrase");
        }

        ui.add_space(10.0);

        // Word input grid
        self.render_word_grid(ui);

        ui.add_space(20.0);

        // Confirmation buttons
        ui.horizontal(|ui| {
            if ui.button("Cancel").clicked() {
                action = Some(MnemonicAction::Cancel);
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let can_proceed = self.is_valid
                    || (!self.get_phrase().is_empty() && self.mode == MnemonicMode::Import);

                if ui
                    .add_enabled(can_proceed, egui::Button::new("Continue ➡"))
                    .clicked()
                    && self.validate()
                {
                    action = Some(MnemonicAction::Confirm(self.get_phrase()));
                }
            });
        });

        // Print dialog
        if self.show_print_dialog {
            self.render_print_dialog(ui.ctx());
        }

        action
    }

    /// Render the word input grid
    fn render_word_grid(&mut self, ui: &mut egui::Ui) {
        let total_words = if self.use_24_words { 24 } else { 12 };
        let words_per_column = if self.use_24_words { 12 } else { 6 };
        let num_columns = 2; // Always use 2 columns

        egui::Grid::new("mnemonic_grid")
            .num_columns(num_columns * 2) // Label + input for each column
            .spacing([10.0, 5.0])
            .show(ui, |ui| {
                for row in 0..words_per_column {
                    for col in 0..num_columns {
                        // Row-major ordering: index goes 1,2,3... sequentially left to right
                        let index = row * num_columns + col;
                        if index < total_words {
                            // Word number
                            ui.label(format!("{}.", index + 1));

                            // Word input
                            let enabled = self.edit_enabled
                                || self.mode != MnemonicMode::Generate
                                || self.generated_phrase.is_none();
                            ui.add_enabled(
                                enabled,
                                egui::TextEdit::singleline(&mut self.words[index])
                                    .desired_width(150.0) // Increased from 120.0 to show full words
                                    .hint_text("word"),
                            );
                        }
                    }
                    ui.end_row();
                }
            });
    }

    /// Render print dialog
    fn render_print_dialog(&mut self, ctx: &egui::Context) {
        egui::Window::new("🖨️ Print Mnemonic Phrase")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.heading("Print Recovery Phrase");
                ui.add_space(10.0);

                ui.colored_label(egui::Color32::YELLOW, "⚠️ SECURITY WARNING:");
                ui.label("• Store this in a safe, secure location");
                ui.label("• Never share with anyone");
                ui.label("• Do not store digitally");
                ui.label("• This is your only way to recover your wallet");

                ui.add_space(10.0);
                ui.separator();
                ui.add_space(10.0);

                // Display phrase in a copyable format
                let phrase = self.get_phrase();
                ui.label("Your Recovery Phrase:");
                ui.add_space(5.0);

                egui::ScrollArea::vertical()
                    .max_height(200.0)
                    .show(ui, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut phrase.as_str())
                                .desired_width(400.0)
                                .font(egui::TextStyle::Monospace),
                        );
                    });

                ui.add_space(10.0);

                // Formatted print view
                ui.label("Print Format:");
                ui.add_space(5.0);

                // Simple readable format without dark background
                ui.group(|ui| {
                    ui.set_width(400.0);

                    ui.heading("TIME Coin Recovery Phrase");
                    ui.add_space(5.0);
                    ui.label(format!("Date: {}", chrono::Utc::now().format("%Y-%m-%d")));
                    ui.add_space(10.0);

                    // Display words in grid
                    egui::Grid::new("print_grid")
                        .num_columns(2)
                        .spacing([20.0, 5.0])
                        .show(ui, |ui| {
                            let words: Vec<&str> = phrase.split_whitespace().collect();
                            let mid = words.len().div_ceil(2);

                            for i in 0..mid {
                                // Left column
                                if let Some(word) = words.get(i) {
                                    ui.label(format!("{}. {}", i + 1, word));
                                }

                                // Right column
                                let right_idx = i + mid;
                                if let Some(word) = words.get(right_idx) {
                                    ui.label(format!("{}. {}", right_idx + 1, word));
                                }

                                ui.end_row();
                            }
                        });

                    ui.add_space(10.0);
                    ui.label("Keep this safe and secure!");
                });

                ui.add_space(10.0);

                ui.horizontal(|ui| {
                    if ui.button("Copy to Clipboard").clicked() {
                        ui.ctx().copy_text(phrase.clone());
                    }

                    if ui.button("Save as PDF").clicked() {
                        if let Err(e) = self.save_as_pdf(&phrase) {
                            eprintln!("Failed to save PDF: {}", e);
                        }
                    }

                    if ui.button("Print PDF").clicked() {
                        if let Err(e) = self.print_to_pdf(&phrase) {
                            eprintln!("Failed to print: {}", e);
                        } else {
                            // Open the PDF in the default viewer for printing
                            #[cfg(target_os = "windows")]
                            {
                                let _ = std::process::Command::new("cmd")
                                    .args(&["/C", "start", "mnemonic_recovery_phrase.pdf"])
                                    .spawn();
                            }
                            #[cfg(target_os = "linux")]
                            {
                                let _ = std::process::Command::new("xdg-open")
                                    .arg("mnemonic_recovery_phrase.pdf")
                                    .spawn();
                            }
                            #[cfg(target_os = "macos")]
                            {
                                let _ = std::process::Command::new("open")
                                    .arg("mnemonic_recovery_phrase.pdf")
                                    .spawn();
                            }
                        }
                    }

                    if ui.button("Close").clicked() {
                        self.show_print_dialog = false;
                    }
                });
            });
    }

    /// Generate PDF with mnemonic phrase
    fn save_as_pdf(&self, phrase: &str) -> Result<(), Box<dyn std::error::Error>> {
        let (doc, page1, layer1) =
            PdfDocument::new("TIME Coin Recovery Phrase", Mm(210.0), Mm(297.0), "Layer 1");

        let font = doc.add_builtin_font(BuiltinFont::Helvetica)?;
        let font_bold = doc.add_builtin_font(BuiltinFont::HelveticaBold)?;
        let current_layer = doc.get_page(page1).get_layer(layer1);

        // Title
        current_layer.use_text(
            "TIME Coin Recovery Phrase",
            24.0,
            Mm(20.0),
            Mm(270.0),
            &font_bold,
        );

        // Date
        let date_str = format!("Date: {}", chrono::Utc::now().format("%Y-%m-%d %H:%M UTC"));
        current_layer.use_text(&date_str, 12.0, Mm(20.0), Mm(260.0), &font);

        // Warning
        current_layer.use_text(
            "SECURITY WARNING: Keep this document safe and secure!",
            10.0,
            Mm(20.0),
            Mm(250.0),
            &font_bold,
        );
        current_layer.use_text(
            "Never share with anyone. Store in a secure location.",
            10.0,
            Mm(20.0),
            Mm(245.0),
            &font,
        );

        // Recovery phrase words
        let words: Vec<&str> = phrase.split_whitespace().collect();
        let mut y_pos = 230.0;
        let left_x = 20.0;
        let right_x = 120.0;

        current_layer.use_text(
            "Your Recovery Phrase:",
            14.0,
            Mm(left_x),
            Mm(y_pos),
            &font_bold,
        );
        y_pos -= 10.0;

        for (i, chunk) in words.chunks(2).enumerate() {
            if let Some(word) = chunk.get(0) {
                let text = format!("{}. {}", i * 2 + 1, word);
                current_layer.use_text(&text, 11.0, Mm(left_x), Mm(y_pos), &font);
            }
            if let Some(word) = chunk.get(1) {
                let text = format!("{}. {}", i * 2 + 2, word);
                current_layer.use_text(&text, 11.0, Mm(right_x), Mm(y_pos), &font);
            }
            y_pos -= 8.0;
        }

        // Save instructions at bottom
        y_pos = 30.0;
        current_layer.use_text("Instructions:", 12.0, Mm(left_x), Mm(y_pos), &font_bold);
        y_pos -= 7.0;
        current_layer.use_text(
            "1. Write down these words in order",
            10.0,
            Mm(left_x),
            Mm(y_pos),
            &font,
        );
        y_pos -= 6.0;
        current_layer.use_text(
            "2. Store this paper in a safe, secure location",
            10.0,
            Mm(left_x),
            Mm(y_pos),
            &font,
        );
        y_pos -= 6.0;
        current_layer.use_text(
            "3. Never share these words with anyone",
            10.0,
            Mm(left_x),
            Mm(y_pos),
            &font,
        );
        y_pos -= 6.0;
        current_layer.use_text(
            "4. These words are your ONLY way to recover your wallet",
            10.0,
            Mm(left_x),
            Mm(y_pos),
            &font,
        );

        // Save to file
        let file = File::create("mnemonic_recovery_phrase.pdf")?;
        let mut writer = BufWriter::new(file);
        doc.save(&mut writer)?;

        Ok(())
    }

    /// Print to PDF (same as save, but intended for printing)
    fn print_to_pdf(&self, phrase: &str) -> Result<(), Box<dyn std::error::Error>> {
        self.save_as_pdf(phrase)
    }
}

pub enum MnemonicAction {
    Confirm(String),
    Cancel,
}
