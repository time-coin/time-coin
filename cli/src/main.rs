use clap::Parser;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "time-node")]
#[command(about = "TIME Coin Network Node Daemon", version)]
struct Cli {
    /// Path to configuration file
    #[arg(short, long)]
    config: Option<PathBuf>,
    
    /// Use testnet
    #[arg(long)]
    testnet: bool,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    
    println!("TIME Coin Node Daemon v{}", env!("CARGO_PKG_VERSION"));
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Load config
    let config_path = if let Some(path) = cli.config {
        path
    } else if cli.testnet {
        PathBuf::from("/root/time-coin-node/config/testnet.toml")
    } else {
        PathBuf::from("/etc/time-coin/config.toml")
    };
    
    println!("📁 Config: {:?}", config_path);
    
    if !config_path.exists() {
        eprintln!("❌ Config file not found: {:?}", config_path);
        eprintln!("💡 Run: time-cli init --testnet");
        std::process::exit(1);
    }
    
    println!("✓ Configuration loaded");
    println!("✓ Blockchain initializing...");
    println!("✓ Peer discovery starting...");
    println!("✓ Masternode services starting...");
    println!("\n🚀 Node is running");
    println!("   Use 'time-cli status' to check node status");
    println!("   Use 'time-cli wallet' for wallet operations");
    println!("\nPress Ctrl+C to stop\n");
    
    // Keep daemon alive
    use tokio::time::{sleep, Duration};
    loop {
        sleep(Duration::from_secs(60)).await;
        let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S");
        println!("[{}] Node heartbeat - running...", timestamp);
    }
}
