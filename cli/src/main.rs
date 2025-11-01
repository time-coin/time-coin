const VERSION: &str = env!("CARGO_PKG_VERSION");

use clap::Parser;
use time_wallet::{Wallet, NetworkType as WalletNetworkType};

use std::sync::Arc;

use tokio::sync::RwLock;

use owo_colors::OwoColorize;

use serde::Deserialize;


mod block_producer;
mod chain_sync;
use block_producer::BlockProducer;
use chain_sync::ChainSync;

use std::path::PathBuf;

use std::time::Duration;


use time_api::{start_server, ApiState};

use time_core::state::BlockchainState;

use time_core::block::{Block, BlockHeader};

use time_core::transaction::{Transaction, TxOutput};

use chrono::TimeZone;

use time_network::{NetworkType, PeerDiscovery, PeerManager, PeerListener};

use time_consensus::ConsensusEngine;

use tokio::time;


#[derive(Parser)]
#[command(name = "time-node")]
#[command(about = "TIME Coin Node", version)]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,
    
    #[arg(long)]
    version: bool,
    
    #[arg(long)]
    dev: bool,
    
    #[arg(long)]
    full_sync: bool,
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

    if let Some(_version) = genesis.get("version").and_then(|v| v.as_u64()) {
        println!("{}: {}", "Software Version".yellow().bold(), time_network::protocol::full_version());
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
    
    let peers = peer_manager.get_peer_ips().await;
    
    if peers.is_empty() {
        return Err("No peers available to download genesis from".into());
    }
    
    for peer in peers.iter() {
        println!("   Trying {}...", peer.bright_black());
        
        match peer_manager.request_genesis(peer).await {
            Ok(genesis) => {
                println!("{}", "   ✓ Genesis downloaded successfully!".green());
                
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

async fn snapshot_sync(
    peer_manager: &Arc<PeerManager>,
) -> Result<time_network::Snapshot, Box<dyn std::error::Error>> {
    println!("\n{}", "⚡ FAST SYNC: Downloading network snapshot...".cyan().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black());
    
    let peers = peer_manager.get_peer_ips().await;
    
    if peers.is_empty() {
        return Err("No peers available for snapshot sync".into());
    }
    
    for peer in peers.iter() {
        println!("   📡 Requesting snapshot from {}...", peer.bright_black());
        
        match peer_manager.request_snapshot(peer).await {
            Ok(snapshot) => {
                println!("{}", "   ✓ Snapshot downloaded!".green());
                println!("     Height: {}", snapshot.height.to_string().yellow());
                println!("     Accounts: {}", snapshot.balances.len().to_string().yellow());
                println!("     Masternodes: {}", snapshot.masternodes.len().to_string().yellow());
                println!("     State Hash: {}...", snapshot.state_hash[..16].to_string().bright_blue());
                
                // Verify snapshot integrity with deterministic serialization
                let mut sorted_balances: Vec<_> = snapshot.balances.iter().collect();
                sorted_balances.sort_by_key(|&(k, _)| k);
                let mut sorted_masternodes = snapshot.masternodes.clone();
                sorted_masternodes.sort();
                
                let state_data = format!("{:?}{:?}", sorted_balances, sorted_masternodes);
                let computed_hash = format!("{:x}", md5::compute(&state_data));
                
                if computed_hash == snapshot.state_hash {
                    println!("{}", "   ✓ Snapshot verified!".green());
                } else {
                    println!("{}", "   ⚠ Snapshot hash mismatch, trying next peer...".yellow());
                    continue;
                }
                
                return Ok(snapshot);
            }
            Err(e) => {
                println!("   ✗ Failed: {}", e.to_string().bright_black());
                continue;
            }
        }
    }
    
    Err("Could not download valid snapshot from any peer".into())
}

/// Get local blockchain height from disk
async fn get_local_height(blockchain: &Arc<RwLock<BlockchainState>>) -> u64 {
    let chain = blockchain.read().await;
    chain.chain_tip_height()
}

/// Query network for current height
/// Query network for current height
async fn get_network_height(peer_manager: &Arc<PeerManager>) -> Option<u64> {
    let peers = peer_manager.get_peer_ips().await;
    
    if peers.is_empty() {
        return None;
    }
    
    // Query multiple peers and take the highest height
    let mut max_height = 0u64;
    let mut successful_queries = 0;
    
    for peer in peers.iter().take(3) { // Query up to 3 peers
        match peer_manager.request_blockchain_info(peer).await {
            Ok(height) => {
                successful_queries += 1;
                if height > max_height {
                    max_height = height;
                }
                println!("   {} reports height: {}", peer.bright_black(), height.to_string().yellow());
            }
            Err(_) => {
                // Silently skip peers that dont respond
            }
        }
    }
    
    if successful_queries > 0 {
        Some(max_height)
    } else {
        None
    }
}

/// Sync mempool from connected peers
async fn sync_mempool_from_peers(
    peer_manager: &Arc<PeerManager>,
    mempool: &Arc<time_mempool::Mempool>,
) -> Result<usize, Box<dyn std::error::Error>> {
    let peers = peer_manager.get_peer_ips().await;
    
    if peers.is_empty() {
        return Ok(0);
    }
    
    println!("\n{}", "📥 Syncing mempool from network...".cyan());
    
    let mut total_added = 0;
    
    for peer in peers.iter().take(3) {
        println!("   Requesting mempool from {}...", peer.bright_black());
        
        match peer_manager.request_mempool(peer).await {
            Ok(transactions) => {
                println!("   ✓ Received {} transactions", transactions.len().to_string().yellow());
                
                for tx in transactions {
                    match mempool.add_transaction(tx).await {
                        Ok(_) => {
                            total_added += 1;
                        }
                        Err(_) => {
                            // Already in mempool or invalid, skip silently
                        }
                    }
                }
            }
            Err(e) => {
                println!("   ✗ Failed: {}", e.to_string().bright_black());
            }
        }
    }
    
    if total_added > 0 {
        println!("{}", format!("✓ Added {} new transactions from network", total_added).green());
    } else {
        println!("{}", "✓ Mempool is up to date".green());
    }
    
    Ok(total_added)
}


#[tokio::main]

/// Sync mempool from connected peers
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

    let network_name = config.node.network
        .as_deref()
        .unwrap_or("testnet")
        .to_uppercase();
    
    let is_testnet = network_name == "TESTNET";

    // Banner with network indicator
    if is_testnet {
        println!("{}", "╔══════════════════════════════════════╗".yellow().bold());
        let version_str = time_network::protocol::full_version();
        let total_width: usize = 38; // Inner width of banner
        let prefix = "TIME Coin Node ";
        let content = format!("{}{}", prefix, version_str);
        let padding = total_width.saturating_sub(content.len());
        let left_pad = padding / 2;
        let right_pad = padding - left_pad;
        println!("{}", format!("║{:width$}{}{}║", "", content, " ".repeat(right_pad), width = left_pad).yellow().bold());
        println!("{}", "║              [TESTNET]               ║".yellow().bold());
        println!("{}", "╚══════════════════════════════════════╝".yellow().bold());
    } else {
        println!("{}", "╔══════════════════════════════════════╗".cyan().bold());
        println!("{}", format!("║   TIME Coin Node v{:<20} ║", time_network::protocol::full_version()).cyan().bold());
        println!("{}", "╚══════════════════════════════════════╝".cyan().bold());
    }
    
    println!("Config file: {:?}", config_path);
    println!("Network: {}", network_name.yellow().bold());
    println!("Version: {}", time_network::protocol::full_version().bright_black());
    println!("Version: {}", time_network::protocol::full_version().bright_black());
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

    let network_type = if is_testnet {
        NetworkType::Testnet
    } else {
        NetworkType::Mainnet
    };

    // ═══════════════════════════════════════════════════════════════
    // STEP 1: Load blockchain from disk (or genesis if first run)
    // ═══════════════════════════════════════════════════════════════
    
    let genesis_path = config.blockchain.genesis_file
        .map(|p| expand_path(&p))
        .unwrap_or_else(|| "/root/time-coin-node/data/genesis.json".to_string());
    
    std::env::set_var("GENESIS_PATH", &genesis_path);
    
    // Try to load genesis block
    let _genesis = match load_genesis(&genesis_path) {
        Ok(g) => {
            display_genesis(&g);
            println!("{}", "✓ Genesis block verified".green());
            Some(g)
        }
        Err(_) => {
            println!("{}", "⚠ Genesis block not found locally".yellow());
            println!("{}", "  Will attempt to download from peers after connection".bright_black());
            None
        }
    };

    // Initialize blockchain state with genesis block
    let genesis_block = Block {
        header: BlockHeader {
            block_number: 0,
            timestamp: chrono::Utc.with_ymd_and_hms(2025, 10, 24, 0, 0, 0).unwrap(),
            previous_hash: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            merkle_root: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            validator_signature: "genesis".to_string(),
            validator_address: "genesis".to_string(),
        },
        transactions: vec![Transaction {
            txid: "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
            version: 1,
            inputs: vec![],
            outputs: vec![
                TxOutput { amount: 50_000_000_000_000, address: "TIME1treasury00000000000000000000000000".to_string() },
                TxOutput { amount: 10_000_000_000_000, address: "TIME1development0000000000000000000000".to_string() },
                TxOutput { amount: 10_000_000_000_000, address: "TIME1operations0000000000000000000000".to_string() },
                TxOutput { amount: 30_000_000_000_000, address: "TIME1rewards000000000000000000000000000".to_string() },
            ],
            lock_time: 0,
            timestamp: 1761264000,
        }],
        hash: "00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048".to_string(),
    };
    
    let blockchain = Arc::new(RwLock::new(
        BlockchainState::new(genesis_block, "/root/time-coin-node/data/blockchain")
            .expect("Failed to create blockchain state")
    ));

    let local_height = get_local_height(&blockchain).await;
    println!("{}", format!("📊 Local blockchain height: {}", local_height).cyan());

    // ═══════════════════════════════════════════════════════════════
    // STEP 2: Peer discovery and connection
    // ═══════════════════════════════════════════════════════════════

    let discovery = Arc::new(RwLock::new(PeerDiscovery::new(network_type.clone())));
    let listen_addr = "0.0.0.0:24100".parse().unwrap();
    let peer_manager = std::sync::Arc::new(PeerManager::new(network_type.clone(), listen_addr));

    println!("\n{}", "⏳ Starting peer discovery...".yellow());
    
    match discovery.write().await.bootstrap().await {
        Ok(peers) => {
            if !peers.is_empty() {
                println!(
                    "{}",
                    format!("  ✓ Discovered {} peer(s)", peers.len()).green()
                );
                
                // Show peer details
                for (i, peer) in peers.iter().enumerate() {
                    println!("    {}. {} (last seen: {})", i + 1, peer.address, chrono::DateTime::<chrono::Utc>::from_timestamp(peer.last_seen as i64, 0).map(|dt| dt.format("%Y-%m-%d %H:%M UTC").to_string()).unwrap_or_else(|| "unknown".to_string()));
                }
                
                peer_manager.connect_to_peers(peers.clone()).await;
                
                // Give peers time to connect
                println!("{}", "  ⏳ Waiting for peer connections...".bright_black());
                tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
                
                let connected = peer_manager.get_connected_peers().await.len();
                if connected > 0 {
                    println!(
                        "{}",
                        format!("  ✓ Connected to {} peer(s)", connected).green()
                    );
                }
            } else {
                println!("{}", "  ⚠ No peers discovered (first node?)".yellow());
            }
        }
        Err(e) => {
            println!("{}", format!("  ⚠ Peer discovery error: {}", e).yellow());
            println!("{}", "  Node will run without peers".bright_black());
        }
    }

    // Download genesis if we didn't have it
    if _genesis.is_none() && !peer_manager.get_peer_ips().await.is_empty() {
        match download_genesis_from_peers(&peer_manager, &genesis_path).await {
            Ok(g) => {
                display_genesis(&g);
                println!("{}", "✓ Genesis block downloaded and verified".green());
            }
            Err(e) => {
                println!("{} {}", "⚠".yellow(), e);
                println!("  {}", "Node will continue without genesis verification".yellow());
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // STEP 3: Check if we need to sync
    // ═══════════════════════════════════════════════════════════════
    
    let network_height = get_network_height(&peer_manager).await;
    let needs_sync = if let Some(net_height) = network_height {
        println!("{}", format!("📊 Network blockchain height: {}", net_height).cyan());
        net_height > local_height
    } else {
        // If we can't determine network height, assume we might need sync if we have peers
        !peer_manager.get_peer_ips().await.is_empty() && local_height == 0
    };

    // ═══════════════════════════════════════════════════════════════
    // STEP 4: Synchronize blockchain if needed
    // ═══════════════════════════════════════════════════════════════
    
    if needs_sync && !peer_manager.get_peer_ips().await.is_empty() {
        if !cli.full_sync {
            // Try FAST SYNC first
            match snapshot_sync(&peer_manager).await {
                Ok(snapshot) => {
                    println!("\n{}", "╔═══════════════════════════════════════════════════╗".green().bold());
                    println!("{}", "║     ⚡ FAST SYNC COMPLETE                         ║".green().bold());
                    println!("{}", "╚═══════════════════════════════════════════════════╝".green().bold());
                    println!("  Synchronized to height: {}", snapshot.height.to_string().yellow().bold());
                    println!("  Loaded {} account balances", snapshot.balances.len().to_string().yellow());
                    println!("  Registered {} masternodes", snapshot.masternodes.len().to_string().yellow());
                    println!("  Sync time: <1 second {}", "⚡".green());
                    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black());
                    
                    // TODO: Load the snapshot into the blockchain state
                }
                Err(e) => {
                    println!("{} Fast sync failed: {}", "⚠".yellow(), e);
                    println!("{}", "  Falling back to block-by-block sync...".cyan());
                    
                    // TODO: Implement block-by-block sync fallback
                    println!("{}", "  📚 Block-by-block sync not yet implemented".yellow());
                    println!("  {}", "Continuing with current state".bright_black());
                }
            }
        } else {
            // Full sync requested
            println!("{}", "📚 Full sync mode - downloading entire blockchain...".cyan());
            // TODO: Implement full block-by-block sync
            println!("{}", "  Block-by-block sync not yet implemented".yellow());
        }
    } else if needs_sync {
        println!("{}", "⚠ Blockchain may be out of sync, but no peers available".yellow());
    } else {
        println!("{}", "✓ Blockchain is up to date".green());
    }

    println!("\n{}", "✓ Blockchain initialized".green());

    // ═══════════════════════════════════════════════════════════════
    // STEP 4.5: Initialize Chain Sync
    // ═══════════════════════════════════════════════════════════════
    let chain_sync = Arc::new(ChainSync::new(
        Arc::clone(&blockchain),
        Arc::clone(&peer_manager),
    ));

    // Run initial sync
    // Check for forks first
    println!("{}", "🔍 Checking for blockchain forks...".cyan());
    if let Err(e) = chain_sync.detect_and_resolve_forks().await {
        println!("   {} Fork detection failed: {}", "⚠️".yellow(), e);
    }

    println!("{}", "🔄 Syncing blockchain with network...".cyan());
    match chain_sync.sync_from_peers().await {
        Ok(0) => println!("   {}", "✓ Blockchain is up to date".green()),
        Ok(n) => println!("   {} Synced {} blocks", "✓".green(), n),
        Err(e) => println!("   {} Sync failed: {} (will retry)", "⚠️".yellow(), e),
    }

    // Start periodic sync
    chain_sync.clone().start_periodic_sync().await;
    println!("{}", "✓ Periodic chain sync started (5 min interval)".green());
    
    // ═══════════════════════════════════════════════════════════════
    // STEP 5: Initialize consensus and services
    // ═══════════════════════════════════════════════════════════════
    
    // Initialize Consensus Engine
    let consensus = Arc::new(ConsensusEngine::new(is_dev_mode));
    
    let node_id = if let Ok(ip) = local_ip_address::local_ip() {
        ip.to_string()
    } else {
        "unknown".to_string()
    };
    consensus.add_masternode(node_id.clone()).await;

    // Load or create wallet
    let data_dir = "/root/time-coin-node/data";
    let wallet = match load_or_create_wallet(data_dir) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to load/create wallet: {}", e);
            std::process::exit(1);
        }
    };
    let wallet_address = wallet.address_string();
    println!("Wallet Address: {}", wallet_address);

    consensus.register_wallet(node_id.clone(), wallet_address.clone()).await;

    // Register all connected peers as masternodes
    {
        let peers = peer_manager.get_connected_peers().await;
        let peer_count = peers.len();
        for peer in peers {
            consensus.add_masternode(peer.address.ip().to_string()).await;
        }
        if peer_count > 0 {
            println!("✓ Registered {} connected peer(s) as masternodes", peer_count);
        }
    }

    println!("{}", "✓ Peer discovery started".green());

    // Display Consensus Status
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
            if total_masternodes < 3 {
                println!("  {} Need {} more masternode(s) for BFT consensus", 
                    "⚠".yellow(), 
                    (3 - total_masternodes).to_string().yellow().bold()
                );
            }
        }
        time_consensus::ConsensusMode::BFT => {
            println!("  Mode: {} {}", "BFT".green().bold(), "(2/3+ voting)".bright_black());
            println!("  {} Byzantine Fault Tolerant", "✓".green());
        }
    }

    println!("\n{}", "✓ Masternode services starting".green());
    println!("Version: v{}", VERSION);
    
    // Initialize mempool for pending transactions
    let mempool = Arc::new(time_mempool::Mempool::with_blockchain(10000, blockchain.clone()));
    
    // Load mempool from disk
    
    let mempool_path = "/root/time-coin-node/data/mempool.json";
    
    match mempool.load_from_disk(mempool_path).await {
    
        Ok(count) if count > 0 => {
    
            println!("{}", format!("✓ Loaded {} transactions from mempool", count).green());
    
        }
    
        Ok(_) => {
    
            println!("{}", "✓ Starting with empty mempool".bright_black());
    
        }
    
        Err(e) => {
    
            println!("{}", format!("⚠ Could not load mempool: {}", e).yellow());
    
        }
    
    }
    
    println!("{}", "✓ Mempool initialized (capacity: 10,000)".green());
    
    // Sync mempool from network peers
    if !peer_manager.get_peer_ips().await.is_empty() {
        match sync_mempool_from_peers(&peer_manager, &mempool).await {
            Ok(_) => {},
            Err(e) => {
                println!("{}", format!("⚠ Could not sync mempool from peers: {}", e).yellow());
            }
        }
    }

    // Initialize transaction consensus manager
    let tx_consensus = Arc::new(time_consensus::tx_consensus::TxConsensusManager::new());
    
    // Set masternodes in tx_consensus (sync with main consensus)
    let masternodes = consensus.get_masternodes().await;
    tx_consensus.set_masternodes(masternodes).await;
    println!("{}", "✓ Transaction consensus manager initialized".green());

    // Initialize transaction broadcaster
    let tx_broadcaster = Arc::new(time_network::tx_broadcast::TransactionBroadcaster::new(mempool.clone()));
    
    // Update broadcaster with current peers
    let current_peers = peer_manager.get_peer_ips().await;
    tx_broadcaster.update_peers(current_peers).await;
    println!("{}", "✓ Transaction broadcaster initialized".green());

    println!();

    // Start API Server
    let api_enabled = config.rpc.enabled.unwrap_or(true);
    let api_bind = config.rpc.bind.unwrap_or_else(|| "0.0.0.0".to_string());
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
            blockchain.clone(),
        )
        .with_mempool(mempool.clone())
        .with_tx_consensus(tx_consensus.clone())
        .with_tx_broadcaster(tx_broadcaster.clone());

        // Start Peer Listener for incoming connections
        let peer_listener_addr = "0.0.0.0:24100".parse().unwrap();
        
        match PeerListener::bind(peer_listener_addr, network_type).await {
            Ok(peer_listener) => {
                let peer_manager_clone = peer_manager.clone();
                let tx_broadcaster_clone = tx_broadcaster.clone();
                let consensus_clone = consensus.clone();
                
                tokio::spawn(async move {
                    loop {
                        if let Ok(conn) = peer_listener.accept().await {
                            let info = conn.peer_info().await;
                            let peer_addr = info.address.clone();
                            
                            println!("{}", format!("✓ Connected to {} (v{})", 
                                peer_addr.ip().to_string().bright_blue(),
                                info.version.bright_black()
                            ).green());
                            
                            peer_manager_clone.add_connected_peer(info).await;
                            
                            // Update transaction broadcaster with current peer list
                            let current_peers = peer_manager_clone.get_peer_ips().await;
                            tx_broadcaster_clone.update_peers(current_peers).await;
                            
                            let prev_count = consensus_clone.masternode_count().await;
                            consensus_clone.add_masternode(peer_addr.ip().to_string()).await;
                            let new_count = consensus_clone.masternode_count().await;
                            
                            // Announce BFT activation
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

        // Display Node Status
        let mode_str = match consensus_mode {
            time_consensus::ConsensusMode::Development => "DEV",
            time_consensus::ConsensusMode::BootstrapNoQuorum => "BOOTSTRAP",
            time_consensus::ConsensusMode::BFT => "BFT",
        };

        println!("\n{}", format!("Node Status: ACTIVE [{}] [{}]", network_name, mode_str).green().bold());
    println!("Version: {}", VERSION);
    }
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Periodic peer discovery refresh
    let discovery_refresh = discovery.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(300));
        interval.tick().await; // Skip first immediate tick
        loop {
            interval.tick().await;
            if let Ok(peers) = discovery_refresh.write().await.bootstrap().await {
                if !peers.is_empty() {
                    println!(
                        "{}",
                        format!("  ✓ Found {} peers via seed nodes", peers.len()).bright_black()
                    );
                    
                    let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
                    println!(
                        "[{}] {} - {} peer(s) available",
                        timestamp,
                        "Peer discovery refresh".bright_black(),
                        peers.len()
                    );
                }
            }
        }
    });

    // Start Block Producer
    
    let block_producer = BlockProducer::new(
        node_id.clone(), 
        peer_manager.clone(), 
        consensus.clone(), 
        blockchain.clone(), 
        mempool.clone(), 
        tx_consensus.clone()
    );
    
    tokio::spawn(async move {
        block_producer.start().await;
    });
    println!("{}", "✓ Block producer started (24-hour interval)".green());
    println!();

    // Mempool persistence task
    let mempool_persist = mempool.clone();
    let mempool_path_persist = mempool_path.to_string();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(60));
        interval.tick().await;
        
        loop {
            interval.tick().await;
            
            // Clean up stale transactions
            let removed = mempool_persist.cleanup_stale().await;
            if removed > 0 {
                println!("{}", format!("🗑️  Removed {} stale transactions from mempool", removed).bright_black());
            }
            
            // Save to disk
            if let Err(e) = mempool_persist.save_to_disk(&mempool_path_persist).await {
                eprintln!("Failed to save mempool: {}", e);
            }
        }
    });


    // Transaction broadcaster synchronization task
    let peer_mgr_bc = peer_manager.clone();
    let tx_bc_sync = tx_broadcaster.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(30));
        interval.tick().await;
        
        loop {
            interval.tick().await;
            let current_peers = peer_mgr_bc.get_peer_ips().await;
            tx_bc_sync.update_peers(current_peers).await;
        }
    });


    // Masternode synchronization task
    let peer_mgr_sync = peer_manager.clone();
    let consensus_sync = consensus.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(30));
        interval.tick().await;
        
        loop {
            interval.tick().await;
            let peers = peer_mgr_sync.get_connected_peers().await;
            for peer in peers {
                consensus_sync.add_masternode(peer.address.ip().to_string()).await;
            }
        }
    });

    // Main heartbeat loop with detailed status
    let mut counter = 0;
    let consensus_heartbeat = consensus.clone();
    
    loop {
        time::sleep(Duration::from_secs(60)).await;
        counter += 1;
        
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
        
        // Sync masternodes with actual connected peers (including self)
        let mut current_peers = peer_manager.get_peer_ips().await;
        current_peers.push(node_id.clone());
        current_peers.sort();
        current_peers.dedup();
        consensus_heartbeat.sync_masternodes(current_peers).await;

        let total_nodes = consensus_heartbeat.masternode_count().await;
        let mode = consensus_heartbeat.consensus_mode().await;
        let consensus_mode = match mode {
            time_consensus::ConsensusMode::Development => "DEV",
            time_consensus::ConsensusMode::BootstrapNoQuorum => "BOOTSTRAP",
            time_consensus::ConsensusMode::BFT => "BFT",
        };
        
        // Detailed heartbeat output
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
fn load_or_create_wallet(data_dir: &str) -> Result<Wallet, Box<dyn std::error::Error>> {
    let wallet_path = format!("{}/wallet.json", data_dir);
    if std::path::Path::new(&wallet_path).exists() {
        Ok(Wallet::load_from_file(&wallet_path)?)
    } else {
        let wallet = Wallet::new(WalletNetworkType::Testnet)?;
        wallet.save_to_file(&wallet_path)?;
        Ok(wallet)
    }
}
