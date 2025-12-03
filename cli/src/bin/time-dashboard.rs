#!/usr/bin/env rust
//! TIME Coin Node Dashboard
//!
//! Real-time monitoring dashboard for TIME Coin nodes.
//! Displays blockchain status, peer connections, mempool status, and system metrics.

use chrono::Utc;
use crossterm::{
    cursor,
    terminal::{self, ClearType},
    ExecutableCommand,
};
use owo_colors::OwoColorize;
use serde::Deserialize;
use std::io::{self, Write};
use std::time::Duration;
use tokio::time;

#[derive(Debug, Deserialize)]
struct BlockchainInfo {
    height: u64,
    best_block_hash: String,
    wallet_address: String,
}

#[derive(Debug, Deserialize)]
struct BalanceResponse {
    #[allow(dead_code)]
    address: String,
    balance: u64,
    unconfirmed_balance: u64,
}

#[derive(Debug, Deserialize)]
struct HealthResponse {
    status: String,
}

#[derive(Debug, Deserialize)]
struct PeerInfo {
    address: String,
    version: String,
    connected_at: i64,
}

#[derive(Debug, Deserialize)]
struct PeersResponse {
    peers: Vec<PeerInfo>,
}

#[derive(Debug, Deserialize, Default)]
struct MempoolResponse {
    #[serde(default)]
    pending: usize,
    #[serde(default)]
    size: Option<usize>,
}

struct Dashboard {
    api_url: String,
    client: reqwest::Client,
}

