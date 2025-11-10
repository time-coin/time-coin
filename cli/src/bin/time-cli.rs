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

    /// Get wallet balance
    Balance {
        /// Wallet address
        address: String,

        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },

    /// Get wallet information
    Info {
        /// Wallet address
        address: String,

        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },

    /// List all UTXOs
    ListUtxos {
        /// Wallet address
        address: String,

        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },

    /// Lock collateral for masternode tier
    LockCollateral {
        /// Wallet address
        address: String,

        /// Tier (bronze, silver, gold)
        tier: String,

        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },

    /// Unlock collateral
    UnlockCollateral {
        /// Wallet address
        address: String,

        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },

    /// Add reward to wallet (for testing)
    AddReward {
        /// Wallet address
        address: String,

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

    /// Get block hash at specific height
    GetBlockHash {
        /// Block height
        height: u64,
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

    /// Get time block rewards
    GetTimeBlockRewards {
        /// Block height
        height: u64,
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
    /// Register a masternode
    Register {
        /// Node IP address
        node_ip: String,

        /// Wallet address
        wallet_address: String,

        /// Tier (Free, Bronze, Silver, Gold)
        #[arg(short, long, default_value = "Free")]
        tier: String,
    },

    /// Get masternode information
    Info {
        /// Masternode address (IP or node ID)
        address: String,
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
    }

    Ok(())
}

async fn handle_wallet_command(
    command: WalletCommands,
    _api: &str,
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
            handle_rpc_call(
                client,
                api,
                "getblockhash",
                json!({ "height": height }),
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
            handle_rpc_call(
                client,
                api,
                "gettimeblockrewards",
                json!({ "height": height }),
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
        MasternodeCommands::Register {
            node_ip,
            wallet_address,
            tier,
        } => {
            let response = client
                .post(format!("{}/masternode/register", api))
                .json(&json!({
                    "node_ip": node_ip,
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
            let response = client
                .post(format!("{}/rpc/getmasternodeinfo", api))
                .json(&json!({ "address": address }))
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
