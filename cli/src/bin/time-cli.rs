//! TIME Coin CLI - Query blockchain data

use clap::{Parser, Subcommand};
use reqwest;
use serde_json::Value;

#[derive(Parser)]
#[command(name = "time-cli")]
#[command(about = "TIME Coin blockchain query tool", version)]
struct Cli {
    /// API endpoint
    #[arg(short, long, default_value = "http://localhost:24101")]
    api: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Get blockchain information
    Info,
    
    /// List recent blocks
    Blocks {
        /// Number of blocks to show
        #[arg(short, long, default_value = "10")]
        count: usize,
    },
    
    /// Get balance for an address
    Balance {
        /// Address to query
        address: String,
    },
    
    /// List connected peers
    Peers,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let client = reqwest::Client::new();

    match cli.command {
        Commands::Info => {
            let response: Value = client
                .get(&format!("{}/blockchain/info", cli.api))
                .send()
                .await?
                .json()
                .await?;

            println!("\n📊 TIME Coin Blockchain Info");
            println!("═══════════════════════════════════");
            if let Some(height) = response["height"].as_u64() {
                println!("Block Height:    {}", height);
            }
            if let Some(supply) = response["total_supply"].as_u64() {
                println!("Total Supply:    {} TIME", supply / 100_000_000);
            }
            if let Some(network) = response["network"].as_str() {
                println!("Network:         {}", network);
            }
            println!();
        }

        Commands::Blocks { count } => {
            let response: Value = client
                .get(&format!("{}/blockchain/info", cli.api))
                .send()
                .await?
                .json()
                .await?;

            println!("\n📦 Recent Blocks");
            println!("═══════════════════════════════════");
            
            if let Some(height) = response["height"].as_u64() {
                println!("Current Height: {}", height);
                println!("\nShowing last {} blocks:", count.min(height as usize));
                
                // TODO: Need to implement /blocks endpoint in API
                println!("\n⚠️  Note: Block listing endpoint not yet implemented");
                println!("   Current height: {}", height);
            }
            println!();
        }

        Commands::Balance { address } => {
            let response: Value = client
                .get(&format!("{}/balance/{}", cli.api, address))
                .send()
                .await?
                .json()
                .await?;

            println!("\n💰 Balance Query");
            println!("═══════════════════════════════════");
            println!("Address: {}", address);
            
            if let Some(balance) = response["balance"].as_u64() {
                println!("Balance: {} TIME", balance / 100_000_000);
            }
            println!();
        }

        Commands::Peers => {
            let response: Value = client
                .get(&format!("{}/peers", cli.api))
                .send()
                .await?
                .json()
                .await?;

            println!("\n🌐 Connected Peers");
            println!("═══════════════════════════════════");
            
            if let Some(peers) = response["peers"].as_array() {
                println!("Total: {}", peers.len());
                for (i, peer) in peers.iter().enumerate() {
                    if let Some(addr) = peer["address"].as_str() {
                        println!("  {}. {}", i + 1, addr);
                    }
                }
            }
            println!();
        }
    }

    Ok(())
}
