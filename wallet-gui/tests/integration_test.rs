//! Integration test for wallet-gui components
//! Tests the complete flow of wallet creation, transaction creation, and key management

use wallet::{NetworkType, UTXO};

// We need to include the modules from the main binary
// For this integration test, we'll test the underlying wallet library directly
// since the GUI is hard to test without a display

#[test]
fn test_complete_wallet_flow() {
    use wallet::Wallet;

    println!("Testing complete wallet flow...");

    // Create sender wallet
    let mut sender = Wallet::new(NetworkType::Testnet).expect("Failed to create sender wallet");
    let sender_address = sender.address_string();
    println!("Sender address: {}", sender_address);

    // Add funds via UTXO
    let utxo = UTXO {
        tx_hash: [1u8; 32],
        output_index: 0,
        amount: 10000,
        address: sender_address.clone(),
    };
    sender.add_utxo(utxo);
    assert_eq!(sender.balance(), 10000);

    // Create recipient wallet
    let recipient = Wallet::new(NetworkType::Testnet).expect("Failed to create recipient wallet");
    let recipient_address = recipient.address_string();
    println!("Recipient address: {}", recipient_address);

    // Create transaction
    let tx = sender
        .create_transaction(&recipient_address, 1000, 10)
        .expect("Failed to create transaction");

    // Verify transaction
    assert_eq!(tx.outputs.len(), 2); // recipient + change
    assert_eq!(tx.outputs[0].amount, 1000);
    assert_eq!(tx.outputs[1].amount, 8990); // 10000 - 1000 - 10

    println!("✅ Complete wallet flow test passed");
}

#[test]
fn test_key_import_export() {
    use wallet::Wallet;

    // Create original wallet
    let wallet1 = Wallet::new(NetworkType::Testnet).expect("Failed to create wallet");
    let private_key = wallet1.export_private_key();
    let address1 = wallet1.address_string();

    // Import to new wallet
    let wallet2 = Wallet::from_private_key_hex(&private_key, NetworkType::Testnet)
        .expect("Failed to import private key");
    let address2 = wallet2.address_string();

    // Verify addresses match
    assert_eq!(address1, address2);
    assert_eq!(wallet1.public_key(), wallet2.public_key());

    println!("✅ Key import/export test passed");
}

#[test]
fn test_multiple_utxos() {
    use wallet::Wallet;

    let mut wallet = Wallet::new(NetworkType::Testnet).expect("Failed to create wallet");
    let address = wallet.address_string();

    // Add multiple UTXOs
    for i in 0..5 {
        let utxo = UTXO {
            tx_hash: [i; 32],
            output_index: i as u32,
            amount: 1000,
            address: address.clone(),
        };
        wallet.add_utxo(utxo);
    }

    assert_eq!(wallet.balance(), 5000);
    assert_eq!(wallet.utxos().len(), 5);

    // Create transaction that needs multiple UTXOs
    let recipient = Wallet::new(NetworkType::Testnet).expect("Failed to create recipient");
    let tx = wallet
        .create_transaction(&recipient.address_string(), 4500, 50)
        .expect("Failed to create transaction");

    // Should use all 5 UTXOs
    assert_eq!(tx.inputs.len(), 5);
    assert_eq!(tx.outputs.len(), 2); // recipient + change

    println!("✅ Multiple UTXOs test passed");
}

#[test]
fn test_insufficient_funds() {
    use wallet::Wallet;

    let mut wallet = Wallet::new(NetworkType::Testnet).expect("Failed to create wallet");
    let address = wallet.address_string();

    // Add small UTXO
    let utxo = UTXO {
        tx_hash: [1u8; 32],
        output_index: 0,
        amount: 100,
        address: address.clone(),
    };
    wallet.add_utxo(utxo);

    // Try to send more than available
    let recipient = Wallet::new(NetworkType::Testnet).expect("Failed to create recipient");
    let result = wallet.create_transaction(&recipient.address_string(), 1000, 10);

    assert!(result.is_err());
    println!("✅ Insufficient funds test passed");
}

#[test]
fn test_wallet_persistence() {
    use wallet::Wallet;
    use std::fs;

    let temp_path = "/tmp/test_wallet_persist.json";

    // Create and save wallet
    let wallet1 = Wallet::new(NetworkType::Testnet).expect("Failed to create wallet");
    let address1 = wallet1.address_string();
    wallet1
        .save_to_file(temp_path)
        .expect("Failed to save wallet");

    // Load wallet
    let wallet2 = Wallet::load_from_file(temp_path).expect("Failed to load wallet");
    let address2 = wallet2.address_string();

    // Verify they match
    assert_eq!(address1, address2);
    assert_eq!(wallet1.public_key(), wallet2.public_key());

    // Cleanup
    fs::remove_file(temp_path).ok();

    println!("✅ Wallet persistence test passed");
}
