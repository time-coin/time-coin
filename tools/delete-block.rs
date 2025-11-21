#!/usr/bin/env cargo script
//! Delete a specific block from the blockchain database
//!
//! Usage: cargo script delete-block.rs -- <db_path> <block_height>

use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    
    if args.len() != 3 {
        eprintln!("Usage: {} <db_path> <block_height>", args[0]);
        eprintln!("Example: {} /var/lib/time-coin/blockchain 39", args[0]);
        std::process::exit(1);
    }
    
    let db_path = &args[1];
    let block_height: u64 = args[2].parse().expect("Invalid block height");
    
    println!("Opening database at: {}", db_path);
    
    let db = sled::open(db_path).expect("Failed to open database");
    let key = format!("block:{}", block_height);
    
    match db.remove(key.as_bytes()) {
        Ok(Some(_)) => {
            println!("✅ Block {} deleted successfully", block_height);
            db.flush().expect("Failed to flush database");
            println!("✅ Database flushed");
        }
        Ok(None) => {
            println!("⚠️  Block {} not found in database", block_height);
        }
        Err(e) => {
            eprintln!("❌ Error deleting block: {}", e);
            std::process::exit(1);
        }
    }
}
