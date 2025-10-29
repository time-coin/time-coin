use wallet::{Wallet, UTXO};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== TIME Coin Wallet Example ===\n");

    // Create a new wallet for testnet
    println!("Creating new wallet...");
    let mut wallet = Wallet::new(true)?;

    println!("✓ Wallet created!");
    println!("  Address: {}", wallet.address_string());
    println!("  Balance: {} TIME", wallet.balance());
    println!("  Nonce: {}\n", wallet.nonce());

    // Simulate receiving some coins by adding a UTXO
    println!("Simulating receiving 1000 TIME...");
    let utxo = UTXO {
        tx_hash: [1u8; 32],
        output_index: 0,
        amount: 1000_00000000, // 1000 TIME (with 8 decimals)
        address: wallet.address_string(),
    };
    wallet.add_utxo(utxo);

    println!("✓ Received 1000 TIME");
    println!("  New balance: {} TIME\n", wallet.balance() / 100000000);

    // Create another wallet to send to
    println!("Creating recipient wallet...");
    let recipient = Wallet::new(true)?;
    println!("✓ Recipient address: {}\n", recipient.address_string());

    // Create a transaction
    println!("Creating transaction to send 100 TIME...");
    let amount = 100_00000000; // 100 TIME
    let tx = wallet.create_transaction(&recipient.address_string(), amount)?;

    println!("✓ Transaction created!");
    println!("  Transaction hash: {}", hex::encode(tx.hash()));
    println!("  Inputs: {}", tx.inputs.len());
    println!("  Outputs: {}", tx.outputs.len());
    println!("  Total output: {} TIME", tx.total_output() / 100000000);
    println!("  Nonce: {}\n", tx.nonce);

    // Verify the transaction
    println!("Verifying transaction signatures...");
    tx.verify_all()?;
    println!("✓ All signatures valid!\n");

    // Save wallet to file
    let wallet_file = "/tmp/test_wallet.json";
    println!("Saving wallet to file...");
    wallet.save_to_file(wallet_file)?;
    println!("✓ Wallet saved to: {}\n", wallet_file);

    // Load wallet from file
    println!("Loading wallet from file...");
    let loaded_wallet = Wallet::load_from_file(wallet_file)?;
    println!("✓ Wallet loaded!");
    println!("  Address matches: {}\n", 
        loaded_wallet.address_string() == wallet.address_string());

    // Display transaction details
    println!("=== Transaction Details ===");
    println!("From: {}", wallet.address_string());
    println!("To: {}", recipient.address_string());
    println!("Amount: {} TIME", amount / 100000000);
    println!("Fee: 0 TIME (no fees in this example)");
    println!("Change: {} TIME", (tx.total_output() - amount) / 100000000);

    println!("\n✅ All operations completed successfully!");

    Ok(())
}
