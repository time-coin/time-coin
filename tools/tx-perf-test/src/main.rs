//! Transaction Performance Test Tool
//!
//! Measures real transaction verification speed on the TIME Coin network.
//! Sends actual transactions and tracks timing metrics.
//! Supports testnet coin generation for testing.

use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use clap::Parser;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use wallet::{NetworkType, Wallet, UTXO};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// API endpoint for the node (e.g., http://localhost:3030)
    #[arg(short, long, default_value = "http://localhost:3030")]
    api_url: String,

    /// Number of transactions to send
    #[arg(short = 'n', long, default_value = "10")]
    tx_count: usize,

    /// Private key (hex) of source wallet
    #[arg(short, long)]
    private_key: String,

    /// Recipient address
    #[arg(short, long)]
    recipient: String,

    /// Amount per transaction (in smallest unit)
    #[arg(short, long, default_value = "1000")]
    amount: u64,

    /// Transaction fee (in smallest unit)
    #[arg(short, long, default_value = "100")]
    fee: u64,

    /// Delay between transactions in milliseconds
    #[arg(short, long, default_value = "100")]
    delay_ms: u64,

    /// Network type (mainnet, testnet, devnet)
    #[arg(long, default_value = "testnet")]
    network: String,

    /// Output results to JSON file
    #[arg(short, long)]
    output: Option<String>,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Generate testnet coins (mint) before running tests
    /// Only works with testnet/devnet. Amount in smallest unit.
    #[arg(long, default_value = "0")]
    mint_coins: u64,
}

#[derive(Debug, Clone, Serialize)]
struct TransactionMetrics {
    tx_id: String,
    tx_number: usize,
    amount: u64,
    fee: u64,
    submission_time: DateTime<Utc>,
    submission_duration_ms: u64,
    success: bool,
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct PerformanceReport {
    total_transactions: usize,
    successful_transactions: usize,
    failed_transactions: usize,
    total_duration_ms: u64,
    average_submission_time_ms: f64,
    min_submission_time_ms: u64,
    max_submission_time_ms: u64,
    transactions_per_second: f64,
    test_started: DateTime<Utc>,
    test_completed: DateTime<Utc>,
    transactions: Vec<TransactionMetrics>,
}

#[derive(Serialize)]
struct TransactionRequest {
    from: String,
    to: String,
    amount: u64,
    fee: u64,
    timestamp: i64,
    signature: String,
    tx_data: String,
}

#[derive(Deserialize)]
struct TransactionResponse {
    success: bool,
    #[allow(dead_code)]
    tx_id: String,
    message: String,
}

async fn submit_transaction(
    client: &Client,
    api_url: &str,
    wallet: &mut Wallet,
    recipient: &str,
    amount: u64,
    fee: u64,
) -> Result<(String, Duration)> {
    let start = Instant::now();

    // Create and sign transaction
    let tx = wallet
        .create_transaction(recipient, amount, fee)
        .context("Failed to create transaction")?;

    let tx_bytes = tx.to_bytes().context("Failed to serialize transaction")?;
    let tx_id = tx.txid();

    // Prepare request
    let request = TransactionRequest {
        from: wallet.address_string(),
        to: recipient.to_string(),
        amount,
        fee,
        timestamp: Utc::now().timestamp(),
        signature: hex::encode(&tx_bytes[0..64.min(tx_bytes.len())]),
        tx_data: hex::encode(&tx_bytes),
    };

    // Submit to network
    let response = client
        .post(format!("{}/transaction", api_url))
        .json(&request)
        .send()
        .await
        .context("Failed to send transaction")?;

    let duration = start.elapsed();

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        anyhow::bail!("Transaction submission failed: {} - {}", status, body);
    }

    let tx_response: TransactionResponse = response
        .json()
        .await
        .context("Failed to parse response")?;

    if !tx_response.success {
        anyhow::bail!("Transaction rejected: {}", tx_response.message);
    }

    Ok((tx_id, duration))
}

/// Generate testnet coins by creating a coinbase-like transaction
/// This simulates minting coins for testing purposes in testnet/devnet
async fn generate_testnet_coins(
    wallet: &mut Wallet,
    amount: u64,
) -> Result<()> {
    println!("💰 Generating {} testnet coins...", amount);
    
    // Create a synthetic UTXO that simulates minted coins
    // In a real blockchain, this would be a coinbase transaction
    let mut tx_hash = [0u8; 32];
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    
    // Create a unique hash based on wallet address and timestamp
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(wallet.address_string().as_bytes());
    hasher.update(&timestamp.to_le_bytes());
    hasher.update(b"testnet_mint");
    let hash_result = hasher.finalize();
    tx_hash.copy_from_slice(&hash_result[..32]);
    
    // Add the UTXO to wallet
    let utxo = UTXO {
        tx_hash,
        output_index: 0,
        amount,
        address: wallet.address_string(),
    };
    
    wallet.add_utxo(utxo);
    
    println!("✅ Generated {} coins (new balance: {})", amount, wallet.balance());
    Ok(())
}


