with open('src/main.rs', 'r', encoding='utf-8') as f:
    lines = f.readlines()

new_lines = []
i = 0
while i < len(lines):
    new_lines.append(lines[i])
    # Look for the line with the closing brace after "Copy Address"
    if 'self.success_message = Some("Address copied to clipboard!' in lines[i]:
        # Add the next line (closing brace)
        i += 1
        new_lines.append(lines[i])
        # Now add the QR code section
        new_lines.append('\n')
        new_lines.append('                        ui.add_space(20.0);\n')
        new_lines.append('\n')
        new_lines.append('                        // Display QR code\n')
        new_lines.append('                        if let Ok(qr_code) = manager.get_address_qr_code(&address) {\n')
        new_lines.append('                            ui.label("📱 Scan QR Code:");\n')
        new_lines.append('                            ui.add_space(10.0);\n')
        new_lines.append('                            ui.monospace(&qr_code);\n')
        new_lines.append('                        }\n')
    i += 1

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.writelines(new_lines)

print('QR code section added!')
