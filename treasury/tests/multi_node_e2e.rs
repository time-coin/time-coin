//! End-to-End Multi-Node Treasury Consensus Tests
//!
//! Tests complete treasury lifecycle scenarios with multiple masternodes
//! voting on proposals, simulating real-world consensus behavior.

use treasury::*;

/// Represents a masternode with its tier and voting power
#[derive(Clone)]
struct Masternode {
    id: String,
    voting_power: u64,
    active: bool,
}

impl Masternode {
    fn new(id: &str, _tier: &str, voting_power: u64) -> Self {
        Self {
            id: id.to_string(),
            voting_power,
            active: true,
        }
    }

    fn gold(id: &str) -> Self {
        Self::new(id, "Gold", 100)
    }

    fn silver(id: &str) -> Self {
        Self::new(id, "Silver", 10)
    }

    fn bronze(id: &str) -> Self {
        Self::new(id, "Bronze", 1)
    }
}

/// Simulates a network of masternodes
struct MasternodeNetwork {
    nodes: Vec<Masternode>,
}

impl MasternodeNetwork {
    fn new(nodes: Vec<Masternode>) -> Self {
        Self { nodes }
    }

    fn total_voting_power(&self) -> u64 {
        self.nodes
            .iter()
            .filter(|n| n.active)
            .map(|n| n.voting_power)
            .sum()
    }
}

