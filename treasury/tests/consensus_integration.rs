//! Comprehensive Treasury Consensus Integration Tests
//!
//! Tests all acceptance scenarios for treasury consensus:
//! 1. Funding via block rewards
//! 2. Proposal creation and approval
//! 3. Rejection by masternodes
//! 4. Proposal expiration
//! 5. Insufficient funds handling

use treasury::*;

/// Helper to create a masternode with a specific tier
fn create_masternode(id: &str, tier_power: u64) -> (String, u64) {
    (id.to_string(), tier_power)
}

/// Scenario 1: Test treasury funding via block rewards
#[test]
fn test_scenario_1_funding_via_block_rewards() {
    let mut pool = TreasuryPool::new();

    // Simulate multiple blocks being mined
    // Each block contributes TREASURY_BLOCK_REWARD (5 TIME)
    for block_num in 1..=10 {
        pool.deposit_block_reward(block_num, 1000 + block_num)
            .expect("Failed to deposit block reward");
    }

    // Verify treasury received correct amount
    // 10 blocks * 5 TIME = 50 TIME
    let expected_balance = TREASURY_BLOCK_REWARD * 10;
    assert_eq!(
        pool.balance(),
        expected_balance,
        "Treasury should have received block rewards"
    );

    // Verify balance in TIME units
    assert_eq!(
        pool.balance_time(),
        50.0,
        "Treasury should have 50 TIME from 10 blocks"
    );

    // Verify transaction history
    let history = pool.transactions();
    assert_eq!(history.len(), 10, "Should have 10 deposit transactions");

    // Also test transaction fee deposits (50% to treasury)
    let tx_fee = TIME_UNIT; // 1 TIME
    pool.deposit_transaction_fee("tx001".to_string(), tx_fee, 2000)
        .expect("Failed to deposit transaction fee");

    // Treasury should receive 50% of fee
    let expected_total = expected_balance + (tx_fee / 2);
    assert_eq!(
        pool.balance(),
        expected_total,
        "Treasury should receive 50% of transaction fees"
    );
}

/// Scenario 2: Test proposal creation, voting, and approval by masternodes
#[test]
fn test_scenario_2_proposal_approval_with_masternode_consensus() {
    // Create proposal
    let mut proposal = TreasuryProposal::new(ProposalParams {
        id: "dev-grant-001".to_string(),
        title: "Development Grant Q1 2025".to_string(),
        description: "Fund core development team for Q1 2025".to_string(),
        recipient: "time1devteam...".to_string(),
        amount: 10_000 * TIME_UNIT, // 10,000 TIME
        submitter: "time1submitter...".to_string(),
        submission_time: 1000,
        voting_period_days: 14,
    });

    // Setup masternode voting power
    // 2 Gold (100 each), 3 Silver (10 each), 5 Bronze (1 each)
    // Total voting power: 200 + 30 + 5 = 235
    let total_power = 235u64;
    proposal.set_total_voting_power(total_power);

    // Create masternodes with tier-based voting power
    let gold1 = create_masternode("gold-mn-1", 100);
    let gold2 = create_masternode("gold-mn-2", 100);
    let silver1 = create_masternode("silver-mn-1", 10);
    let silver2 = create_masternode("silver-mn-2", 10);
    let silver3 = create_masternode("silver-mn-3", 10);
    let bronze1 = create_masternode("bronze-mn-1", 1);
    let bronze2 = create_masternode("bronze-mn-2", 1);
    let bronze3 = create_masternode("bronze-mn-3", 1);

    // Vote YES: Both Gold nodes + 2 Silver = 200 + 20 = 220 power
    proposal
        .add_vote(gold1.0.clone(), VoteChoice::Yes, gold1.1, 2000)
        .expect("Gold1 vote failed");
    proposal
        .add_vote(gold2.0.clone(), VoteChoice::Yes, gold2.1, 2100)
        .expect("Gold2 vote failed");
    proposal
        .add_vote(silver1.0.clone(), VoteChoice::Yes, silver1.1, 2200)
        .expect("Silver1 vote failed");
    proposal
        .add_vote(silver2.0.clone(), VoteChoice::Yes, silver2.1, 2300)
        .expect("Silver2 vote failed");

    // Vote NO: 1 Silver + 2 Bronze = 10 + 2 = 12 power
    proposal
        .add_vote(silver3.0.clone(), VoteChoice::No, silver3.1, 2400)
        .expect("Silver3 vote failed");
    proposal
        .add_vote(bronze1.0.clone(), VoteChoice::No, bronze1.1, 2500)
        .expect("Bronze1 vote failed");
    proposal
        .add_vote(bronze2.0.clone(), VoteChoice::No, bronze2.1, 2600)
        .expect("Bronze2 vote failed");

    // Abstain: 1 Bronze = 1 power
    proposal
        .add_vote(bronze3.0.clone(), VoteChoice::Abstain, bronze3.1, 2700)
        .expect("Bronze3 vote failed");

    // Calculate results
    let results = proposal.calculate_results();
    assert_eq!(results.yes_power, 220);
    assert_eq!(results.no_power, 12);
    assert_eq!(results.abstain_power, 1);
    assert_eq!(results.total_votes, 233);

    // YES percentage: 220/233 = 94.4% (well above 67% threshold)
    assert!(
        proposal.has_approval(),
        "Proposal should be approved with 94.4% YES votes"
    );

    // Update status after voting deadline
    let after_deadline = proposal.voting_deadline + 1;
    proposal.update_status(after_deadline);

    assert_eq!(
        proposal.status,
        ProposalStatus::Approved,
        "Proposal should be approved after voting deadline"
    );

    // Verify participation rate
    let participation = results.participation_rate();
    assert_eq!(participation, 99); // 233/235 = 99%
}