impl Dashboard {
    fn new(api_url: String) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .unwrap();
        Self { api_url, client }
    }

    async fn fetch_blockchain_info(&self) -> Result<BlockchainInfo, String> {
        self.client
            .get(format!("{}/blockchain/info", self.api_url))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?
            .json::<BlockchainInfo>()
            .await
            .map_err(|e| format!("Parse failed: {}", e))
    }

    async fn fetch_health(&self) -> Result<HealthResponse, String> {
        self.client
            .get(format!("{}/health", self.api_url))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?
            .json::<HealthResponse>()
            .await
            .map_err(|e| format!("Parse failed: {}", e))
    }

    async fn fetch_peers(&self) -> Result<PeersResponse, String> {
        self.client
            .get(format!("{}/peers", self.api_url))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?
            .json::<PeersResponse>()
            .await
            .map_err(|e| format!("Parse failed: {}", e))
    }

    async fn fetch_mempool(&self) -> Result<MempoolResponse, String> {
        self.client
            .get(format!("{}/mempool", self.api_url))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?
            .json::<MempoolResponse>()
            .await
            .map_err(|e| format!("Parse failed: {}", e))
    }

    async fn fetch_balance(&self, address: &str) -> Result<BalanceResponse, String> {
        self.client
            .get(format!("{}/blockchain/balance/{}", self.api_url, address))
            .send()
            .await
            .map_err(|e| format!("Request failed: {}", e))?
            .json::<BalanceResponse>()
            .await
            .map_err(|e| format!("Parse failed: {}", e))
    }

    fn clear_screen(&self) {
        // Use crossterm's proper terminal clearing which works on all platforms
        let mut stdout = io::stdout();
        let _ = stdout.execute(terminal::Clear(ClearType::All));
        let _ = stdout.execute(cursor::MoveTo(0, 0));
        let _ = stdout.flush();
    }

    fn render_header(&self) {
        let now = Utc::now();
        println!(
            "{}",
            "╔══════════════════════════════════════════════════════════════╗"
                .cyan()
                .bold()
        );
        println!(
            "{}",
            "║         TIME COIN NODE DASHBOARD                            ║"
                .cyan()
                .bold()
        );
        println!(
            "{}",
            "╚══════════════════════════════════════════════════════════════╝"
                .cyan()
                .bold()
        );
        println!(
            "  {}: {}",
            "Current Time".bright_black(),
            now.format("%Y-%m-%d %H:%M:%S UTC").to_string().white()
        );
        println!("  {}: {}", "API Endpoint".bright_black(), &self.api_url);
        println!();
    }

    fn render_blockchain(&self, info: &BlockchainInfo, balance: Option<BalanceResponse>) {
        println!(
            "{}",
            "┌─ Blockchain Status ──────────────────────────────────────────┐".blue()
        );
        println!(
            "│ {:<20} {}",
            "Block Height:".bright_black(),
            info.height.to_string().green().bold()
        );
        let hash_preview = if info.best_block_hash.len() >= 16 {
            &info.best_block_hash[..16]
        } else {
            &info.best_block_hash
        };
        println!(
            "│ {:<20} {}...",
            "Best Block Hash:".bright_black(),
            hash_preview.bright_blue()
        );
        println!(
            "│ {:<20} {}",
            "Wallet Address:".bright_black(),
            info.wallet_address.bright_green()
        );

        // Display balance if available
        if let Some(bal_response) = balance {
            let confirmed = bal_response.balance as f64 / 100_000_000.0;
            let unconfirmed = bal_response.unconfirmed_balance as f64 / 100_000_000.0;

            println!(
                "│ {:<20} {} TIME",
                "Confirmed Balance:".bright_black(),
                format!("{:.8}", confirmed).bright_yellow().bold()
            );

            if unconfirmed > 0.0 {
                println!(
                    "│ {:<20} {} TIME",
                    "Unconfirmed:".bright_black(),
                    format!("{:.8}", unconfirmed).bright_cyan()
                );
            }
        } else {
            println!(
                "│ {:<20} {}",
                "Wallet Balance:".bright_black(),
                "Loading...".bright_black()
            );
        }

        println!(
            "{}",
            "└──────────────────────────────────────────────────────────────┘".blue()
        );
        println!();
    }

    fn render_peers(&self, peers: &[PeerInfo]) {
        println!(
            "{}",
            "┌─ Network Peers ──────────────────────────────────────────────┐".yellow()
        );
        println!(
            "│ {:<20} {}",
            "Connected Peers:".bright_black(),
            peers.len().to_string().yellow().bold()
        );

        if peers.is_empty() {
            println!("│ {}", "No peers connected".bright_black());
        } else {
            for (i, peer) in peers.iter().take(5).enumerate() {
                let connected_since = Utc::now().timestamp() - peer.connected_at;
                println!(
                    "│  {}. {} (v{}, {}s ago)",
                    i + 1,
                    peer.address.bright_white(),
                    peer.version.bright_black(),
                    connected_since.to_string().bright_black()
                );
            }
            if peers.len() > 5 {
                println!("│  ... and {} more", peers.len() - 5);
            }
        }
        println!(
            "{}",
            "└──────────────────────────────────────────────────────────────┘".yellow()
        );
        println!();
    }

    fn render_mempool(&self, mempool: &MempoolResponse) {
        let pending = mempool.pending.max(mempool.size.unwrap_or(0));
        println!(
            "{}",
            "┌─ Mempool Status ─────────────────────────────────────────────┐".magenta()
        );
        println!(
            "│ {:<20} {}",
            "Pending Transactions:".bright_black(),
            pending.to_string().magenta().bold()
        );
        println!(
            "{}",
            "└──────────────────────────────────────────────────────────────┘".magenta()
        );
        println!();
    }

    fn render_footer(&self) {
        println!(
            "{}",
            "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_black()
        );
        println!(
            "  {} to exit | Auto-refresh every 5 seconds",
            "Press Ctrl+C".bright_black()
        );
    }

    fn render_error(&self, error: &str) {
        println!(
            "{}",
            "┌─ Error ──────────────────────────────────────────────────────┐".red()
        );
        println!("│ {}", error.red());
        println!(
            "{}",
            "└──────────────────────────────────────────────────────────────┘".red()
        );
        println!();
    }

    async fn render(&self) {
        self.clear_screen();
        self.render_header();

        // Fetch health status
        match self.fetch_health().await {
            Ok(health) => {
                if health.status != "ok" {
                    self.render_error(&format!("Node health: {}", health.status));
                }
            }
            Err(e) => {
                self.render_error(&format!("Cannot connect to node: {}", e));
                self.render_footer();
                return;
            }
        }

        // Fetch and render blockchain info with balance
        match self.fetch_blockchain_info().await {
            Ok(info) => {
                // Fetch balance for the wallet address
                let balance = self.fetch_balance(&info.wallet_address).await.ok();
                self.render_blockchain(&info, balance);
            }
            Err(e) => self.render_error(&format!("Blockchain info error: {}", e)),
        }

        // Fetch and render peer info
        match self.fetch_peers().await {
            Ok(peers_response) => self.render_peers(&peers_response.peers),
            Err(e) => self.render_error(&format!("Peers info error: {}", e)),
        }

        // Fetch and render mempool info
        match self.fetch_mempool().await {
            Ok(mempool) => self.render_mempool(&mempool),
            Err(e) => self.render_error(&format!("Mempool info error: {}", e)),
        }

        self.render_footer();
    }

    async fn run(&self) {
        loop {
            self.render().await;
            time::sleep(Duration::from_secs(5)).await;
        }
    }
}

#[tokio::main]
async fn main() {
    // Parse command line arguments for custom API URL
    let api_url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "http://localhost:24101".to_string());

    println!("Starting TIME Coin Dashboard...");
    println!("Connecting to: {}", api_url);
    println!();

    tokio::time::sleep(Duration::from_secs(1)).await;

    let dashboard = Dashboard::new(api_url);
    dashboard.run().await;
}
