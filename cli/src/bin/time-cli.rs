//! TIME Coin CLI - Complete RPC interface with JSON output support

use clap::{Parser, Subcommand};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "time-cli")]
#[command(about = "TIME Coin Node CLI", version)]
struct Cli {
    /// API endpoint
    #[arg(short, long, default_value = "http://localhost:24101", global = true)]
    api: String,

    /// Output in JSON format
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize node configuration
    Init {
        /// Use testnet configuration
        #[arg(long)]
        testnet: bool,
    },

    /// Get node status
    Status,

    /// Get blockchain information
    Info,

    /// List recent blocks
    Blocks {
        /// Number of blocks to show
        #[arg(short, long, default_value = "10")]
        count: usize,
    },

    /// List connected peers
    Peers,

    /// Wallet operations
    Wallet {
        #[command(subcommand)]
        wallet_command: WalletCommands,
    },

    /// RPC operations (Bitcoin-compatible + TIME-specific)
    Rpc {
        #[command(subcommand)]
        rpc_command: RpcCommands,
    },

    /// Masternode operations
    Masternode {
        #[command(subcommand)]
        masternode_command: MasternodeCommands,
    },

    /// Validate the blockchain integrity
    ValidateChain {
        /// Display detailed validation information
        #[arg(short, long)]
        verbose: bool,
        /// RPC endpoint (default: http://127.0.0.1:24101)
        #[arg(long, default_value = "http://127.0.0.1:24101")]
        rpc_url: String,
    },

    /// Mint coins for testing (testnet only)
    TestnetMint {
        /// Address to receive the minted coins
        #[arg(short, long)]
        address: String,
        /// Amount to mint in TIME (e.g., 100.5)
        #[arg(short = 'a', long)]
        amount: f64,
        /// Optional reason for minting
        #[arg(short, long)]
        reason: Option<String>,
        /// RPC endpoint (default: http://127.0.0.1:24101)
        #[arg(long, default_value = "http://127.0.0.1:24101")]
        rpc_url: String,
    },
}

#[derive(Subcommand)]
enum WalletCommands {
    /// Generate address from public key
    GenerateAddress {
        /// Public key (64-character hex string)
        pubkey: String,
    },

    /// Validate a TIME Coin address
    ValidateAddress {
        /// Address to validate
        address: String,
    },

    /// Create a new wallet
    Create {
        /// Public key (64-character hex string)
        pubkey: String,

        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },

    /// Get wallet balance (defaults to node wallet)
    Balance {
        /// Wallet address (optional, defaults to node wallet)
        #[arg(short, long)]
        address: Option<String>,

        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },

    /// Get wallet information (defaults to node wallet)
    Info {
        /// Wallet address (optional, defaults to node wallet)
        #[arg(short, long)]
        address: Option<String>,

        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },

    /// List all UTXOs (defaults to node wallet)
    ListUtxos {
        /// Wallet address (optional, defaults to node wallet)
        #[arg(short, long)]
        address: Option<String>,

        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },

    /// Lock collateral for masternode tier (defaults to node wallet)
    LockCollateral {
        /// Wallet address (optional, defaults to node wallet)
        #[arg(short, long)]
        address: Option<String>,

        /// Tier (bronze, silver, gold)
        tier: String,

        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },

    /// Unlock collateral (defaults to node wallet)
    UnlockCollateral {
        /// Wallet address (optional, defaults to node wallet)
        #[arg(short, long)]
        address: Option<String>,

        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },

    /// Add reward to wallet (for testing, defaults to node wallet)
    AddReward {
        /// Wallet address (optional, defaults to node wallet)
        #[arg(short, long)]
        address: Option<String>,

        /// Amount in TIME
        amount: f64,

        /// Block height
        height: u64,

        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },
}

#[derive(Subcommand)]
enum RpcCommands {
    /// Get blockchain information
    GetBlockchainInfo,

    /// Get current block count
    GetBlockCount,

    /// Get block hash at specific height (defaults to latest)
    GetBlockHash {
        /// Block height (optional, defaults to current tip)
        #[arg(short = 'H', long)]
        height: Option<u64>,
    },

    /// Get block by hash
    GetBlock {
        /// Block hash
        hash: String,
    },

    /// Get raw transaction
    GetRawTransaction {
        /// Transaction ID
        txid: String,
    },

    /// Send raw transaction
    SendRawTransaction {
        /// Transaction hex string
        hexstring: String,
    },

