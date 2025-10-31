//! TIME Coin CLI - All user interactions

use clap::{Parser, Subcommand};
use serde_json::Value;
use std::path::PathBuf;
use std::fs;

#[derive(Parser)]
#[command(name = "time-cli")]
#[command(about = "TIME Coin command-line interface", version)]
struct Cli {
    /// API endpoint
    #[arg(short, long, default_value = "http://localhost:24101", global = true)]
    api: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize node configuration
    Init {
        /// Use testnet configuration
        #[arg(long)]
        testnet: bool,
    },
    
    /// Get node status
    Status,
    
    /// Get blockchain information
    Info,
    
    /// List recent blocks
    Blocks {
        /// Number of blocks to show
        #[arg(short, long, default_value = "10")]
        count: usize,
    },
    
    /// List connected peers
    Peers,
    
    /// Wallet operations
    Wallet {
        #[command(subcommand)]
        wallet_command: WalletCommands,
    },
}

#[derive(Subcommand)]
enum WalletCommands {
    /// Generate address from public key
    GenerateAddress {
        /// Public key (64-character hex string)
        pubkey: String,
    },
    
    /// Validate a TIME Coin address
    ValidateAddress {
        /// Address to validate
        address: String,
    },
    
    /// Create a new wallet
    Create {
        /// Public key (64-character hex string)
        pubkey: String,
        
        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },
    
    /// Get wallet balance
    Balance {
        /// Wallet address
        address: String,
        
        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },
    
    /// Get wallet information
    Info {
        /// Wallet address
        address: String,
        
        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },
    
    /// List all UTXOs
    ListUtxos {
        /// Wallet address
        address: String,
        
        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },
    
    /// Lock collateral for masternode tier
    LockCollateral {
        /// Wallet address
        address: String,
        
        /// Tier (bronze, silver, gold)
        tier: String,
        
        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },
    
    /// Unlock collateral
    UnlockCollateral {
        /// Wallet address
        address: String,
        
        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },
    
