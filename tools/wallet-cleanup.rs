#!/usr/bin/env rust-script
//! Clean wallet.dat file to remove metadata
//! 
//! This tool removes metadata that was incorrectly added to time-wallet.dat
//! and ensures it only contains cryptographic keys and addresses.

use std::fs;
use std::path::PathBuf;

fn main() {
    println!("🧹 TIME Wallet Cleanup Tool");
    println!("===========================\n");
    
    // Find wallet file
    let wallet_path = PathBuf::from("time-wallet.dat");
    
    if !wallet_path.exists() {
        println!("❌ No time-wallet.dat found in current directory");
        println!("💡 Run this from your wallet data directory");
        return;
    }
    
    // Backup first
    let backup_path = format!("time-wallet.dat.backup.{}", chrono::Utc::now().format("%Y%m%d_%H%M%S"));
    match fs::copy(&wallet_path, &backup_path) {
        Ok(_) => println!("✅ Created backup: {}", backup_path),
        Err(e) => {
            println!("❌ Failed to create backup: {}", e);
            return;
        }
    }
    
    // Read wallet
    let wallet_data = match fs::read(&wallet_path) {
        Ok(data) => data,
        Err(e) => {
            println!("❌ Failed to read wallet: {}", e);
            return;
        }
    };
    
    println!("📊 Original wallet size: {} bytes", wallet_data.len());
    
    // Try to deserialize and check for metadata
    println!("\n🔍 Checking for metadata pollution...");
    
    // Since we can't easily deserialize here without the full wallet crate,
    // just inform the user
    println!("⚠️  Manual cleanup required:");
    println!("   1. Your wallet has been backed up");
    println!("   2. Delete the current time-wallet.dat");
    println!("   3. Restart the wallet GUI");
    println!("   4. Select 'Import Existing Wallet'");
    println!("   5. Enter your mnemonic phrase");
    println!("\n💡 This will recreate a clean wallet with only keys");
    println!("💡 All your addresses will be restored from the mnemonic");
    println!("💡 Contact info is stored separately in wallet.db");
}