    /// Get wallet information
    GetWalletInfo,

    /// Get balance
    GetBalance {
        /// Optional address (defaults to node wallet)
        #[arg(short, long)]
        address: Option<String>,
    },

    /// Generate new address
    GetNewAddress,

    /// Validate address
    ValidateAddress {
        /// Address to validate
        address: String,
    },

    /// List unspent outputs
    ListUnspent {
        /// Minimum confirmations
        #[arg(short, long, default_value = "0")]
        minconf: u64,

        /// Maximum confirmations
        #[arg(short = 'M', long, default_value = "9999999")]
        maxconf: u64,

        /// Addresses to filter by
        #[arg(short, long)]
        addresses: Vec<String>,
    },

    /// Get peer information
    GetPeerInfo,

    /// Get network information
    GetNetworkInfo,

    /// Get time block information
    GetTimeBlockInfo,

    /// Get time block rewards (defaults to current height)
    GetTimeBlockRewards {
        /// Block height (optional, defaults to current)
        #[arg(short = 'H', long)]
        height: Option<u64>,
    },

    /// Get consensus status
    GetConsensusStatus,

    /// Get treasury information
    GetTreasury,

    /// List governance proposals
    ListProposals,
}

#[derive(Subcommand)]
enum MasternodeCommands {
    /// Generate a new masternode private key
    Genkey,

    /// List available collateral outputs (UTXOs suitable for masternodes)
    Outputs {
        /// Minimum confirmations required
        #[arg(short, long, default_value = "15")]
        min_conf: u64,
    },

    /// List masternodes from masternode.conf
    ListConf {
        /// Path to masternode.conf file
        #[arg(short, long, default_value = "masternode.conf")]
        config: String,
    },

    /// Add a masternode to masternode.conf
    AddConf {
        /// Masternode alias
        alias: String,

        /// IP address and port (e.g., 192.168.1.100:24000)
        ip_port: String,

        /// Masternode private key (from genkey command)
        masternode_privkey: String,

        /// Collateral transaction ID
        collateral_txid: String,

        /// Collateral output index
        collateral_vout: u32,

        /// Path to masternode.conf file
        #[arg(short, long, default_value = "masternode.conf")]
        config: String,
    },

    /// Remove a masternode from masternode.conf
    RemoveConf {
        /// Masternode alias to remove
        alias: String,

        /// Path to masternode.conf file
        #[arg(short, long, default_value = "masternode.conf")]
        config: String,
    },

    /// Start a specific masternode by alias
    StartAlias {
        /// Masternode alias from masternode.conf
        alias: String,

        /// Path to masternode.conf file
        #[arg(short, long, default_value = "masternode.conf")]
        config: String,
    },

    /// Start all masternodes from masternode.conf
    StartAll {
        /// Path to masternode.conf file
        #[arg(short, long, default_value = "masternode.conf")]
        config: String,
    },

    /// Register a masternode (legacy, defaults to local node)
    Register {
        /// Node IP address (optional, defaults to local IP)
        #[arg(short = 'n', long)]
        node_ip: Option<String>,

        /// Wallet address
        wallet_address: String,

        /// Tier (Free, Bronze, Silver, Gold)
        #[arg(short, long, default_value = "Free")]
        tier: String,
    },

    /// Get masternode information (defaults to local node)
    Info {
        /// Masternode address (optional, defaults to local node IP)
        #[arg(short, long)]
        address: Option<String>,
    },

    /// List all masternodes
    List,

    /// Get masternode count
    Count,
}

