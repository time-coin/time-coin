//! Dump the wallet's send_record entries from sled.
//!
//! Read-only inspection tool for debugging "where did my TX go?" issues.
//! The wallet must be CLOSED before running (sled takes an exclusive lock).
//!
//! Usage:
//!   cargo run --example dump_db
//!   cargo run --example dump_db -- 751c5168          # filter by txid prefix

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
enum TransactionStatus {
    Pending,
    Approved,
    Declined,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TransactionRecord {
    txid: String,
    vout: u32,
    is_send: bool,
    address: String,
    amount: u64,
    fee: u64,
    timestamp: i64,
    status: TransactionStatus,
    #[serde(default)]
    is_fee: bool,
    #[serde(default)]
    is_change: bool,
    #[serde(default)]
    block_hash: String,
    #[serde(default)]
    block_height: u64,
    #[serde(default)]
    confirmations: u64,
    #[serde(default)]
    memo: Option<String>,
    #[serde(default)]
    is_consolidation: bool,
}

fn fmt_time(satoshis: u64) -> String {
    let whole = satoshis / 100_000_000;
    let frac = satoshis % 100_000_000;
    format!("{}.{:08}", whole, frac)
}

fn fmt_ts(ts: i64) -> String {
    chrono::DateTime::<chrono::Utc>::from_timestamp(ts, 0)
        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| ts.to_string())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let filter = std::env::args().nth(1);

    let home = dirs::home_dir().ok_or("no home dir")?;
    let db_path: PathBuf = home.join(".time-wallet").join("wallet_db");

    println!("Opening: {}", db_path.display());
    if !db_path.exists() {
        return Err(format!("DB path does not exist: {}", db_path.display()).into());
    }

    let db = sled::open(&db_path)?;

    let mut total = 0usize;
    let mut decoded = 0usize;
    let mut failed = 0usize;
    let mut records: Vec<(String, TransactionRecord)> = Vec::new();

    for item in db.scan_prefix(b"send_record:") {
        let (key, value) = item?;
        total += 1;
        let key_str = String::from_utf8_lossy(&key).to_string();
        let txid = key_str
            .strip_prefix("send_record:")
            .unwrap_or("")
            .to_string();

        if let Some(ref f) = filter {
            if !txid.starts_with(f) {
                continue;
            }
        }

        match bincode::deserialize::<TransactionRecord>(&value) {
            Ok(rec) => {
                decoded += 1;
                records.push((txid, rec));
            }
            Err(e) => {
                failed += 1;
                println!(
                    "  ✗ {} — decode failed: {} ({} bytes)",
                    txid,
                    e,
                    value.len()
                );
            }
        }
    }

    records.sort_by_key(|(_, r)| -r.timestamp);

    println!();
    println!(
        "=== send_record entries ({} total, {} decoded, {} failed) ===",
        total, decoded, failed
    );
    println!();

    for (txid, r) in &records {
        let is_target = filter
            .as_deref()
            .map(|f| txid.starts_with(f))
            .unwrap_or(false);
        let marker = if is_target { ">>>" } else { "   " };

        println!(
            "{} {} [{}] {} → {}",
            marker,
            fmt_ts(r.timestamp),
            format!("{:?}", r.status),
            &txid[..16.min(txid.len())],
            r.address,
        );
        println!(
            "      amount={} TIME  fee={} TIME  vout={}  is_send={}  is_fee={}  is_change={}  is_consolidation={}",
            fmt_time(r.amount),
            fmt_time(r.fee),
            r.vout,
            r.is_send,
            r.is_fee,
            r.is_change,
            r.is_consolidation,
        );
        if !r.block_hash.is_empty() || r.block_height > 0 {
            println!(
                "      block_hash={}  block_height={}  confirmations={}",
                r.block_hash, r.block_height, r.confirmations
            );
        }
        if let Some(ref m) = r.memo {
            println!("      memo={:?}", m);
        }
    }

    println!();
    println!("=== other prefixes present in DB ===");
    let mut prefix_counts: std::collections::BTreeMap<String, usize> = Default::default();
    for item in db.iter() {
        let (key, _) = item?;
        let key_str = String::from_utf8_lossy(&key);
        let prefix = key_str.split(':').next().unwrap_or("").to_string();
        *prefix_counts.entry(prefix).or_insert(0) += 1;
    }
    for (prefix, count) in prefix_counts {
        println!("  {:30} {}", prefix, count);
    }

    Ok(())
}
