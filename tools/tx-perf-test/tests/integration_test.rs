//! Integration test for testnet coin generation
//!
//! This test verifies that the testnet coin generation feature works correctly.

use wallet::{NetworkType, Wallet, UTXO};

#[test]
fn test_testnet_coin_generation() {
    // Create a test wallet
    let mut wallet = Wallet::new(NetworkType::Testnet).expect("Failed to create wallet");
    
    // Verify initial balance is zero
    assert_eq!(wallet.balance(), 0);
    assert_eq!(wallet.utxos().len(), 0);
    
    // Simulate coin generation (similar to what the tool does)
    let mint_amount = 1000000u64;
    let mut tx_hash = [0u8; 32];
    
    // Create a unique hash
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(wallet.address_string().as_bytes());
    hasher.update(&1234567890u64.to_le_bytes());
    hasher.update(b"testnet_mint");
    let hash_result = hasher.finalize();
    tx_hash.copy_from_slice(&hash_result[..32]);
    
    // Create and add UTXO
    let utxo = UTXO {
        tx_hash,
        output_index: 0,
        amount: mint_amount,
        address: wallet.address_string(),
    };
    
    wallet.add_utxo(utxo);
    
    // Verify balance updated
    assert_eq!(wallet.balance(), mint_amount);
    assert_eq!(wallet.utxos().len(), 1);
    
    // Verify UTXO properties
    let added_utxo = &wallet.utxos()[0];
    assert_eq!(added_utxo.amount, mint_amount);
    assert_eq!(added_utxo.output_index, 0);
    assert_eq!(added_utxo.address, wallet.address_string());
}

#[test]
fn test_multiple_utxo_generation() {
    let mut wallet = Wallet::new(NetworkType::Testnet).expect("Failed to create wallet");
    
    // Generate multiple UTXOs
    let amounts = vec![100000, 200000, 300000];
    let mut total_expected = 0u64;
    
    for (idx, amount) in amounts.iter().enumerate() {
        let mut tx_hash = [0u8; 32];
        
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(wallet.address_string().as_bytes());
        hasher.update(&(idx as u64).to_le_bytes());
        hasher.update(b"testnet_mint");
        let hash_result = hasher.finalize();
        tx_hash.copy_from_slice(&hash_result[..32]);
        
        let utxo = UTXO {
            tx_hash,
            output_index: idx as u32,
            amount: *amount,
            address: wallet.address_string(),
        };
        
        wallet.add_utxo(utxo);
        total_expected += amount;
    }
    
    // Verify total balance
    assert_eq!(wallet.balance(), total_expected);
    assert_eq!(wallet.utxos().len(), amounts.len());
}

#[test]
fn test_network_type_validation() {
    // Testnet should work
    let testnet_wallet = Wallet::new(NetworkType::Testnet);
    assert!(testnet_wallet.is_ok());
    
    // Mainnet should also work (validation happens at tool level)
    let mainnet_wallet = Wallet::new(NetworkType::Mainnet);
    assert!(mainnet_wallet.is_ok());
}
