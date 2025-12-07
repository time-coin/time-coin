//! TIME Coin CLI - Complete RPC interface with JSON output support

use clap::{Parser, Subcommand};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::path::PathBuf;
use time_masternode as masternode;

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

    /// Treasury operations
    Treasury {
        #[command(subcommand)]
        treasury_command: TreasuryCommands,
    },

    /// Mempool operations
    Mempool {
        #[command(subcommand)]
        mempool_command: MempoolCommands,
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

    /// Treasury grant proposals (governance system)
    Proposal {
        #[command(subcommand)]
        command: ProposalCommands,
    },

    /// Instant finality operations
    InstantFinality {
        #[command(subcommand)]
        command: InstantFinalityCommands,
    },
}

#[derive(Subcommand)]
enum ProposalCommands {
    /// Create a new treasury grant proposal
    Create {
        /// Recipient wallet address
        #[arg(short = 'a', long)]
        address: String,
        /// Amount in TIME (e.g., 100.5)
        #[arg(short = 'm', long)]
        amount: f64,
        /// Reason for the grant
        #[arg(short, long)]
        reason: String,
    },

    /// Vote on a proposal
    Vote {
        /// Proposal ID
        #[arg(short, long)]
        id: String,
        /// Approve the proposal
        #[arg(long)]
        approve: bool,
    },

    /// List all proposals
    List {
        /// Show only pending proposals
        #[arg(long)]
        pending: bool,
    },

    /// Get details of a specific proposal
    Get {
        /// Proposal ID
        id: String,
    },
}

#[derive(Subcommand)]
enum InstantFinalityCommands {
    /// Get transaction finality status
    Status {
        /// Transaction ID
        txid: String,
        /// RPC endpoint (default: http://127.0.0.1:24101)
        #[arg(long, default_value = "http://127.0.0.1:24101")]
        rpc_url: String,
    },

    /// List approved transactions ready for block inclusion
    Approved {
        /// RPC endpoint (default: http://127.0.0.1:24101)
        #[arg(long, default_value = "http://127.0.0.1:24101")]
        rpc_url: String,
    },