/// End-to-end test: Complete proposal lifecycle with mixed masternode tiers
#[test]
fn test_e2e_complete_proposal_lifecycle_mixed_tiers() {
    // Setup: Create a network with diverse masternode tiers
    let network = MasternodeNetwork::new(vec![
        Masternode::gold("gold-node-1"),
        Masternode::gold("gold-node-2"),
        Masternode::gold("gold-node-3"),
        Masternode::silver("silver-node-1"),
        Masternode::silver("silver-node-2"),
        Masternode::silver("silver-node-3"),
        Masternode::silver("silver-node-4"),
        Masternode::silver("silver-node-5"),
        Masternode::bronze("bronze-node-1"),
        Masternode::bronze("bronze-node-2"),
        Masternode::bronze("bronze-node-3"),
        Masternode::bronze("bronze-node-4"),
        Masternode::bronze("bronze-node-5"),
    ]);

    // Total power: 3×100 + 5×10 + 5×1 = 300 + 50 + 5 = 355
    let total_power = network.total_voting_power();
    assert_eq!(
        total_power, 355,
        "Network should have 355 total voting power"
    );

    // Phase 1: Fund the treasury
    let mut treasury = TreasuryPool::new();
    for block in 1..=500 {
        treasury
            .deposit_block_reward(block, 1000 + block)
            .expect("Block deposit failed");
    }

    // 500 blocks × 5 TIME = 2,500 TIME
    assert_eq!(treasury.balance_time(), 2500.0);

    // Phase 2: Create a proposal
    let mut proposal = TreasuryProposal::new(ProposalParams {
        id: "e2e-proposal-001".to_string(),
        title: "Community Development Fund".to_string(),
        description: "Fund community development initiatives for Q1 2025".to_string(),
        recipient: "time1community...".to_string(),
        amount: 1_000 * TIME_UNIT, // Request 1,000 TIME
        submitter: "time1submitter...".to_string(),
        submission_time: 10000,
        voting_period_days: 14,
    });

    proposal.set_total_voting_power(total_power);

    // Phase 3: Voting process - simulate asynchronous voting over time
    // All Gold nodes vote YES (300 power)
    proposal
        .add_vote("gold-node-1".to_string(), VoteChoice::Yes, 100, 10100)
        .expect("Gold-1 vote failed");
    proposal
        .add_vote("gold-node-2".to_string(), VoteChoice::Yes, 100, 10200)
        .expect("Gold-2 vote failed");
    proposal
        .add_vote("gold-node-3".to_string(), VoteChoice::Yes, 100, 10300)
        .expect("Gold-3 vote failed");

    // 3 Silver nodes vote YES, 2 vote NO (30 YES, 20 NO)
    proposal
        .add_vote("silver-node-1".to_string(), VoteChoice::Yes, 10, 10400)
        .expect("Silver-1 vote failed");
    proposal
        .add_vote("silver-node-2".to_string(), VoteChoice::Yes, 10, 10500)
        .expect("Silver-2 vote failed");
    proposal
        .add_vote("silver-node-3".to_string(), VoteChoice::Yes, 10, 10600)
        .expect("Silver-3 vote failed");
    proposal
        .add_vote("silver-node-4".to_string(), VoteChoice::No, 10, 10700)
        .expect("Silver-4 vote failed");
    proposal
        .add_vote("silver-node-5".to_string(), VoteChoice::No, 10, 10800)
        .expect("Silver-5 vote failed");

    // 3 Bronze vote YES, 1 votes NO, 1 abstains
    proposal
        .add_vote("bronze-node-1".to_string(), VoteChoice::Yes, 1, 10900)
        .expect("Bronze-1 vote failed");
    proposal
        .add_vote("bronze-node-2".to_string(), VoteChoice::Yes, 1, 11000)
        .expect("Bronze-2 vote failed");
    proposal
        .add_vote("bronze-node-3".to_string(), VoteChoice::Yes, 1, 11100)
        .expect("Bronze-3 vote failed");
    proposal
        .add_vote("bronze-node-4".to_string(), VoteChoice::No, 1, 11200)
        .expect("Bronze-4 vote failed");
    proposal
        .add_vote("bronze-node-5".to_string(), VoteChoice::Abstain, 1, 11300)
        .expect("Bronze-5 vote failed");

    // Phase 4: Calculate results
    let results = proposal.calculate_results();

    // YES: 300 + 30 + 3 = 333
    // NO: 20 + 1 = 21
    // ABSTAIN: 1
    // TOTAL: 355
    assert_eq!(results.yes_power, 333);
    assert_eq!(results.no_power, 21);
    assert_eq!(results.abstain_power, 1);
    assert_eq!(results.total_votes, 355);

    // 333/355 = 93.8% YES - should be approved
    let approval_pct = results.approval_percentage();
    assert!(approval_pct >= 67, "Should have ≥67% approval");
    assert!(proposal.has_approval(), "Proposal should be approved");

    // Phase 5: Update status after voting deadline
    let after_deadline = proposal.voting_deadline + 1;
    proposal.update_status(after_deadline);
    assert_eq!(proposal.status, ProposalStatus::Approved);

    // Phase 6: Execute the proposal (withdraw funds)
    let withdrawal = TreasuryWithdrawal {
        id: "withdrawal-e2e-001".to_string(),
        proposal_id: proposal.id.clone(),
        milestone_id: None,
        amount: proposal.amount,
        recipient: proposal.recipient.clone(),
        scheduled_time: after_deadline,
        executed_time: None,
        status: treasury::pool::WithdrawalStatus::Scheduled,
    };

    treasury
        .schedule_withdrawal(withdrawal)
        .expect("Failed to schedule withdrawal");

    let withdrawn = treasury
        .execute_withdrawal("withdrawal-e2e-001", after_deadline + 100)
        .expect("Failed to execute withdrawal");

    assert_eq!(withdrawn, 1_000 * TIME_UNIT);
    assert_eq!(
        treasury.balance_time(),
        1500.0,
        "Treasury should have 1,500 TIME remaining"
    );

    // Phase 7: Mark proposal as executed
    proposal
        .mark_executed()
        .expect("Failed to mark as executed");
    assert_eq!(proposal.status, ProposalStatus::Executed);
}