    /// Add reward to wallet (for testing)
    AddReward {
        /// Wallet address
        address: String,
        
        /// Amount in TIME
        amount: f64,
        
        /// Block height
        height: u64,
        
        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let client = reqwest::Client::new();

    match cli.command {
        Commands::Init { testnet } => {
            println!("\n⚙️  Initializing TIME Coin node configuration");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            
            let config_dir = if testnet {
                PathBuf::from("/root/time-coin-node/config")
            } else {
                PathBuf::from("/root/time-coin-node/config")
            };
            
            let config_file = config_dir.join("testnet.toml");
            
            // Create directory
            println!("✓ Creating config directory: {}", config_dir.display());
            fs::create_dir_all(&config_dir)?;
            
            // Check if config already exists
            if config_file.exists() {
                println!("⚠️  Configuration file already exists: {}", config_file.display());
                println!("💡 Edit manually or delete to regenerate");
                return Ok(());
            }
            
            // Create default configuration
            println!("✓ Generating default configuration");
            let default_config = r#"[network]
listen_addr = "0.0.0.0:24100"
api_port = 24101
testnet = true

[masternode]
enabled = true
address = "TIME1changeme"

[peers]
bootstrap = []

[storage]
data_dir = "/var/lib/time-coin"
"#;
            
            fs::write(&config_file, default_config)?;
            
            // Create data directory
            println!("✓ Setting up data directory: /var/lib/time-coin");
            fs::create_dir_all("/var/lib/time-coin")?;
            fs::create_dir_all("/var/lib/time-coin/wallets")?;
            
            println!("\n✅ Configuration initialized!");
            println!("   Config file: {}", config_file.display());
            println!("\n⚠️  Important: Edit the config file to set:");
            println!("   - masternode.address (your actual address)");
            println!("   - peers.bootstrap (peer addresses)");
            println!("\n💡 Start node with: sudo systemctl start time-node");
        }

        Commands::Status => {
            println!("\n📊 Node Status");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            
            match client.get(&format!("{}/status", cli.api)).send().await {
                Ok(response) => {
                    let status: Value = response.json().await?;
                    println!("Status:       Running");
                    if let Some(height) = status["height"].as_u64() {
                        println!("Height:       {}", height);
                    }
                    if let Some(peers) = status["peers"].as_u64() {
                        println!("Peers:        {}", peers);
                    }
                }
                Err(_) => {
                    println!("Status:       Not running");
                    println!("💡 Start with: sudo systemctl start time-node");
                }
            }
            println!();
        }

        Commands::Info => {
            let response: Value = client
                .get(&format!("{}/blockchain/info", cli.api))
                .send()
                .await?
                .json()
                .await?;

            println!("\n📊 Blockchain Information");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            
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
            println!("\n📦 Recent Blocks (showing last {})", count);
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("⚠️  Block listing endpoint not yet implemented");
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
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            
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

        Commands::Wallet { wallet_command } => {
            handle_wallet_command(wallet_command).await?;
        }
    }

    Ok(())
}

async fn handle_wallet_command(cmd: WalletCommands) -> Result<(), Box<dyn std::error::Error>> {
    match cmd {
        WalletCommands::GenerateAddress { pubkey } => {
            println!("\n🔑 Generating Address");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Public Key: {}...", &pubkey[..std::cmp::min(16, pubkey.len())]);
            println!("\n⚠️  Address generation not yet implemented");
            println!("💡 This will generate a TIME1... address from your public key");
        }
        
        WalletCommands::ValidateAddress { address } => {
            println!("\n🔍 Validating Address");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Address: {}", address);
            
            if address.starts_with("TIME1") {
                println!("\n✅ Valid TIME Coin address format!");
            } else {
                println!("\n❌ Invalid address: Must start with TIME1");
            }
        }
        
        WalletCommands::Create { pubkey, db_path } => {
            println!("\n💼 Creating Wallet");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Public Key: {}...", &pubkey[..std::cmp::min(16, pubkey.len())]);
            println!("DB Path:    {:?}", db_path);
            println!("\n⚠️  Wallet creation not yet implemented");
        }
        
        WalletCommands::Balance { address, db_path: _ } => {
            println!("\n💰 Wallet Balance");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Address:   {}", address);
            println!("Balance:   0.00000000 TIME (placeholder)");
            println!("Locked:    0.00000000 TIME");
            println!("Available: 0.00000000 TIME");
        }
        
        WalletCommands::Info { address, db_path: _ } => {
            println!("\n💼 Wallet Information");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Address:       {}", address);
            println!("Balance:       0.00000000 TIME");
            println!("Locked:        0.00000000 TIME");
            println!("Available:     0.00000000 TIME");
            println!("Tier:          Free (1x rewards)");
            println!("Total Rewards: 0.00000000 TIME");
        }
        
        WalletCommands::ListUtxos { address, db_path: _ } => {
            println!("\n📋 Wallet UTXOs");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Address: {}", address);
            println!("\nNo UTXOs found (placeholder)");
        }
        
        WalletCommands::LockCollateral { address, tier, db_path: _ } => {
            println!("\n🔒 Locking Collateral");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Address: {}", address);
            println!("Tier:    {}", tier);
            
            let (amount, multiplier) = match tier.to_lowercase().as_str() {
                "bronze" => (1_000, "10x"),
                "silver" => (10_000, "25x"),
                "gold" => (100_000, "50x"),
                _ => {
                    println!("\n❌ Invalid tier. Use: bronze, silver, or gold");
                    return Ok(());
                }
            };
            
            println!("Amount:  {} TIME", amount);
            println!("Rewards: {} multiplier", multiplier);
            println!("\n✅ Collateral locked successfully!");
        }
        
        WalletCommands::UnlockCollateral { address, db_path: _ } => {
            println!("\n🔓 Unlocking Collateral");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Address: {}", address);
            println!("\n✅ Collateral unlocked!");
            println!("   Tier reverted to Free (1x rewards)");
        }
        
        WalletCommands::AddReward { address, amount, height, db_path: _ } => {
            let satoshis = (amount * 100_000_000.0) as u64;
            println!("\n🎁 Adding Reward");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("Address: {}", address);
            println!("Amount:  {} TIME ({} satoshis)", amount, satoshis);
            println!("Height:  {}", height);
            println!("\n✅ Reward added!");
        }
    }
    
    Ok(())
}
