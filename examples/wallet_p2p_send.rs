//! Example: Wallet sending transaction via P2P network
//!
//! Demonstrates the CORRECT way for wallets to communicate with masternodes:
//! - Direct TCP P2P connection (not HTTP)
//! - Efficient binary protocol
//! - Real-time notifications
//! - Proper blockchain architecture

use std::net::SocketAddr;
use wallet::WalletP2PClient;
use time_core::{Transaction, TxInput, TxOutput, OutPoint};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔗 TIME Coin Wallet P2P Example\n");

    // Step 1: Connect to masternode via P2P network (port 24000)
    let masternode_addr: SocketAddr = "127.0.0.1:24000".parse()?;
    
    println!("📡 Connecting to masternode at {}...", masternode_addr);
    let client = WalletP2PClient::connect(masternode_addr).await?;
    
    // Step 2: Subscribe to address notifications
    println!("\n📬 Subscribing to address notifications...");
    client.subscribe_to_addresses(
        vec!["time1_your_address_here".to_string()],
        "wallet_123".to_string(),
    ).await?;

    // Step 3: Create a transaction
    println!("\n💸 Creating transaction...");
    let tx = Transaction {
        txid: format!("tx_{}", chrono::Utc::now().timestamp()),
        inputs: vec![TxInput {
            previous_output: OutPoint {
                txid: "previous_tx_abc123".to_string(),
                vout: 0,
            },
            script_sig: vec![],
            sequence: 0xFFFFFFFF,
        }],
        outputs: vec![
            TxOutput {
                amount: 50_000_000, // 0.5 TIME
                address: "time1_recipient".to_string(),
                script_pubkey: vec![],
            },
            TxOutput {
                amount: 49_000_000, // 0.49 TIME (change)
                address: "time1_sender".to_string(),
                script_pubkey: vec![],
            },
        ],
        timestamp: chrono::Utc::now().timestamp(),
        signature: vec![],
        nonce: 0,
        public_key: vec![],
        fee: 1_000_000, // 0.01 TIME
    };

    // Step 4: Send transaction via P2P (NOT HTTP!)
    println!("\n🚀 Sending transaction via P2P network...");
    client.send_transaction(tx).await?;

    println!("\n✅ Transaction sent successfully!");
    println!("\nWhat happens next:");
    println!("  1. Masternode validates transaction");
    println!("  2. Masternode locks UTXOs (prevents double-spend)");
    println!("  3. Masternode broadcasts to other masternodes via P2P");
    println!("  4. Masternodes vote on transaction");
    println!("  5. 2/3+1 votes → Instant Finality (< 1 second)");
    println!("  6. Transaction included in next block");
    println!("  7. You receive real-time notifications via P2P");

    // Step 5: Listen for notifications (optional)
    println!("\n👂 Listening for notifications...");
    println!("   (Press Ctrl+C to stop)\n");

    client.receive_loop(|message| {
        println!("📨 Received: {:?}", message);
    }).await?;

    Ok(())
}
