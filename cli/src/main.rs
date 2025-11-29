use clap::Parser;
use wallet::{NetworkType as WalletNetworkType, Wallet};

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::sync::RwLock;

use owo_colors::OwoColorize;

use serde::Deserialize;

mod bft_consensus;
mod block_producer;
mod chain_sync;
mod deterministic_consensus;
use block_producer::BlockProducer;
use chain_sync::ChainSync;

use std::path::PathBuf;

use std::time::Duration;

use time_api::{start_server, ApiState};

use time_core::state::BlockchainState;

use time_network::{NetworkType, PeerDiscovery, PeerListener, PeerManager};

use time_consensus::ConsensusEngine;

/// Safely truncate a string to a maximum length, handling short strings
fn truncate_str(s: &str, max_len: usize) -> &str {
    if s.len() <= max_len {
        s
    } else {
        &s[..max_len]
    }
}

use tokio::time;

use clap::Subcommand;

#[derive(Parser)]
#[command(name = "timed")]
#[command(about = "TIME Coin Daemon", long_version = None)]
#[command(disable_version_flag = true)]
struct Cli {
    #[arg(short, long, value_name = "FILE")]
    config: Option<PathBuf>,

    #[arg(short = 'V', long)]
    version: bool,

    #[arg(long)]
    dev: bool,

    #[arg(long)]
    full_sync: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Start the TIME Coin node (default)
    Start,
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

    #[serde(default)]
    sync: SyncConfig,
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
    #[allow(dead_code)]
    data_dir: Option<String>,

    /// Path to genesis block JSON file
    genesis_file: Option<String>,

    /// Allow loading genesis from file (DANGEROUS - only for initial setup)
    /// Default: false (must download from peers)
    load_genesis_from_file: Option<bool>,