/// Scenario 3: Test proposal rejection by masternodes
#[test]
fn test_scenario_3_proposal_rejection_by_masternodes() {
    let mut proposal = TreasuryProposal::new(ProposalParams {
        id: "controversial-001".to_string(),
        title: "Controversial Proposal".to_string(),
        description: "A proposal that will be rejected".to_string(),
        recipient: "time1recipient...".to_string(),
        amount: 50_000 * TIME_UNIT,
        submitter: "time1submitter...".to_string(),
        submission_time: 1000,
        voting_period_days: 14,
    });

    // Total power: 3 Gold (300) + 2 Silver (20) + 3 Bronze (3) = 323
    proposal.set_total_voting_power(323);

    // Vote YES: 1 Gold + 1 Silver = 110 power
    proposal
        .add_vote("gold-mn-1".to_string(), VoteChoice::Yes, 100, 2000)
        .expect("Vote failed");
    proposal
        .add_vote("silver-mn-1".to_string(), VoteChoice::Yes, 10, 2100)
        .expect("Vote failed");

    // Vote NO: 2 Gold + 1 Silver + 2 Bronze = 212 power
    proposal
        .add_vote("gold-mn-2".to_string(), VoteChoice::No, 100, 2200)
        .expect("Vote failed");
    proposal
        .add_vote("gold-mn-3".to_string(), VoteChoice::No, 100, 2300)
        .expect("Vote failed");
    proposal
        .add_vote("silver-mn-2".to_string(), VoteChoice::No, 10, 2400)
        .expect("Vote failed");
    proposal
        .add_vote("bronze-mn-1".to_string(), VoteChoice::No, 1, 2500)
        .expect("Vote failed");
    proposal
        .add_vote("bronze-mn-2".to_string(), VoteChoice::No, 1, 2600)
        .expect("Vote failed");

    // Calculate results
    let results = proposal.calculate_results();
    assert_eq!(results.yes_power, 110);
    assert_eq!(results.no_power, 212);
    assert_eq!(results.total_votes, 322);

    // YES percentage: 110/322 = 34.2% (well below 67% threshold)
    assert!(
        !proposal.has_approval(),
        "Proposal should not be approved with only 34% YES votes"
    );

    // Update status after voting deadline
    let after_deadline = proposal.voting_deadline + 1;
    proposal.update_status(after_deadline);

    assert_eq!(
        proposal.status,
        ProposalStatus::Rejected,
        "Proposal should be rejected after voting deadline"
    );
}