/// Test: Multiple concurrent proposals with different outcomes
#[test]
fn test_e2e_multiple_concurrent_proposals() {
    let network = MasternodeNetwork::new(vec![
        Masternode::gold("gold-1"),
        Masternode::gold("gold-2"),
        Masternode::silver("silver-1"),
        Masternode::silver("silver-2"),
        Masternode::bronze("bronze-1"),
    ]);

    let total_power = network.total_voting_power(); // 200 + 20 + 1 = 221

    // Create three concurrent proposals
    let mut proposal_a = TreasuryProposal::new(ProposalParams {
        id: "proposal-a".to_string(),
        title: "Proposal A - Will Pass".to_string(),
        description: "This will be approved".to_string(),
        recipient: "recipient-a".to_string(),
        amount: 100 * TIME_UNIT,
        submitter: "submitter".to_string(),
        submission_time: 1000,
        voting_period_days: 14,
    });
    proposal_a.set_total_voting_power(total_power);

    let mut proposal_b = TreasuryProposal::new(ProposalParams {
        id: "proposal-b".to_string(),
        title: "Proposal B - Will Fail".to_string(),
        description: "This will be rejected".to_string(),
        recipient: "recipient-b".to_string(),
        amount: 200 * TIME_UNIT,
        submitter: "submitter".to_string(),
        submission_time: 1000,
        voting_period_days: 14,
    });
    proposal_b.set_total_voting_power(total_power);

    let mut proposal_c = TreasuryProposal::new(ProposalParams {
        id: "proposal-c".to_string(),
        title: "Proposal C - Close Call".to_string(),
        description: "This will barely pass".to_string(),
        recipient: "recipient-c".to_string(),
        amount: 50 * TIME_UNIT,
        submitter: "submitter".to_string(),
        submission_time: 1000,
        voting_period_days: 14,
    });
    proposal_c.set_total_voting_power(total_power);

    // Voting on Proposal A: Strong YES (both Gold + 1 Silver = 210 YES, 11 NO)
    proposal_a
        .add_vote("gold-1".to_string(), VoteChoice::Yes, 100, 2000)
        .unwrap();
    proposal_a
        .add_vote("gold-2".to_string(), VoteChoice::Yes, 100, 2100)
        .unwrap();
    proposal_a
        .add_vote("silver-1".to_string(), VoteChoice::Yes, 10, 2200)
        .unwrap();
    proposal_a
        .add_vote("silver-2".to_string(), VoteChoice::No, 10, 2300)
        .unwrap();
    proposal_a
        .add_vote("bronze-1".to_string(), VoteChoice::No, 1, 2400)
        .unwrap();

    // Voting on Proposal B: Strong NO (1 Gold YES, rest NO = 100 YES, 121 NO)
    proposal_b
        .add_vote("gold-1".to_string(), VoteChoice::Yes, 100, 2000)
        .unwrap();
    proposal_b
        .add_vote("gold-2".to_string(), VoteChoice::No, 100, 2100)
        .unwrap();
    proposal_b
        .add_vote("silver-1".to_string(), VoteChoice::No, 10, 2200)
        .unwrap();
    proposal_b
        .add_vote("silver-2".to_string(), VoteChoice::No, 10, 2300)
        .unwrap();
    proposal_b
        .add_vote("bronze-1".to_string(), VoteChoice::No, 1, 2400)
        .unwrap();

    // Voting on Proposal C: Exactly at threshold (148 YES, 73 NO = 67%)
    // Need 67% of 221 = ~148.07
    proposal_c
        .add_vote("gold-1".to_string(), VoteChoice::Yes, 100, 2000)
        .unwrap();
    proposal_c
        .add_vote("gold-2".to_string(), VoteChoice::No, 100, 2100)
        .unwrap();
    proposal_c
        .add_vote("silver-1".to_string(), VoteChoice::Yes, 10, 2200)
        .unwrap();
    proposal_c
        .add_vote("silver-2".to_string(), VoteChoice::No, 10, 2300)
        .unwrap();
    // Don't let bronze vote to test partial participation

    // Check results
    let results_a = proposal_a.calculate_results();
    assert_eq!(results_a.yes_power, 210);
    assert_eq!(results_a.no_power, 11);
    assert!(
        proposal_a.has_approval(),
        "Proposal A should pass with 95% YES"
    );

    let results_b = proposal_b.calculate_results();
    assert_eq!(results_b.yes_power, 100);
    assert_eq!(results_b.no_power, 121);
    assert!(
        !proposal_b.has_approval(),
        "Proposal B should fail with 45% YES"
    );

    let results_c = proposal_c.calculate_results();
    assert_eq!(results_c.yes_power, 110);
    assert_eq!(results_c.no_power, 110);
    // 110/220 = 50% - should NOT pass (need 67%)
    assert!(
        !proposal_c.has_approval(),
        "Proposal C should fail with 50% YES"
    );

    // Update all statuses
    let after_deadline = proposal_a.voting_deadline + 1;
    proposal_a.update_status(after_deadline);
    proposal_b.update_status(after_deadline);
    proposal_c.update_status(after_deadline);

    assert_eq!(proposal_a.status, ProposalStatus::Approved);
    assert_eq!(proposal_b.status, ProposalStatus::Rejected);
    assert_eq!(proposal_c.status, ProposalStatus::Rejected);
}

