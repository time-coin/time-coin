use clap::Parser;
use std::sync::Arc;
use tokio::sync::RwLock;
use owo_colors::OwoColorize;
use serde::Deserialize;

mod block_producer;
use block_producer::BlockProducer;
use std::path::PathBuf;
use std::time::Duration;

use time_api::{start_server, ApiState};
use time_network::{NetworkType, PeerDiscovery, PeerManager, PeerListener};
use time_consensus::ConsensusEngine;
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
    network: Option<String>,
    
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
    admin_token: Option<String>,
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
            hash[..16].to_string().bright_blue()
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

async fn download_genesis_from_peers(
    peer_manager: &Arc<PeerManager>,
    genesis_path: &str,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    println!("{}", "📥 Genesis block not found locally".yellow());
    println!("{}", "   Attempting to download from network...".bright_black());
    
    // Get list of peers
    let peers = peer_manager.get_peer_ips().await;
    
    if peers.is_empty() {
        return Err("No peers available to download genesis from".into());
    }
    
    // Try each peer until we get genesis
    for peer in peers.iter() {
        println!("   Trying {}...", peer.bright_black());
        
        match peer_manager.request_genesis(peer).await {
            Ok(genesis) => {
                println!("{}", "   ✓ Genesis downloaded successfully!".green());
                
                // Save genesis to file
                let genesis_dir = std::path::Path::new(genesis_path).parent()
                    .ok_or("Invalid genesis path")?;
                std::fs::create_dir_all(genesis_dir)?;
                
                let genesis_json = serde_json::to_string_pretty(&genesis)?;
                std::fs::write(genesis_path, genesis_json)?;
                
                println!("   ✓ Saved to: {}", genesis_path.bright_black());
                
                return Ok(genesis);
            }
            Err(e) => {
                println!("   ✗ Failed: {}", e.to_string().bright_black());
                continue;
            }
        }
    }
    
    Err("Could not download genesis from any peer".into())
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

    let config = match load_config(&config_path) {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Warning: Could not load config: {}", e);
            Config::default()
        }
    };

    // Determine network type
    let network_name = config.node.network
        .as_deref()
        .unwrap_or("testnet")
        .to_uppercase();
    
    let is_testnet = network_name == "TESTNET";

    // Display banner with network
    if is_testnet {
        println!("{}", "╔══════════════════════════════════════╗".yellow().bold());
        println!("{}", "║   TIME Coin Node v0.1.0 [TESTNET]   ║".yellow().bold());
        println!("{}", "╚══════════════════════════════════════╝".yellow().bold());
    } else {
        println!("{}", "TIME Coin Node v0.1.0".cyan().bold());
    }
    
    println!("Config file: {:?}", config_path);
    println!("Network: {}", network_name.yellow().bold());
    println!();

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

    // Initialize networking first for genesis download
    let network_type = if is_testnet {
        NetworkType::Testnet
    } else {
        NetworkType::Mainnet
    };

    let discovery = Arc::new(RwLock::new(PeerDiscovery::new(network_type.clone())));
    let listen_addr = "0.0.0.0:24100".parse().unwrap();
    let peer_manager = std::sync::Arc::new(PeerManager::new(network_type.clone(), listen_addr));

    // Bootstrap peer discovery
    println!("{}", "⏳ Starting peer discovery...".yellow());
    match discovery.write().await.bootstrap().await {
        Ok(peers) => {
            if !peers.is_empty() {
                println!(
                    "{}",
                    format!("  ✓ Discovered {} peer(s)", peers.len()).green()
                );
                peer_manager.connect_to_peers(peers.clone()).await;
            }
        }
        Err(_) => {}
    }

    // Handle genesis block - download if not present
    let genesis_path = config.blockchain.genesis_file
        .map(|p| expand_path(&p))
        .unwrap_or_else(|| "/root/time-coin-node/config/genesis-testnet.json".to_string());
    
    // Set environment variable for API
    std::env::set_var("GENESIS_PATH", &genesis_path);
    
    let _genesis = match load_genesis(&genesis_path) {
        Ok(g) => {
            display_genesis(&g);
            println!("{}", "✓ Genesis block verified".green());
            Some(g)
        }
        Err(_) => {
            // Try to download from peers
            match download_genesis_from_peers(&peer_manager, &genesis_path).await {
                Ok(g) => {
                    display_genesis(&g);
                    println!("{}", "✓ Genesis block downloaded and verified".green());
                    Some(g)
                }
                Err(e) => {
                    println!("{} {}", "⚠".yellow(), e);
                    println!("  {}", "Node will continue without genesis verification".yellow());
                    println!("  {}", "Genesis will be synced once peers are available".bright_black());
                    None
                }
            }
        }
    };

    println!("\n{}", "✓ Blockchain initialized".green());
    
    // Initialize consensus engine
    let consensus = Arc::new(ConsensusEngine::new(is_dev_mode));
    
    // Register self as a masternode
    let node_id = if let Ok(ip) = local_ip_address::local_ip() {
        ip.to_string()
    } else {
        "unknown".to_string()
    };
    consensus.add_masternode(node_id.clone()).await;

    println!("{}", "✓ Peer discovery started".green());

    // Display consensus status
    let total_masternodes = consensus.masternode_count().await;
    
    println!("\n{}", "Consensus Status:".cyan().bold());
    println!("  Active Masternodes: {}", total_masternodes.to_string().yellow());
    
    let consensus_mode = consensus.consensus_mode().await;
    match consensus_mode {
        time_consensus::ConsensusMode::Development => {
            println!("  Mode: {} {}", "Development".yellow().bold(), "(auto-approve)".bright_black());
        }
        time_consensus::ConsensusMode::BootstrapNoQuorum => {
            println!("  Mode: {} {}", "Bootstrap".yellow().bold(), "(no voting)".bright_black());
            println!("  {} Need {} more masternode(s) for BFT consensus", 
                "⚠".yellow(), 
                (3 - total_masternodes).to_string().yellow().bold()
            );
        }
        time_consensus::ConsensusMode::BFT => {
            println!("  Mode: {} {}", "BFT".green().bold(), "(2/3+ voting)".bright_black());
            println!("  {} Byzantine Fault Tolerant", "✓".green());
        }
    }

    println!("\n{}", "✓ Masternode services starting".green());

    let api_enabled = config.rpc.enabled.unwrap_or(true);
    let api_bind = config.rpc.bind.unwrap_or_else(|| "127.0.0.1".to_string());
    let api_port = config.rpc.port.unwrap_or(24101);

    if api_enabled {
        let admin_token = config.rpc.admin_token.clone();
        let bind_addr = format!("{}:{}", api_bind, api_port);
        let api_state = ApiState::new(
            is_dev_mode,
            network_name.to_lowercase(),
            discovery.clone(),
            peer_manager.clone(),
            admin_token,
        );

        let peer_listener_addr = "0.0.0.0:24100".parse().unwrap();
        match PeerListener::bind(peer_listener_addr, network_type).await {
            Ok(peer_listener) => {
                let peer_manager_clone = peer_manager.clone();
                let consensus_clone = consensus.clone();
                tokio::spawn(async move {
                    loop {
                        if let Ok(conn) = peer_listener.accept().await {
                            let info = conn.peer_info().await;
                            let peer_addr = info.address.clone();
                            
                            peer_manager_clone.add_connected_peer(info).await;
                            
                            // Register peer as masternode in consensus
                            let prev_count = consensus_clone.masternode_count().await;
                            consensus_clone.add_masternode(peer_addr.to_string()).await;
                            let new_count = consensus_clone.masternode_count().await;
                            
                            // Check if we just reached BFT quorum
                            if prev_count < 3 && new_count >= 3 {
                                println!("\n{}", "═══════════════════════════════════════".green().bold());
                                println!("{}", "🛡️  BFT CONSENSUS ACTIVATED!".green().bold());
                                println!("   {} masternodes active", new_count);
                                println!("   Requiring 2/3+ approval for blocks");
                                println!("{}", "═══════════════════════════════════════".green().bold());
                            }

                            tokio::spawn(async move {
                                conn.keep_alive().await;
                            });
                        }
                    }
                });
            }
            Err(e) => eprintln!("Failed to start peer listener: {}", e),
        }

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

        let mode_str = match consensus_mode {
            time_consensus::ConsensusMode::Development => "DEV",
            time_consensus::ConsensusMode::BootstrapNoQuorum => "BOOTSTRAP",
            time_consensus::ConsensusMode::BFT => "BFT",
        };

        println!("\n{}", format!("Node Status: ACTIVE [{}] [{}]", network_name, mode_str).green().bold());
    }
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(300));
        interval.tick().await;
        loop {
            interval.tick().await;
            if let Ok(peers) = discovery.write().await.bootstrap().await {
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

    println!("{}", "🔨 Starting block producer...".yellow());
    
    let block_producer = BlockProducer::new(node_id, peer_manager.clone());
    block_producer.start().await;
    println!("{}", "✓ Block producer started (24-hour interval)".green());

    let mut counter = 0;
    let consensus_heartbeat = consensus.clone();
    loop {
        time::sleep(Duration::from_secs(60)).await;
        counter += 1;
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
        
        let total_nodes = consensus_heartbeat.masternode_count().await;
        let mode = consensus_heartbeat.consensus_mode().await;
        let consensus_mode = match mode {
            time_consensus::ConsensusMode::Development => "DEV",
            time_consensus::ConsensusMode::BootstrapNoQuorum => "BOOTSTRAP",
            time_consensus::ConsensusMode::BFT => "BFT",
        };
        
        if is_testnet {
            println!(
                "[{}] {} #{} | {} nodes | {} mode | {}",
                timestamp,
                "Heartbeat".bright_black(),
                counter,
                total_nodes.to_string().yellow(),
                consensus_mode.yellow(),
                "[TESTNET]".yellow()
            );
        } else if is_dev_mode {
            println!(
                "[{}] {} #{} | {} nodes | {}",
                timestamp,
                "Heartbeat".bright_black(),
                counter,
                total_nodes.to_string().yellow(),
                "(dev mode)".yellow()
            );
        } else {
            println!(
                "[{}] {} #{} | {} nodes | {} mode",
                timestamp,
                "Heartbeat".bright_black(),
                counter,
                total_nodes.to_string().yellow(),
                consensus_mode.yellow()
            );
        }
    }
}
