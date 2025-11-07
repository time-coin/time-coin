use reqwest::blocking::Client;
use serde::Deserialize;
use std::{io::{self, Write}, thread, time::{Duration, Instant}, sync::Arc, sync::atomic::{AtomicBool, Ordering}};
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
    })?;
    
    // Enter alternate screen buffer and hide cursor
    execute!(io::stdout(), EnterAlternateScreen, Hide)?;
    
    let result = run_dashboard(&client, running);
    
    // Cleanup: Leave alternate screen buffer and show cursor on exit
    execute!(io::stdout(), Show, LeaveAlternateScreen)?;
    
    println!("Dashboard stopped gracefully.");
    
    result
}

fn run_dashboard(client: &Client, running: Arc<AtomicBool>) -> Result<(), Box<dyn std::error::Error>> {
    let mut first_run = true;
    
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
        
        // Ping peers and sort by fastest
        let peers_with_ping = if let Some(ref peer_data) = peers {
            ping_and_sort_peers(client, &peer_data.peers)
        } else {
            Vec::new()
        };
        
        // Display dashboard (each line clears itself)
        display_dashboard(&blockchain_info, &peers, &peers_with_ping, &mempool);
        
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
) {
    println_clear!("{}{}╔═════════════════════════════════════════════════════╗{}", BOLD, CYAN, RESET);
    println_clear!("{}{}║    TIME COIN MASTERNODE DASHBOARD [TESTNET]         ║{}", BOLD, CYAN, RESET);
    println_clear!("{}{}╚═════════════════════════════════════════════════════╝{}\n", BOLD, CYAN, RESET);
    
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