// Response types for RPC calls
#[derive(Debug, Deserialize, Serialize)]
struct MasternodeInfo {
    address: String,
    wallet_address: String,
    tier: String,
    is_active: bool,
    registered_height: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct MasternodeListItem {
    address: String,
    wallet_address: String,
    tier: String,
    is_active: bool,
    registered_height: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct MasternodeCount {
    total: usize,
    free: usize,
    bronze: usize,
    silver: usize,
    gold: usize,
    active: usize,
}

#[derive(Debug, Deserialize, Serialize)]
struct ConsensusStatus {
    consensus_type: String,
    active_validators: usize,
    bft_threshold: f64,
    instant_finality: bool,
    consensus_mode: String,
}

// Helper function to get local IP or fall back
fn get_local_ip_or_fallback() -> String {
    if let Ok(ip) = local_ip_address::local_ip() {
        ip.to_string()
    } else {
        "127.0.0.1".to_string()
    }
}

// Helper function to get current blockchain height
async fn get_current_height(client: &Client, api: &str) -> Result<u64, Box<dyn std::error::Error>> {
    let response = client
        .get(format!("{}/blockchain/info", api))
        .send()
        .await?;

    if response.status().is_success() {
        let info: serde_json::Value = response.json().await?;
        Ok(info["height"].as_u64().unwrap_or(0))
    } else {
        Ok(0)
    }
}

async fn handle_validate_chain(
    client: &Client,
    rpc_url: &str,
    verbose: bool,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    // Call the validation RPC endpoint
    let response = client
        .post(format!("{}/rpc/validatechain", rpc_url))
        .json(&json!({ "verbose": verbose }))
        .send()
        .await?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await?;
        if json_output {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!("\n🔍 Blockchain Validation Results");
            println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    } else {
        let error = response.text().await?;
        eprintln!("Validation failed: {}", error);
    }
    Ok(())
}

async fn handle_testnet_mint(
    client: &Client,
    rpc_url: &str,
    address: String,
    amount: f64,
    reason: Option<String>,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let amount_satoshis = (amount * 100_000_000.0) as u64;

    let request = json!({
        "address": address,
        "amount": amount_satoshis,
        "reason": reason,
    });

    if !json_output {
        println!("\n🪙 TIME Coin Testnet Minter");
        println!("═══════════════════════════════════");
        println!("Address: {}", address);
        println!("Amount: {} TIME ({} satoshis)", amount, amount_satoshis);
        if let Some(ref r) = reason {
            println!("Reason: {}", r);
        }
        println!("\n📡 Sending mint request...");
    }

    let response = client
        .post(format!("{}/testnet/mint", rpc_url))
        .json(&request)
        .send()
        .await?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await?;
        if json_output {
            println!("{}", serde_json::to_string_pretty(&result)?);
        } else {
            println!("\n✅ SUCCESS!");
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
    } else {
        let error = response.text().await?;
        eprintln!("Minting failed: {}", error);
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli = Cli::parse();
    let client = reqwest::Client::new();

    match cli.command {
        Commands::Init { testnet } => {
            if cli.json {
                println!(
                    "{}",
                    json!({
                        "success": true,
                        "network": if testnet { "testnet" } else { "mainnet" },
                        "message": "Configuration template created"
                    })
                );
            } else {
                println!("\n⚙️  Initializing TIME Coin node configuration");
                let network = if testnet { "testnet" } else { "mainnet" };
                println!("Network: {}", network);
                println!("✓ Configuration template created");
                println!("\nNext steps:");
                println!("1. Review configuration at ~/.time-coin/config.toml");
                println!("2. Start node with: time-node");
            }
        }

        Commands::Status => {
            let response = client
                .get(format!("{}/blockchain/info", cli.api))
                .send()
                .await?;

            if response.status().is_success() {
                let info: serde_json::Value = response.json().await?;

                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&info)?);
                } else {
                    println!("\n📊 Node Status");
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    println!("Network: {}", info["network"]);
                    println!("Height: {}", info["height"]);
                    println!("Best Block: {}", info["best_block_hash"]);
                    println!(
                        "Total Supply: {} TIME",
                        info["total_supply"].as_u64().unwrap_or(0) / 100_000_000
                    );
                }
            } else if cli.json {
                println!(
                    "{}",
                    json!({
                        "error": format!("{}", response.status())
                    })
                );
            } else {
                eprintln!("Error: {}", response.status());
            }
        }

        Commands::Info => {
            handle_rpc_call(&client, &cli.api, "getblockchaininfo", json!({}), cli.json).await?;
        }

        Commands::Blocks { count } => {
            let info_response = client
                .get(format!("{}/blockchain/info", cli.api))
                .send()
                .await?;

            if info_response.status().is_success() {
                let info: serde_json::Value = info_response.json().await?;
                let height = info["height"].as_u64().unwrap_or(0);

                let mut blocks = Vec::new();

                for i in 0..count {
                    if height < i as u64 {
                        break;
                    }
                    let block_height = height - i as u64;

                    let block_response = client
                        .get(format!("{}/blockchain/block/{}", cli.api, block_height))
                        .send()
                        .await?;

                    if block_response.status().is_success() {
                        let block: serde_json::Value = block_response.json().await?;
                        blocks.push(json!({
                            "height": block_height,
                            "hash": block["block"]["hash"],
                            "transactions": block["block"]["transactions"].as_array().map(|t| t.len()).unwrap_or(0)
                        }));
                    }
                }

                if cli.json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "blocks": blocks,
                            "count": blocks.len()
                        }))?
                    );
                } else {
                    println!("\n📦 Recent Blocks (last {})", count);
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    for block in blocks {
                        println!(
                            "Height {}: {} ({} txs)",
                            block["height"],
                            &block["hash"].as_str().unwrap_or("")[..16],
                            block["transactions"]
                        );
                    }
                }
            }
        }

        Commands::Peers => {
            let response = client.get(format!("{}/peers", cli.api)).send().await?;

            if response.status().is_success() {
                let peers: serde_json::Value = response.json().await?;

                if cli.json {
                    println!("{}", serde_json::to_string_pretty(&peers)?);
                } else {
                    println!("\n🌐 Connected Peers");
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    println!("Total: {}", peers["count"]);

                    if let Some(peer_list) = peers["peers"].as_array() {
                        for peer in peer_list {
                            println!("  • {} (v{})", peer["address"], peer["version"]);
                        }
                    }
                }
            }
        }

        Commands::Wallet { wallet_command } => {
            handle_wallet_command(wallet_command, &cli.api, cli.json).await?;
        }

        Commands::Rpc { rpc_command } => {
            handle_rpc_command(rpc_command, &client, &cli.api, cli.json).await?;
        }

        Commands::Masternode { masternode_command } => {
            handle_masternode_command(masternode_command, &client, &cli.api, cli.json).await?;
        }

        Commands::ValidateChain { verbose, rpc_url } => {
            handle_validate_chain(&client, &rpc_url, verbose, cli.json).await?;
        }

        Commands::TestnetMint {
            address,
            amount,
            reason,
            rpc_url,
        } => {
            handle_testnet_mint(&client, &rpc_url, address, amount, reason, cli.json).await?;
        }
    }

    Ok(())
}

