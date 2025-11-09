with open('src/wallet_manager.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Replace the text QR code method with SVG version
old_method = '''    /// Get QR code for an address
    pub fn get_address_qr_code(&self, address: &str) -> Result<String, String> {
        self.wallet_dat
            .get_keys()
            .iter()
            .find(|k| k.address == address)
            .ok_or_else(|| "Address not found".to_string())
            .and_then(|_| {
                let wallet = Wallet::from_secret_key(
                    &self.wallet_dat.get_keys().iter().find(|k| k.address == address).unwrap().keypair_bytes,
                    self.wallet_dat.network
                ).map_err(|e| e.to_string())?;
                wallet.address_qr_code().map_err(|e| e.to_string())
            })
    }'''

new_method = '''    /// Get QR code for an address as SVG
    pub fn get_address_qr_code_svg(&self, address: &str) -> Result<String, String> {
        self.wallet_dat
            .get_keys()
            .iter()
            .find(|k| k.address == address)
            .ok_or_else(|| "Address not found".to_string())
            .and_then(|_| {
                let wallet = Wallet::from_secret_key(
                    &self.wallet_dat.get_keys().iter().find(|k| k.address == address).unwrap().keypair_bytes,
                    self.wallet_dat.network
                ).map_err(|e| e.to_string())?;
                wallet.address_qr_code_svg().map_err(|e| e.to_string())
            })
    }'''

content = content.replace(old_method, new_method)

with open('src/wallet_manager.rs', 'w', encoding='utf-8') as f:
    f.write(content)

print('Updated wallet_manager to use SVG QR code')
