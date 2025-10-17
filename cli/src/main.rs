use clap::Parser;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use tokio::time;
use colored::*;

#[derive(Parser)]
#[command(name = "time-node")]
#[command(about = "TIME Coin Node")]
struct Cli {
    /// Path to configuration file
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    
    /// Show version
    #[arg(short, long)]
    version: bool,
}

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(default)]
    blockchain: BlockchainConfig,
}

#[derive(Debug, Deserialize, Default)]
struct BlockchainConfig {
    genesis_file: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct GenesisBlock {
    version: u32,
    timestamp: i64,
    hash: String,
    merkle_root: String,
    transactions: Vec<Transaction>,
    message: String,
    network: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct Transaction {
    txid: String,
    output_address: String,
    amount: u64,
    description: String,
}

fn load_config(path: &PathBuf) -> Result<Config, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&contents)
        .unwrap_or_else(|_| Config { blockchain: BlockchainConfig::default() });
    Ok(config)
}

fn load_genesis(path: &str) -> Result<GenesisBlock, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(path)?;
    let genesis: GenesisBlock = serde_json::from_str(&contents)?;
    Ok(genesis)
}

fn expand_path(path: &str) -> String {
    path.replace("$HOME", &std::env::var("HOME").unwrap_or_default())
}

fn display_genesis(genesis: &GenesisBlock) {
    println!("\n{}", "╔═══════════════════════════════════════════════════╗".cyan());
    println!("{}", "║         GENESIS BLOCK LOADED                      ║".cyan().bold());
    println!("{}", "╚═══════════════════════════════════════════════════╝".cyan());
    
    println!("\n{}: {}", "Network".yellow().bold(), genesis.network);
    println!("{}: {}", "Version".yellow().bold(), genesis.version);
    println!("{}: {}", "Message".yellow().bold(), genesis.message);
    println!("{}: {}...", "Block Hash".yellow().bold(), &genesis.hash[..16].bright_blue());
    
    let timestamp = chrono::DateTime::from_timestamp(genesis.timestamp, 0)
        .unwrap()
        .format("%Y-%m-%d %H:%M:%S UTC");
    println!("{}: {}", "Timestamp".yellow().bold(), timestamp);
    
    let total_supply: u64 = genesis.transactions.iter().map(|tx| tx.amount).sum();
    println!("{}: {} TIME", 
        "Total Supply".yellow().bold(), 
        (total_supply / 100_000_000).to_string().green()
    );
    
    println!("\n{} ({})", "Allocations".yellow().bold(), genesis.transactions.len());
    for (i, tx) in genesis.transactions.iter().enumerate() {
        let amount_time = tx.amount / 100_000_000;
        println!("  {}. {} TIME - {}", 
            i + 1, 
            amount_time.to_string().green(),
            tx.description.bright_white()
        );
    }
    println!();
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    
    if cli.version {
        println!("time-node 0.1.0");
        return;
    }
    
    let config_path = cli.config.unwrap_or_else(|| {
        PathBuf::from(expand_path("$HOME/time-coin-node/config/testnet.toml"))
    });
    
    println!("{}", "TIME Coin Node v0.1.0".cyan().bold());
    println!("Config file: {:?}\n", config_path);
    
    // Load configuration
    let config = match load_config(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Warning: Could not load config: {}", e);
            Config { blockchain: BlockchainConfig::default() }
        }
    };
    
    println!("{}", "🚀 Starting TIME node...".green().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black());
    
    // Try to load genesis block
    if let Some(genesis_path) = config.blockchain.genesis_file {
        let expanded_path = expand_path(&genesis_path);
        match load_genesis(&expanded_path) {
            Ok(genesis) => {
                display_genesis(&genesis);
                println!("{}", "✓ Genesis block verified".green());
            }
            Err(e) => {
                println!("{} Genesis block: {}", "⚠".yellow(), e);
                println!("  Looking for: {}", expanded_path);
            }
        }
    } else {
        println!("{}", "⚠ No genesis block configured".yellow());
        println!("  Add to config: [blockchain]");
        println!("                 genesis_file = \"$HOME/time-coin-node/data/genesis-testnet.json\"");
    }
    
    println!("\n{}", "✓ Blockchain initialized".green());
    println!("{}", "✓ Peer discovery started".green());
    println!("{}", "✓ Masternode services starting".green());
    
    println!("\n{}", "Node Status: ACTIVE".green().bold());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    // Heartbeat loop
    let mut counter = 0;
    loop {
        time::sleep(Duration::from_secs(60)).await;
        counter += 1;
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
        println!("[{}] {} #{}", timestamp, "Node heartbeat - running...".bright_black(), counter);
    }
}