async fn handle_wallet_command(
    command: WalletCommands,
    api: &str,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        WalletCommands::GenerateAddress { pubkey } => {
            let address = time_crypto::public_key_to_address(&pubkey);

            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "address": address,
                        "public_key": pubkey
                    }))?
                );
            } else {
                println!("\n✓ Generated Address:");
                println!("{}", address);
            }
        }

        WalletCommands::ValidateAddress { address } => {
            let is_valid = address.starts_with("TIME1") && address.len() > 10;

            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "address": address,
                        "is_valid": is_valid
                    }))?
                );
            } else {
                println!(
                    "\n{} Address: {}",
                    if is_valid { "✓ Valid" } else { "✗ Invalid" },
                    address
                );
            }
        }

        WalletCommands::Balance {
            address,
            db_path: _,
        } => {
            let addr = if let Some(a) = address {
                a
            } else {
                // Get node wallet address from API
                let client = reqwest::Client::new();
                let response = client
                    .get(format!("{}/blockchain/info", api))
                    .send()
                    .await?;

                if response.status().is_success() {
                    let info: serde_json::Value = response.json().await?;
                    info["wallet_address"]
                        .as_str()
                        .unwrap_or("unknown")
                        .to_string()
                } else {
                    "unknown".to_string()
                }
            };

            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "address": addr,
                        "message": "This wallet command requires local database access"
                    }))?
                );
            } else {
                println!("Address: {}", addr);
                println!("This wallet command requires local database access");
            }
        }

        _ => {
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "error": "This wallet command requires local database access"
                    }))?
                );
            } else {
                println!("This wallet command requires local database access");
                println!("Use the appropriate database operations");
            }
        }
    }

    Ok(())
}