/// Scenario 4: Test proposal expiration (approved but not executed in time)
#[test]
fn test_scenario_4_proposal_expiration() {
    let mut proposal = TreasuryProposal::new(ProposalParams {
        id: "expire-001".to_string(),
        title: "Expiring Proposal".to_string(),
        description: "This will be approved but expire without execution".to_string(),
        recipient: "time1recipient...".to_string(),
        amount: 5_000 * TIME_UNIT,
        submitter: "time1submitter...".to_string(),
        submission_time: 1000,
        voting_period_days: 14,
    });

    proposal.set_total_voting_power(200);

    // Get overwhelming approval (2 Gold nodes)
    proposal
        .add_vote("gold-mn-1".to_string(), VoteChoice::Yes, 100, 2000)
        .expect("Vote failed");
    proposal
        .add_vote("gold-mn-2".to_string(), VoteChoice::Yes, 100, 2100)
        .expect("Vote failed");

    assert!(proposal.has_approval(), "Proposal should be approved");

    // Update status after voting deadline - should be approved
    let after_voting = proposal.voting_deadline + 1;
    proposal.update_status(after_voting);
    assert_eq!(proposal.status, ProposalStatus::Approved);

    // Check expiration before deadline - should not be expired
    let before_expiry = proposal.execution_deadline;
    assert!(
        !proposal.is_expired(before_expiry),
        "Proposal should not be expired before execution deadline"
    );

    // Check expiration after deadline - should be expired
    let after_expiry = proposal.execution_deadline + 1;
    assert!(
        proposal.is_expired(after_expiry),
        "Proposal should be expired after execution deadline"
    );

    // Verify execution deadline is 30 days after voting deadline
    let expected_deadline = proposal.voting_deadline + (30 * 86400);
    assert_eq!(
        proposal.execution_deadline, expected_deadline,
        "Execution deadline should be 30 days after voting deadline"
    );
}

/// Scenario 5: Test insufficient funds handling
#[test]
fn test_scenario_5_insufficient_funds_validation() {
    // Create treasury with limited funds
    let mut pool = TreasuryPool::new();

    // Add only 1,000 TIME to treasury
    for block_num in 1..=200 {
        pool.deposit_block_reward(block_num, 1000 + block_num)
            .expect("Failed to deposit block reward");
    }

    // 200 blocks * 5 TIME = 1,000 TIME
    assert_eq!(pool.balance(), 200 * TREASURY_BLOCK_REWARD);
    assert_eq!(pool.balance_time(), 1000.0);

    // Create proposal requesting more than available
    let mut proposal = TreasuryProposal::new(ProposalParams {
        id: "overspend-001".to_string(),
        title: "Large Request".to_string(),
        description: "Requesting more than treasury has".to_string(),
        recipient: "time1recipient...".to_string(),
        amount: 5_000 * TIME_UNIT, // Request 5,000 TIME but only 1,000 available
        submitter: "time1submitter...".to_string(),
        submission_time: 1000,
        voting_period_days: 14,
    });

    proposal.set_total_voting_power(100);

    // Get approval
    proposal
        .add_vote("gold-mn-1".to_string(), VoteChoice::Yes, 100, 2000)
        .expect("Vote failed");

    assert!(proposal.has_approval());

    // Update status - should be approved
    let after_deadline = proposal.voting_deadline + 1;
    proposal.update_status(after_deadline);
    assert_eq!(proposal.status, ProposalStatus::Approved);

    // Try to schedule withdrawal - should fail due to insufficient balance
    let withdrawal = TreasuryWithdrawal {
        id: "withdrawal-001".to_string(),
        proposal_id: proposal.id.clone(),
        milestone_id: None,
        amount: proposal.amount,
        recipient: proposal.recipient.clone(),
        scheduled_time: 1000,
        executed_time: None,
        status: treasury::pool::WithdrawalStatus::Scheduled,
    };

    // Schedule the withdrawal - should fail due to insufficient balance
    let schedule_result = pool.schedule_withdrawal(withdrawal);

    assert!(
        schedule_result.is_err(),
        "Scheduling should fail when treasury has insufficient funds"
    );

    // Verify error message indicates insufficient funds
    match schedule_result {
        Err(TreasuryError::InsufficientBalance {
            requested,
            available,
        }) => {
            assert_eq!(requested, proposal.amount);
            assert_eq!(available, 200 * TREASURY_BLOCK_REWARD);
        }
        _ => panic!("Expected InsufficientBalance error"),
    }

    // Verify treasury balance unchanged
    assert_eq!(
        pool.balance(),
        200 * TREASURY_BLOCK_REWARD,
        "Treasury balance should be unchanged after failed withdrawal"
    );
}

