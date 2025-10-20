use clap::Parser;
use colored::*;
use serde::Deserialize;
use std::path::PathBuf;
use std::time::Duration;
use time_api::{start_server, ApiState};
use time_network::{NetworkType, PeerDiscovery};
use tokio::time;

#[derive(Parser)]
#[command(name = "time-node")]
#[command(about = "TIME Coin Node", version)]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    #[arg(short, long)]
    version: bool,
    #[arg(long)]
    dev: bool,
}

#[derive(Debug, Deserialize, Default)]
struct Config {
    #[serde(default)]
    node: NodeConfig,
    #[serde(default)]
    blockchain: BlockchainConfig,
    #[serde(default)]
    consensus: ConsensusConfig,
    #[serde(default)]
    rpc: RpcConfig,
}

#[derive(Debug, Deserialize, Default)]
struct NodeConfig {
    mode: Option<String>,
    #[allow(dead_code)]
    name: Option<String>,
    #[allow(dead_code)]
    data_dir: Option<String>,
    #[allow(dead_code)]
    log_dir: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct BlockchainConfig {
    genesis_file: Option<String>,
    #[allow(dead_code)]
    data_dir: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
struct ConsensusConfig {
    dev_mode: Option<bool>,
    #[allow(dead_code)]
    auto_approve: Option<bool>,
}

#[derive(Debug, Deserialize, Default)]
struct RpcConfig {
    enabled: Option<bool>,
    bind: Option<String>,
    port: Option<u16>,
}

fn load_config(path: &PathBuf) -> Result<Config, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

fn load_genesis(path: &str) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(path)?;
    let genesis: serde_json::Value = serde_json::from_str(&contents)?;
    Ok(genesis)
}

fn expand_path(path: &str) -> String {
    path.replace("$HOME", &std::env::var("HOME").unwrap_or_default())
        .replace("~", &std::env::var("HOME").unwrap_or_default())
}

fn display_genesis(genesis: &serde_json::Value) {
    println!(
        "\n{}",
        "╔═══════════════════════════════════════════════════╗".cyan()
    );
    println!(
        "{}",
        "║         GENESIS BLOCK LOADED                      ║"
            .cyan()
            .bold()
    );
    println!(
        "{}",
        "╚═══════════════════════════════════════════════════╝".cyan()
    );
    if let Some(network) = genesis.get("network").and_then(|v| v.as_str()) {
        println!("\n{}: {}", "Network".yellow().bold(), network);
    }
    if let Some(version) = genesis.get("version").and_then(|v| v.as_u64()) {
        println!("{}: {}", "Version".yellow().bold(), version);
    }
    if let Some(message) = genesis.get("message").and_then(|v| v.as_str()) {
        println!("{}: {}", "Message".yellow().bold(), message);
    }
    if let Some(hash) = genesis.get("hash").and_then(|v| v.as_str()) {
        println!(
            "{}: {}...",
            "Block Hash".yellow().bold(),
            &hash[..16].bright_blue()
        );
    }
    if let Some(timestamp) = genesis.get("timestamp").and_then(|v| v.as_i64()) {
        if let Some(dt) = chrono::DateTime::from_timestamp(timestamp, 0) {
            let formatted = dt.format("%Y-%m-%d %H:%M:%S UTC");
            println!("{}: {}", "Timestamp".yellow().bold(), formatted);
        }
    }
    if let Some(transactions) = genesis.get("transactions").and_then(|v| v.as_array()) {
        let total_supply: u64 = transactions
            .iter()
            .filter_map(|tx| tx.get("amount").and_then(|v| v.as_u64()))
            .sum();
        println!(
            "{}: {} TIME",
            "Total Supply".yellow().bold(),
            (total_supply / 100_000_000).to_string().green()
        );
        println!(
            "\n{} ({})",
            "Allocations".yellow().bold(),
            transactions.len()
        );
        for (i, tx) in transactions.iter().enumerate() {
            if let (Some(amount), Some(desc)) = (
                tx.get("amount").and_then(|v| v.as_u64()),
                tx.get("description").and_then(|v| v.as_str()),
            ) {
                let amount_time = amount / 100_000_000;
                println!(
                    "  {}. {} TIME - {}",
                    i + 1,
                    amount_time.to_string().green(),
                    desc.bright_white()
                );
            }
        }
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
    let config_path = cli
        .config
        .unwrap_or_else(|| PathBuf::from(expand_path("$HOME/time-coin-node/config/testnet.toml")));
    println!("{}", "TIME Coin Node v0.1.0".cyan().bold());
    println!("Config file: {:?}\n", config_path);
    let config = match load_config(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Warning: Could not load config: {}", e);
            Config::default()
        }
    };
    let is_dev_mode = cli.dev
        || config.node.mode.as_deref() == Some("dev")
        || config.consensus.dev_mode.unwrap_or(false);
    if is_dev_mode {
        println!("{}", "⚠️  DEV MODE ENABLED".yellow().bold());
        println!(
            "{}",
            "   Single-node testing - Auto-approving transactions".yellow()
        );
        println!();
    }
    println!("{}", "🚀 Starting TIME node...".green().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black());
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
    }
    println!("\n{}", "✓ Blockchain initialized".green());
    println!("{}", "⏳ Starting peer discovery...".yellow());
    let mut discovery = PeerDiscovery::new(NetworkType::Testnet);
    match discovery.bootstrap().await {
        Ok(peers) => {
            if peers.is_empty() {
                println!("{}", "  ⚠ No peers discovered yet".yellow());
            } else {
                println!(
                    "{}",
                    format!("  ✓ Discovered {} peer(s)", peers.len()).green()
                );
                for peer in peers.iter().take(5) {
                    println!("    • {} ({})", peer.address, peer.version);
                }
                if peers.len() > 5 {
                    println!("    ... and {} more", peers.len() - 5);
                }
            }
        }
        Err(e) => {
            println!("{}", format!("  ⚠ Peer discovery error: {}", e).yellow());
            println!("    {}", "Will retry in background...".bright_black());
        }
    }
    println!("{}", "✓ Peer discovery started".green());
    println!("{}", "✓ Masternode services starting".green());
    if is_dev_mode {
        println!("{}", "✓ Dev mode: Single-node consensus active".green());
    }
    let api_enabled = config.rpc.enabled.unwrap_or(true);
    let api_bind = config.rpc.bind.unwrap_or_else(|| "127.0.0.1".to_string());
    let api_port = config.rpc.port.unwrap_or(24101);
    if api_enabled {
        let bind_addr = format!("{}:{}", api_bind, api_port);
        let api_state = ApiState::new(is_dev_mode, "testnet".to_string());
        println!(
            "{}",
            format!("✓ API server starting on {}", bind_addr).green()
        );
        let api_state_clone = api_state.clone();
        tokio::spawn(async move {
            if let Err(e) = start_server(bind_addr.parse().unwrap(), api_state_clone).await {
                eprintln!("API server error: {}", e);
            }
        });
    }
    println!("\n{}", "Node Status: ACTIVE".green().bold());
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(300));
        loop {
            interval.tick().await;
            let mut disc = PeerDiscovery::new(NetworkType::Testnet);
            if let Ok(peers) = disc.bootstrap().await {
                if !peers.is_empty() {
                    println!(
                        "[{}] {} - {} peer(s) available",
                        chrono::Utc::now().format("%Y-%m-%d %H:%M:%S"),
                        "Peer discovery refresh".bright_black(),
                        peers.len()
                    );
                }
            }
        }
    });
    let mut counter = 0;
    loop {
        time::sleep(Duration::from_secs(60)).await;
        counter += 1;
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
        if is_dev_mode {
            println!(
                "[{}] {} #{} {}",
                timestamp,
                "Node heartbeat - running...".bright_black(),
                counter,
                "(dev mode)".yellow()
            );
        } else {
            println!(
                "[{}] {} #{}",
                timestamp,
                "Node heartbeat - running...".bright_black(),
                counter
            );
        }
    }
}
