#!/bin/bash
set -e

echo "Creating Cargo.toml files for all crates..."

# Helper function to create Cargo.toml
create_cargo_toml() {
    local crate_name=$1
    local crate_dir=$2
    
    cat > ${crate_dir}/Cargo.toml << EOF
[package]
name = "time-${crate_name}"
version = "0.1.0"
edition = "2021"

[dependencies]
EOF
    
    echo "// ${crate_name} module" > ${crate_dir}/src/lib.rs
}

# Create Cargo.toml for each library crate
create_cargo_toml "masternode" "masternode"
create_cargo_toml "network" "network"
create_cargo_toml "purchase" "purchase"
create_cargo_toml "wallet" "wallet"
create_cargo_toml "api" "api"
create_cargo_toml "storage" "storage"
create_cargo_toml "crypto" "crypto"

# CLI is a binary crate (different structure)
cat > cli/Cargo.toml << 'EOF'
[package]
name = "time-cli"
version = "0.1.0"
edition = "2021"

[[bin]]
name = "time-node"
path = "src/main.rs"

[dependencies]
clap = { version = "4.4", features = ["derive"] }
EOF

cat > cli/src/main.rs << 'EOF'
use clap::Parser;

#[derive(Parser)]
#[command(name = "time-node")]
#[command(about = "TIME Coin Node", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser)]
enum Commands {
    /// Start the node
    Start,
    /// Get node status
    Status,
    /// Initialize configuration
    Init,
}

fn main() {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Start => {
            println!("Starting TIME node...");
        }
        Commands::Status => {
            println!("Checking node status...");
        }
        Commands::Init => {
            println!("Initializing node configuration...");
        }
    }
}
EOF

# Update workspace Cargo.toml
cat > Cargo.toml << 'EOF'
[workspace]
members = [
    "core",
    "masternode",
    "network",
    "purchase",
    "wallet",
    "api",
    "cli",
    "storage",
    "crypto",
]
resolver = "2"

[workspace.package]
version = "0.1.0"
edition = "2021"
authors = ["TIME Coin Developers"]
license = "MIT"
repository = "https://github.com/time-coin/time-coin"

[workspace.dependencies]
# Async runtime
tokio = { version = "1.35", features = ["full"] }
async-trait = "0.1"

# Serialization
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
bincode = "1.3"

# Cryptography
ed25519-dalek = "2.1"
sha3 = "0.10"
rand = "0.8"

# Networking
libp2p = "0.53"

# Database
rocksdb = "0.21"

# Web framework
axum = "0.7"

# Utilities
thiserror = "1.0"
anyhow = "1.0"
tracing = "0.1"
clap = { version = "4.4", features = ["derive"] }
EOF

echo "✅ All crates configured!"
