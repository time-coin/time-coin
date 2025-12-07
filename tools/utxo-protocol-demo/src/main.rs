//! Example: TIME Coin Protocol - UTXO-Based Instant Finality
//!
//! This example demonstrates the TIME Coin Protocol, which combines
//! Bitcoin's UTXO model with instant finality through masternode consensus.

use std::collections::HashSet;
use time_consensus::instant_finality::{
    InstantFinalityManager, TransactionStatus, TransactionVote,
};
use time_core::utxo_state_manager::{UTXOStateManager, UTXOSubscription};
use time_core::{OutPoint, Transaction, TxInput, TxOutput};

#[tokio::main]
async fn main() {
    println!("═══════════════════════════════════════════════════════════");
    println!("  TIME Coin Protocol Demo");
    println!("  UTXO-Based Instant Finality with Real-Time State Tracking");
    println!("═══════════════════════════════════════════════════════════\n");

    // Initialize managers
    let utxo_manager = UTXOStateManager::new("node_192.168.1.1".to_string());
    let finality_manager = InstantFinalityManager::new(67); // 67% quorum

    // Setup masternodes
    finality_manager
        .update_masternodes(vec![
            "192.168.1.1".to_string(),
            "192.168.1.2".to_string(),
            "192.168.1.3".to_string(),
        ])
        .await;

    println!("📊 Network Configuration:");
    println!("   Masternodes: 3");
    println!("   Quorum: 67% (2 out of 3 votes)\n");

    // Setup notification handler
    utxo_manager
        .set_notification_handler(|notification| async move {
            println!("🔔 UTXO State Change Notification:");
            println!(
                "   OutPoint: {}:{}",
                notification.outpoint.txid, notification.outpoint.vout
            );
            println!("   Previous State: {:?}", notification.old_state);
            println!("   New State: {:?}", notification.new_state);
            println!("   Timestamp: {}", notification.timestamp);
            println!();
        })
        .await;

    // ═══════════════════════════════════════════════════════════════
    // STEP 1: Create Initial UTXOs
    // ═══════════════════════════════════════════════════════════════
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 1: Creating Initial UTXOs");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let genesis_outpoint = OutPoint::new("genesis_tx".to_string(), 0);
    let genesis_output = TxOutput::new(10_000, "TIME1alice_addr".to_string());

    utxo_manager
        .add_utxo(genesis_outpoint.clone(), genesis_output.clone())
        .await
        .unwrap();

    println!("✅ Genesis UTXO created:");
    println!(
        "   OutPoint: {}:{}",
        genesis_outpoint.txid, genesis_outpoint.vout
    );
    println!("   Amount: {} TIME", genesis_output.amount);
    println!("   Owner: {}", genesis_output.address);
    println!();

    // Check state
    let state = utxo_manager.get_state(&genesis_outpoint).await.unwrap();
    println!("   Current State: {:?}\n", state);

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // ═══════════════════════════════════════════════════════════════
    // STEP 2: Subscribe to Address
    // ═══════════════════════════════════════════════════════════════
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 2: Setting Up Subscriptions");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    let mut watched_addresses = HashSet::new();
    watched_addresses.insert("TIME1alice_addr".to_string());
    watched_addresses.insert("TIME1bob_addr".to_string());

    let subscription = UTXOSubscription {
        outpoints: HashSet::new(),
        addresses: watched_addresses,
        subscriber_id: "wallet_alice".to_string(),
    };

    utxo_manager.subscribe(subscription).await;

    println!("✅ Wallet subscribed to addresses:");
    println!("   - TIME1alice_addr");
    println!("   - TIME1bob_addr");
    println!();

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // ═══════════════════════════════════════════════════════════════
    // STEP 3: Create and Submit Transaction
    // ═══════════════════════════════════════════════════════════════
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 3: Creating Transaction");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Alice sends 6000 TIME to Bob
    let input = TxInput::new(
        "genesis_tx".to_string(),
        0,
        vec![1, 2, 3], // mock public key
        vec![4, 5, 6], // mock signature
    );

    let outputs = vec![
        TxOutput::new(6_000, "TIME1bob_addr".to_string()), // To Bob
        TxOutput::new(3_950, "TIME1alice_addr".to_string()), // Change back to Alice
                                                           // 50 TIME fee
    ];

    let tx = Transaction::new(vec![input], outputs);

    println!("📝 Transaction Details:");
    println!("   TxID: {}", tx.txid);
    println!("   Inputs: 1 (10,000 TIME)");
    println!("   Outputs:");
    println!("     - Bob:   6,000 TIME");
    println!("     - Alice: 3,950 TIME (change)");
    println!("     - Fee:      50 TIME");
    println!();

    // Submit to instant finality manager
    let txid = finality_manager
        .submit_transaction(tx.clone())
        .await
        .unwrap();

    println!("✅ Transaction submitted to instant finality system");
    println!("   TxID: {}\n", txid);

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // ═══════════════════════════════════════════════════════════════
    // STEP 4: Lock UTXOs (Prevent Double-Spend)
    // ═══════════════════════════════════════════════════════════════
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 4: Locking Input UTXOs");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    for input in &tx.inputs {
        utxo_manager
            .lock_utxo(&input.previous_output, tx.txid.clone())
            .await
            .unwrap();

        println!("🔒 UTXO Locked:");
        println!(
            "   OutPoint: {}:{}",
            input.previous_output.txid, input.previous_output.vout
        );
        println!("   Locked by: {}", tx.txid);
    }

    let state = utxo_manager.get_state(&genesis_outpoint).await.unwrap();
    println!("\n   Current State: {:?}", state);
    println!("   ✅ Double-spend protection active!\n");

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // ═══════════════════════════════════════════════════════════════
    // STEP 5: Masternode Voting
    // ═══════════════════════════════════════════════════════════════
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 5: Masternode Voting Process");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Simulate voting from masternodes
    let masternodes = ["192.168.1.1", "192.168.1.2", "192.168.1.3"];

    for (i, masternode) in masternodes.iter().enumerate() {
        println!("🗳️  Masternode {} validating...", i + 1);
        println!("   Address: {}", masternode);

        // Simulate validation delay
        tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;

        let vote = TransactionVote {
            txid: tx.txid.clone(),
            voter: masternode.to_string(),
            approved: true,
            reason: None,
            timestamp: chrono::Utc::now().timestamp(),
            signature: vec![],
        };

        let status = finality_manager.record_vote(vote).await.unwrap();

        println!("   ✅ Vote recorded: APPROVE");

        match status {
            TransactionStatus::Approved { votes, total_nodes } => {
                println!("\n🎉 ════════════════════════════════════════════════════");
                println!("🎉 INSTANT FINALITY ACHIEVED!");
                println!("🎉 ════════════════════════════════════════════════════");
                println!("   Votes: {}/{}", votes, total_nodes);
                println!(
                    "   Percentage: {:.1}%",
                    (votes as f64 / total_nodes as f64) * 100.0
                );
                println!("   Time: <3 seconds");
                println!("   Status: IRREVERSIBLE\n");
                break;
            }
            TransactionStatus::Validated => {
                println!("   Status: Waiting for more votes...\n");
            }
            _ => {}
        }
    }

    // Update UTXO states to finalized
    for input in &tx.inputs {
        utxo_manager
            .mark_spent_finalized(&input.previous_output, tx.txid.clone(), 2)
            .await
            .unwrap();
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // ═══════════════════════════════════════════════════════════════
    // STEP 6: Add New UTXOs
    // ═══════════════════════════════════════════════════════════════
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 6: Creating New UTXOs from Transaction Outputs");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    for (vout, output) in tx.outputs.iter().enumerate() {
        let outpoint = OutPoint::new(tx.txid.clone(), vout as u32);
        utxo_manager
            .add_utxo(outpoint.clone(), output.clone())
            .await
            .unwrap();

        println!("✨ New UTXO Created:");
        println!("   OutPoint: {}:{}", outpoint.txid, outpoint.vout);
        println!("   Amount: {} TIME", output.amount);
        println!("   Owner: {}", output.address);
        println!();
    }

    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    // ═══════════════════════════════════════════════════════════════
    // STEP 7: Query Final State
    // ═══════════════════════════════════════════════════════════════
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 7: Final State Summary");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    // Get stats
    let stats = utxo_manager.get_stats().await;

    println!("📊 UTXO Set Statistics:");
    println!("   Total UTXOs: {}", stats.total_utxos);
    println!("   Unspent: {}", stats.unspent);
    println!("   Locked: {}", stats.locked);
    println!("   Spent Pending: {}", stats.spent_pending);
    println!("   Spent Finalized: {}", stats.spent_finalized);
    println!("   Confirmed: {}", stats.confirmed);
    println!("   Active Subscriptions: {}", stats.active_subscriptions);
    println!();

    // Check balances
    let alice_utxos = utxo_manager.get_utxos_by_address("TIME1alice_addr").await;
    let bob_utxos = utxo_manager.get_utxos_by_address("TIME1bob_addr").await;

    let alice_balance: u64 = alice_utxos.iter().map(|u| u.output.amount).sum();
    let bob_balance: u64 = bob_utxos.iter().map(|u| u.output.amount).sum();

    println!("💰 Balances:");
    println!(
        "   Alice: {} TIME ({} UTXO)",
        alice_balance,
        alice_utxos.len()
    );
    println!("   Bob:   {} TIME ({} UTXO)", bob_balance, bob_utxos.len());
    println!();

    // Show finality status
    let finality_stats = finality_manager.get_stats().await;

    println!("⚡ Instant Finality Statistics:");
    println!(
        "   Total Transactions: {}",
        finality_stats.total_transactions
    );
    println!("   Pending: {}", finality_stats.pending);
    println!("   Validated: {}", finality_stats.validated);
    println!("   Approved: {}", finality_stats.approved);
    println!("   Rejected: {}", finality_stats.rejected);
    println!("   Confirmed: {}", finality_stats.confirmed);
    println!("   Locked UTXOs: {}", finality_stats.locked_utxos);
    println!();

    println!("═══════════════════════════════════════════════════════════");
    println!("  ✅ Demo Complete!");
    println!("  Transaction achieved instant finality in <3 seconds");
    println!("  UTXO states tracked in real-time");
    println!("  Double-spend protection active");
    println!("═══════════════════════════════════════════════════════════");
}
