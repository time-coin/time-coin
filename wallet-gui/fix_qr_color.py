with open('src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Replace the gray monospace QR code with black colored label
old_qr = '''                        // Display QR code
                        if let Ok(qr_code) = manager.get_address_qr_code(&address) {
                            ui.label("📱 Scan QR Code:");
                            ui.add_space(10.0);
                            ui.monospace(&qr_code);
                        }'''

new_qr = '''                        // Display QR code
                        if let Ok(qr_code) = manager.get_address_qr_code(&address) {
                            ui.label("📱 Scan QR Code:");
                            ui.add_space(10.0);
                            ui.colored_label(egui::Color32::BLACK, &qr_code);
                        }'''

content = content.replace(old_qr, new_qr)

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Fixed QR code color!')
