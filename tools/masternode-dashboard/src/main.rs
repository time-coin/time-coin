use reqwest::blocking::Client;
use serde::Deserialize;
use std::{io::{self, Write}, thread, time::{Duration, Instant}, sync::Arc, sync::atomic::{AtomicBool, Ordering}, path::PathBuf, fs};
use chrono::Utc;
use crossterm::{
    execute,
    cursor::{MoveTo, Hide, Show},
    terminal::{Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
};

#[derive(Debug, Deserialize)]
struct BlockchainInfo {
    height: u64,
    #[serde(default)]
    synced: bool,
}

#[derive(Debug, Deserialize)]
struct PeersResponse {
    peers: Vec<PeerInfo>,
    count: usize,
}

#[derive(Debug, Deserialize, Clone)]
struct PeerInfo {
    address: String,
}

#[derive(Debug, Deserialize)]
struct MempoolStatus {
    size: usize,
}

#[derive(Debug, Deserialize)]
struct WalletBalance {
    address: String,
    balance: u64,
    #[serde(default)]
    balance_time: Option<String>,
    #[serde(default)]
    pending: u64,
}

#[derive(Debug, Deserialize)]
struct WalletData {
    address: String,
}

#[derive(Debug, Deserialize)]
struct Config {
    node: Option<NodeConfig>,
}

#[derive(Debug, Deserialize)]
struct NodeConfig {
    data_dir: Option<String>,
}

struct PeerWithPing {
    address: String,
    ping_ms: u64,
}

// ANSI color codes
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const RED: &str = "\x1b[31m";
const CYAN: &str = "\x1b[36m";
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const CLEAR_LINE: &str = "\x1b[2K"; // Clear entire line

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::builder()
        .timeout(Duration::from_secs(2))
        .build()?;
    
    // Setup Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    }).expect("Error setting Ctrl-C handler");
    
    // Enter alternate screen buffer and hide cursor
    execute!(io::stdout(), EnterAlternateScreen, Hide)?;
    
    let result = run_dashboard(&client, running);
    
    // Cleanup: Leave alternate screen buffer and show cursor on exit
    execute!(io::stdout(), Show, LeaveAlternateScreen)?;
    
    println!("Dashboard stopped gracefully.");
    
    result
}

fn expand_path(path: &str) -> String {
    // Try Windows environment variables first
    if let Ok(userprofile) = std::env::var("USERPROFILE") {
        if path.contains("%USERPROFILE%") {
            return path.replace("%USERPROFILE%", &userprofile);
        }
        if path.contains("$USERPROFILE") {
            return path.replace("$USERPROFILE", &userprofile);
        }
    }
    
    // Try Unix/Linux HOME variable
    if let Ok(home) = std::env::var("HOME") {
        if path.contains("$HOME") {
            return path.replace("$HOME", &home);
        }
        if path.starts_with("~") {
            return path.replacen("~", &home, 1);
        }
    }
    
    // Windows USERNAME for path construction
    if let Ok(username) = std::env::var("USERNAME") {
        if path.contains("$USERNAME") {
            return path.replace("$USERNAME", &username);
        }
    }
    
    // Unix/Linux USER for path construction
    if let Ok(user) = std::env::var("USER") {
        if path.contains("$USER") {
            return path.replace("$USER", &user);
        }
    }
    
    path.to_string()
}