    /// Allow recreating missing historical blocks via consensus
    /// Default: false (download only)
    allow_block_recreation: Option<bool>,
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

#[derive(Debug, Deserialize, Default)]
struct SyncConfig {
    /// Enable midnight window sync skipping
    midnight_window_enabled: Option<bool>,
    /// Hour before midnight to start window (default: 23 = 11 PM)
    midnight_window_start_hour: Option<u32>,
    /// Hour after midnight to end window (default: 1 = 1 AM)
    midnight_window_end_hour: Option<u32>,
    /// Check consensus status before skipping (default: true)
    midnight_window_check_consensus: Option<bool>,
}

fn load_config(path: &PathBuf) -> Result<Config, Box<dyn std::error::Error>> {
    let contents = std::fs::read_to_string(path)?;
    let config: Config = toml::from_str(&contents)?;
    Ok(config)
}

fn expand_path(path: &str) -> String {
    path.replace("$HOME", &std::env::var("HOME").unwrap_or_default())
        .replace("~", &std::env::var("HOME").unwrap_or_default())
}

fn ensure_data_directories(base_dir: &str) -> Result<(), Box<dyn std::error::Error>> {
    use std::fs;

    // Create base data directory
    fs::create_dir_all(base_dir)?;

    // Create subdirectories
    fs::create_dir_all(format!("{}/blockchain", base_dir))?;
    fs::create_dir_all(format!("{}/wallets", base_dir))?;
    fs::create_dir_all(format!("{}/logs", base_dir))?;

    Ok(())
}

/// Get local blockchain height from disk
async fn get_local_height(blockchain: &Arc<RwLock<BlockchainState>>) -> u64 {
    let chain = blockchain.read().await;
    chain.chain_tip_height()
}

/// Query network for current height
async fn get_network_height(peer_manager: &Arc<PeerManager>) -> Option<u64> {
    let peers = peer_manager.get_peer_ips().await;

    if peers.is_empty() {
        return None;
    }

    // Query multiple peers and take the highest height
    let mut max_height = 0u64;
    let mut successful_queries = 0;

    for peer in peers.iter().take(3) {
        // Query up to 3 peers
        match peer_manager.request_blockchain_info(peer).await {
            Ok(Some(height)) => {
                successful_queries += 1;
                if height > max_height {
                    max_height = height;
                }
                println!(
                    "   {} reports height: {}",
                    peer.bright_black(),
                    height.to_string().yellow()
                );
            }
            Ok(None) => {
                println!("   {} has no genesis yet", peer.bright_black());
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

/// Load genesis block from JSON file
fn load_genesis_from_json(
    json_data: &str,
    db_path: &str,
) -> Result<String, Box<dyn std::error::Error>> {
    use time_core::block::{Block, MasternodeCounts};

    // Parse the genesis JSON
    let genesis_json: serde_json::Value = serde_json::from_str(json_data)?;
    let block_json = genesis_json
        .get("block")
        .ok_or("Missing 'block' field in genesis JSON")?;

    // Deserialize the block, adding masternode_counts if missing
    let mut block: Block = serde_json::from_value(block_json.clone())?;

    // Ensure masternode_counts exists (for backwards compatibility)
    if block.header.masternode_counts.free == 0
        && block.header.masternode_counts.bronze == 0
        && block.header.masternode_counts.silver == 0
        && block.header.masternode_counts.gold == 0
    {
        // Set default counts for genesis
        block.header.masternode_counts = MasternodeCounts {
            free: 0,
            bronze: 0,
            silver: 0,
            gold: 0,
        };
    }

    // Recalculate hash to ensure it matches
    let calculated_hash = block.calculate_hash();

    // Validate the block structure
    block.validate_structure()?;

    // Save to database
    let db = time_core::db::BlockchainDB::open(db_path)?;
    db.save_block(&block)?;

    Ok(calculated_hash)
}

/// Sync finalized transactions from connected peers
async fn sync_finalized_transactions_from_peers(
    peer_manager: &Arc<time_network::PeerManager>,
    blockchain: &Arc<RwLock<time_core::state::BlockchainState>>,
    since_timestamp: i64,
) -> Result<u32, Box<dyn std::error::Error>> {
    let peers = peer_manager.get_peer_ips().await;

    if peers.is_empty() {
        println!("   ℹ️  No peers available for finalized transaction sync");
        return Ok(0);
    }

    println!("📥 Syncing finalized transactions from network...");
    println!(
        "   Requesting transactions since timestamp: {}",
        since_timestamp
    );

    let mut total_synced = 0;

    for peer_ip in &peers {
        let ip_only = if peer_ip.contains(':') {
            peer_ip.split(':').next().unwrap_or(peer_ip)
        } else {
            peer_ip.as_str()
        };

        // Get network-aware port
        let p2p_port = match peer_manager.network {
            time_network::discovery::NetworkType::Mainnet => 24000,
            time_network::discovery::NetworkType::Testnet => 24100,
        };
        let peer_addr_with_port = format!("{}:{}", ip_only, p2p_port);

        // Use TCP to request finalized transactions
        match peer_manager
            .request_finalized_transactions(&peer_addr_with_port, since_timestamp)
            .await
        {
            Ok(transactions_with_timestamps) => {
                println!(
                    "   ✓ Received {} finalized transactions from {}",
                    transactions_with_timestamps.len(),
                    ip_only
                );

                if transactions_with_timestamps.is_empty() {
                    continue;
                }

                let mut blockchain_write = blockchain.write().await;

                for (tx, _finalized_at) in transactions_with_timestamps {
                    // Apply to UTXO set
                    match blockchain_write.utxo_set_mut().apply_transaction(&tx) {
                        Ok(_) => {
                            total_synced += 1;
                            println!("      ✓ Applied tx {}", truncate_str(&tx.txid, 16));
                        }
                        Err(e) => {
                            println!(
                                "      ⚠️  Skipped tx {} ({})",
                                truncate_str(&tx.txid, 16),
                                e
                            );
                        }
                    }
                }

                if total_synced > 0 {
                    // Save UTXO snapshot after applying synced transactions
                    if let Err(e) = blockchain_write.save_utxo_snapshot() {
                        println!("   ⚠️  Failed to save UTXO snapshot: {}", e);
                    }
                }

                drop(blockchain_write);
                break; // Successfully synced from one peer
            }
            Err(e) => {
                println!("   ✗ Failed to contact {}: {}", ip_only, e);
            }
        }
    }

    if total_synced > 0 {
        println!(
            "✓ Synced {} finalized transactions from network",
            total_synced
        );
    }

    Ok(total_synced)
}

/// Sync mempool from connected peers
async fn sync_mempool_from_peers(
    peer_manager: &Arc<time_network::PeerManager>,
    mempool: &Arc<time_mempool::Mempool>,
    tx_broadcaster: Option<&Arc<time_network::tx_broadcast::TransactionBroadcaster>>,
) -> Result<u32, Box<dyn std::error::Error>> {
    let peers = peer_manager.get_peer_ips().await;

    if peers.is_empty() {
        println!("   ℹ️  No peers available for mempool sync");
        return Ok(0);
    }

    println!("📥 Syncing mempool from network...");

    // Get list of transactions we already have
    let existing_txs = mempool.get_all_transactions().await;
    let existing_txids: std::collections::HashSet<String> =
        existing_txs.iter().map(|tx| tx.txid.clone()).collect();

    let mut total_transactions = 0;
    let mut new_transactions = 0;
    let mut successful_peers = 0;
    let mut failed_peers = Vec::new();

    for peer_ip in &peers {
        // Extract just the IP address (remove port if present)
        let ip_only = if peer_ip.contains(':') {
            peer_ip.split(':').next().unwrap_or(peer_ip)
        } else {
            peer_ip.as_str()
        };

        // Get network-aware port
        let p2p_port = match peer_manager.network {
            time_network::discovery::NetworkType::Mainnet => 24000,
            time_network::discovery::NetworkType::Testnet => 24100,
        };
        let peer_addr_with_port = format!("{}:{}", ip_only, p2p_port);

        // Retry logic with exponential backoff
        let mut retry_count = 0;
        let max_retries = 3;
        let mut success = false;

        while retry_count < max_retries && !success {
            println!(
                "   Requesting mempool from {} (TCP)...",
                peer_addr_with_port
            );

            match peer_manager.request_mempool(&peer_addr_with_port).await {
                Ok(transactions) => {
                    let tx_count = transactions.len();
                    println!("   ✓ Received {} transactions", tx_count);

                    // Track new transactions we didn't already have
                    let mut newly_added = 0;

                    // Iterate over references to avoid moving the vector
                    for tx in &transactions {
                        // Skip if we already have this transaction
                        if existing_txids.contains(&tx.txid) {
                            continue;
                        }

                        // Try to add to mempool
                        if mempool.add_transaction(tx.clone()).await.is_ok() {
                            newly_added += 1;

                            // Only broadcast NEW transactions (not ones we had from disk)
                            if let Some(broadcaster) = tx_broadcaster {
                                broadcaster.broadcast_transaction(tx.clone()).await;
                            }
                        }
                    }

                    if newly_added > 0 {
                        println!("      {} new transactions", newly_added);
                    }

                    total_transactions += tx_count as u32;
                    new_transactions += newly_added;
                    successful_peers += 1;
                    success = true;
                }
                Err(e) => {
                    if retry_count < max_retries - 1 {
                        let wait_secs = 2_u64.pow(retry_count);
                        println!(
                            "   ⏳ Retry {}/{} in {}s: {}",
                            retry_count + 1,
                            max_retries,
                            wait_secs,
                            e
                        );
                        tokio::time::sleep(tokio::time::Duration::from_secs(wait_secs)).await;
                    } else {
                        eprintln!("   ✗ Failed to get mempool from {}: {}", ip_only, e);
                        failed_peers.push((peer_ip.clone(), format!("request failed: {}", e)));
                    }
                }
            }
            retry_count += 1;
        }
    }

    println!("✓ Mempool is up to date");

    if !failed_peers.is_empty() {
        println!("   ⚠️  {} peer(s) failed to sync:", failed_peers.len());
        for (peer, reason) in failed_peers {
            println!("      - {}: {}", peer, reason);
        }
    }

    println!(
        "   📊 Synced with {}/{} peers ({} total tx, {} new)",
        successful_peers,
        peers.len(),
        total_transactions,
        new_transactions
    );
    Ok(new_transactions)
}

/// Return true if we can open a TCP connection to `addr` within `timeout_ms`.
async fn peer_is_online(addr: &std::net::SocketAddr, _timeout_ms: u64) -> bool {
    // Try to establish TCP connection (this is what the function name implies)
    matches!(
        tokio::time::timeout(
            std::time::Duration::from_millis(_timeout_ms),
            tokio::net::TcpStream::connect(addr),
        )
        .await,
        Ok(Ok(_stream))
    )
}

fn detect_public_ip() -> Option<String> {
    let services = [
        "https://ifconfig.me/ip",
        "https://api.ipify.org",
        "https://icanhazip.com",
    ];

    for service in &services {
        if let Ok(response) = reqwest::blocking::get(*service) {
            if let Ok(ip) = response.text() {
                let ip = ip.trim().to_string();
                // Validate it's an IP address
                if ip.parse::<std::net::IpAddr>().is_ok() {
                    return Some(ip);
                }
            }
        }
    }

    None
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if cli.version {
        println!("timed {}", time_network::protocol::full_version());
        println!("Committed: {}", time_network::protocol::GIT_COMMIT_DATE);
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

    // Handle subcommands
    match cli.command {
        Some(Commands::Start) | None => {
            // Continue with normal node startup
        }
    }

    let network_name = config
        .node
        .network
        .as_deref()
        .unwrap_or("testnet")
        .to_uppercase();

    let is_testnet = network_name == "TESTNET";

    // Banner with network indicator and build information
    if is_testnet {
        println!(
            "{}",
            "╔════════════════════════════════════════════════════════════╗"
                .yellow()
                .bold()
        );

        let version_str = time_network::protocol::full_version();
        let build_info = format!(
            "{} | {} | Committed: {}",
            version_str,
            time_network::protocol::GIT_BRANCH,
            time_network::protocol::GIT_COMMIT_DATE
        );

        let total_width: usize = 62; // Inner width of banner
        let padding = total_width.saturating_sub(build_info.len());
        let left_pad = padding / 2;
        let right_pad = padding - left_pad;

        println!(
            "{}",
            format!(
                "║{:width$}{}{}║",
                "",
                build_info,
                " ".repeat(right_pad),
                width = left_pad
            )
            .yellow()
            .bold()
        );

        println!(
            "{}",
            "║                   [TESTNET]                                  ║"
                .yellow()
                .bold()
        );
        println!(
            "{}",
            "╚════════════════════════════════════════════════════════════╝"
                .yellow()
                .bold()
        );
    } else {
        println!(
            "{}",
            "╔════════════════════════════════════════════════════════════╗"
                .cyan()
                .bold()
        );

        let version_str = time_network::protocol::full_version();
        let build_info = format!(
            "TIME Coin Node {} | {}",
            version_str,
            time_network::protocol::GIT_COMMIT_DATE
        );

        let total_width: usize = 62;
        let padding = total_width.saturating_sub(build_info.len());
        let left_pad = padding / 2;
        let right_pad = padding - left_pad;

        println!(
            "{}",
            format!(
                "║{:width$}{}{}║",
                "",
                build_info,
                " ".repeat(right_pad),
                width = left_pad
            )
            .cyan()
            .bold()
        );

        println!(
            "{}",
            "║              [MAINNET]                                  ║"
                .cyan()
                .bold()
        );
        println!(
            "{}",
            "╚════════════════════════════════════════════════════════════╝"
                .cyan()
                .bold()
        );
    }

    println!("Config file: {:?}", config_path);
    println!("Network: {}", network_name.yellow().bold());
    println!(
        "Version: {}",
        time_network::protocol::full_version().bright_black()
    );
    println!(
        "Committed: {}",
        time_network::protocol::GIT_COMMIT_DATE.bright_black()
    );
    println!(
        "Branch: {} (commit #{})",
        time_network::protocol::GIT_BRANCH.bright_black(),
        time_network::protocol::GIT_COMMIT_COUNT.bright_black()
    );
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

    println!("{}", "🚀 Starting TIME Daemon...".green().bold());
    println!("{}", "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black());

    let network_type = if is_testnet {
        NetworkType::Testnet
    } else {
        NetworkType::Mainnet
    };

    // ═══════════════════════════════════════════════════════════════
    // STEP 1: Get data directory and ensure it exists
    // ═══════════════════════════════════════════════════════════════

    // Get data directory from config or use default
    let data_dir = config
        .node
        .data_dir
        .as_ref()
        .map(|p| expand_path(p))
        .or_else(|| config.blockchain.data_dir.as_ref().map(|p| expand_path(p)))
        .unwrap_or_else(|| "/var/lib/time-coin".to_string());

    // Ensure all data directories exist
    if let Err(e) = ensure_data_directories(&data_dir) {
        eprintln!("Failed to create data directories: {}", e);
        std::process::exit(1);
    }

    println!(
        "{}",
        format!("✓ Data directories verified: {}", data_dir).green()
    );
    println!("{}\n", format!("Data Directory: {}", data_dir).cyan());

    // ═══════════════════════════════════════════════════════════════
    // STEP 2: Peer discovery and connection
    // ═══════════════════════════════════════════════════════════════

    // Detect public IP for peer-to-peer handshakes
    let node_id = std::env::var("NODE_PUBLIC_IP").unwrap_or_else(|_| {
        // Try to auto-detect public IP via HTTP
        println!("🔍 Auto-detecting public IP address...");
        let public_ip = detect_public_ip();

        if let Some(ip) = public_ip {
            println!("✓ Auto-detected public IP: {}", ip);
            ip
        } else if let Ok(ip) = local_ip_address::local_ip() {
            let ip_str = ip.to_string();
            eprintln!("⚠️  WARNING: Could not auto-detect public IP!");
            eprintln!(
                "⚠️  Using local IP: {} (this may cause consensus issues)",
                ip_str
            );
            eprintln!("⚠️  Set NODE_PUBLIC_IP environment variable if this is incorrect");
            ip_str
        } else {
            eprintln!("❌ CRITICAL: Cannot determine IP address!");
            eprintln!("❌ Set NODE_PUBLIC_IP environment variable");
            std::process::exit(1);
        }
    });
    let discovery = Arc::new(RwLock::new(PeerDiscovery::new(network_type)));
    let p2p_listen_addr = "0.0.0.0:24100".parse().unwrap();
    let p2p_manager_public = format!("{}:24100", node_id).parse().unwrap();
    let peer_manager = Arc::new(PeerManager::new(
        network_type,
        p2p_listen_addr,
        p2p_manager_public,
    ));

    let discovery_quiet = std::env::var("TIMECOIN_QUIET_DISCOVERY").is_ok();
    let strict_discovery = std::env::var("TIMECOIN_STRICT_DISCOVERY").is_ok();

    if !discovery_quiet {
        println!("\n{}", "⏳ Starting peer discovery...".yellow());
    }

    match discovery.write().await.bootstrap().await {
        Ok(peers) => {
            if !peers.is_empty() {
                // Optionally filter unreachable peers (strict mode)
                let mut peers_to_show = peers.clone();
                let mut peers_to_connect = peers.clone();

                if strict_discovery {
                    // Check reachability for each discovered peer (timeout 2000ms per check)
                    let mut reachable = Vec::new();
                    for peer in peers.iter() {
                        if peer_is_online(&peer.address, 2000).await {
                            reachable.push(peer.clone());
                        }
                    }
                    peers_to_show = reachable.clone();
                    peers_to_connect = reachable;
                }

                if !discovery_quiet {
                    println!(
                        "{}",
                        format!("  ✓ Discovered {} peer(s)", peers_to_show.len()).green()
                    );

                    // Show peer details for the filtered set
                    for (i, peer) in peers_to_show.iter().enumerate() {
                        println!("    {}. {}", i + 1, peer.address);
                    }

                    if peers_to_show.len() < peers.len() {
                        println!(
                            "  {} unreachable peer(s) were filtered out",
                            peers.len() - peers_to_show.len()
                        );
                    }
                }

                // Connect to the chosen set (filtered if strict_discovery, otherwise all discovered peers)
                peer_manager.connect_to_peers(peers_to_connect).await;

                // connect_to_peers now waits internally, just check results
                let connected = peer_manager.get_connected_peers().await.len();
                if connected > 0 && !discovery_quiet {
                    println!(
                        "{}",
                        format!("  ✓ Connected to {} peer(s)", connected).green()
                    );
                } else if !discovery_quiet {
                    println!("{}", "  ⚠️  No peers connected".yellow());
                }

                // SKIP SECOND WAVE - it causes hangs
                // Peers will be discovered through incoming connections instead
            } else if !discovery_quiet {
                println!("{}", "  ⚠ No peers discovered (first node?)".yellow());
            }
        }
        Err(e) => {
            if !discovery_quiet {
                println!("{}", format!("  ⚠ Peer discovery error: {}", e).yellow());
                println!("{}", "  Node will run without peers".bright_black());
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // STEP 2.5: Check if we need genesis and try to download immediately
    // ═══════════════════════════════════════════════════════════════

    // Check if we have genesis before loading from file
    let db_path = format!("{}/blockchain", data_dir);
    let has_genesis_on_disk = {
        match time_core::db::BlockchainDB::open(&db_path) {
            Ok(db) => match db.load_all_blocks() {
                Ok(blocks) => {
                    // Check if we have the genesis block specifically (height 0)
                    blocks.iter().any(|b| b.header.block_number == 0)
                }
                Err(_) => false,
            },
            Err(_) => false,
        }
    };

    // If we don't have genesis and have connected peers, try to download it immediately
    if !has_genesis_on_disk && !peer_manager.get_connected_peers().await.is_empty() {
        println!("{}", "🔍 No genesis block found - checking peers...".cyan());

        // Create a temporary blockchain state to download into
        let temp_blockchain = Arc::new(RwLock::new(
            BlockchainState::new_from_disk_or_sync(&db_path)
                .expect("Failed to initialize temporary blockchain state"),
        ));

        // Create temporary chain sync to use download method
        let temp_chain_sync = ChainSync::with_midnight_config(
            Arc::clone(&temp_blockchain),
            Arc::clone(&peer_manager),
            None,
            Arc::new(RwLock::new(false)),
        );

        // Try to download genesis from all known peers
        match temp_chain_sync.try_download_genesis_from_all_peers().await {
            Ok(()) => {
                println!("{}", "   ✅ Genesis block downloaded successfully!".green());
            }
            Err(e) => {
                println!(
                    "{}",
                    format!("   ⚠️  Could not download genesis yet: {}", e).yellow()
                );
                println!("{}", "   Will retry during periodic sync...".bright_black());
            }
        }
    }

    // ═══════════════════════════════════════════════════════════════
    // STEP 3: Initialize blockchain state (load from disk or prepare for sync)
    // ═══════════════════════════════════════════════════════════════

    // Check if genesis file should be loaded (only if flag is enabled)
    let load_genesis_enabled = config.blockchain.load_genesis_from_file.unwrap_or(false);

    if load_genesis_enabled {
        println!("{}", "🔍 Genesis loading is enabled".cyan());
        let genesis_file_path = config.blockchain.genesis_file.clone();

        if let Some(genesis_path) = genesis_file_path {
            println!(
                "{}",
                format!("   Genesis file path: {}", genesis_path).bright_black()
            );
            // Check if we need to load genesis (no blocks on disk)
            let db_path = format!("{}/blockchain", data_dir);
            let needs_genesis = {
                let db =
                    time_core::db::BlockchainDB::open(&db_path).expect("Failed to open database");
                let blocks = db.load_all_blocks().expect("Failed to check blocks");
                println!(
                    "{}",
                    format!("   Blocks on disk: {}", blocks.len()).bright_black()
                );
                blocks.is_empty()
            };

            if needs_genesis {
                println!("{}", "📥 Loading genesis block from file...".cyan());
                match std::fs::read_to_string(&genesis_path) {
                    Ok(json_data) => match load_genesis_from_json(&json_data, &db_path) {
                        Ok(genesis_hash) => {
                            println!(
                                "{}",
                                format!(
                                    "✅ Genesis block loaded: {}...",
                                    truncate_str(&genesis_hash, 16)
                                )
                                .green()
                            );
                        }
                        Err(e) => {
                            eprintln!(
                                "{}",
                                format!("❌ Failed to load genesis block: {}", e).red()
                            );
                            eprintln!("{}", "   Will attempt to download from peers".yellow());
                        }
                    },
                    Err(e) => {
                        eprintln!(
                            "{}",
                            format!("⚠️  Could not read genesis file {}: {}", genesis_path, e)
                                .yellow()
                        );
                        eprintln!("{}", "   Will attempt to download from peers".yellow());
                    }
                }
            } else {
                println!("{}", "   Genesis already exists on disk".bright_black());
            }
        } else {
            eprintln!(
                "{}",
                "⚠️  load_genesis_from_file is true but genesis_file is not set".yellow()
            );
        }
    } else {
        println!(
            "{}",
            "ℹ️  Genesis loading from file is disabled - will sync from peers".bright_black()
        );
    }

    let blockchain = Arc::new(RwLock::new(
        BlockchainState::new_from_disk_or_sync(&format!("{}/blockchain", data_dir))
            .expect("Failed to initialize blockchain state"),
    ));

    // Load finalized transactions from database and apply to UTXO set
    {
        let mut blockchain_write = blockchain.write().await;
        match blockchain_write.load_finalized_txs() {
            Ok(finalized_txs) if !finalized_txs.is_empty() => {
                println!(
                    "{}",
                    format!(
                        "📥 Loading {} finalized transactions from database...",
                        finalized_txs.len()
                    )
                    .cyan()
                );

                let mut applied_count = 0;
                let mut failed_count = 0;

                for tx in &finalized_txs {
                    // Try to apply to UTXO set
                    match blockchain_write.utxo_set_mut().apply_transaction(tx) {
                        Ok(_) => {
                            applied_count += 1;
                            if applied_count % 10 == 0 {
                                println!("   ✓ Applied {} transactions...", applied_count);
                            }
                        }
                        Err(e) => {
                            // Transaction may have already been included in a block or invalid
                            failed_count += 1;
                            println!("   ⚠️  Skipped tx {} ({})", truncate_str(&tx.txid, 16), e);
                        }
                    }
                }

                println!(
                    "{}",
                    format!(
                        "✓ Loaded {} finalized transactions ({} applied, {} skipped)",
                        finalized_txs.len(),
                        applied_count,
                        failed_count
                    )
                    .green()
                );

                // Save UTXO snapshot after applying finalized transactions
                if applied_count > 0 {
                    if let Err(e) = blockchain_write.save_utxo_snapshot() {
                        println!("   ⚠️  Failed to save UTXO snapshot: {}", e);
                    } else {
                        println!("   💾 UTXO snapshot updated with finalized transactions");
                    }
                }
            }
            Ok(_) => {
                println!(
                    "{}",
                    "✓ No finalized transactions in database".bright_black()
                );
            }
            Err(e) => {
                println!(
                    "{}",
                    format!("⚠ Could not load finalized transactions: {}", e).yellow()
                );
            }
        }
    }

    let local_height = get_local_height(&blockchain).await;
    println!(
        "{}",
        format!("📊 Local blockchain height: {}", local_height).cyan()
    );

    // ═══════════════════════════════════════════════════════════════
    // STEP 4: Check if we need to sync
    // ═══════════════════════════════════════════════════════════════

    let network_height = tokio::time::timeout(
        tokio::time::Duration::from_secs(10),
        get_network_height(&peer_manager),
    )
    .await
    .unwrap_or_else(|_| {
        eprintln!("⚠️  Timeout querying network height");
        None
    });
    let needs_sync = if let Some(net_height) = network_height {
        println!(
            "{}",
            format!("📊 Network blockchain height: {}", net_height).cyan()
        );
        net_height > local_height || local_height == 0
    } else {
        // If we can't determine network height, assume we need sync if we have peers and no blocks
        !peer_manager.get_peer_ips().await.is_empty() && local_height == 0
    };

    // ═══════════════════════════════════════════════════════════════
    // STEP 5: Synchronize blockchain if needed
    // ═══════════════════════════════════════════════════════════════

    if needs_sync && !peer_manager.get_peer_ips().await.is_empty() {
        println!("{}", "📚 Downloading blockchain blocks...".cyan());
        // TODO: Implement block-by-block sync
        println!("{}", "  Block-by-block sync not yet implemented".yellow());
        println!("  {}", "Continuing with current state".bright_black());
    } else if needs_sync {
        println!(
            "{}",
            "⚠ Blockchain may be out of sync, but no peers available".yellow()
        );
    } else {
        println!("{}", "✓ Blockchain is up to date".green());
    }

    println!("\n{}", "✓ Blockchain initialized".green());

    // ═══════════════════════════════════════════════════════════════
    // STEP 6: Initialize Chain Sync
    // ═══════════════════════════════════════════════════════════════

    // Create shared state for block producer activity
    let block_producer_active_state = Arc::new(RwLock::new(false));

    // Configure midnight window from config
    let midnight_config = if config.sync.midnight_window_enabled.unwrap_or(true) {
        Some(chain_sync::MidnightWindowConfig {
            start_hour: config.sync.midnight_window_start_hour.unwrap_or(23),
            end_hour: config.sync.midnight_window_end_hour.unwrap_or(1),
            check_consensus: config.sync.midnight_window_check_consensus.unwrap_or(true),
        })
    } else {
        None
    };

    let chain_sync = Arc::new(ChainSync::with_midnight_config(
        Arc::clone(&blockchain),
        Arc::clone(&peer_manager),
        midnight_config,
        block_producer_active_state.clone(),
    ));

    // Get quarantine reference for API and monitoring
    let quarantine = chain_sync.quarantine();

    // If blockchain is empty (only genesis or nothing), clear quarantine to allow fresh downloads
    {
        let blockchain_read = blockchain.read().await;
        if blockchain_read.chain_tip_height() == 0 && blockchain_read.genesis_hash().is_empty() {
            println!(
                "{}",
                "   🔄 Empty blockchain detected - clearing peer quarantine for fresh start"
                    .bright_black()
            );
            quarantine.clear_all().await;
        }
    }

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
        Err(e) => {
            // Check if this is a fork-related error
            if e.contains("Fork detected") {
                println!("   {} {}", "⚠️".yellow(), e);
                println!("   {} Re-running fork resolution...", "🔄".yellow());
                if let Err(fork_err) = chain_sync.detect_and_resolve_forks().await {
                    println!("   {} Fork resolution failed: {}", "⚠️".yellow(), fork_err);
                } else {
                    // Try sync again after fork resolution
                    match chain_sync.sync_from_peers().await {
                        Ok(0) => println!(
                            "   {}",
                            "✓ Blockchain is up to date after fork resolution".green()
                        ),
                        Ok(n) => println!(
                            "   {} Synced {} blocks after fork resolution",
                            "✓".green(),
                            n
                        ),
                        Err(e2) => {
                            println!("   {} Sync failed: {} (will retry)", "⚠️".yellow(), e2)
                        }
                    }
                }
            } else {
                println!("   {} Sync failed: {} (will retry)", "⚠️".yellow(), e);
            }
        }
    }

    // Start periodic sync
    chain_sync.clone().start_periodic_sync().await;
    println!(
        "{}",
        "✓ Periodic chain sync started (5 min interval)".green()
    );

    // Note: Genesis is now downloaded proactively at startup (Step 2.5)
    // and during periodic chain sync, so no separate downloader task is needed.

    // Start periodic quarantine logging (every 15 minutes)
    let quarantine_logger = quarantine.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(900)); // 15 minutes
        interval.tick().await; // Skip first immediate tick

        loop {
            interval.tick().await;

            let quarantined_peers = quarantine_logger.get_quarantined_peers().await;
            if !quarantined_peers.is_empty() {
                println!("\n🛡️  Quarantine Status Report:");
                println!(
                    "   {} peer(s) currently quarantined:",
                    quarantined_peers.len()
                );
                for entry in quarantined_peers.iter() {
                    println!("   • {} - {}", entry.peer_ip, entry.reason);
                }
                println!();
            }
        }
    });
    println!(
        "{}",
        "✓ Quarantine monitoring started (15 min logging)".green()
    );

    // ═══════════════════════════════════════════════════════════════
    // STEP 5: Initialize consensus and services
    // ═══════════════════════════════════════════════════════════════

    // Initialize Consensus Engine with network type
    let network_str = if is_testnet { "testnet" } else { "mainnet" };
    let mut consensus = ConsensusEngine::new_with_network(is_dev_mode, network_str.to_string());

    // Initialize proposal manager
    let proposal_manager = Arc::new(time_consensus::proposals::ProposalManager::new(
        data_dir.clone(),
    ));

    // Load proposals from disk
    if let Err(e) = proposal_manager.load().await {
        eprintln!("⚠️  Failed to load proposals: {}", e);
    }

    consensus.set_proposal_manager(proposal_manager);

    let consensus = Arc::new(consensus);

    // node_id already defined earlier for peer manager

    println!("Node ID: {}", node_id);
    consensus.add_masternode(node_id.clone()).await;

    // Load or create wallet
    let wallet = match load_or_create_wallet(&data_dir) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("Failed to load/create wallet: {}", e);
            std::process::exit(1);
        }
    };
    let wallet_address = wallet.address_string();
    println!("Wallet Address: {}", wallet_address);

    consensus
        .register_wallet(node_id.clone(), wallet_address.clone())
        .await;

    // Load wallet balance from database
    println!("\n{}", "💼 Loading wallet balance...".cyan());
    load_wallet_balance(&wallet_address, blockchain.clone()).await;

    // Register all connected peers as masternodes
    {
        let peers = peer_manager.get_connected_peers().await;
        let peer_count = peers.len();
        for peer in peers {
            let peer_ip = peer.address.ip().to_string();
            consensus.add_masternode(peer_ip.clone()).await;
        }
        if peer_count > 0 {
            println!(
                "✓ Registered {} connected peer(s) as masternodes",
                peer_count
            );
        }
    }

    println!("{}", "✓ Peer discovery started".green());

    // Display Consensus Status
    let total_masternodes = consensus.masternode_count().await;

    println!("\n{}", "Consensus Status:".cyan().bold());
    println!(
        "  Active Masternodes: {}",
        total_masternodes.to_string().yellow()
    );

    let consensus_mode = consensus.consensus_mode().await;
    match consensus_mode {
        time_consensus::ConsensusMode::Development => {
            println!(
                "  Mode: {} {}",
                "Development".yellow().bold(),
                "(auto-approve)".bright_black()
            );
        }
        time_consensus::ConsensusMode::BootstrapNoQuorum => {
            println!(
                "  Mode: {} {}",
                "Bootstrap".yellow().bold(),
                "(no voting)".bright_black()
            );
            if total_masternodes < 3 {
                println!(
                    "  {} Need {} more masternode(s) for BFT consensus",
                    "⚠".yellow(),
                    (3 - total_masternodes).to_string().yellow().bold()
                );
            }
        }
        time_consensus::ConsensusMode::BFT => {
            println!(
                "  Mode: {} {}",
                "BFT".green().bold(),
                "(2/3+ voting)".bright_black()
            );
            println!("  {} Byzantine Fault Tolerant", "✓".green());
        }
    }

    println!("\n{}", "✓ Masternode services starting".green());
    println!("Version: {}", time_network::protocol::full_version());

    // Initialize mempool for pending transactions with dynamic sizing
    let mempool = Arc::new(time_mempool::Mempool::with_blockchain(
        blockchain.clone(),
        network_name.clone(),
    ));

    // Load mempool from disk
    let mempool_path = format!("{}/mempool.json", data_dir);

    match mempool.load_from_disk(&mempool_path).await {
        Ok(count) if count > 0 => {
            println!(
                "{}",
                format!("✓ Loaded {} transactions from mempool", count).green()
            );

            // Revalidate loaded transactions against current blockchain
            println!(
                "{}",
                "   🔍 Validating loaded transactions...".bright_black()
            );
            let invalid_count = mempool.revalidate_against_blockchain().await;
            if invalid_count > 0 {
                println!(
                    "{}",
                    format!("   Removed {} invalid transactions", invalid_count).yellow()
                );
            }
        }

        Ok(_) => {
            println!("{}", "✓ Starting with empty mempool".bright_black());
        }

        Err(e) => {
            println!("{}", format!("⚠ Could not load mempool: {}", e).yellow());
        }
    }

    // Calculate dynamic mempool capacity based on available RAM
    use sysinfo::System;
    let mut sys = System::new_all();
    sys.refresh_memory();

    let available_gb = sys.available_memory() as f64 / 1_073_741_824.0;
    let avg_tx_size_bytes = 500; // Conservative estimate per transaction

    // Use 25% of available RAM for mempool (leave plenty for other operations)
    let available_for_mempool = (available_gb * 0.25 * 1_073_741_824.0) as u64;
    let _estimated_capacity = (available_for_mempool / avg_tx_size_bytes).min(10_000_000);

    println!("{}", "✓ Mempool initialized".to_string().green());

    // Initialize transaction broadcaster before mempool sync
    let tx_broadcaster = Arc::new(time_network::tx_broadcast::TransactionBroadcaster::new(
        mempool.clone(),
        peer_manager.clone(),
    ));
    println!("{}", "✓ Transaction broadcaster initialized".green());

    // Check if blockchain is synced before syncing mempool
    let local_height = get_local_height(&blockchain).await;
    let network_height = tokio::time::timeout(
        tokio::time::Duration::from_secs(10),
        get_network_height(&peer_manager),
    )
    .await
    .unwrap_or_else(|_| {
        eprintln!("⚠️  Timeout querying network height for mempool sync");
        None
    });
    let is_synced = if let Some(net_height) = network_height {
        // Consider synced if within 2 blocks of network height
        local_height >= net_height.saturating_sub(2)
    } else {
        // No peers available or can't determine - assume synced
        true
    };

    // Only sync mempool from network if blockchain is synced
    if is_synced {
        if !peer_manager.get_peer_ips().await.is_empty() {
            // First sync finalized transactions from peers
            println!(
                "{}",
                "📥 Syncing finalized transactions from network...".cyan()
            );
            // Request transactions from the last 24 hours
            let since_timestamp = chrono::Utc::now().timestamp() - 86400;
            match sync_finalized_transactions_from_peers(
                &peer_manager,
                &blockchain,
                since_timestamp,
            )
            .await
            {
                Ok(synced_count) => {
                    if synced_count > 0 {
                        println!(
                            "   {} Synced {} finalized transactions from network",
                            "✓".green(),
                            synced_count
                        );
                    } else {
                        println!(
                            "   {} No new finalized transactions from network",
                            "✓".green()
                        );
                    }
                }
                Err(e) => {
                    println!(
                        "{}",
                        format!("⚠ Could not sync finalized transactions: {}", e).yellow()
                    );
                }
            }

            // Then sync mempool
            println!(
                "{}",
                "📥 Blockchain is synced - syncing mempool from network...".cyan()
            );
            match sync_mempool_from_peers(&peer_manager, &mempool, None).await {
                Ok(synced_count) => {
                    if synced_count > 0 {
                        println!(
                            "   {} Added {} new transactions from network",
                            "✓".green(),
                            synced_count
                        );
                    }
                }
                Err(e) => {
                    println!(
                        "{}",
                        format!("⚠ Could not sync mempool from peers: {}", e).yellow()
                    );
                }
            }
        }
    } else {
        println!(
            "{}",
            "⏳ Blockchain still syncing - skipping mempool sync".bright_black()
        );
        println!(
            "   {} Mempool will sync after blockchain catches up",
            "ℹ️".bright_blue()
        );
    }

    // Initialize transaction consensus manager
    let tx_consensus = Arc::new(time_consensus::tx_consensus::TxConsensusManager::new());

    // Set masternodes in tx_consensus (sync with main consensus)
    let masternodes = consensus.get_masternodes().await;
    tx_consensus.set_masternodes(masternodes.clone()).await;
    let block_consensus = Arc::new(time_consensus::block_consensus::BlockConsensusManager::new());
    block_consensus.set_masternodes(masternodes.clone()).await;
    println!("{}", "✓ Block consensus manager initialized".green());

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
            wallet_address.clone(),
            consensus.clone(),
        )
        .with_mempool(mempool.clone())
        .with_tx_consensus(tx_consensus.clone())
        .with_block_consensus(block_consensus.clone())
        .with_tx_broadcaster(tx_broadcaster.clone())
        .with_quarantine(quarantine.clone());

        // Start Peer Listener for incoming connections
        let p2p_bind_addr = "0.0.0.0:24100".parse().unwrap();
        let p2p_public_addr = format!("{}:24100", node_id).parse().unwrap();
        match PeerListener::bind(
            p2p_bind_addr,
            network_type,
            p2p_public_addr,
            Some(blockchain.clone()),
            Some(consensus.clone()),
            Some(block_consensus.clone()),
        )
        .await
        {
            Ok(peer_listener) => {
                let peer_manager_clone = peer_manager.clone();
                let _blockchain_clone = blockchain.clone();
                let consensus_clone = consensus.clone();
                let tx_consensus_clone = tx_consensus.clone();
                let block_consensus_clone = block_consensus.clone();
                let quarantine_clone = quarantine.clone();
                let blockchain_clone = blockchain.clone();
                let wallet_address_clone = wallet_address.clone();
                let mempool_clone = mempool.clone();
                let chain_sync_clone = chain_sync.clone();
                let network_type_clone = network_type;

                tokio::spawn(async move {
                    loop {
                        if let Ok(conn) = peer_listener.accept().await {
                            let mut info = conn.peer_info().await;

                            // CRITICAL: Get real TCP peer IP (not handshake listen_addr which may be 0.0.0.0)
                            let real_peer_addr = match conn.peer_addr() {
                                Ok(addr) => addr,
                                Err(_) => {
                                    continue; // Skip if we can't get real address
                                }
                            };

                            // Normalize ephemeral port to standard server port
                            // When peers connect TO us, conn.peer_addr() returns their ephemeral client port
                            // We need to store the standard server port for connecting BACK to them
                            let standard_port = match network_type_clone {
                                time_network::NetworkType::Mainnet => 24000,
                                time_network::NetworkType::Testnet => 24100,
                            };
                            let peer_addr = if real_peer_addr.port() != standard_port {
                                // Non-standard port detected - use standard p2p port instead
                                // This handles ephemeral ports from any range (Linux: 32768-60999, Windows: 49152-65535)
                                SocketAddr::new(real_peer_addr.ip(), standard_port)
                            } else {
                                real_peer_addr
                            };

                            // Use normalized address for all peer operations
                            info.address = peer_addr;

                            // Check if peer is quarantined (log only once per hour)
                            if quarantine_clone.is_quarantined(&peer_addr.ip()).await {
                                // Silently reject - logged in periodic quarantine report
                                // Drop the connection without adding the peer
                                continue;
                            }

                            // Register peer version WITH commit info for Round 2 filtering
                            if let (Some(commit_date), Some(commits)) =
                                (&info.commit_date, &info.commit_count)
                            {
                                let commit_num = commits.parse::<u64>().unwrap_or(0);
                                block_consensus_clone
                                    .register_peer_version_with_build_info(
                                        peer_addr.ip().to_string(),
                                        info.version.clone(),
                                        commit_date.clone(),
                                        commit_num,
                                    )
                                    .await;
                            }

                            // Check for version updates
                            if time_network::protocol::should_warn_version_update(
                                info.commit_date.as_deref(),
                                info.commit_count.as_deref(),
                            ) {
                                let warning = time_network::protocol::version_update_warning(
                                    &peer_addr.ip().to_string(),
                                    &info.version,
                                    info.commit_date.as_deref().unwrap_or("unknown"),
                                    info.commit_count.as_deref().unwrap_or("0"),
                                );
                                eprintln!("{}", warning);
                            }

                            // Silently connected (reduce log verbosity)

                            // Wrap connection in Arc before storing
                            let conn_arc = Arc::new(tokio::sync::Mutex::new(conn));

                            // IMPORTANT: Store both peer info AND connection to prevent ephemeral connections
                            let peer_ip = peer_addr.ip(); // Extract IP before moving variables
                            peer_manager_clone
                                .add_connected_peer_with_connection_arc(
                                    info.clone(),
                                    conn_arc.clone(),
                                )
                                .await;

                            // Trigger genesis download if needed (event-driven)
                            // Call directly to avoid race condition with connection storage
                            chain_sync_clone.clone().on_peer_connected().await;

                            let prev_count = consensus_clone.masternode_count().await;
                            consensus_clone
                                .add_masternode(peer_addr.ip().to_string())
                                .await;

                            // Register wallet address if provided
                            if let Some(wallet_addr) = &info.wallet_address {
                                consensus_clone
                                    .register_wallet(
                                        peer_addr.ip().to_string(),
                                        wallet_addr.clone(),
                                    )
                                    .await;
                            }

                            let updated_masternodes = consensus_clone.get_masternodes().await;
                            tx_consensus_clone
                                .set_masternodes(updated_masternodes.clone())
                                .await;
                            block_consensus_clone
                                .set_masternodes(updated_masternodes)
                                .await;
                            let new_count = consensus_clone.masternode_count().await;

                            // Announce BFT activation
                            if prev_count < 3 && new_count >= 3 {
                                println!(
                                    "\n{}",
                                    "═══════════════════════════════════════".green().bold()
                                );
                                println!("{}", "🛡️  BFT CONSENSUS ACTIVATED!".green().bold());
                                println!("   {} masternodes active", new_count);
                                println!("   Requiring 2/3+ approval for blocks");
                                println!(
                                    "{}",
                                    "═══════════════════════════════════════".green().bold()
                                );
                            }

                            // Connection is now managed by the peer manager
                            // Spawn message handler for incoming Ping/Pong with timeout
                            let peer_manager_listen = Arc::clone(&peer_manager_clone);
                            let conn_arc_clone = conn_arc.clone();
                            let peer_ip_listen = peer_ip;
                            let blockchain_listen = Arc::clone(&blockchain_clone);
                            let block_consensus_listen = Arc::clone(&block_consensus_clone);
                            let wallet_address_listen = wallet_address_clone.clone();
                            let mempool_listen = Arc::clone(&mempool_clone);
                            let chain_sync_listen = Arc::clone(&chain_sync_clone);
                            tokio::spawn(async move {
                                loop {
                                    // Use timeout to prevent holding lock indefinitely
                                    let msg_result = tokio::time::timeout(
                                        tokio::time::Duration::from_secs(60),
                                        async {
                                            let mut conn = conn_arc_clone.lock().await;
                                            conn.receive_message().await
                                        },
                                    )
                                    .await;

                                    match msg_result {
                                        Ok(Ok(msg)) => {
                                            match msg {
                                                time_network::protocol::NetworkMessage::Ping => {
                                                    // Respond to ping with pong
                                                    if peer_manager_listen.send_message_to_peer(
                                                        SocketAddr::new(peer_ip_listen, 24100),
                                                        time_network::protocol::NetworkMessage::Pong
                                                    ).await.is_err() {
                                                        // Silently ignore pong send failures
                                                    }
                                                }
                                                time_network::protocol::NetworkMessage::UpdateTip { height, hash } => {
                                                    println!("📡 Peer {} announced new tip: block {} ({})",
                                                        peer_ip_listen, height, truncate_str(&hash, 16));

                                                    // Trigger sync if we're behind (with rate limiting to prevent spam)
                                                    let our_height = {
                                                        let blockchain_guard = blockchain_listen.read().await;
                                                        blockchain_guard.chain_tip_height()
                                                    };

                                                    // Only trigger sync if significantly behind (avoid triggering on every single block)
                                                    if height > our_height + 2 {
                                                        println!("   ℹ️  We're significantly behind (height {}), peer is at {} - triggering sync", our_height, height);
                                                        // Trigger async sync without blocking message processing
                                                        let chain_sync_trigger = chain_sync_listen.clone();
                                                        tokio::spawn(async move {
                                                            if let Err(e) = chain_sync_trigger.sync_from_peers().await {
                                                                eprintln!("   ⚠️  Auto-sync failed: {}", e);
                                                            }
                                                        });
                                                    }
                                                }
                                                time_network::protocol::NetworkMessage::GetPeerList => {
                                                    // Respond with our known peers directly on this connection
                                                    let peers = peer_manager_listen.get_peers().await;
                                                    let peer_list: Vec<time_network::protocol::PeerAddress> = peers
                                                        .iter()
                                                        .map(|p| time_network::protocol::PeerAddress {
                                                            ip: p.address.ip().to_string(),
                                                            port: 24100, // Always use standard port, not ephemeral ports
                                                            version: p.version.clone(),
                                                        })
                                                        .collect();

                                                    // Send response directly on the connection
                                                    let mut conn = conn_arc_clone.lock().await;
                                                    if conn.send_message(time_network::protocol::NetworkMessage::PeerList(peer_list))
                                                        .await
                                                        .is_err()
                                                    {
                                                        // Silently ignore send failures
                                                    }
                                                }
                                                time_network::protocol::NetworkMessage::RequestWalletTransactions { xpub } => {
                                                    println!("📨 Received wallet transaction request from {} for xpub: {}", peer_ip_listen, xpub);

                                                    // Subscribe this peer to transaction notifications for this xpub
                                                    peer_manager_listen.subscribe_wallet(&xpub, peer_ip_listen).await;

                                                    // TODO: Implement xpub → address derivation and blockchain search
                                                    // For now, send empty response
                                                    // Steps:
                                                    // 1. Derive addresses from xpub (use BIP44 m/44'/0'/0'/0/0 through m/44'/0'/0'/0/19 for gap limit 20)
                                                    // 2. Search blockchain for transactions to/from these addresses
                                                    // 3. Send WalletTransactionsResponse with results

                                                    let current_height = {
                                                        let blockchain_guard = blockchain_listen.read().await;
                                                        blockchain_guard.chain_tip_height()
                                                    };

                                                    let transactions = vec![];
                                                    if peer_manager_listen
                                                        .send_message_to_peer(
                                                            SocketAddr::new(peer_ip_listen, 24100),
                                                            time_network::protocol::NetworkMessage::WalletTransactionsResponse {
                                                                transactions,
                                                                last_synced_height: current_height,
                                                            },
                                                        )
                                                        .await
                                                        .is_err()
                                                    {
                                                        eprintln!("Failed to send wallet transactions response to {}", peer_ip_listen);
                                                    }
                                                }
                                                time_network::protocol::NetworkMessage::RegisterXpub { xpub } => {
                                                    println!("📝 Received RegisterXpub from {} for xpub: {}...", peer_ip_listen, &xpub[..20]);

                                                    // Subscribe this peer to transaction notifications
                                                    peer_manager_listen.subscribe_wallet(&xpub, peer_ip_listen).await;

                                                    // Send success response directly on this connection
                                                    let response = time_network::protocol::NetworkMessage::XpubRegistered {
                                                        success: true,
                                                        message: "Xpub registered successfully".to_string(),
                                                    };

                                                    let mut conn = conn_arc_clone.lock().await;
                                                    if conn.send_message(response).await.is_ok() {
                                                        println!("✅ Sent XpubRegistered response to {}", peer_ip_listen);

                                                        // Drop the lock before scanning blockchain
                                                        drop(conn);

                                                        // Scan blockchain for existing UTXOs for this xpub
                                                        println!("🔍 Scanning blockchain for UTXOs for xpub: {}...", &xpub[..20]);
                                                        let blockchain_guard = blockchain_listen.read().await;
                                                        let height = blockchain_guard.chain_tip_height();

                                                        // Derive addresses from xpub (first 20 addresses for now)
                                                        let mut all_utxos = Vec::new();
                                                        for i in 0..20 {
                                                            if let Ok(address) = wallet::xpub_to_address(&xpub, 0, i, WalletNetworkType::Testnet) {
                                                                println!("  📍 Derived address {}: {}", i, address);
                                                                // Scan all blocks for this address
                                                                for block_height in 0..=height {
                                                                    if let Some(block) = blockchain_guard.get_block_by_height(block_height) {
                                                                        // Check coinbase and all transactions
                                                                        if let Some(coinbase_tx) = block.coinbase() {
                                                                            for (vout, output) in coinbase_tx.outputs.iter().enumerate() {
                                                                                if output.address == address {
                                                                                    all_utxos.push(time_network::protocol::UtxoInfo {
                                                                                        txid: coinbase_tx.txid.clone(),
                                                                                        vout: vout as u32,
                                                                                        address: address.clone(),
                                                                                        amount: output.amount,
                                                                                        block_height: Some(block_height),
                                                                                        confirmations: height - block_height,
                                                                                    });
                                                                                }
                                                                            }
                                                                        }

                                                                        for tx in block.regular_transactions() {
                                                                            for (vout, output) in tx.outputs.iter().enumerate() {
                                                                                if output.address == address {
                                                                                    all_utxos.push(time_network::protocol::UtxoInfo {
                                                                                        txid: tx.txid.clone(),
                                                                                        vout: vout as u32,
                                                                                        address: address.clone(),
                                                                                        amount: output.amount,
                                                                                        block_height: Some(block_height),
                                                                                        confirmations: height - block_height,
                                                                                    });
                                                                                }
                                                                            }
                                                                        }
                                                                    }
                                                                }
                                                            }
                                                        }

                                                        drop(blockchain_guard);

                                                        if !all_utxos.is_empty() {
                                                            println!("✅ Found {} UTXOs for xpub, sending to wallet", all_utxos.len());

                                                            // Send UTXO update to wallet
                                                            let utxo_update = time_network::protocol::NetworkMessage::UtxoUpdate {
                                                                xpub: xpub.clone(),
                                                                utxos: all_utxos,
                                                            };

                                                            let mut conn = conn_arc_clone.lock().await;
                                                            if let Err(e) = conn.send_message(utxo_update).await {
                                                                println!("❌ Failed to send UtxoUpdate: {}", e);
                                                            }
                                                        } else {
                                                            println!("ℹ️  No UTXOs found for xpub");
                                                        }
                                                    } else {
                                                        println!("❌ Failed to send XpubRegistered response to {}", peer_ip_listen);
                                                    }
                                                }
                                                time_network::protocol::NetworkMessage::GetBlockchainInfo => {
                                                    println!("📊 Received GetBlockchainInfo request from {}", peer_ip_listen);

                                                    let blockchain_guard = blockchain_listen.read().await;
                                                    let chain_height = blockchain_guard.chain_tip_height();

                                                    // Check if we have genesis block
                                                    let has_genesis = blockchain_guard.get_block_by_height(0).is_some();

                                                    // height is None if no genesis, otherwise it's the actual chain height
                                                    let height = if has_genesis {
                                                        Some(chain_height)
                                                    } else {
                                                        None
                                                    };

                                                    let best_block_hash = if has_genesis {
                                                        blockchain_guard
                                                            .get_block_by_height(chain_height)
                                                            .map(|b| hex::encode(&b.hash))
                                                            .unwrap_or_else(|| "unknown".to_string())
                                                    } else {
                                                        "no_genesis".to_string()
                                                    };

                                                    println!("   🔍 Our state: height={:?}", height);
                                                    drop(blockchain_guard);

                                                    let response = time_network::protocol::NetworkMessage::BlockchainInfo {
                                                        height,
                                                        best_block_hash,
                                                    };

                                                    println!("   📤 Sending response: height={:?}", height);
                                                    let mut conn = conn_arc_clone.lock().await;
                                                    match conn.send_message(response).await {
                                                        Ok(_) => {
                                                            println!("✅ Sent BlockchainInfo (height {:?}) to {}", height, peer_ip_listen);
                                                        }
                                                        Err(e) => {
                                                            println!("❌ Failed to send BlockchainInfo to {}: {:?}", peer_ip_listen, e);
                                                        }
                                                    }
                                                }
                                                time_network::protocol::NetworkMessage::GetMempool => {
                                                    println!("📦 Received GetMempool request from {}", peer_ip_listen);

                                                    let transactions = mempool_listen.get_all_transactions().await;
                                                    let response = time_network::protocol::NetworkMessage::MempoolResponse(transactions);

                                                    let mut conn = conn_arc_clone.lock().await;
                                                    if conn.send_message(response).await.is_ok() {
                                                        println!("✅ Sent {} mempool transactions to {}", mempool_listen.size().await, peer_ip_listen);
                                                    } else {
                                                        println!("❌ Failed to send mempool response to {}", peer_ip_listen);
                                                    }
                                                }
                                                time_network::protocol::NetworkMessage::RequestFinalizedTransactions { since_timestamp } => {
                                                    println!("📥 Received RequestFinalizedTransactions from {} (since: {})", peer_ip_listen, since_timestamp);

                                                    let blockchain_guard = blockchain_listen.read().await;

                                                    // Get all finalized transactions from blocks after the given timestamp
                                                    let mut finalized_txs = Vec::new();
                                                    let mut finalized_timestamps = Vec::new();

                                                    let current_height = blockchain_guard.chain_tip_height();

                                                    // Iterate through blocks to find transactions finalized after the timestamp
                                                    for height in 0..=current_height {
                                                        if let Some(block) = blockchain_guard.get_block_by_height(height) {
                                                            let block_timestamp = block.header.timestamp.timestamp();
                                                            if block_timestamp > since_timestamp {
                                                                for tx in &block.transactions {
                                                                    finalized_txs.push(tx.clone());
                                                                    finalized_timestamps.push(block_timestamp);
                                                                }
                                                            }
                                                        }
                                                    }

                                                    drop(blockchain_guard);

                                                    let response = time_network::protocol::NetworkMessage::FinalizedTransactionsResponse {
                                                        transactions: finalized_txs.clone(),
                                                        finalized_at: finalized_timestamps,
                                                    };

                                                    let mut conn = conn_arc_clone.lock().await;
                                                    if conn.send_message(response).await.is_ok() {
                                                        println!("✅ Sent {} finalized transactions to {}", finalized_txs.len(), peer_ip_listen);
                                                    } else {
                                                        println!("❌ Failed to send finalized transactions response to {}", peer_ip_listen);
                                                    }
                                                }
                                                time_network::protocol::NetworkMessage::GetBlocks { start_height, end_height } => {
                                                    println!("📦 Received GetBlocks request from {} (heights {}-{})", peer_ip_listen, start_height, end_height);

                                                    let blockchain_guard = blockchain_listen.read().await;
                                                    let chain_height = blockchain_guard.chain_tip_height();
                                                    let has_genesis = !blockchain_guard.genesis_hash().is_empty();
                                                    println!("   🔍 Our blockchain: height={}", if has_genesis { chain_height.to_string() } else { "no genesis".to_string() });
                                                    let mut blocks = Vec::new();

                                                    // Limit the range to prevent abuse
                                                    let max_blocks = 100;
                                                    let actual_end = std::cmp::min(end_height, start_height + max_blocks - 1);

                                                    for height in start_height..=actual_end {
                                                        if let Some(block) = blockchain_guard.get_block_by_height(height) {
                                                            println!("   ✓ Found block at height {}", height);
                                                            blocks.push(block.clone());
                                                        } else {
                                                            println!("   ✗ No block found at height {}", height);
                                                        }
                                                    }

                                                    drop(blockchain_guard);

                                                    let response = time_network::protocol::NetworkMessage::Blocks { blocks: blocks.clone() };

                                                    let mut conn = conn_arc_clone.lock().await;
                                                    if conn.send_message(response).await.is_ok() {
                                                        println!("✅ Sent {} blocks to {}", blocks.len(), peer_ip_listen);
                                                    } else {
                                                        println!("❌ Failed to send blocks response to {}", peer_ip_listen);
                                                    }
                                                }
                                                time_network::protocol::NetworkMessage::ConsensusBlockProposal(proposal_json) => {
                                                    // Parse and store block proposal
                                                    if let Ok(proposal) = serde_json::from_str::<time_consensus::block_consensus::BlockProposal>(&proposal_json) {
                                                        println!("📦 Received block proposal for height {} from {}", proposal.block_height, peer_ip_listen);

                                                        // Check if we're behind and trigger sync
                                                        let our_height = {
                                                            let blockchain_guard = blockchain_listen.read().await;
                                                            blockchain_guard.chain_tip_height()
                                                        };

                                                        // Only trigger sync if significantly behind (avoid spam)
                                                        if proposal.block_height > our_height + 5 {
                                                            println!("   ℹ️  We're significantly behind (height {}), peer is at {} - triggering sync", our_height, proposal.block_height);
                                                            // Trigger async sync without blocking message processing
                                                            let chain_sync_trigger = chain_sync_listen.clone();
                                                            tokio::spawn(async move {
                                                                if let Err(e) = chain_sync_trigger.sync_from_peers().await {
                                                                    eprintln!("   ⚠️  Auto-sync failed: {}", e);
                                                                }
                                                            });
                                                        }

                                                        block_consensus_listen.propose_block(proposal).await;
                                                    }
                                                }
                                                time_network::protocol::NetworkMessage::ConsensusBlockVote(vote_json) => {
                                                    // Parse and record block vote
                                                    match serde_json::from_str::<time_consensus::block_consensus::BlockVote>(&vote_json) {
                                                        Ok(vote) => {
                                                            println!("🗳️  Received block vote from {} for block #{} (hash: {}..., voter: {})",
                                                                peer_ip_listen, vote.block_height, truncate_str(&vote.block_hash, 16), vote.voter);

                                                            let vote_result = block_consensus_listen.vote_on_block(vote.clone()).await;
                                                            match vote_result {
                                                                Ok(_) => println!("   ✅ Vote recorded successfully"),
                                                                Err(e) => println!("   ⚠️  Failed to record vote: {}", e),
                                                            }

                                                            // Send acknowledgment back to sender
                                                            let ack = time_network::protocol::NetworkMessage::ConsensusVoteAck {
                                                                block_hash: vote.block_hash.clone(),
                                                                voter: vote.voter.clone(),
                                                                received_at: chrono::Utc::now().timestamp() as u64,
                                                            };

                                                            let mut conn_guard = conn_arc_clone.lock().await;
                                                            if let Err(e) = conn_guard.send_message(ack).await {
                                                                println!("   ⚠️  Failed to send vote ACK: {}", e);
                                                            }
                                                            drop(conn_guard); // Release lock
                                                        }
                                                        Err(e) => {
                                                            println!("   ⚠️  Failed to parse vote from {}: {}", peer_ip_listen, e);
                                                        }
                                                    }
                                                }
                                                time_network::protocol::NetworkMessage::InstantFinalityRequest(tx) => {
                                                    println!("⚡ Received instant finality request from {} for tx {}", peer_ip_listen, truncate_str(&tx.txid, 16));

                                                    // Validate the transaction
                                                    let blockchain_guard = blockchain_listen.read().await;
                                                    let is_valid = blockchain_guard.validate_transaction(&tx).is_ok();
                                                    drop(blockchain_guard);

                                                    let vote_result = if is_valid { "APPROVE ✓" } else { "REJECT ✗" };
                                                    println!("   Voting {} for tx {}", vote_result, truncate_str(&tx.txid, 16));

                                                    // Send vote response back to requester
                                                    let vote_msg = time_network::protocol::NetworkMessage::InstantFinalityVote {
                                                        txid: tx.txid.clone(),
                                                        voter: wallet_address_listen.clone(),
                                                        approve: is_valid,
                                                        timestamp: chrono::Utc::now().timestamp() as u64,
                                                    };

                                                    let mut conn = conn_arc_clone.lock().await;
                                                    if let Err(e) = conn.send_message(vote_msg).await {
                                                        println!("❌ Failed to send instant finality vote: {}", e);
                                                    } else {
                                                        println!("✅ Sent {} vote for tx {}", vote_result, truncate_str(&tx.txid, 16));
                                                    }
                                                }
                                                _ => {
                                                    // Handle other messages if needed
                                                }
                                            }
                                        }
                                        Ok(Err(_)) => {
                                            // Connection closed - exit loop
                                            // Manager's keep-alive will handle cleanup
                                            break;
                                        }
                                        Err(_) => {
                                            // Timeout - continue loop to check for messages again
                                            // This allows keep-alive to acquire lock periodically
                                            continue;
                                        }
                                    }
                                }
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

        // Auto-register masternode
        {
            let wallet_addr = wallet_address.clone();
            let node_ip = node_id.clone();
            let port = api_port;

            tokio::spawn(async move {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;

                let url = format!("http://localhost:{}/masternode/register", port);
                let payload = serde_json::json!({
                    "node_ip": node_ip,
                    "wallet_address": wallet_addr,
                    "tier": "Free"
                });

                match reqwest::Client::new()
                    .post(&url)
                    .json(&payload)
                    .timeout(std::time::Duration::from_secs(5))
                    .send()
                    .await
                {
                    Ok(resp) if resp.status().is_success() => {
                        println!(
                            "{}",
                            format!("Masternode auto-registered: {} -> {}", node_ip, wallet_addr)
                                .green()
                        );
                    }
                    Ok(resp) => {
                        println!(
                            "{}",
                            format!("Registration status: {}", resp.status()).yellow()
                        );
                    }
                    Err(e) => {
                        println!("{}", format!("Auto-registration failed: {}", e).yellow());
                    }
                }
            });
        }
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

        println!(
            "\n{}",
            format!("Node Status: ACTIVE [{}] [{}]", network_name, mode_str)
                .green()
                .bold()
        );
        println!("Version: {}", time_network::protocol::full_version());
    }

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Periodic peer discovery refresh (every 30 minutes to reduce API load)
    let discovery_refresh = discovery.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(1800)); // 30 minutes
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

    // Sync block_height.txt with actual blockchain height on startup
    {
        let current_height = {
            let blockchain = blockchain.read().await;
            blockchain.chain_tip_height()
        };

        let height_file = format!("{}/block_height.txt", data_dir);
        if let Err(e) = std::fs::write(&height_file, current_height.to_string()) {
            eprintln!("⚠️  Failed to write block height file: {}", e);
        } else {
            println!("✓ Synced block height file: {} blocks", current_height);
        }
    }

    // Get allow_block_recreation flag from config
    // Default to true for testnet, false for mainnet
    let allow_block_recreation = config
        .blockchain
        .allow_block_recreation
        .unwrap_or(network_type == NetworkType::Testnet);
    println!(
        "🔧 Block recreation: {}",
        if allow_block_recreation {
            "ENABLED"
        } else {
            "DISABLED"
        }
    );

    let block_producer = BlockProducer::with_shared_state(
        node_id.clone(),
        peer_manager.clone(),
        consensus.clone(),
        blockchain.clone(),
        mempool.clone(),
        block_consensus.clone(),
        tx_consensus.clone(),
        block_producer_active_state,
        allow_block_recreation,
        quarantine.clone(),
    );

    tokio::spawn(async move {
        block_producer.start().await;
    });
    println!();

    // Mempool persistence task
    let mempool_persist = mempool.clone();
    let mempool_path_persist = mempool_path.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(60));
        interval.tick().await;

        loop {
            interval.tick().await;

            // Clean up stale transactions (older than 24 hours)
            let removed_stale = mempool_persist.cleanup_stale().await;
            if removed_stale > 0 {
                println!(
                    "{}",
                    format!(
                        "🗑️  Removed {} stale transactions from mempool",
                        removed_stale
                    )
                    .bright_black()
                );
            }

            // Revalidate all transactions against current blockchain state
            // This catches transactions with spent UTXOs
            let removed_invalid = mempool_persist.revalidate_against_blockchain().await;
            if removed_invalid > 0 {
                println!(
                    "{}",
                    format!(
                        "🗑️  Removed {} invalid transactions from mempool",
                        removed_invalid
                    )
                    .bright_black()
                );
            }

            // Save to disk
            if let Err(e) = mempool_persist.save_to_disk(&mempool_path_persist).await {
                eprintln!("Failed to save mempool: {}", e);
            }
        }
    });

    // Finalized transaction cleanup task - runs every hour
    let blockchain_cleanup = blockchain.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(3600)); // 1 hour
        interval.tick().await;

        loop {
            interval.tick().await;

            // Clean up finalized transactions older than 25 hours
            // (25 hours to ensure they've had time to be included in daily block)
            let blockchain_read = blockchain_cleanup.read().await;
            if let Ok(finalized_txs) = blockchain_read.load_finalized_txs() {
                drop(blockchain_read);

                let cutoff_time = chrono::Utc::now().timestamp() - (25 * 3600);
                let mut removed_count = 0;

                for tx in &finalized_txs {
                    // Check if transaction is older than cutoff
                    if tx.timestamp < cutoff_time {
                        let blockchain_write = blockchain_cleanup.read().await;
                        if let Ok(()) = blockchain_write.remove_finalized_tx(&tx.txid) {
                            removed_count += 1;
                        }
                    }
                }

                if removed_count > 0 {
                    println!(
                        "{}",
                        format!(
                            "🗑️  Cleaned up {} old finalized transactions (>25 hours)",
                            removed_count
                        )
                        .bright_black()
                    );
                }
            }
        }
    });

    // Helper function to extract peer IPs from PeerInfo list
    fn extract_peer_ips(peers: &[time_network::PeerInfo]) -> Vec<String> {
        peers
            .iter()
            .map(|peer| peer.address.ip().to_string())
            .collect()
    }

    // Masternode synchronization task (KEEP THIS!)
    let peer_mgr_sync = peer_manager.clone();
    let consensus_sync = consensus.clone();
    let tx_consensus_sync = tx_consensus.clone();
    let block_consensus_sync = block_consensus.clone();
    let quarantine_sync = quarantine.clone();
    tokio::spawn(async move {
        let mut interval = time::interval(Duration::from_secs(30));
        interval.tick().await;

        loop {
            interval.tick().await;
            let peers = peer_mgr_sync.get_connected_peers().await;

            // Filter out quarantined peers before syncing
            let mut non_quarantined_peers = Vec::new();
            for peer in peers {
                if !quarantine_sync.is_quarantined(&peer.address.ip()).await {
                    non_quarantined_peers.push(peer);
                }
            }

            // Get connected peer IPs using helper
            let connected_ips = extract_peer_ips(&non_quarantined_peers);

            // Sync block consensus manager with connected peers
            block_consensus_sync
                .sync_with_connected_peers(connected_ips.clone())
                .await;

            // Also update the main consensus and tx consensus
            for peer in &non_quarantined_peers {
                consensus_sync
                    .add_masternode(peer.address.ip().to_string())
                    .await;
            }
            let updated_masternodes = consensus_sync.get_masternodes().await;
            tx_consensus_sync
                .set_masternodes(updated_masternodes.clone())
                .await;
        }
    });

    // Main heartbeat loop with detailed status (REPLACE WITH NEW VERSION)
    let mut counter = 0;
    let consensus_heartbeat = consensus.clone();
    let block_consensus_heartbeat = block_consensus.clone();
    let peer_mgr_heartbeat = peer_manager.clone();
    let mempool_heartbeat = mempool.clone();
    let tx_broadcaster_heartbeat = tx_broadcaster.clone();

    loop {
        time::sleep(Duration::from_secs(60)).await;
        counter += 1;

        // Sync with connected peers before getting the count (with timeout to avoid blocking)
        let peers = peer_mgr_heartbeat.get_connected_peers().await;
        let connected_ips = extract_peer_ips(&peers);

        // Use timeout to prevent blocking heartbeat
        let sync_result = tokio::time::timeout(
            Duration::from_secs(5),
            block_consensus_heartbeat.sync_with_connected_peers(connected_ips),
        )
        .await;

        if sync_result.is_err() {
            eprintln!("⚠️  Peer sync timed out (>5s)");
        }

        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");

        let total_nodes = block_consensus_heartbeat.active_masternode_count().await;
        let mode = consensus_heartbeat.consensus_mode().await;
        let consensus_mode = match mode {
            time_consensus::ConsensusMode::Development => "DEV",
            time_consensus::ConsensusMode::BootstrapNoQuorum => "BOOTSTRAP",
            time_consensus::ConsensusMode::BFT => "BFT",
        };

        // TCP keepalive ping - send to all connected peers every heartbeat to maintain connections
        for peer in peers.iter() {
            // Send Ping message via TCP to keep connection alive (fire and forget - don't wait for response)
            let _ = peer_mgr_heartbeat
                .send_message_to_peer(peer.address, time_network::protocol::NetworkMessage::Ping)
                .await;
        }

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

        // Retry instant finality for pending transactions every 2 heartbeats (2 minutes)
        // Spawn in background to avoid blocking heartbeat
        if counter % 2 == 0 {
            let mempool_clone = mempool_heartbeat.clone();
            let tx_broadcaster_clone = tx_broadcaster_heartbeat.clone();

            tokio::spawn(async move {
                let pending_txs = mempool_clone.get_all_transactions().await;
                if !pending_txs.is_empty() {
                    println!(
                        "   🔄 Retrying instant finality for {} pending transaction(s)...",
                        pending_txs.len()
                    );

                    for tx in pending_txs {
                        let txid = tx.txid.clone();
                        println!(
                            "      ⚡ Re-broadcasting transaction {} to trigger voting...",
                            truncate_str(&txid, 16)
                        );

                        // Re-broadcast using tx_broadcaster - this will trigger instant finality
                        tx_broadcaster_clone.broadcast_transaction(tx).await;
                        println!("         📡 Re-broadcasted to network");
                    }
                }
            });
        }

        // Check for version updates every 10 minutes (every 10 heartbeats)
        if counter % 10 == 0 {
            for peer in peers.iter() {
                if time_network::protocol::should_warn_version_update(
                    peer.commit_date.as_deref(),
                    peer.commit_count.as_deref(),
                ) {
                    eprintln!(
                        "\n⚠️  UPDATE REMINDER: Peer {} is running newer version {} (committed: {})",
                        peer.address.ip(),
                        peer.version,
                        peer.commit_date.as_deref().unwrap_or("unknown")
                    );
                    eprintln!(
                        "   Your version: {} (committed: {})",
                        time_network::protocol::full_version(),
                        time_network::protocol::GIT_COMMIT_DATE
                    );
                    eprintln!("   Please update your node!\n");
                    break; // Only warn once per check cycle
                }
            }
        }
    }
}
fn load_or_create_wallet(data_dir: &str) -> Result<Wallet, Box<dyn std::error::Error>> {
    // Ensure wallet directory exists
    let wallet_dir = format!("{}/wallets", data_dir);
    std::fs::create_dir_all(&wallet_dir)?;

    let wallet_path = format!("{}/node.json", wallet_dir);
    if std::path::Path::new(&wallet_path).exists() {
        Ok(Wallet::load_from_file(&wallet_path)?)
    } else {
        let wallet = Wallet::new(WalletNetworkType::Testnet)?;
        wallet.save_to_file(&wallet_path)?;
        Ok(wallet)
    }
}

/// Load and display wallet balance from database
async fn load_wallet_balance(wallet_address: &str, blockchain: Arc<RwLock<BlockchainState>>) {
    let blockchain_read = blockchain.read().await;

    // Try to load balance from database
    if let Ok(Some(balance)) = blockchain_read.db().load_wallet_balance(wallet_address) {
        let utxo_count = blockchain_read
            .utxo_set()
            .get_utxos_for_address(wallet_address)
            .len();

        if balance > 0 {
            let balance_time = balance as f64 / 100_000_000.0;
            println!(
                "💰 Wallet balance loaded: {} TIME ({} UTXOs)",
                balance_time, utxo_count
            );
        } else {
            println!("ℹ️  Wallet has zero balance");
        }
    } else {
        println!(
            "ℹ️  No saved balance found - use 'time-cli wallet rescan' to sync from blockchain"
        );
    }
}

/// Sync wallet balance from blockchain UTXO set and save to database
/// Note: Currently unused - wallet balance sync handled by API handlers
#[allow(dead_code)]
async fn sync_wallet_balance(
    wallet_address: &str,
    blockchain: Arc<RwLock<BlockchainState>>,
) -> u64 {
    let blockchain_read = blockchain.read().await;
    let balance = blockchain_read.get_balance(wallet_address);

    // Save balance to database
    if let Err(e) = blockchain_read
        .db()
        .save_wallet_balance(wallet_address, balance)
    {
        eprintln!("⚠️  Failed to save wallet balance: {}", e);
    }

    // Count UTXOs for logging
    let utxo_count = blockchain_read
        .utxo_set()
        .get_utxos_for_address(wallet_address)
        .len();

    if balance > 0 {
        let balance_time = balance as f64 / 100_000_000.0;
        println!(
            "💰 Wallet balance synced: {} TIME ({} UTXOs)",
            balance_time, utxo_count
        );
    } else if utxo_count > 0 {
        println!(
            "ℹ️  Wallet has {} UTXOs but zero spendable balance",
            utxo_count
        );
    } else {
        println!("ℹ️  No UTXOs found for wallet address");
    }

    balance
}