/// Test: Masternode network with some inactive nodes
#[test]
fn test_e2e_inactive_masternodes_excluded_from_voting() {
    let mut network = MasternodeNetwork::new(vec![
        Masternode::gold("gold-active"),
        Masternode::gold("gold-inactive"),
        Masternode::silver("silver-active"),
        Masternode::bronze("bronze-active"),
    ]);

    // Deactivate one gold node
    if let Some(node) = network.nodes.iter_mut().find(|n| n.id == "gold-inactive") {
        node.active = false;
    }

    // Active power: 100 (gold) + 10 (silver) + 1 (bronze) = 111
    let active_power = network.total_voting_power();
    assert_eq!(active_power, 111);

    let mut proposal = TreasuryProposal::new(ProposalParams {
        id: "inactive-test".to_string(),
        title: "Test Inactive Nodes".to_string(),
        description: "Only active nodes should count".to_string(),
        recipient: "recipient".to_string(),
        amount: 100 * TIME_UNIT,
        submitter: "submitter".to_string(),
        submission_time: 1000,
        voting_period_days: 14,
    });

    // Set total power based on active nodes only
    proposal.set_total_voting_power(active_power);

    // Active nodes vote
    proposal
        .add_vote("gold-active".to_string(), VoteChoice::Yes, 100, 2000)
        .unwrap();
    proposal
        .add_vote("silver-active".to_string(), VoteChoice::Yes, 10, 2100)
        .unwrap();
    proposal
        .add_vote("bronze-active".to_string(), VoteChoice::No, 1, 2200)
        .unwrap();

    let results = proposal.calculate_results();
    assert_eq!(results.yes_power, 110);
    assert_eq!(results.no_power, 1);
    assert_eq!(results.total_votes, 111);

    // 110/111 = 99.1% - should pass
    assert!(proposal.has_approval());
}