fn load_wallet_address() -> Option<String> {
    // Method 1: Check environment variable first
    if let Ok(addr) = std::env::var("WALLET_ADDRESS") {
        return Some(addr);
    }
    
    // Method 2: Build platform-specific wallet paths
    let mut wallet_paths = Vec::new();
    
    // Windows paths
    if cfg!(target_os = "windows") {
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            wallet_paths.push(PathBuf::from(format!("{}\\time-coin-node\\data\\wallets\\node.json", userprofile)));
        }
        if let Ok(username) = std::env::var("USERNAME") {
            wallet_paths.push(PathBuf::from(format!("C:\\Users\\{}\\time-coin-node\\data\\wallets\\node.json", username)));
        }
        wallet_paths.push(PathBuf::from("C:\\var\\lib\\time-coin\\wallets\\node.json"));
        wallet_paths.push(PathBuf::from("C:\\time-coin-node\\data\\wallets\\node.json"));
        
        // Add current directory relative paths
        if let Ok(current_dir) = std::env::current_dir() {
            wallet_paths.push(current_dir.join("data\\wallets\\node.json"));
            wallet_paths.push(current_dir.join("..\\..\\data\\wallets\\node.json"));
        }
    }
    
    // Linux/Unix paths
    if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        wallet_paths.push(PathBuf::from("/var/lib/time-coin/wallets/node.json"));
        wallet_paths.push(PathBuf::from("/root/time-coin-node/data/wallets/node.json"));
        
        if let Ok(home) = std::env::var("HOME") {
            wallet_paths.push(PathBuf::from(format!("{}/time-coin-node/data/wallets/node.json", home)));
            wallet_paths.push(PathBuf::from(format!("{}/.time-coin/wallets/node.json", home)));
        }
        
        if let Ok(user) = std::env::var("USER") {
            wallet_paths.push(PathBuf::from(format!("/home/{}/time-coin-node/data/wallets/node.json", user)));
        }
        
        // Add current directory relative paths
        if let Ok(current_dir) = std::env::current_dir() {
            wallet_paths.push(current_dir.join("data/wallets/node.json"));
            wallet_paths.push(current_dir.join("../../data/wallets/node.json"));
        }
    }
    
    // Try each wallet path
    for wallet_path in &wallet_paths {
        if let Ok(contents) = fs::read_to_string(wallet_path) {
            if let Ok(wallet) = serde_json::from_str::<WalletData>(&contents) {
                return Some(wallet.address);
            }
        }
    }
    
    // Method 3: Try to find it via config file
    let mut config_paths = Vec::new();
    
    // Windows config paths
    if cfg!(target_os = "windows") {
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            config_paths.push(PathBuf::from(format!("{}\\time-coin-node\\config\\testnet.toml", userprofile)));
        }
        config_paths.push(PathBuf::from("C:\\time-coin-node\\config\\testnet.toml"));
    }
    
    // Linux config paths
    if cfg!(target_os = "linux") || cfg!(target_os = "macos") {
        config_paths.push(PathBuf::from("/root/time-coin-node/config/testnet.toml"));
        
        if let Ok(home) = std::env::var("HOME") {
            config_paths.push(PathBuf::from(format!("{}/time-coin-node/config/testnet.toml", home)));
        }
    }
    
    for config_path in config_paths {
        if let Ok(contents) = fs::read_to_string(&config_path) {
            if let Ok(config) = toml::from_str::<Config>(&contents) {
                if let Some(node) = config.node {
                    if let Some(data_dir) = node.data_dir {
                        let expanded = expand_path(&data_dir);
                        
                        // Try both Unix and Windows path separators
                        let wallet_paths = vec![
                            PathBuf::from(format!("{}/wallets/node.json", expanded)),
                            PathBuf::from(format!("{}\\wallets\\node.json", expanded)),
                        ];
                        
                        for wallet_path in wallet_paths {
                            if let Ok(contents) = fs::read_to_string(&wallet_path) {
                                if let Ok(wallet) = serde_json::from_str::<WalletData>(&contents) {
                                    return Some(wallet.address);
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    None
}

fn run_dashboard(client: &Client, running: Arc<AtomicBool>) -> Result<(), Box<dyn std::error::Error>> {
    let mut first_run = true;
    
    // Try to load the masternode's wallet address
    let masternode_address = load_wallet_address();
    
    if let Some(ref addr) = masternode_address {
        println!("Found masternode wallet: {}", addr);
        thread::sleep(Duration::from_secs(2)); // Show the message briefly
    }
    
    while running.load(Ordering::SeqCst) {
        // Clear screen only on first run, then just move cursor
        if first_run {
            execute!(io::stdout(), Clear(ClearType::All), MoveTo(0, 0))?;
            first_run = false;
        } else {
            execute!(io::stdout(), MoveTo(0, 0))?;
        }
        
        // Fetch data
        let blockchain_info = fetch_blockchain_info(client);
        let peers = fetch_peers(client);
        let mempool = fetch_mempool(client);
        let wallet = if let Some(ref addr) = masternode_address {
            fetch_wallet_balance(client, addr)
        } else {
            None
        };
        
        // Ping peers and sort by fastest
        let peers_with_ping = if let Some(ref peer_data) = peers {
            ping_and_sort_peers(client, &peer_data.peers)
        } else {
            Vec::new()
        };
        
        // Display dashboard (each line clears itself)
        display_dashboard(&blockchain_info, &peers, &peers_with_ping, &mempool, &wallet, &masternode_address);
        
        io::stdout().flush()?;
        
        // Wait 5 seconds, but check running flag more frequently
        for _ in 0..50 {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            thread::sleep(Duration::from_millis(100));
        }
    }
    
    Ok(())
}

fn fetch_blockchain_info(client: &Client) -> Option<BlockchainInfo> {
    client.get("http://localhost:24101/blockchain/info")
        .send()
        .ok()?
        .json()
        .ok()
}

fn fetch_peers(client: &Client) -> Option<PeersResponse> {
    client.get("http://localhost:24101/peers")
        .send()
        .ok()?
        .json()
        .ok()
}

fn fetch_mempool(client: &Client) -> Option<MempoolStatus> {
    client.get("http://localhost:24101/mempool/status")
        .send()
        .ok()?
        .json()
        .ok()
}

fn fetch_wallet_balance(client: &Client, address: &str) -> Option<WalletBalance> {
    client.get(format!("http://localhost:24101/balance/{}", address))
        .send()
        .ok()?
        .json()
        .ok()
}

fn ping_peer(client: &Client, peer: &PeerInfo) -> Option<u64> {
    // Extract IP from address (format: "ip:port")
    let ip = peer.address.split(':').next()?;
    let url = format!("http://{}:24101/blockchain/info", ip);
    
    let start = Instant::now();
    match client.get(&url).send() {
        Ok(_) => Some(start.elapsed().as_millis() as u64),
        Err(_) => None,
    }
}

fn ping_and_sort_peers(client: &Client, peers: &[PeerInfo]) -> Vec<PeerWithPing> {
    let mut peers_with_ping: Vec<PeerWithPing> = peers
        .iter()
        .filter_map(|peer| {
            ping_peer(client, peer).map(|ping_ms| PeerWithPing {
                address: peer.address.clone(),
                ping_ms,
            })
        })
        .collect();
    
    // Sort by ping time (fastest first)
    peers_with_ping.sort_by_key(|p| p.ping_ms);
    
    peers_with_ping
}

// Helper macro to print and clear rest of line
macro_rules! println_clear {
    ($($arg:tt)*) => {
        print!("{}", CLEAR_LINE);
        println!($($arg)*);
    };
}

fn display_dashboard(
    blockchain: &Option<BlockchainInfo>,
    peers: &Option<PeersResponse>,
    peers_with_ping: &[PeerWithPing],
    mempool: &Option<MempoolStatus>,
    wallet: &Option<WalletBalance>,
    masternode_address: &Option<String>,
) {
    println_clear!("{}{}╔═════════════════════════════════════════════════════╗{}", BOLD, CYAN, RESET);
    println_clear!("{}{}║    TIME COIN MASTERNODE DASHBOARD [TESTNET]         ║{}", BOLD, CYAN, RESET);
    println_clear!("{}{}╚═════════════════════════════════════════════════════╝{}\n", BOLD, CYAN, RESET);
    
    // Wallet Balance (automatically detected)
    println_clear!("{}💰 Wallet Balance:{}", BOLD, RESET);
    if let Some(wallet_info) = wallet {
        println_clear!("   Address: {}...{}", 
            &wallet_info.address[..12], 
            &wallet_info.address[wallet_info.address.len()-8..]
        );
        
        // Calculate balance_time if not provided by API
        let balance_display = if let Some(ref balance_time) = wallet_info.balance_time {
            balance_time.clone()
        } else {
            format!("{:.8} TIME", wallet_info.balance as f64 / 100_000_000.0)
        };
        
        println_clear!("   Balance: {}{}{}", GREEN, balance_display, RESET);
        
        if wallet_info.pending > 0 {
            let pending_time = wallet_info.pending as f64 / 100_000_000.0;
            println_clear!("   Pending: {}{:.8} TIME{}", YELLOW, pending_time, RESET);
        }
    } else if masternode_address.is_some() {
        println_clear!("   {}⚠️  Unable to fetch wallet balance{}", YELLOW, RESET);
    } else {
        println_clear!("   {}⚠️  Wallet not found{}", YELLOW, RESET);
    }
    println_clear!();
    
    // Blockchain Status
    println_clear!("{}📊 Blockchain Status:{}", BOLD, RESET);
    if let Some(info) = blockchain {
        println_clear!("   Height: {}", info.height);
        let status = if info.synced || info.height > 0 {
            format!("{}Synchronized ✓{}", GREEN, RESET)
        } else {
            format!("{}Syncing...{}", YELLOW, RESET)
        };
        println_clear!("   Status: {}", status);
    } else {
        println_clear!("   {}⚠️  Unable to fetch blockchain info{}", YELLOW, RESET);
    }
    println_clear!();
    
    // Network Connections
    println_clear!("{}🌐 Network Connections:{}", BOLD, RESET);
    if let Some(peer_data) = peers {
        println_clear!("   {} peer(s) connected", peer_data.count);
        
        if peers_with_ping.is_empty() {
            println_clear!("   {}⚠️  Unable to ping peers{}", YELLOW, RESET);
        } else {
            // Show top 5 fastest peers
            let display_count = peers_with_ping.len().min(5);
            println_clear!("\n   Top {} Fastest Peers:", display_count);
            for (i, peer) in peers_with_ping.iter().take(5).enumerate() {
                let (color, indicator) = if peer.ping_ms < 50 {
                    (GREEN, "●") // Green circle for fast
                } else if peer.ping_ms < 150 {
                    (YELLOW, "●") // Yellow circle for medium
                } else {
                    (RED, "●") // Red circle for slow
                };
                println_clear!("   {}{}{} {}. {} ({}ms)", 
                    color,
                    indicator,
                    RESET,
                    i + 1, 
                    peer.address, 
                    peer.ping_ms
                );
            }
            
            if peer_data.count > 5 {
                println_clear!("   ... and {} more peers", peer_data.count - 5);
            }
        }
    } else {
        println_clear!("   {}⚠️  Unable to fetch peer info{}", YELLOW, RESET);
    }
    println_clear!();
    
    // Mempool Status
    println_clear!("{}📝 Mempool Status:{}", BOLD, RESET);
    if let Some(mem) = mempool {
        println_clear!("   Transactions: {}", mem.size);
        let capacity = 10000; // Default capacity
        let usage_pct = (mem.size as f64 / capacity as f64 * 100.0) as usize;
        let bar = make_bar_graph(usage_pct);
        println_clear!("   Capacity: {} {}%", bar, usage_pct);
    } else {
        println_clear!("   {}⚠️  Unable to fetch mempool info{}", YELLOW, RESET);
    }
    println_clear!();
    
    // Timestamp
    let now = Utc::now();
    println_clear!("Last updated: {} UTC", now.format("%Y-%m-%d %H:%M:%S"));
    println_clear!();
    println_clear!("{}Press Ctrl+C to exit{}", YELLOW, RESET);
    
    // Add extra cleared lines to cover any leftover text from previous render
    for _ in 0..3 {
        println_clear!();
    }
}

fn make_bar_graph(percentage: usize) -> String {
    let bar_length = 20;
    let filled = (percentage as f64 / 100.0 * bar_length as f64).round() as usize;
    let filled = filled.min(bar_length);
    
    let color = if percentage < 50 {
        GREEN
    } else if percentage < 80 {
        YELLOW
    } else {
        RED
    };
    
    format!("{}[{}{}]{}", 
        color,
        "█".repeat(filled),
        "░".repeat(bar_length - filled),
        RESET
    )
}