/// Test one-vote-per-masternode enforcement
#[test]
fn test_one_vote_per_masternode_validation() {
    let mut proposal = TreasuryProposal::new(ProposalParams {
        id: "double-vote-test".to_string(),
        title: "Test Double Voting".to_string(),
        description: "Test that masternodes can only vote once".to_string(),
        recipient: "time1recipient...".to_string(),
        amount: 1_000 * TIME_UNIT,
        submitter: "time1submitter...".to_string(),
        submission_time: 1000,
        voting_period_days: 14,
    });

    // First vote should succeed
    let result1 = proposal.add_vote("mn-1".to_string(), VoteChoice::Yes, 100, 2000);
    assert!(result1.is_ok(), "First vote should succeed");

    // Second vote from same masternode should fail
    let result2 = proposal.add_vote("mn-1".to_string(), VoteChoice::No, 100, 2100);
    assert!(
        result2.is_err(),
        "Second vote from same masternode should fail"
    );

    // Verify only one vote recorded
    assert_eq!(proposal.votes.len(), 1, "Should only have one vote");

    // Verify the original vote is preserved
    let vote = proposal.votes.get("mn-1").expect("Vote should exist");
    assert_eq!(vote.vote_choice, VoteChoice::Yes);
}

/// Test masternode tier weights are correctly applied
#[test]
fn test_masternode_tier_weights() {
    let mut proposal = TreasuryProposal::new(ProposalParams {
        id: "tier-test".to_string(),
        title: "Tier Weight Test".to_string(),
        description: "Test that tier weights are correctly applied".to_string(),
        recipient: "time1recipient...".to_string(),
        amount: 1_000 * TIME_UNIT,
        submitter: "time1submitter...".to_string(),
        submission_time: 1000,
        voting_period_days: 14,
    });

    // Total power: 1 Gold (100) + 1 Silver (10) + 1 Bronze (1) = 111
    proposal.set_total_voting_power(111);

    // All vote YES with their respective tier weights
    proposal
        .add_vote("gold-mn".to_string(), VoteChoice::Yes, 100, 2000)
        .expect("Gold vote failed");
    proposal
        .add_vote("silver-mn".to_string(), VoteChoice::Yes, 10, 2100)
        .expect("Silver vote failed");
    proposal
        .add_vote("bronze-mn".to_string(), VoteChoice::Yes, 1, 2200)
        .expect("Bronze vote failed");

    let results = proposal.calculate_results();

    // Verify weights
    assert_eq!(results.yes_power, 111, "Total YES power should be 111");
    assert_eq!(results.total_votes, 111);

    // Gold should have 100x weight of Bronze
    // Silver should have 10x weight of Bronze
    // This is already validated by the voting power values
    assert!(
        proposal.has_approval(),
        "Should be approved with 100% YES votes"
    );
}