async fn run_performance_test(args: Args) -> Result<PerformanceReport> {
    println!("🚀 TIME Coin Transaction Performance Test");
    println!("==========================================");
    println!("API Endpoint:    {}", args.api_url);
    println!("Transaction Count: {}", args.tx_count);
    println!("Amount per TX:   {} TIME", args.amount);
    println!("Fee per TX:      {} TIME", args.fee);
    println!("Delay:           {} ms", args.delay_ms);
    println!("Network:         {}", args.network);
    println!();

    // Parse network type
    let network = match args.network.to_lowercase().as_str() {
        "mainnet" => NetworkType::Mainnet,
        "testnet" | "devnet" => NetworkType::Testnet, // Treat devnet as testnet
        _ => anyhow::bail!("Invalid network type: {}", args.network),
    };

    // Initialize wallet
    let mut wallet = Wallet::from_private_key_hex(&args.private_key, network)
        .context("Failed to create wallet from private key")?;

    println!("✅ Wallet loaded: {}", wallet.address_string());
    println!("   Balance: {} TIME", wallet.balance());
    
    // Generate testnet coins if requested
    if args.mint_coins > 0 {
        if network == NetworkType::Mainnet {
            anyhow::bail!("Cannot mint coins on mainnet! Use testnet or devnet.");
        }
        generate_testnet_coins(&mut wallet, args.mint_coins)
            .await
            .context("Failed to generate testnet coins")?;
    }
    
    println!();

    // Check if we have enough funds
    let total_needed = (args.amount + args.fee) * args.tx_count as u64;
    if wallet.balance() < total_needed {
        anyhow::bail!(
            "Insufficient funds: have {} TIME, need {} TIME",
            wallet.balance(),
            total_needed
        );
    }

    // Create HTTP client
    let client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("Failed to create HTTP client")?;

    // Test metrics
    let mut metrics = Vec::new();
    let test_start = Utc::now();
    let overall_start = Instant::now();

    println!("📤 Sending transactions...");
    println!();

    for i in 0..args.tx_count {
        let tx_start = Instant::now();
        let submission_time = Utc::now();

        match submit_transaction(
            &client,
            &args.api_url,
            &mut wallet,
            &args.recipient,
            args.amount,
            args.fee,
        )
        .await
        {
            Ok((tx_id, duration)) => {
                let duration_ms = duration.as_millis() as u64;

                if args.verbose {
                    println!(
                        "  ✓ TX {}/{}: {} ({} ms)",
                        i + 1,
                        args.tx_count,
                        &tx_id[..16],
                        duration_ms
                    );
                } else if (i + 1) % 10 == 0 || i == 0 {
                    println!("  Sent {}/{} transactions...", i + 1, args.tx_count);
                }

                metrics.push(TransactionMetrics {
                    tx_id,
                    tx_number: i + 1,
                    amount: args.amount,
                    fee: args.fee,
                    submission_time,
                    submission_duration_ms: duration_ms,
                    success: true,
                    error: None,
                });
            }
            Err(e) => {
                let duration_ms = tx_start.elapsed().as_millis() as u64;
                eprintln!("  ✗ TX {}/{}: {}", i + 1, args.tx_count, e);

                metrics.push(TransactionMetrics {
                    tx_id: String::from("failed"),
                    tx_number: i + 1,
                    amount: args.amount,
                    fee: args.fee,
                    submission_time,
                    submission_duration_ms: duration_ms,
                    success: false,
                    error: Some(e.to_string()),
                });
            }
        }

        // Delay between transactions
        if i < args.tx_count - 1 {
            tokio::time::sleep(Duration::from_millis(args.delay_ms)).await;
        }
    }

    let overall_duration = overall_start.elapsed();
    let test_end = Utc::now();

    // Calculate statistics
    let successful: Vec<_> = metrics.iter().filter(|m| m.success).collect();
    let successful_count = successful.len();
    let failed_count = metrics.len() - successful_count;

    let submission_times: Vec<u64> = successful
        .iter()
        .map(|m| m.submission_duration_ms)
        .collect();

    let avg_submission_time = if !submission_times.is_empty() {
        submission_times.iter().sum::<u64>() as f64 / submission_times.len() as f64
    } else {
        0.0
    };

    let min_submission_time = submission_times.iter().min().copied().unwrap_or(0);
    let max_submission_time = submission_times.iter().max().copied().unwrap_or(0);

    let tps = if overall_duration.as_secs() > 0 {
        successful_count as f64 / overall_duration.as_secs_f64()
    } else {
        0.0
    };

    let report = PerformanceReport {
        total_transactions: args.tx_count,
        successful_transactions: successful_count,
        failed_transactions: failed_count,
        total_duration_ms: overall_duration.as_millis() as u64,
        average_submission_time_ms: avg_submission_time,
        min_submission_time_ms: min_submission_time,
        max_submission_time_ms: max_submission_time,
        transactions_per_second: tps,
        test_started: test_start,
        test_completed: test_end,
        transactions: metrics,
    };

    println!();
    println!("📊 Performance Report");
    println!("==========================================");
    println!("Total transactions:   {}", report.total_transactions);
    println!("Successful:           {}", report.successful_transactions);
    println!("Failed:               {}", report.failed_transactions);
    println!("Total duration:       {} ms", report.total_duration_ms);
    println!(
        "Average submit time:  {:.2} ms",
        report.average_submission_time_ms
    );
    println!("Min submit time:      {} ms", report.min_submission_time_ms);
    println!("Max submit time:      {} ms", report.max_submission_time_ms);
    println!("Throughput:           {:.2} TPS", report.transactions_per_second);
    println!();

    // Save to file if requested
    if let Some(output_path) = &args.output {
        let json = serde_json::to_string_pretty(&report)
            .context("Failed to serialize report")?;
        std::fs::write(output_path, json)
            .context("Failed to write report to file")?;
        println!("✅ Report saved to: {}", output_path);
    }

    Ok(report)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    match run_performance_test(args).await {
        Ok(_) => {
            println!("✅ Performance test completed successfully");
            Ok(())
        }
        Err(e) => {
            eprintln!("❌ Performance test failed: {}", e);
            std::process::exit(1);
        }
    }
}