/// Test: Proposal with milestone-based funding
#[test]
fn test_e2e_milestone_based_proposal_execution() {
    let mut treasury = TreasuryPool::new();

    // Fund treasury with 10,000 TIME
    for block in 1..=2000 {
        treasury.deposit_block_reward(block, 1000 + block).unwrap();
    }
    assert_eq!(treasury.balance_time(), 10000.0);

    // Create proposal requesting 5,000 TIME in 3 milestones
    let mut proposal = TreasuryProposal::new(ProposalParams {
        id: "milestone-proposal".to_string(),
        title: "Multi-Milestone Development".to_string(),
        description: "Three-phase development with milestone payments".to_string(),
        recipient: "time1dev...".to_string(),
        amount: 5_000 * TIME_UNIT,
        submitter: "submitter".to_string(),
        submission_time: 1000,
        voting_period_days: 14,
    });

    proposal.set_total_voting_power(300);

    // Get approval with 100% YES votes
    proposal
        .add_vote("gold-1".to_string(), VoteChoice::Yes, 100, 2000)
        .unwrap();
    proposal
        .add_vote("gold-2".to_string(), VoteChoice::Yes, 100, 2100)
        .unwrap();
    proposal
        .add_vote("gold-3".to_string(), VoteChoice::Yes, 100, 2200)
        .unwrap();

    assert!(proposal.has_approval());

    let after_deadline = proposal.voting_deadline + 1;
    proposal.update_status(after_deadline);
    assert_eq!(proposal.status, ProposalStatus::Approved);

    // Execute milestone 1: 2,000 TIME
    let milestone1 = TreasuryWithdrawal {
        id: "milestone-1".to_string(),
        proposal_id: proposal.id.clone(),
        milestone_id: Some("M1".to_string()),
        amount: 2_000 * TIME_UNIT,
        recipient: proposal.recipient.clone(),
        scheduled_time: after_deadline,
        executed_time: None,
        status: treasury::pool::WithdrawalStatus::Scheduled,
    };

    treasury.schedule_withdrawal(milestone1).unwrap();
    treasury
        .execute_withdrawal("milestone-1", after_deadline + 100)
        .unwrap();
    assert_eq!(treasury.balance_time(), 8000.0);

    // Execute milestone 2: 2,000 TIME
    let milestone2 = TreasuryWithdrawal {
        id: "milestone-2".to_string(),
        proposal_id: proposal.id.clone(),
        milestone_id: Some("M2".to_string()),
        amount: 2_000 * TIME_UNIT,
        recipient: proposal.recipient.clone(),
        scheduled_time: after_deadline + 86400 * 30, // 30 days later
        executed_time: None,
        status: treasury::pool::WithdrawalStatus::Scheduled,
    };

    treasury.schedule_withdrawal(milestone2).unwrap();
    treasury
        .execute_withdrawal("milestone-2", after_deadline + 86400 * 30 + 100)
        .unwrap();
    assert_eq!(treasury.balance_time(), 6000.0);

    // Execute milestone 3: 1,000 TIME (final)
    let milestone3 = TreasuryWithdrawal {
        id: "milestone-3".to_string(),
        proposal_id: proposal.id.clone(),
        milestone_id: Some("M3".to_string()),
        amount: 1_000 * TIME_UNIT,
        recipient: proposal.recipient.clone(),
        scheduled_time: after_deadline + 86400 * 60, // 60 days later
        executed_time: None,
        status: treasury::pool::WithdrawalStatus::Scheduled,
    };

    treasury.schedule_withdrawal(milestone3).unwrap();
    treasury
        .execute_withdrawal("milestone-3", after_deadline + 86400 * 60 + 100)
        .unwrap();
    assert_eq!(treasury.balance_time(), 5000.0);

    // Verify all milestones executed correctly
    let withdrawals = treasury.scheduled_withdrawals();
    assert_eq!(withdrawals.len(), 3, "Should have 3 milestone withdrawals");
}