/// Test voting after deadline is rejected
#[test]
fn test_voting_after_deadline_rejected() {
    let mut proposal = TreasuryProposal::new(ProposalParams {
        id: "late-vote-test".to_string(),
        title: "Late Vote Test".to_string(),
        description: "Test that votes after deadline are rejected".to_string(),
        recipient: "time1recipient...".to_string(),
        amount: 1_000 * TIME_UNIT,
        submitter: "time1submitter...".to_string(),
        submission_time: 1000,
        voting_period_days: 14,
    });

    // Try to vote after deadline
    let after_deadline = proposal.voting_deadline + 1;
    let result = proposal.add_vote("mn-1".to_string(), VoteChoice::Yes, 100, after_deadline);

    assert!(result.is_err(), "Vote after deadline should be rejected");
    assert_eq!(proposal.votes.len(), 0, "No votes should be recorded");
}

/// Test voting on non-active proposal is rejected
#[test]
fn test_voting_on_non_active_proposal_rejected() {
    let mut proposal = TreasuryProposal::new(ProposalParams {
        id: "inactive-test".to_string(),
        title: "Inactive Proposal Test".to_string(),
        description: "Test that votes on non-active proposals are rejected".to_string(),
        recipient: "time1recipient...".to_string(),
        amount: 1_000 * TIME_UNIT,
        submitter: "time1submitter...".to_string(),
        submission_time: 1000,
        voting_period_days: 14,
    });

    // Change status to Approved
    proposal.status = ProposalStatus::Approved;

    // Try to vote
    let result = proposal.add_vote("mn-1".to_string(), VoteChoice::Yes, 100, 2000);

    assert!(
        result.is_err(),
        "Vote on non-active proposal should be rejected"
    );
    assert_eq!(proposal.votes.len(), 0, "No votes should be recorded");
}

/// Test edge case: Exactly 67% approval threshold
#[test]
fn test_exact_67_percent_approval_threshold() {
    let mut proposal = TreasuryProposal::new(ProposalParams {
        id: "threshold-test".to_string(),
        title: "Threshold Test".to_string(),
        description: "Test exact 67% threshold".to_string(),
        recipient: "time1recipient...".to_string(),
        amount: 1_000 * TIME_UNIT,
        submitter: "time1submitter...".to_string(),
        submission_time: 1000,
        voting_period_days: 14,
    });

    // Use power values that give exactly 67%
    // 67 YES, 33 NO = 67/100 = 67%
    proposal
        .add_vote("mn-yes".to_string(), VoteChoice::Yes, 67, 2000)
        .expect("YES vote failed");
    proposal
        .add_vote("mn-no".to_string(), VoteChoice::No, 33, 2100)
        .expect("NO vote failed");

    let results = proposal.calculate_results();
    assert_eq!(results.yes_power, 67);
    assert_eq!(results.no_power, 33);
    assert_eq!(results.approval_percentage(), 67);

    assert!(
        proposal.has_approval(),
        "Proposal with exactly 67% YES should be approved"
    );
}

/// Test edge case: Just below 67% approval threshold
#[test]
fn test_below_67_percent_threshold_rejected() {
    let mut proposal = TreasuryProposal::new(ProposalParams {
        id: "below-threshold-test".to_string(),
        title: "Below Threshold Test".to_string(),
        description: "Test just below 67% threshold".to_string(),
        recipient: "time1recipient...".to_string(),
        amount: 1_000 * TIME_UNIT,
        submitter: "time1submitter...".to_string(),
        submission_time: 1000,
        voting_period_days: 14,
    });

    // 66 YES, 34 NO = 66/100 = 66% (below threshold)
    proposal
        .add_vote("mn-yes".to_string(), VoteChoice::Yes, 66, 2000)
        .expect("YES vote failed");
    proposal
        .add_vote("mn-no".to_string(), VoteChoice::No, 34, 2100)
        .expect("NO vote failed");

    let results = proposal.calculate_results();
    assert_eq!(results.approval_percentage(), 66);

    assert!(
        !proposal.has_approval(),
        "Proposal with 66% YES should not be approved"
    );
}
