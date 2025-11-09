with open('src/main.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Remove unused import
content = content.replace('use image::ImageBuffer;\n', '')

# Fix WalletManager::load to load_default
content = content.replace(
    'match WalletManager::load(self.network) {',
    'match WalletManager::load_default(self.network) {'
)

# Fix WalletManager::create to create_new with label
content = content.replace(
    'match WalletManager::create(self.network) {',
    'match WalletManager::create_new(self.network, "Default".to_string()) {'
)

# Fix unused variable warning
content = content.replace(
    'egui::ScrollArea::vertical().show(ui, |ui| {',
    'egui::ScrollArea::vertical().show(ui, |_ui| {'
)

with open('src/main.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Fixed function names and warnings!')