/// Test: Network consensus with varying participation rates
#[test]
fn test_e2e_varying_participation_rates() {
    let network = MasternodeNetwork::new(vec![
        Masternode::gold("g1"),
        Masternode::gold("g2"),
        Masternode::silver("s1"),
        Masternode::silver("s2"),
        Masternode::bronze("b1"),
        Masternode::bronze("b2"),
    ]);

    let total_power = network.total_voting_power(); // 212

    // Scenario 1: High participation (all nodes vote)
    let mut proposal_high = TreasuryProposal::new(ProposalParams {
        id: "high-participation".to_string(),
        title: "High Participation".to_string(),
        description: "Everyone votes".to_string(),
        recipient: "recipient".to_string(),
        amount: 100 * TIME_UNIT,
        submitter: "submitter".to_string(),
        submission_time: 1000,
        voting_period_days: 14,
    });
    proposal_high.set_total_voting_power(total_power);

    proposal_high
        .add_vote("g1".to_string(), VoteChoice::Yes, 100, 2000)
        .unwrap();
    proposal_high
        .add_vote("g2".to_string(), VoteChoice::Yes, 100, 2100)
        .unwrap();
    proposal_high
        .add_vote("s1".to_string(), VoteChoice::No, 10, 2200)
        .unwrap();
    proposal_high
        .add_vote("s2".to_string(), VoteChoice::No, 10, 2300)
        .unwrap();
    proposal_high
        .add_vote("b1".to_string(), VoteChoice::No, 1, 2400)
        .unwrap();
    proposal_high
        .add_vote("b2".to_string(), VoteChoice::No, 1, 2500)
        .unwrap();

    let results_high = proposal_high.calculate_results();
    assert_eq!(results_high.participation_rate(), 100); // All voted
    assert!(proposal_high.has_approval()); // 200/222 = 90%

    // Scenario 2: Low participation (only Gold nodes vote)
    let mut proposal_low = TreasuryProposal::new(ProposalParams {
        id: "low-participation".to_string(),
        title: "Low Participation".to_string(),
        description: "Only Gold votes".to_string(),
        recipient: "recipient".to_string(),
        amount: 100 * TIME_UNIT,
        submitter: "submitter".to_string(),
        submission_time: 1000,
        voting_period_days: 14,
    });
    proposal_low.set_total_voting_power(total_power);

    proposal_low
        .add_vote("g1".to_string(), VoteChoice::Yes, 100, 2000)
        .unwrap();
    proposal_low
        .add_vote("g2".to_string(), VoteChoice::Yes, 100, 2100)
        .unwrap();

    let results_low = proposal_low.calculate_results();
    // 200 voted out of 212 total
    let participation = (200 * 100) / total_power;
    assert_eq!(results_low.participation_rate(), participation);
    assert!(proposal_low.has_approval()); // 200/200 = 100% of those who voted
}

/// Test: Stress test with many proposals and votes
#[test]
fn test_e2e_stress_many_proposals() {
    let network =
        MasternodeNetwork::new(vec![Masternode::gold("gold-1"), Masternode::gold("gold-2")]);

    let total_power = network.total_voting_power();

    // Create 50 proposals
    let mut proposals = Vec::new();
    for i in 0..50 {
        let mut proposal = TreasuryProposal::new(ProposalParams {
            id: format!("stress-proposal-{:03}", i),
            title: format!("Stress Test Proposal #{}", i),
            description: format!("Testing system with many proposals: #{}", i),
            recipient: format!("recipient-{}", i),
            amount: (10 + i) * TIME_UNIT,
            submitter: "stress-tester".to_string(),
            submission_time: 1000,
            voting_period_days: 14,
        });

        proposal.set_total_voting_power(total_power);

        // Alternate approval pattern
        if i % 3 == 0 {
            // Approve every third proposal
            proposal
                .add_vote("gold-1".to_string(), VoteChoice::Yes, 100, 2000)
                .unwrap();
            proposal
                .add_vote("gold-2".to_string(), VoteChoice::Yes, 100, 2100)
                .unwrap();
        } else {
            // Reject others
            proposal
                .add_vote("gold-1".to_string(), VoteChoice::No, 100, 2000)
                .unwrap();
            proposal
                .add_vote("gold-2".to_string(), VoteChoice::Yes, 100, 2100)
                .unwrap();
        }

        proposals.push(proposal);
    }

    // Count approvals
    let approved_count = proposals.iter().filter(|p| p.has_approval()).count();

    // Every 3rd proposal (0, 3, 6, ..., 48) = 17 proposals
    assert_eq!(approved_count, 17, "Should have 17 approved proposals");

    // Update all statuses
    let after_deadline = proposals[0].voting_deadline + 1;
    for proposal in &mut proposals {
        proposal.update_status(after_deadline);
    }

    // Verify status distribution
    let approved = proposals
        .iter()
        .filter(|p| p.status == ProposalStatus::Approved)
        .count();
    let rejected = proposals
        .iter()
        .filter(|p| p.status == ProposalStatus::Rejected)
        .count();

    assert_eq!(approved, 17);
    assert_eq!(rejected, 33);
}