    /// Get instant finality statistics
    Stats {
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

    /// Show UTXO consolidation statistics
    UtxoStats {
        /// Wallet address (optional, defaults to node wallet)
        #[arg(short, long)]
        address: Option<String>,

        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },

    /// Consolidate UTXOs into fewer outputs
    ConsolidateUtxos {
        /// Wallet address (optional, defaults to node wallet)
        #[arg(short, long)]
        address: Option<String>,

        /// Maximum number of UTXOs to consolidate in one transaction
        #[arg(long, default_value = "100")]
        max_utxos: usize,

        /// Transaction fee in TIME
        #[arg(long, default_value = "0.001")]
        fee: f64,

        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,

        /// Skip confirmation prompt
        #[arg(short, long)]
        yes: bool,
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

    /// Send coins from one address to another
    SendFrom {
        /// Source address (from)
        from: String,

        /// Destination address (to)
        to: String,

        /// Amount to send in TIME
        amount: f64,

        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },

    /// Send coins to an address (automatically selects from available UTXOs)
    Send {
        /// Destination address
        to: String,

        /// Amount to send in TIME
        amount: f64,

        /// Database path
        #[arg(long, default_value = "/var/lib/time-coin/wallets")]
        db_path: PathBuf,
    },

    /// Rescan the blockchain to update wallet balance from UTXO set
    Rescan {
        /// Wallet address to rescan (optional, defaults to node wallet)
        #[arg(short, long)]
        address: Option<String>,
    },

    /// Reindex the entire blockchain and rebuild UTXO set from all blocks
    /// Use this if balance is incorrect after restart
    Reindex,
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

#[derive(Subcommand)]
enum TreasuryCommands {
    /// Get treasury information
    Info,

    /// List all treasury proposals
    ListProposals,

    /// Get specific proposal details
    GetProposal {
        /// Proposal ID
        proposal_id: String,
    },

    /// Submit a new treasury proposal
    Propose {
        /// Proposal title
        #[arg(short, long)]
        title: String,

        /// Proposal description
        #[arg(short, long)]
        description: String,

        /// Recipient address
        #[arg(short, long)]
        recipient: String,

        /// Amount in TIME (e.g., 1000.0)
        #[arg(long)]
        amount: f64,

        /// Voting period in days (default: 14)
        #[arg(short = 'p', long, default_value = "14")]
        voting_period: u64,
    },

    /// Vote on a treasury proposal
    Vote {
        /// Proposal ID
        proposal_id: String,

        /// Vote choice (yes, no, abstain)
        #[arg(value_parser = ["yes", "no", "abstain"])]
        vote: String,

        /// Masternode ID (optional, defaults to local node)
        #[arg(short, long)]
        masternode_id: Option<String>,
    },
}

#[derive(Subcommand)]
enum MempoolCommands {
    /// Get mempool status (transaction count and IDs)
    Status,

    /// List all mempool transactions with full details
    List,

    /// Clear all transactions from mempool
    Clear,
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

async fn handle_proposal_command(
    command: ProposalCommands,
    client: &Client,
    api: &str,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        ProposalCommands::Create {
            address,
            amount,
            reason,
        } => {
            let amount_satoshis = (amount * 100_000_000.0) as u64;

            let request = json!({
                "recipient": address,
                "amount": amount_satoshis,
                "reason": reason,
            });

            if !json_output {
                println!("\n📜 Creating Treasury Grant Proposal");
                println!("═══════════════════════════════════════");
                println!("Recipient: {}", address);
                println!("Amount: {} TIME ({} satoshis)", amount, amount_satoshis);
                println!("Reason: {}", reason);
                println!("\n📡 Submitting proposal...");
            }

            let response = client
                .post(format!("{}/proposals/create", api))
                .json(&request)
                .send()
                .await?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                if json_output {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("\n✅ Proposal Created!");
                    println!("ID: {}", result["id"].as_str().unwrap_or("unknown"));
                    println!("\nMasternodes can now vote with:");
                    println!(
                        "  time-cli proposal vote --id {} --approve",
                        result["id"].as_str().unwrap_or("ID")
                    );
                }
            } else {
                eprintln!("Failed to create proposal: {}", response.text().await?);
            }
        }

        ProposalCommands::Vote { id, approve } => {
            let request = json!({
                "proposal_id": id,
                "approve": approve,
            });

            if !json_output {
                println!("\n🗳️  Voting on Proposal");
                println!("═══════════════════════════════");
                println!("Proposal ID: {}", id);
                println!("Vote: {}", if approve { "✅ APPROVE" } else { "❌ REJECT" });
                println!("\n📡 Submitting vote...");
            }

            let response = client
                .post(format!("{}/proposals/vote", api))
                .json(&request)
                .send()
                .await?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                if json_output {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("\n✅ Vote Recorded!");
                    if let Some(status) = result["status"].as_str() {
                        println!("Proposal Status: {}", status);
                    }
                }
            } else {
                eprintln!("Failed to vote: {}", response.text().await?);
            }
        }

        ProposalCommands::List { pending } => {
            let url = if pending {
                format!("{}/proposals/list?pending=true", api)
            } else {
                format!("{}/proposals/list", api)
            };

            let response = client.get(&url).send().await?;

            if response.status().is_success() {
                let proposals: serde_json::Value = response.json().await?;

                if json_output {
                    println!("{}", serde_json::to_string_pretty(&proposals)?);
                } else {
                    println!("\n📋 Treasury Grant Proposals");
                    println!("═══════════════════════════════════════");

                    if let Some(list) = proposals["proposals"].as_array() {
                        if list.is_empty() {
                            println!("No proposals found.");
                        } else {
                            for proposal in list {
                                println!(
                                    "\n🆔 ID: {}",
                                    proposal["id"].as_str().unwrap_or("unknown")
                                );
                                println!(
                                    "   Recipient: {}",
                                    proposal["recipient"].as_str().unwrap_or("unknown")
                                );
                                println!(
                                    "   Amount: {} TIME",
                                    proposal["amount"].as_u64().unwrap_or(0) as f64 / 100_000_000.0
                                );
                                println!(
                                    "   Status: {}",
                                    proposal["status"].as_str().unwrap_or("unknown")
                                );
                                println!(
                                    "   Votes: {} for, {} against",
                                    proposal["votes_for"]
                                        .as_array()
                                        .map(|v| v.len())
                                        .unwrap_or(0),
                                    proposal["votes_against"]
                                        .as_array()
                                        .map(|v| v.len())
                                        .unwrap_or(0)
                                );
                                println!(
                                    "   Reason: {}",
                                    proposal["reason"].as_str().unwrap_or("")
                                );
                            }
                        }
                    }
                }
            } else {
                eprintln!("Failed to list proposals: {}", response.text().await?);
            }
        }

        ProposalCommands::Get { id } => {
            let response = client
                .get(format!("{}/proposals/{}", api, id))
                .send()
                .await?;

            if response.status().is_success() {
                let proposal: serde_json::Value = response.json().await?;

                if json_output {
                    println!("{}", serde_json::to_string_pretty(&proposal)?);
                } else {
                    println!("\n📜 Proposal Details");
                    println!("═══════════════════════════════════════");
                    println!("ID: {}", proposal["id"].as_str().unwrap_or("unknown"));
                    println!(
                        "Proposer: {}",
                        proposal["proposer"].as_str().unwrap_or("unknown")
                    );
                    println!(
                        "Recipient: {}",
                        proposal["recipient"].as_str().unwrap_or("unknown")
                    );
                    println!(
                        "Amount: {} TIME",
                        proposal["amount"].as_u64().unwrap_or(0) as f64 / 100_000_000.0
                    );
                    println!(
                        "Status: {}",
                        proposal["status"].as_str().unwrap_or("unknown")
                    );
                    println!("Reason: {}", proposal["reason"].as_str().unwrap_or(""));
                    println!(
                        "\nVotes For ({}):",
                        proposal["votes_for"]
                            .as_array()
                            .map(|v| v.len())
                            .unwrap_or(0)
                    );
                    if let Some(votes) = proposal["votes_for"].as_array() {
                        for vote in votes {
                            println!("  ✅ {}", vote.as_str().unwrap_or("unknown"));
                        }
                    }
                    println!(
                        "\nVotes Against ({}):",
                        proposal["votes_against"]
                            .as_array()
                            .map(|v| v.len())
                            .unwrap_or(0)
                    );
                    if let Some(votes) = proposal["votes_against"].as_array() {
                        for vote in votes {
                            println!("  ❌ {}", vote.as_str().unwrap_or("unknown"));
                        }
                    }
                }
            } else {
                eprintln!("Failed to get proposal: {}", response.text().await?);
            }
        }
    }

    Ok(())
}

async fn handle_instant_finality_command(
    command: InstantFinalityCommands,
    client: &Client,
    api: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        InstantFinalityCommands::Status { txid, rpc_url } => {
            let api_url = if api != "http://127.0.0.1:24101" {
                api
            } else {
                &rpc_url
            };

            let response = client
                .post(format!("{}/instant-finality/status", api_url))
                .json(&json!({ "txid": txid }))
                .send()
                .await?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;

                println!("\n⚡ Transaction Finality Status");
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("TXID: {}", txid);

                if let Some(status) = result.get("status") {
                    if status.is_null() {
                        println!("Status: ❓ Not found in instant finality system");
                    } else if status.as_str() == Some("Pending") {
                        println!("Status: ⏳ Pending validation");
                    } else if status.as_str() == Some("Validated") {
                        println!("Status: 🔄 Validated, awaiting quorum");
                    } else if let Some(approved) = status.get("Approved") {
                        let votes = approved.get("votes").and_then(|v| v.as_u64()).unwrap_or(0);
                        let total = approved
                            .get("total_nodes")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        println!("Status: ✅ APPROVED");
                        println!(
                            "Votes: {}/{} ({:.1}%)",
                            votes,
                            total,
                            (votes as f64 / total as f64) * 100.0
                        );
                    } else if let Some(rejected) = status.get("Rejected") {
                        let reason = rejected
                            .get("reason")
                            .and_then(|v| v.as_str())
                            .unwrap_or("Unknown");
                        println!("Status: ❌ REJECTED");
                        println!("Reason: {}", reason);
                    } else if let Some(confirmed) = status.get("Confirmed") {
                        let height = confirmed
                            .get("block_height")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);
                        println!("Status: 🎉 CONFIRMED in block #{}", height);
                    }
                } else {
                    println!("Status: ❓ Unknown");
                }
            } else {
                eprintln!("❌ Error: {}", response.status());
            }
        }

        InstantFinalityCommands::Approved { rpc_url } => {
            let api_url = if api != "http://127.0.0.1:24101" {
                api
            } else {
                &rpc_url
            };

            let response = client
                .get(format!("{}/instant-finality/approved", api_url))
                .send()
                .await?;

            if response.status().is_success() {
                let transactions: Vec<serde_json::Value> = response.json().await?;

                println!("\n⚡ Approved Transactions (Ready for Block)");
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

                if transactions.is_empty() {
                    println!("No approved transactions pending block inclusion");
                } else {
                    println!("Total: {}", transactions.len());
                    println!();
                    for (i, tx) in transactions.iter().enumerate() {
                        let txid = tx.get("txid").and_then(|v| v.as_str()).unwrap_or("unknown");
                        let amount: u64 = tx
                            .get("outputs")
                            .and_then(|v| v.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|out| out.get("amount"))
                            .and_then(|v| v.as_u64())
                            .unwrap_or(0);

                        println!(
                            "{}. {} - {} TIME",
                            i + 1,
                            &txid[..16],
                            amount as f64 / 100_000_000.0
                        );
                    }
                }
            } else {
                eprintln!("❌ Error: {}", response.status());
            }
        }

        InstantFinalityCommands::Stats { rpc_url } => {
            let api_url = if api != "http://127.0.0.1:24101" {
                api
            } else {
                &rpc_url
            };

            let response = client
                .get(format!("{}/instant-finality/stats", api_url))
                .send()
                .await?;

            if response.status().is_success() {
                let stats: serde_json::Value = response.json().await?;

                println!("\n⚡ Instant Finality Statistics");
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!(
                    "Pending transactions: {}",
                    stats
                        .get("pending_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                );
                println!(
                    "Approved (cached): {}",
                    stats
                        .get("approved_count")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                );
                println!(
                    "Active masternodes: {}",
                    stats
                        .get("active_masternodes")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0)
                );
                println!(
                    "Quorum threshold: {}%",
                    stats
                        .get("quorum_threshold")
                        .and_then(|v| v.as_u64())
                        .unwrap_or(67)
                );
            } else {
                eprintln!("❌ Error: {}", response.status());
            }
        }
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
                println!("1. Review configuration at ~/.timecoin/config.toml");
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

        Commands::Treasury { treasury_command } => {
            handle_treasury_command(treasury_command, &client, &cli.api, cli.json).await?;
        }

        Commands::Mempool { mempool_command } => {
            handle_mempool_command(mempool_command, &client, &cli.api, cli.json).await?;
        }

        Commands::ValidateChain { verbose, rpc_url } => {
            handle_validate_chain(&client, &rpc_url, verbose, cli.json).await?;
        }

        Commands::Proposal { command } => {
            handle_proposal_command(command, &client, &cli.api, cli.json).await?;
        }

        Commands::InstantFinality { command } => {
            handle_instant_finality_command(command, &client, &cli.api).await?;
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
            let client = reqwest::Client::new();

            let addr = if let Some(a) = address {
                a
            } else {
                // Get node wallet address from API
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

            // Get balance from API
            let response = client
                .get(format!("{}/balance/{}", api, addr))
                .send()
                .await?;

            if response.status().is_success() {
                // Try to parse as JSON object first, fallback to plain number
                let balance: u64 = match response.json::<serde_json::Value>().await {
                    Ok(json_val) => {
                        // Try as object with "balance" field
                        if let Some(bal) = json_val.get("balance").and_then(|v| v.as_u64()) {
                            bal
                        } else {
                            // Try as plain number
                            json_val.as_u64().unwrap_or_default()
                        }
                    }
                    Err(_) => 0,
                };
                let balance_time = balance as f64 / 100_000_000.0;

                if json_output {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "address": addr,
                            "balance": balance_time,
                            "balance_satoshis": balance
                        }))?
                    );
                } else {
                    println!("Address: {}", addr);
                    println!("Balance: {} TIME", balance_time);
                }
            } else {
                let error = response.text().await?;
                if json_output {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "address": addr,
                            "error": error
                        }))?
                    );
                } else {
                    println!("Address: {}", addr);
                    println!("Error: {}", error);
                }
            }
        }

        WalletCommands::SendFrom {
            from,
            to,
            amount,
            db_path: _,
        } => {
            // Create transaction via API
            let client = reqwest::Client::new();
            let response = client
                .post(format!("{}/wallet/send", api))
                .json(&json!({
                    "from": from,
                    "to": to,
                    "amount": amount
                }))
                .send()
                .await?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;

                if json_output {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("\n✓ Transaction created successfully");
                    println!("From:   {}", from);
                    println!("To:     {}", to);
                    println!("Amount: {} TIME", amount);
                    if let Some(txid) = result.get("txid") {
                        println!("TxID:   {}", txid);
                    }
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
                    println!("✗ Failed to send transaction: {}", error);
                }
            }
        }

        WalletCommands::Send {
            to,
            amount,
            db_path: _,
        } => {
            // Convert TIME amount to smallest unit (TIME has 8 decimals like Bitcoin)
            const TIME_UNIT: u64 = 100_000_000; // 1 TIME = 100,000,000 units
            let amount_units = (amount * TIME_UNIT as f64) as u64;

            // Load wallet from node.json
            let data_dir = get_data_dir();
            let wallet_path = format!("{}/node.json", data_dir);

            let wallet_data = tokio::fs::read_to_string(&wallet_path).await.map_err(|e| {
                format!("Failed to load wallet from {}: {}", wallet_path, e)
            })?;

            let wallet_json: serde_json::Value = serde_json::from_str(&wallet_data)
                .map_err(|e| format!("Failed to parse wallet JSON: {}", e))?;

            let from_address = wallet_json["address"]
                .as_str()
                .ok_or_else(|| "No address found in wallet".to_string())?;

            // Create transaction locally via API (which will sign it)
            let client = reqwest::Client::new();
            let response = client
                .post(format!("{}/transaction/create", api))
                .json(&json!({
                    "from": from_address,
                    "to": to,
                    "amount": amount_units
                }))
                .send()
                .await?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;

                if json_output {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("\n⏳ Transaction submitted for approval...");
                    println!("To:     {}", to);
                    println!("Amount: {} TIME", amount);

                    if let Some(txid_val) = result.get("txid") {
                        let txid = txid_val.as_str().unwrap_or("");
                        println!("TxID:   {}", txid);

                        // Poll for instant finality status
                        println!("\n🔄 Waiting for masternode consensus...");

                        let start = std::time::Instant::now();
                        let max_wait = std::time::Duration::from_secs(10);
                        let mut attempts = 0;
                        let mut last_status = None;

                        while start.elapsed() < max_wait {
                            attempts += 1;

                            // Check transaction status
                            if let Ok(status_resp) = client
                                .post(format!("{}/consensus/tx-status", api))
                                .json(&json!({"txid": txid}))
                                .send()
                                .await
                            {
                                if let Ok(status_result) =
                                    status_resp.json::<serde_json::Value>().await
                                {
                                    if let Some(status) = status_result.get("status") {
                                        last_status = Some(status.clone());

                                        // Check if finalized
                                        if let Some(status_obj) = status.as_object() {
                                            if status_obj.contains_key("Finalized") {
                                                let elapsed = start.elapsed();
                                                println!("\n✅ APPROVED by BFT consensus");
                                                println!(
                                                    "Transaction finalized in {:.1} seconds",
                                                    elapsed.as_secs_f64()
                                                );
                                                println!("\n💡 Balance updated immediately - no need to wait for block confirmation");
                                                break;
                                            } else if status_obj.contains_key("Declined") {
                                                let reason = status_obj
                                                    .get("Declined")
                                                    .and_then(|d| d.get("reason"))
                                                    .and_then(|r| r.as_str())
                                                    .unwrap_or("Unknown reason");
                                                println!("\n❌ DECLINED by masternode consensus");
                                                println!("Reason: {}", reason);
                                                println!("Your coins remain in your wallet");
                                                break;
                                            }
                                        }

                                        // Show progress for pending
                                        if attempts % 2 == 0 {
                                            print!(".");
                                            use std::io::Write;
                                            std::io::stdout().flush().ok();
                                        }
                                    }
                                }
                            }

                            tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                        }

                        // Timeout handling
                        if start.elapsed() >= max_wait {
                            if let Some(status) = last_status {
                                if let Some(status_obj) = status.as_object() {
                                    if status_obj.contains_key("Pending") {
                                        println!("\n\n⏰ Consensus taking longer than expected");
                                        println!(
                                            "Transaction is still pending masternode approval"
                                        );
                                        println!("\n💡 Check status with: time-cli instant-finality status --txid {}", txid);
                                    }
                                }
                            } else {
                                println!("\n\n⚠️  Unable to verify consensus status");
                                println!("Transaction was broadcast but status is unknown");
                                println!("\n💡 Check status with: time-cli instant-finality status --txid {}", txid);
                            }
                        }
                    }
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
                    println!("✗ Failed to send transaction: {}", error);
                }
            }
        }

        WalletCommands::Rescan { address } => {
            // Get the wallet address to rescan
            let client = reqwest::Client::new();
            let addr = if let Some(a) = address {
                a
            } else {
                // Get node wallet address from API
                let response = client
                    .get(format!("{}/blockchain/info", api))
                    .send()
                    .await?;

                if response.status().is_success() {
                    let info: serde_json::Value = response.json().await?;
                    info["wallet_address"].as_str().unwrap_or("").to_string()
                } else {
                    if json_output {
                        println!(
                            "{}",
                            serde_json::to_string_pretty(&json!({
                                "error": "Failed to get node wallet address"
                            }))?
                        );
                    } else {
                        println!("✗ Failed to get node wallet address");
                    }
                    return Ok(());
                }
            };

            if addr.is_empty() {
                if json_output {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "error": "No wallet address found"
                        }))?
                    );
                } else {
                    println!("✗ No wallet address found");
                }
                return Ok(());
            }

            if !json_output {
                println!("\n🔍 Rescanning blockchain for address: {}", addr);
                println!("This will update your balance from the UTXO set...\n");
            }

            // Call the wallet sync API endpoint to rescan
            let response = client
                .post(format!("{}/wallet/sync", api))
                .json(&json!({
                    "addresses": vec![addr.clone()]
                }))
                .send()
                .await?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;

                if json_output {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    let balance = result["total_balance"].as_u64().unwrap_or(0);
                    let utxo_count = result["utxos"]
                        .as_object()
                        .and_then(|obj| obj.get(&addr))
                        .and_then(|arr| arr.as_array())
                        .map(|arr| arr.len())
                        .unwrap_or(0);
                    let current_height = result["current_height"].as_u64().unwrap_or(0);

                    // Convert from smallest units to TIME (8 decimals)
                    let balance_time = balance as f64 / 100_000_000.0;

                    println!("✅ Rescan complete!");
                    println!("Address:        {}", addr);
                    println!("Balance:        {} TIME", balance_time);
                    println!("UTXOs:          {}", utxo_count);
                    println!("Current Height: {}", current_height);
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
                    println!("✗ Failed to rescan: {}", error);
                }
            }
        }

        WalletCommands::Reindex => {
            if !json_output {
                println!("\n🔨 Starting blockchain reindex...");
                println!("This will rebuild the UTXO set from all blocks.\n");
            }

            // Call the reindex API endpoint
            let client = reqwest::Client::new();
            let response = client
                .post(format!("{}/blockchain/reindex", api))
                .send()
                .await?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;

                if json_output {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    let blocks_processed = result["blocks_processed"].as_u64().unwrap_or(0);
                    let utxo_count = result["utxo_count"].as_u64().unwrap_or(0);
                    let total_supply = result["total_supply"].as_u64().unwrap_or(0);
                    let processing_time = result["processing_time_ms"].as_u64().unwrap_or(0);

                    let total_supply_time = total_supply as f64 / 100_000_000.0;

                    println!("✅ Blockchain reindex complete!\n");
                    println!("Blocks Processed:  {}", blocks_processed);
                    println!("Total UTXOs:       {}", utxo_count);
                    println!("Total Supply:      {} TIME", total_supply_time);
                    println!("Processing Time:   {} ms\n", processing_time);

                    // Get node wallet address to show only that balance
                    let wallet_response = client
                        .get(format!("{}/blockchain/info", api))
                        .send()
                        .await?;

                    if wallet_response.status().is_success() {
                        let info: serde_json::Value = wallet_response.json().await?;
                        if let Some(wallet_addr) = info["wallet_address"].as_str() {
                            if let Some(balances) = result["wallet_balances"].as_object() {
                                if let Some(balance) = balances.get(wallet_addr) {
                                    if let Some(bal) = balance.as_u64() {
                                        let bal_time = bal as f64 / 100_000_000.0;
                                        println!("💰 Your Wallet Balance: {} TIME\n", bal_time);
                                    }
                                }
                            }
                        }
                    }

                    println!("The UTXO snapshot and wallet balances have been saved.");
                    println!("Your balance should now be correct!");
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
                    println!("✗ Failed to reindex blockchain: {}", error);
                }
            }
        }

        WalletCommands::Info {
            address,
            db_path: _,
        } => {
            let client = reqwest::Client::new();

            let addr = if let Some(a) = address {
                a
            } else {
                // Get node wallet address from API
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

            // Get balance
            let balance_response = client
                .get(format!("{}/balance/{}", api, addr))
                .send()
                .await?;

            let balance = if balance_response.status().is_success() {
                // Try to parse as JSON object first, fallback to plain number
                match balance_response.json::<serde_json::Value>().await {
                    Ok(json_val) => {
                        // Try as object with "balance" field
                        if let Some(bal) = json_val.get("balance").and_then(|v| v.as_u64()) {
                            bal
                        } else {
                            // Try as plain number
                            json_val.as_u64().unwrap_or_default()
                        }
                    }
                    Err(_) => 0,
                }
            } else {
                0
            };

            // Get UTXOs
            let utxos_response = client.get(format!("{}/utxos/{}", api, addr)).send().await?;

            let utxo_count = if utxos_response.status().is_success() {
                let utxos: serde_json::Value = utxos_response.json().await?;
                utxos.as_array().map(|arr| arr.len()).unwrap_or(0)
            } else {
                0
            };

            let balance_time = balance as f64 / 100_000_000.0;

            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "address": addr,
                        "balance": balance_time,
                        "balance_satoshis": balance,
                        "utxo_count": utxo_count
                    }))?
                );
            } else {
                println!("Wallet Information:");
                println!("  Address:    {}", addr);
                println!("  Balance:    {} TIME", balance_time);
                println!("  UTXOs:      {}", utxo_count);
            }
        }

        WalletCommands::ListUtxos {
            address,
            db_path: _,
        } => {
            let client = reqwest::Client::new();

            let addr = if let Some(a) = address {
                a
            } else {
                // Get node wallet address from API
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

            // Get UTXOs
            let response = client.get(format!("{}/utxos/{}", api, addr)).send().await?;

            if response.status().is_success() {
                let utxos: serde_json::Value = response.json().await?;

                if json_output {
                    println!("{}", serde_json::to_string_pretty(&utxos)?);
                } else {
                    println!("UTXOs for address: {}", addr);

                    if let Some(utxo_array) = utxos.as_array() {
                        if utxo_array.is_empty() {
                            println!("  No UTXOs found");
                        } else {
                            for (i, utxo) in utxo_array.iter().enumerate() {
                                let txid = utxo["txid"].as_str().unwrap_or("unknown");
                                let vout = utxo["vout"].as_u64().unwrap_or(0);
                                let amount = utxo["amount"].as_u64().unwrap_or(0);
                                let amount_time = amount as f64 / 100_000_000.0;

                                println!("\n  UTXO #{}:", i + 1);
                                println!("    TxID:   {}", txid);
                                println!("    Vout:   {}", vout);
                                println!("    Amount: {} TIME", amount_time);
                            }
                        }
                    }
                }
            } else {
                let error = response.text().await?;
                if json_output {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "address": addr,
                            "error": error
                        }))?
                    );
                } else {
                    println!("Error fetching UTXOs for {}: {}", addr, error);
                }
            }
        }

        WalletCommands::UtxoStats {
            address,
            db_path: _,
        } => {
            let client = reqwest::Client::new();

            let addr = if let Some(a) = address {
                a
            } else {
                // Get node wallet address from API
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

            // Get UTXO stats from API
            let response = client
                .get(format!("{}/utxos/{}/stats", api, addr))
                .send()
                .await?;

            if response.status().is_success() {
                let stats: serde_json::Value = response.json().await?;

                if json_output {
                    println!("{}", serde_json::to_string_pretty(&stats)?);
                } else {
                    println!("\n📊 UTXO Statistics for {}\n", addr);
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

                    let total_utxos = stats["total_utxos"].as_u64().unwrap_or(0);
                    let dust_utxos = stats["dust_utxos"].as_u64().unwrap_or(0);
                    let small_utxos = stats["small_utxos"].as_u64().unwrap_or(0);
                    let large_utxos = stats["large_utxos"].as_u64().unwrap_or(0);
                    let total_value = stats["total_value"].as_u64().unwrap_or(0);
                    let dust_value = stats["dust_value"].as_u64().unwrap_or(0);
                    let needs_consolidation =
                        stats["needs_consolidation"].as_bool().unwrap_or(false);

                    println!("Total UTXOs:     {}", total_utxos);
                    println!(
                        "  Dust (<1 TIME):  {} ({:.2}%)",
                        dust_utxos,
                        if total_utxos > 0 {
                            (dust_utxos as f64 / total_utxos as f64) * 100.0
                        } else {
                            0.0
                        }
                    );
                    println!("  Small (1-10):    {}", small_utxos);
                    println!("  Large (>10):     {}", large_utxos);
                    println!();
                    println!(
                        "Total Value:     {:.8} TIME",
                        total_value as f64 / 100_000_000.0
                    );
                    println!(
                        "Dust Value:      {:.8} TIME",
                        dust_value as f64 / 100_000_000.0
                    );
                    println!();

                    if needs_consolidation {
                        println!("⚠️  Consolidation recommended");
                        println!("   Run: time-cli wallet consolidate-utxos");
                    } else {
                        println!("✓ UTXOs are well-organized");
                    }
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                }
            } else {
                let error = response.text().await?;
                if json_output {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "address": addr,
                            "error": error
                        }))?
                    );
                } else {
                    println!("Error fetching UTXO stats for {}: {}", addr, error);
                }
            }
        }

        WalletCommands::ConsolidateUtxos {
            address,
            max_utxos,
            fee,
            db_path: _,
            yes,
        } => {
            let client = reqwest::Client::new();

            let addr = if let Some(a) = address {
                a
            } else {
                // Get node wallet address from API
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

            // Get UTXO stats first
            let stats_response = client
                .get(format!("{}/utxos/{}/stats", api, addr))
                .send()
                .await?;

            if !stats_response.status().is_success() {
                let error = stats_response.text().await?;
                if json_output {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "error": error
                        }))?
                    );
                } else {
                    println!("Error: {}", error);
                }
                return Ok(());
            }

            let stats: serde_json::Value = stats_response.json().await?;
            let total_utxos = stats["total_utxos"].as_u64().unwrap_or(0);

            if total_utxos <= 10 {
                if json_output {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "message": "No consolidation needed",
                            "utxo_count": total_utxos
                        }))?
                    );
                } else {
                    println!("✓ No consolidation needed ({} UTXOs)", total_utxos);
                }
                return Ok(());
            }

            // Confirmation prompt unless --yes flag
            if !yes && !json_output {
                println!("\n⚠️  UTXO Consolidation");
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("Address:      {}", addr);
                println!("Current UTXOs: {}", total_utxos);
                println!("Max to consolidate: {}", max_utxos);
                println!("Fee:          {} TIME", fee);
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!(
                    "\nThis will create a transaction combining up to {} UTXOs.",
                    max_utxos
                );
                print!("\nProceed? (y/N): ");
                use std::io::{self, Write};
                io::stdout().flush()?;

                let mut input = String::new();
                io::stdin().read_line(&mut input)?;

                if !input.trim().eq_ignore_ascii_case("y") {
                    println!("Cancelled.");
                    return Ok(());
                }
            }

            // Send consolidation request
            let response = client
                .post(format!("{}/wallet/consolidate", api))
                .json(&json!({
                    "address": addr,
                    "max_utxos": max_utxos,
                    "fee": (fee * 100_000_000.0) as u64
                }))
                .send()
                .await?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;

                if json_output {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("\n✓ Consolidation transaction created");
                    if let Some(txid) = result.get("txid") {
                        println!("TxID: {}", txid);
                    }
                    if let Some(utxos_consolidated) = result.get("utxos_consolidated") {
                        println!("UTXOs consolidated: {}", utxos_consolidated);
                    }
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
                    println!("✗ Consolidation failed: {}", error);
                }
            }
        }

        _ => {
            if json_output {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&json!({
                        "error": "This wallet command is not yet implemented"
                    }))?
                );
            } else {
                println!("This wallet command is not yet implemented");
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
                                &txid[..16],
                                vout,
                                amount,
                                tier,
                                confirmations
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
                                println!(
                                    "   Collateral: {}:{}",
                                    &entry.collateral_txid[..16],
                                    entry.collateral_output_index
                                );
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
                Ok(mut conf) => match conf.remove_entry(&alias) {
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
                },
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
                                println!(
                                    "{}",
                                    result
                                        .get("message")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("Success")
                                );
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

                        let success = response
                            .as_ref()
                            .map(|r| r.status().is_success())
                            .unwrap_or(false);
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
                            println!("  {} {}", alias, if success { "✓" } else { "✗" });
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

async fn handle_treasury_command(
    command: TreasuryCommands,
    client: &Client,
    api: &str,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        TreasuryCommands::Info => {
            let response = client.get(format!("{}/treasury/stats", api)).send().await?;

            if response.status().is_success() {
                let stats: serde_json::Value = response.json().await?;

                if json_output {
                    println!("{}", serde_json::to_string_pretty(&stats)?);
                } else {
                    println!("\n💰 Treasury Information");
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    println!("Balance: {} TIME", stats["balance_time"]);
                    println!("Total Allocated: {} satoshis", stats["total_allocated"]);
                    println!("Total Distributed: {} satoshis", stats["total_distributed"]);
                    println!("Allocations: {}", stats["allocation_count"]);
                    println!("Withdrawals: {}", stats["withdrawal_count"]);
                    println!("Pending Proposals: {}", stats["pending_proposals"]);
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

        TreasuryCommands::ListProposals => {
            let response = client
                .post(format!("{}/rpc/listproposals", api))
                .json(&json!({}))
                .send()
                .await?;

            if response.status().is_success() {
                let proposals: Vec<serde_json::Value> = response.json().await?;

                if json_output {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "proposals": proposals,
                            "count": proposals.len()
                        }))?
                    );
                } else {
                    println!("\n📋 Treasury Proposals ({} total)", proposals.len());
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

                    if proposals.is_empty() {
                        println!("No proposals found");
                    } else {
                        for (i, proposal) in proposals.iter().enumerate() {
                            println!(
                                "\n{}. {} ({})",
                                i + 1,
                                proposal["title"].as_str().unwrap_or("Unknown"),
                                proposal["id"].as_str().unwrap_or("unknown")
                            );
                            println!("   Amount: {} TIME", proposal["amount"]);
                            println!("   Status: {}", proposal["status"]);
                            println!(
                                "   Votes: {} Yes / {} No",
                                proposal["votes_yes"], proposal["votes_no"]
                            );
                        }
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

        TreasuryCommands::GetProposal { proposal_id } => {
            let response = client
                .post(format!("{}/treasury/proposal/{}", api, proposal_id))
                .send()
                .await?;

            if response.status().is_success() {
                let proposal: serde_json::Value = response.json().await?;

                if json_output {
                    println!("{}", serde_json::to_string_pretty(&proposal)?);
                } else {
                    println!("\n📄 Proposal Details");
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    println!("ID: {}", proposal["id"]);
                    println!("Title: {}", proposal["title"]);
                    println!("Description: {}", proposal["description"]);
                    println!("Recipient: {}", proposal["recipient"]);
                    println!(
                        "Amount: {} TIME",
                        proposal["amount"].as_f64().unwrap_or(0.0) / 100_000_000.0
                    );
                    println!("Status: {}", proposal["status"]);
                    println!("Submitter: {}", proposal["submitter"]);

                    if let Some(votes) = proposal["votes"].as_object() {
                        println!("\nVotes ({}):", votes.len());
                        for (voter, vote) in votes {
                            println!("  {} -> {}", voter, vote["vote_choice"]);
                        }
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

        TreasuryCommands::Propose {
            title,
            description,
            recipient,
            amount,
            voting_period,
        } => {
            let amount_satoshis = (amount * 100_000_000.0) as u64;

            let request = json!({
                "title": title,
                "description": description,
                "recipient": recipient,
                "amount": amount_satoshis,
                "voting_period_days": voting_period,
            });

            if !json_output {
                println!("\n📝 Submitting Treasury Proposal");
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("Title: {}", title);
                println!("Description: {}", description);
                println!("Recipient: {}", recipient);
                println!("Amount: {} TIME ({} satoshis)", amount, amount_satoshis);
                println!("Voting Period: {} days", voting_period);
                println!("\n📡 Submitting proposal...");
            }

            let response = client
                .post(format!("{}/treasury/propose", api))
                .json(&request)
                .send()
                .await?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                if json_output {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("\n✅ SUCCESS!");
                    println!("Proposal ID: {}", result["proposal_id"]);
                    println!("{}", result["message"]);
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
                    eprintln!("Proposal submission failed: {}", error);
                }
            }
        }

        TreasuryCommands::Vote {
            proposal_id,
            vote,
            masternode_id,
        } => {
            let vote_choice = match vote.to_lowercase().as_str() {
                "yes" => "Yes",
                "no" => "No",
                "abstain" => "Abstain",
                _ => {
                    eprintln!("Invalid vote choice. Must be: yes, no, or abstain");
                    return Ok(());
                }
            };

            let masternode = if let Some(id) = masternode_id {
                id
            } else {
                // Get local node IP as default
                get_local_ip_or_fallback()
            };

            let request = json!({
                "proposal_id": proposal_id,
                "masternode_id": masternode,
                "vote": vote_choice,
            });

            if !json_output {
                println!("\n🗳️  Casting Vote");
                println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                println!("Proposal ID: {}", proposal_id);
                println!("Vote: {}", vote_choice);
                println!("Masternode: {}", masternode);
                println!("\n📡 Submitting vote...");
            }

            let response = client
                .post(format!("{}/treasury/vote", api))
                .json(&request)
                .send()
                .await?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;
                if json_output {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("\n✅ SUCCESS!");
                    println!("{}", result["message"]);
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
                    eprintln!("Vote submission failed: {}", error);
                }
            }
        }
    }

    Ok(())
}

async fn handle_mempool_command(
    command: MempoolCommands,
    client: &Client,
    api: &str,
    json_output: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    match command {
        MempoolCommands::Status => {
            let response = client.get(format!("{}/mempool/status", api)).send().await?;

            if response.status().is_success() {
                let status: serde_json::Value = response.json().await?;

                if json_output {
                    println!("{}", serde_json::to_string_pretty(&status)?);
                } else {
                    println!("\n💾 Mempool Status");
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    println!("Transaction Count: {}", status["size"]);

                    if let Some(txs) = status["transactions"].as_array() {
                        if !txs.is_empty() {
                            println!("\nTransaction IDs:");
                            for (i, tx_id) in txs.iter().enumerate() {
                                if let Some(id) = tx_id.as_str() {
                                    println!("  {}. {}", i + 1, id);
                                }
                            }
                        } else {
                            println!("\nNo transactions in mempool");
                        }
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

        MempoolCommands::List => {
            let response = client.get(format!("{}/mempool/all", api)).send().await?;

            if response.status().is_success() {
                let transactions: Vec<serde_json::Value> = response.json().await?;

                if json_output {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&json!({
                            "transactions": transactions,
                            "count": transactions.len()
                        }))?
                    );
                } else {
                    println!("\n💾 Mempool Transactions ({} total)", transactions.len());
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

                    if transactions.is_empty() {
                        println!("No transactions in mempool");
                    } else {
                        for (i, tx) in transactions.iter().enumerate() {
                            println!(
                                "\n{}. Transaction: {}",
                                i + 1,
                                tx["txid"].as_str().unwrap_or("unknown")
                            );
                            println!("   Version: {}", tx["version"]);
                            println!("   Lock Time: {}", tx["lock_time"]);

                            if let Some(inputs) = tx["inputs"].as_array() {
                                println!("   Inputs: {}", inputs.len());
                                for (idx, input) in inputs.iter().enumerate() {
                                    if let Some(prev_out) = input.get("previous_output") {
                                        let txid = prev_out["txid"].as_str().unwrap_or("");
                                        let vout = prev_out["vout"].as_u64().unwrap_or(0);
                                        println!(
                                            "     {}.  {}:{}",
                                            idx + 1,
                                            &txid[..16.min(txid.len())],
                                            vout
                                        );
                                    }
                                }
                            }

                            if let Some(outputs) = tx["outputs"].as_array() {
                                println!("   Outputs: {}", outputs.len());
                                for (idx, output) in outputs.iter().enumerate() {
                                    let amount = output["amount"].as_u64().unwrap_or(0);
                                    let address = output["address"].as_str().unwrap_or("unknown");
                                    let amount_time = amount as f64 / 100_000_000.0;
                                    println!(
                                        "     {}.  {} TIME -> {}",
                                        idx + 1,
                                        amount_time,
                                        address
                                    );
                                }
                            }
                        }
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

        MempoolCommands::Clear => {
            let response = client.post(format!("{}/mempool/clear", api)).send().await?;

            if response.status().is_success() {
                let result: serde_json::Value = response.json().await?;

                if json_output {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("\n🗑️  Mempool Cleared");
                    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
                    println!("✅ All transactions removed from mempool");
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

fn get_data_dir() -> String {
    if let Ok(dir) = std::env::var("TIME_DATA_DIR") {
        return dir;
    }

    let home = if cfg!(windows) {
        std::env::var("USERPROFILE").unwrap_or_else(|_| ".".to_string())
    } else {
        std::env::var("HOME").unwrap_or_else(|_| ".".to_string())
    };

    format!("{}/.timecoin", home)
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
