//! Example of using the simple_client for wallet operations
//! 
//! This shows how to use the new SimpleClient without any blocking or mutexes

use wallet::NetworkType;

#[tokio::main]
async fn main() {
    // Note: This would be in the wallet-gui crate, but shown here for demonstration
    use wallet_gui::simple_client::SimpleClient;
    
    // Create client pointing to a masternode
    let client = SimpleClient::new(
        "134.199.175.106:24100".to_string(),
        NetworkType::Testnet,
    );
    
    // Register xpub for notifications
    let xpub = "xpub6CZosSrjGTSQi9Hb...";
    match client.register_xpub(xpub).await {
        Ok(_) => println!("✅ Registered xpub with masternode"),
        Err(e) => println!("❌ Registration failed: {}", e),
    }
    
    // Get transaction history
    match client.get_transactions(xpub).await {
        Ok(transactions) => {
            println!("📋 Found {} transactions", transactions.len());
            
            for tx in &transactions {
                println!("  {} → {} : {} TIME", 
                    &tx.from_address[..20],
                    &tx.to_address[..20],
                    tx.amount as f64 / 1e8
                );
            }
        }
        Err(e) => println!("❌ Failed to get transactions: {}", e),
    }
    
    println!("\n✨ No blocking, no mutexes, no timeouts - just simple async!");
}