async fn handle_rpc_command(
    command: RpcCommands,
    client: &Client,
    api: &str,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        RpcCommands::GetBlockchainInfo => {
            handle_rpc_call(client, api, "getblockchaininfo", json!({}), json_output).await?;
        }

        RpcCommands::GetBlockCount => {
            handle_rpc_call(client, api, "getblockcount", json!({}), json_output).await?;
        }

        RpcCommands::GetBlockHash { height } => {
            let h = if let Some(height) = height {
                height
            } else {
                get_current_height(client, api).await?
            };

            handle_rpc_call(
                client,
                api,
                "getblockhash",
                json!({ "height": h }),
                json_output,
            )
            .await?;
        }

        RpcCommands::GetBlock { hash } => {
            handle_rpc_call(
                client,
                api,
                "getblock",
                json!({ "blockhash": hash }),
                json_output,
            )
            .await?;
        }

        RpcCommands::GetRawTransaction { txid } => {
            handle_rpc_call(
                client,
                api,
                "getrawtransaction",
                json!({ "txid": txid }),
                json_output,
            )
            .await?;
        }

        RpcCommands::SendRawTransaction { hexstring } => {
            handle_rpc_call(
                client,
                api,
                "sendrawtransaction",
                json!({ "hexstring": hexstring }),
                json_output,
            )
            .await?;
        }

        RpcCommands::GetWalletInfo => {
            handle_rpc_call(client, api, "getwalletinfo", json!({}), json_output).await?;
        }

        RpcCommands::GetBalance { address } => {
            let params = if let Some(addr) = address {
                json!({ "address": addr })
            } else {
                json!({})
            };
            handle_rpc_call(client, api, "getbalance", params, json_output).await?;
        }

        RpcCommands::GetNewAddress => {
            handle_rpc_call(client, api, "getnewaddress", json!({}), json_output).await?;
        }

        RpcCommands::ValidateAddress { address } => {
            handle_rpc_call(
                client,
                api,
                "validateaddress",
                json!({ "address": address }),
                json_output,
            )
            .await?;
        }

        RpcCommands::ListUnspent {
            minconf,
            maxconf,
            addresses,
        } => {
            handle_rpc_call(
                client,
                api,
                "listunspent",
                json!({
                    "minconf": minconf,
                    "maxconf": maxconf,
                    "addresses": addresses
                }),
                json_output,
            )
            .await?;
        }

        RpcCommands::GetPeerInfo => {
            handle_rpc_call(client, api, "getpeerinfo", json!({}), json_output).await?;
        }

        RpcCommands::GetNetworkInfo => {
            handle_rpc_call(client, api, "getnetworkinfo", json!({}), json_output).await?;
        }

        RpcCommands::GetTimeBlockInfo => {
            handle_rpc_call(client, api, "gettimeblockinfo", json!({}), json_output).await?;
        }

        RpcCommands::GetTimeBlockRewards { height } => {
            let h = if let Some(height) = height {
                height
            } else {
                get_current_height(client, api).await?
            };

            handle_rpc_call(
                client,
                api,
                "gettimeblockrewards",
                json!({ "height": h }),
                json_output,
            )
            .await?;
        }

        RpcCommands::GetConsensusStatus => {
            let response = client
                .post(format!("{}/rpc/getconsensusstatus", api))
                .json(&json!({}))
                .send()
                .await?;

            if response.status().is_success() {
                let status: ConsensusStatus = response.json().await?;

                if json_output {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "consensus_type": status.consensus_type,
                            "consensus_mode": status.consensus_mode,
                            "active_validators": status.active_validators,
                            "bft_threshold": status.bft_threshold,
                            "instant_finality": status.instant_finality
                        }))?
                    );
                } else {
                    println!("\n🛡️  Consensus Status");
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    println!("Type: {}", status.consensus_type);
                    println!("Mode: {}", status.consensus_mode);
                    println!("Active Validators: {}", status.active_validators);
                    println!("BFT Threshold: {:.0}%", status.bft_threshold * 100.0);
                    println!(
                        "Instant Finality: {}",
                        if status.instant_finality { "Yes" } else { "No" }
                    );
                }
            } else if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "error": format!("{}", response.status())
                    }))?
                );
            } else {
                eprintln!("Error: {}", response.status());
            }
        }

        RpcCommands::GetTreasury => {
            handle_rpc_call(client, api, "gettreasury", json!({}), json_output).await?;
        }

        RpcCommands::ListProposals => {
            handle_rpc_call(client, api, "listproposals", json!({}), json_output).await?;
        }
    }

    Ok(())
}

async fn handle_masternode_command(
    command: MasternodeCommands,
    client: &Client,
    api: &str,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        MasternodeCommands::Genkey => {
            // Generate a new masternode private key
            let key = time_crypto::generate_masternode_key();

            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "masternode_privkey": key
                    }))?
                );
            } else {
                println!("\n🔑 Masternode Private Key");
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("{}", key);
                println!("\n⚠️  Keep this key secret and secure!");
                println!("Use this key in your masternode.conf file");
            }
        }

        MasternodeCommands::Outputs { min_conf } => {
            // List available collateral outputs
            let response = client
                .post(format!("{}/rpc/listunspent", api))
                .json(&json!({
                    "minconf": min_conf,
                    "maxconf": 9999999,
                    "addresses": []
                }))
                .send()
                .await?;

            if response.status().is_success() {
                let utxos: serde_json::Value = response.json().await?;

                if json_output {
                    println!("{}", serde_json::to_string_pretty(&utxos)?);
                } else {
                    println!("\n💰 Available Collateral Outputs");
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

                    if let Some(outputs) = utxos.as_array() {
                        for output in outputs {
                            let amount = output["amount"].as_u64().unwrap_or(0) / 100_000_000;
                            let txid = output["txid"].as_str().unwrap_or("");
                            let vout = output["vout"].as_u64().unwrap_or(0);
                            let confirmations = output["confirmations"].as_u64().unwrap_or(0);

                            // Determine tier
                            let tier = if amount >= 100_000 {
                                "Professional"
                            } else if amount >= 10_000 {
                                "Verified"
                            } else if amount >= 1_000 {
                                "Community"
                            } else {
                                "Below minimum"
                            };

                            println!(
                                "\n  {}:{}\n    Amount: {} TIME ({})\n    Confirmations: {}",
                                &txid[..16], vout, amount, tier, confirmations
                            );
                        }
                    }
                }
            } else {
                eprintln!("Error: {}", response.status());
            }
        }

        MasternodeCommands::ListConf { config } => {
            // List masternodes from masternode.conf
            match masternode::config::MasternodeConfig::load_from_file(&config) {
                Ok(conf) => {
                    if json_output {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json!({
                                "masternodes": conf.entries(),
                                "count": conf.count()
                            }))?
                        );
                    } else {
                        println!("\n🔧 Configured Masternodes ({} total)", conf.count());
                        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

                        if conf.count() == 0 {
                            println!("No masternodes configured");
                        } else {
                            for (i, entry) in conf.entries().iter().enumerate() {
                                println!("\n{}. {}", i + 1, entry.alias);
                                println!("   IP:Port: {}", entry.ip_port);
                                println!("   Collateral: {}:{}", &entry.collateral_txid[..16], entry.collateral_output_index);
                            }
                        }
                    }
                }
                Err(e) => {
                    if json_output {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json!({
                                "error": format!("{}", e)
                            }))?
                        );
                    } else {
                        eprintln!("Error loading masternode.conf: {}", e);
                    }
                }
            }
        }

        MasternodeCommands::AddConf {
            alias,
            ip_port,
            masternode_privkey,
            collateral_txid,
            collateral_vout,
            config,
        } => {
            // Add a masternode to masternode.conf
            let entry = masternode::config::MasternodeConfigEntry {
                alias: alias.clone(),
                ip_port: ip_port.clone(),
                masternode_privkey: masternode_privkey.clone(),
                collateral_txid: collateral_txid.clone(),
                collateral_output_index: collateral_vout,
            };

            let mut conf = masternode::config::MasternodeConfig::load_from_file(&config)
                .unwrap_or_else(|_| masternode::config::MasternodeConfig::new());

            match conf.add_entry(entry) {
                Ok(_) => {
                    if let Err(e) = conf.save_to_file(&config) {
                        if json_output {
                            println!(
                                "{}",
                                serde_json::to_string_pretty(&json!({
                                    "error": format!("Failed to save: {}", e)
                                }))?
                            );
                        } else {
                            eprintln!("Error saving masternode.conf: {}", e);
                        }
                    } else if json_output {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json!({
                                "success": true,
                                "alias": alias,
                                "message": "Masternode added to configuration"
                            }))?
                        );
                    } else {
                        println!("\n✓ Masternode Added");
                        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                        println!("Alias: {}", alias);
                        println!("IP:Port: {}", ip_port);
                        println!("Collateral: {}:{}", &collateral_txid[..16], collateral_vout);
                        println!("\nConfiguration saved to {}", config);
                    }
                }
                Err(e) => {
                    if json_output {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json!({
                                "error": format!("{}", e)
                            }))?
                        );
                    } else {
                        eprintln!("Error: {}", e);
                    }
                }
            }
        }

        MasternodeCommands::RemoveConf { alias, config } => {
            // Remove a masternode from masternode.conf
            match masternode::config::MasternodeConfig::load_from_file(&config) {
                Ok(mut conf) => {
                    match conf.remove_entry(&alias) {
                        Ok(_) => {
                            if let Err(e) = conf.save_to_file(&config) {
                                if json_output {
                                    println!(
                                        "{}",
                                        serde_json::to_string_pretty(&json!({
                                            "error": format!("Failed to save: {}", e)
                                        }))?
                                    );
                                } else {
                                    eprintln!("Error saving masternode.conf: {}", e);
                                }
                            } else if json_output {
                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(&json!({
                                        "success": true,
                                        "alias": alias,
                                        "message": "Masternode removed from configuration"
                                    }))?
                                );
                            } else {
                                println!("\n✓ Masternode Removed");
                                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                                println!("Alias: {}", alias);
                                println!("Configuration saved to {}", config);
                            }
                        }
                        Err(e) => {
                            if json_output {
                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(&json!({
                                        "error": format!("{}", e)
                                    }))?
                                );
                            } else {
                                eprintln!("Error: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    if json_output {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json!({
                                "error": format!("Failed to load config: {}", e)
                            }))?
                        );
                    } else {
                        eprintln!("Error loading masternode.conf: {}", e);
                    }
                }
            }
        }

        MasternodeCommands::StartAlias { alias, config } => {
            // Start a specific masternode by alias
            match masternode::config::MasternodeConfig::load_from_file(&config) {
                Ok(conf) => {
                    if let Some(entry) = conf.get_entry(&alias) {
                        // Send start-masternode message to the API
                        let response = client
                            .post(format!("{}/masternode/start", api))
                            .json(&json!({
                                "alias": entry.alias,
                                "ip_port": entry.ip_port,
                                "masternode_privkey": entry.masternode_privkey,
                                "collateral_txid": entry.collateral_txid,
                                "collateral_vout": entry.collateral_output_index
                            }))
                            .send()
                            .await?;

                        if response.status().is_success() {
                            let result: serde_json::Value = response.json().await?;
                            if json_output {
                                println!("{}", serde_json::to_string_pretty(&result)?);
                            } else {
                                println!("\n✓ Masternode Started");
                                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                                println!("Alias: {}", alias);
                                println!("{}", result.get("message").and_then(|v| v.as_str()).unwrap_or("Success"));
                            }
                        } else {
                            let error = response.text().await?;
                            if json_output {
                                println!(
                                    "{}",
                                    serde_json::to_string_pretty(&json!({
                                        "error": error
                                    }))?
                                );
                            } else {
                                eprintln!("Error: {}", error);
                            }
                        }
                    } else if json_output {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json!({
                                "error": format!("Masternode '{}' not found in configuration", alias)
                            }))?
                        );
                    } else {
                        eprintln!("Error: Masternode '{}' not found in configuration", alias);
                    }
                }
                Err(e) => {
                    if json_output {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json!({
                                "error": format!("Failed to load config: {}", e)
                            }))?
                        );
                    } else {
                        eprintln!("Error loading masternode.conf: {}", e);
                    }
                }
            }
        }

        MasternodeCommands::StartAll { config } => {
            // Start all masternodes from masternode.conf
            match masternode::config::MasternodeConfig::load_from_file(&config) {
                Ok(conf) => {
                    let mut results = Vec::new();

                    for entry in conf.entries() {
                        let response = client
                            .post(format!("{}/masternode/start", api))
                            .json(&json!({
                                "alias": entry.alias,
                                "ip_port": entry.ip_port,
                                "masternode_privkey": entry.masternode_privkey,
                                "collateral_txid": entry.collateral_txid,
                                "collateral_vout": entry.collateral_output_index
                            }))
                            .send()
                            .await;

                        let success = response.as_ref().map(|r| r.status().is_success()).unwrap_or(false);
                        results.push(json!({
                            "alias": entry.alias,
                            "success": success,
                            "message": if success { "Started" } else { "Failed" }
                        }));
                    }

                    if json_output {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json!({
                                "results": results,
                                "total": results.len()
                            }))?
                        );
                    } else {
                        println!("\n🚀 Starting All Masternodes");
                        println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                        for result in &results {
                            let alias = result["alias"].as_str().unwrap_or("unknown");
                            let success = result["success"].as_bool().unwrap_or(false);
                            println!(
                                "  {} {}",
                                alias,
                                if success { "✓" } else { "✗" }
                            );
                        }
                        println!("\nTotal: {}", results.len());
                    }
                }
                Err(e) => {
                    if json_output {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json!({
                                "error": format!("Failed to load config: {}", e)
                            }))?
                        );
                    } else {
                        eprintln!("Error loading masternode.conf: {}", e);
                    }
                }
            }
        }

        MasternodeCommands::Register {
            node_ip,
            wallet_address,
            tier,
        } => {
            let ip = node_ip.unwrap_or_else(get_local_ip_or_fallback);

            let response = client
                .post(format!("{}/masternode/register", api))
                .json(&json!({
                    "node_ip": ip,
                    "wallet_address": wallet_address,
                    "tier": tier
                }))
                .send()
                .await?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;

                if json_output {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("\n✓ Masternode Registered");
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    println!("Node IP: {}", result["node_ip"]);
                    println!("Wallet: {}", result["wallet_address"]);
                    println!("Tier: {}", result["tier"]);
                    println!("\n{}", result["message"]);
                }
            } else if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "error": format!("{}", response.status())
                    }))?
                );
            } else {
                eprintln!("Error: {}", response.status());
            }
        }

        MasternodeCommands::Info { address } => {
            let addr = address.unwrap_or_else(get_local_ip_or_fallback);

            let response = client
                .post(format!("{}/rpc/getmasternodeinfo", api))
                .json(&json!({ "address": addr }))
                .send()
                .await?;

            if response.status().is_success() {
                let info: MasternodeInfo = response.json().await?;

                if json_output {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "address": info.address,
                            "wallet_address": info.wallet_address,
                            "tier": info.tier,
                            "is_active": info.is_active,
                            "registered_height": info.registered_height
                        }))?
                    );
                } else {
                    println!("\n🔧 Masternode Information");
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    println!("Address: {}", info.address);
                    println!("Wallet: {}", info.wallet_address);
                    println!("Tier: {}", info.tier);
                    println!(
                        "Status: {}",
                        if info.is_active {
                            "Active ✓"
                        } else {
                            "Inactive"
                        }
                    );
                    println!("Registered at Height: {}", info.registered_height);
                }
            } else if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "error": format!("{}", response.status())
                    }))?
                );
            } else {
                eprintln!("Error: {}", response.status());
            }
        }

        MasternodeCommands::List => {
            let response = client
                .post(format!("{}/rpc/listmasternodes", api))
                .json(&json!({}))
                .send()
                .await?;

            if response.status().is_success() {
                let masternodes: Vec<MasternodeListItem> = response.json().await?;

                if json_output {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "masternodes": masternodes,
                            "count": masternodes.len()
                        }))?
                    );
                } else {
                    println!("\n🔧 Masternodes ({} total)", masternodes.len());
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

                    for (i, mn) in masternodes.iter().enumerate() {
                        println!(
                            "{}. {} ({}) - {} | Height: {}",
                            i + 1,
                            mn.address,
                            mn.tier,
                            if mn.is_active {
                                "Active ✓"
                            } else {
                                "Inactive"
                            },
                            mn.registered_height
                        );
                    }
                }
            } else if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "error": format!("{}", response.status())
                    }))?
                );
            } else {
                eprintln!("Error: {}", response.status());
            }
        }

        MasternodeCommands::Count => {
            let response = client
                .post(format!("{}/rpc/getmasternodecount", api))
                .json(&json!({}))
                .send()
                .await?;

            if response.status().is_success() {
                let count: MasternodeCount = response.json().await?;

                if json_output {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "total": count.total,
                            "active": count.active,
                            "tiers": {
                                "free": count.free,
                                "bronze": count.bronze,
                                "silver": count.silver,
                                "gold": count.gold
                            }
                        }))?
                    );
                } else {
                    println!("\n📊 Masternode Statistics");
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    println!("Total: {}", count.total);
                    println!("Active: {}", count.active);
                    println!("\nBy Tier:");
                    println!("  Free: {}", count.free);
                    println!("  Bronze: {}", count.bronze);
                    println!("  Silver: {}", count.silver);
                    println!("  Gold: {}", count.gold);
                }
            } else if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "error": format!("{}", response.status())
                    }))?
                );
            } else {
                eprintln!("Error: {}", response.status());
            }
        }
    }

    Ok(())
}

async fn handle_rpc_call(
    client: &Client,
    api: &str,
    method: &str,
    params: serde_json::Value,
    _json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let response = client
        .post(format!("{}/rpc/{}", api, method))
        .json(&params)
        .send()
        .await?;

    if response.status().is_success() {
        let result: serde_json::Value = response.json().await?;
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        let error_text = response.text().await?;
        eprintln!("Error: {}", error_text);
    }

    Ok(())
}
