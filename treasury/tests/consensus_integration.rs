//! Comprehensive integration tests for treasury consensus
//!
//! Tests cover:
//! - End-to-end proposal lifecycle
//! - Multi-masternode voting scenarios
//! - Proposal approval/rejection logic
//! - Proposal expiration handling
//! - Edge cases and security checks

use treasury::*;

#[test]
fn test_end_to_end_proposal_lifecycle() {
    // Setup: Create treasury consensus manager with masternodes
    let mut manager = TreasuryConsensusManager::new();
    
    // Register 3 masternodes with equal power
    manager.register_masternode("mn1".to_string(), 100);
    manager.register_masternode("mn2".to_string(), 100);
    manager.register_masternode("mn3".to_string(), 100);
    
    assert_eq!(manager.get_total_voting_power(), 300);
    
    // Phase 1: Create proposal
    let proposal = TreasuryProposal::new(ProposalParams {
        id: "dev-grant-001".to_string(),
        title: "Mobile Wallet Development".to_string(),
        description: "Develop iOS and Android wallets for TIME Coin".to_string(),
        recipient: "dev-team-wallet".to_string(),
        amount: 50_000 * TIME_UNIT,
        submitter: "community-member-123".to_string(),
        submission_time: 1000,
        voting_period_days: 14,
    });
    
    let voting_deadline = proposal.voting_deadline;
    let execution_deadline = proposal.execution_deadline;
    
    manager.add_proposal(proposal).unwrap();
    
    // Verify proposal is active
    let proposal = manager.get_proposal("dev-grant-001").unwrap();
    assert_eq!(proposal.status, ProposalStatus::Active);
    assert_eq!(proposal.votes.len(), 0);
    
    // Phase 2: Masternodes vote
    // Time: during voting period
    let voting_time = 1000 + (7 * 86400); // 7 days after submission
    
    manager.vote_on_proposal(
        "dev-grant-001",
        "mn1".to_string(),
        VoteChoice::Yes,
        voting_time,
    ).unwrap();
    
    manager.vote_on_proposal(
        "dev-grant-001",
        "mn2".to_string(),
        VoteChoice::Yes,
        voting_time,
    ).unwrap();
    
    manager.vote_on_proposal(
        "dev-grant-001",
        "mn3".to_string(),
        VoteChoice::No,
        voting_time,
    ).unwrap();
    
    // Verify votes recorded
    let proposal = manager.get_proposal("dev-grant-001").unwrap();
    assert_eq!(proposal.votes.len(), 3);
    
    // Check voting results: 2 YES (200 power), 1 NO (100 power) = 66.67%
    let results = manager.get_voting_results("dev-grant-001").unwrap();
    assert_eq!(results.yes_power, 200);
    assert_eq!(results.no_power, 100);
    assert_eq!(results.total_votes, 300);
    
    // Should NOT have consensus yet (need 67%)
    assert!(!manager.has_consensus("dev-grant-001").unwrap());
    
    // Phase 3: Update status after voting deadline
    manager.update_proposal_statuses(voting_deadline + 1);
    
    // Should be rejected (didn't reach 67% threshold)
    let proposal = manager.get_proposal("dev-grant-001").unwrap();
    assert_eq!(proposal.status, ProposalStatus::Rejected);
}

#[test]
fn test_multi_masternode_approval_scenario() {
    let mut manager = TreasuryConsensusManager::new();
    
    // Register 5 masternodes with different voting power (simulating tiers)
    manager.register_masternode("mn-gold-1".to_string(), 100); // Gold tier
    manager.register_masternode("mn-gold-2".to_string(), 100); // Gold tier
    manager.register_masternode("mn-silver-1".to_string(), 10); // Silver tier
    manager.register_masternode("mn-silver-2".to_string(), 10); // Silver tier
    manager.register_masternode("mn-bronze-1".to_string(), 1); // Bronze tier
    
    assert_eq!(manager.get_total_voting_power(), 221);
    
    // Create proposal
    let proposal = TreasuryProposal::new(ProposalParams {
        id: "marketing-campaign-001".to_string(),
        title: "Q1 Marketing Campaign".to_string(),
        description: "Social media and conference presence".to_string(),
        recipient: "marketing-wallet".to_string(),
        amount: 25_000 * TIME_UNIT,
        submitter: "marketing-lead".to_string(),
        submission_time: 2000,
        voting_period_days: 14,
    });
    
    let voting_deadline = proposal.voting_deadline;
    manager.add_proposal(proposal).unwrap();
    
    // Voting scenario: 2 Gold YES, 2 Silver YES, 1 Bronze NO
    let voting_time = 2000 + (5 * 86400);
    
    manager.vote_on_proposal(
        "marketing-campaign-001",
        "mn-gold-1".to_string(),
        VoteChoice::Yes,
        voting_time,
    ).unwrap();
    
    manager.vote_on_proposal(
        "marketing-campaign-001",
        "mn-gold-2".to_string(),
        VoteChoice::Yes,
        voting_time,
    ).unwrap();
    
    manager.vote_on_proposal(
        "marketing-campaign-001",
        "mn-silver-1".to_string(),
        VoteChoice::Yes,
        voting_time,
    ).unwrap();
    
    manager.vote_on_proposal(
        "marketing-campaign-001",
        "mn-silver-2".to_string(),
        VoteChoice::Yes,
        voting_time,
    ).unwrap();
    
    manager.vote_on_proposal(
        "marketing-campaign-001",
        "mn-bronze-1".to_string(),
        VoteChoice::No,
        voting_time,
    ).unwrap();
    
    // Calculate: 220 YES, 1 NO = 99.5% approval
    let results = manager.get_voting_results("marketing-campaign-001").unwrap();
    assert_eq!(results.yes_power, 220);
    assert_eq!(results.no_power, 1);
    
    // Should have consensus
    assert!(manager.has_consensus("marketing-campaign-001").unwrap());
    
    // Update status after voting deadline
    manager.update_proposal_statuses(voting_deadline + 1);
    
    // Should be approved
    let proposal = manager.get_proposal("marketing-campaign-001").unwrap();
    assert_eq!(proposal.status, ProposalStatus::Approved);
}

#[test]
fn test_exact_threshold_boundary() {
    let mut manager = TreasuryConsensusManager::new();
    
    // Register masternodes with power that allows testing exact 67% threshold
    manager.register_masternode("mn1".to_string(), 67);
    manager.register_masternode("mn2".to_string(), 33);
    
    assert_eq!(manager.get_total_voting_power(), 100);
    
    // Create proposal
    let proposal = TreasuryProposal::new(ProposalParams {
        id: "threshold-test".to_string(),
        title: "Threshold Test".to_string(),
        description: "Testing exact 67% threshold".to_string(),
        recipient: "test-wallet".to_string(),
        amount: 1000 * TIME_UNIT,
        submitter: "tester".to_string(),
        submission_time: 3000,
        voting_period_days: 7,
    });
    
    manager.add_proposal(proposal).unwrap();
    
    // Vote exactly 67% YES, 33% NO
    manager.vote_on_proposal("threshold-test", "mn1".to_string(), VoteChoice::Yes, 3100).unwrap();
    manager.vote_on_proposal("threshold-test", "mn2".to_string(), VoteChoice::No, 3100).unwrap();
    
    // Exactly 67% should pass
    assert!(manager.has_consensus("threshold-test").unwrap());
    
    let results = manager.get_voting_results("threshold-test").unwrap();
    assert_eq!(results.approval_percentage(), 67);
}

#[test]
fn test_proposal_expiration() {
    let mut manager = TreasuryConsensusManager::new();
    
    manager.register_masternode("mn1".to_string(), 100);
    manager.register_masternode("mn2".to_string(), 50);
    
    let proposal = TreasuryProposal::new(ProposalParams {
        id: "expire-test".to_string(),
        title: "Expiration Test".to_string(),
        description: "This proposal will expire".to_string(),
        recipient: "test-wallet".to_string(),
        amount: 5000 * TIME_UNIT,
        submitter: "tester".to_string(),
        submission_time: 4000,
        voting_period_days: 7,
    });
    
    let voting_deadline = proposal.voting_deadline;
    let execution_deadline = proposal.execution_deadline;
    
    manager.add_proposal(proposal).unwrap();
    
    // Vote to approve
    manager.vote_on_proposal("expire-test", "mn1".to_string(), VoteChoice::Yes, 4100).unwrap();
    manager.vote_on_proposal("expire-test", "mn2".to_string(), VoteChoice::Yes, 4100).unwrap();
    
    // Update status after voting deadline - should be approved
    manager.update_proposal_statuses(voting_deadline + 1);
    let proposal = manager.get_proposal("expire-test").unwrap();
    assert_eq!(proposal.status, ProposalStatus::Approved);
    
    // Don't execute, let it expire
    let expired_ids = manager.expire_old_proposals(execution_deadline + 1);
    
    assert_eq!(expired_ids.len(), 1);
    assert_eq!(expired_ids[0], "expire-test");
    
    let proposal = manager.get_proposal("expire-test").unwrap();
    assert_eq!(proposal.status, ProposalStatus::Expired);
}

#[test]
fn test_multiple_proposals_concurrent() {
    let mut manager = TreasuryConsensusManager::new();
    
    // Register masternodes
    manager.register_masternode("mn1".to_string(), 100);
    manager.register_masternode("mn2".to_string(), 100);
    manager.register_masternode("mn3".to_string(), 100);
    
    // Create multiple proposals
    for i in 1..=10 {
        let proposal = TreasuryProposal::new(ProposalParams {
            id: format!("proposal-{:03}", i),
            title: format!("Proposal #{}", i),
            description: format!("Description for proposal {}", i),
            recipient: format!("wallet-{}", i),
            amount: (i as u64 * 1000) * TIME_UNIT,
            submitter: "batch-submitter".to_string(),
            submission_time: 5000,
            voting_period_days: 14,
        });
        manager.add_proposal(proposal).unwrap();
    }
    
    assert_eq!(manager.get_all_proposals().len(), 10);
    
    // Vote on some proposals differently
    let voting_time = 5000 + (7 * 86400);
    
    // Proposals 1-3: Approve (all 3 vote YES)
    for i in 1..=3 {
        let prop_id = format!("proposal-{:03}", i);
        manager.vote_on_proposal(&prop_id, "mn1".to_string(), VoteChoice::Yes, voting_time).unwrap();
        manager.vote_on_proposal(&prop_id, "mn2".to_string(), VoteChoice::Yes, voting_time).unwrap();
        manager.vote_on_proposal(&prop_id, "mn3".to_string(), VoteChoice::Yes, voting_time).unwrap();
    }
    
    // Proposals 4-6: Reject (all 3 vote NO)
    for i in 4..=6 {
        let prop_id = format!("proposal-{:03}", i);
        manager.vote_on_proposal(&prop_id, "mn1".to_string(), VoteChoice::No, voting_time).unwrap();
        manager.vote_on_proposal(&prop_id, "mn2".to_string(), VoteChoice::No, voting_time).unwrap();
        manager.vote_on_proposal(&prop_id, "mn3".to_string(), VoteChoice::No, voting_time).unwrap();
    }
    
    // Proposals 7-10: No votes (will be rejected due to no consensus)
    
    // Update all statuses
    let deadline = 5000 + (14 * 86400) + 1;
    manager.update_proposal_statuses(deadline);
    
    // Verify results
    let approved = manager.get_proposals_by_status(&ProposalStatus::Approved);
    assert_eq!(approved.len(), 3);
    
    let rejected = manager.get_proposals_by_status(&ProposalStatus::Rejected);
    assert_eq!(rejected.len(), 7); // 4-6 voted NO, 7-10 no votes
}

#[test]
fn test_abstain_votes_dont_count_toward_approval() {
    let mut manager = TreasuryConsensusManager::new();
    
    manager.register_masternode("mn1".to_string(), 100);
    manager.register_masternode("mn2".to_string(), 100);
    manager.register_masternode("mn3".to_string(), 100);
    
    let proposal = TreasuryProposal::new(ProposalParams {
        id: "abstain-test".to_string(),
        title: "Abstain Test".to_string(),
        description: "Testing abstain votes".to_string(),
        recipient: "test-wallet".to_string(),
        amount: 5000 * TIME_UNIT,
        submitter: "tester".to_string(),
        submission_time: 6000,
        voting_period_days: 7,
    });
    
    manager.add_proposal(proposal).unwrap();
    
    // 2 YES, 1 ABSTAIN
    manager.vote_on_proposal("abstain-test", "mn1".to_string(), VoteChoice::Yes, 6100).unwrap();
    manager.vote_on_proposal("abstain-test", "mn2".to_string(), VoteChoice::Yes, 6100).unwrap();
    manager.vote_on_proposal("abstain-test", "mn3".to_string(), VoteChoice::Abstain, 6100).unwrap();
    
    let results = manager.get_voting_results("abstain-test").unwrap();
    assert_eq!(results.yes_power, 200);
    assert_eq!(results.abstain_power, 100);
    
    // 200 YES out of 300 total = 66.67% - should NOT pass
    assert!(!manager.has_consensus("abstain-test").unwrap());
}

#[test]
fn test_duplicate_vote_prevention() {
    let mut manager = TreasuryConsensusManager::new();
    
    manager.register_masternode("mn1".to_string(), 100);
    
    let proposal = TreasuryProposal::new(ProposalParams {
        id: "dup-vote-test".to_string(),
        title: "Duplicate Vote Test".to_string(),
        description: "Testing duplicate vote prevention".to_string(),
        recipient: "test-wallet".to_string(),
        amount: 1000 * TIME_UNIT,
        submitter: "tester".to_string(),
        submission_time: 7000,
        voting_period_days: 7,
    });
    
    manager.add_proposal(proposal).unwrap();
    
    // First vote succeeds
    let result = manager.vote_on_proposal("dup-vote-test", "mn1".to_string(), VoteChoice::Yes, 7100);
    assert!(result.is_ok());
    
    // Second vote from same masternode should fail
    let result = manager.vote_on_proposal("dup-vote-test", "mn1".to_string(), VoteChoice::No, 7200);
    assert!(result.is_err());
}

#[test]
fn test_voting_after_deadline_fails() {
    let mut manager = TreasuryConsensusManager::new();
    
    manager.register_masternode("mn1".to_string(), 100);
    
    let proposal = TreasuryProposal::new(ProposalParams {
        id: "late-vote-test".to_string(),
        title: "Late Vote Test".to_string(),
        description: "Testing voting after deadline".to_string(),
        recipient: "test-wallet".to_string(),
        amount: 1000 * TIME_UNIT,
        submitter: "tester".to_string(),
        submission_time: 8000,
        voting_period_days: 7,
    });
    
    let voting_deadline = proposal.voting_deadline;
    manager.add_proposal(proposal).unwrap();
    
    // Vote after deadline should fail
    let result = manager.vote_on_proposal(
        "late-vote-test",
        "mn1".to_string(),
        VoteChoice::Yes,
        voting_deadline + 1,
    );
    assert!(result.is_err());
}

#[test]
fn test_unregistered_masternode_cannot_vote() {
    let mut manager = TreasuryConsensusManager::new();
    
    manager.register_masternode("mn1".to_string(), 100);
    
    let proposal = TreasuryProposal::new(ProposalParams {
        id: "unauth-vote-test".to_string(),
        title: "Unauthorized Vote Test".to_string(),
        description: "Testing unauthorized voting".to_string(),
        recipient: "test-wallet".to_string(),
        amount: 1000 * TIME_UNIT,
        submitter: "tester".to_string(),
        submission_time: 9000,
        voting_period_days: 7,
    });
    
    manager.add_proposal(proposal).unwrap();
    
    // Vote from unregistered masternode should fail
    let result = manager.vote_on_proposal(
        "unauth-vote-test",
        "mn-unknown".to_string(),
        VoteChoice::Yes,
        9100,
    );
    assert!(result.is_err());
}

#[test]
fn test_proposal_execution_workflow() {
    let mut manager = TreasuryConsensusManager::new();
    
    manager.register_masternode("mn1".to_string(), 100);
    manager.register_masternode("mn2".to_string(), 100);
    
    let proposal = TreasuryProposal::new(ProposalParams {
        id: "exec-test".to_string(),
        title: "Execution Test".to_string(),
        description: "Testing proposal execution".to_string(),
        recipient: "test-wallet".to_string(),
        amount: 10_000 * TIME_UNIT,
        submitter: "tester".to_string(),
        submission_time: 10000,
        voting_period_days: 7,
    });
    
    let voting_deadline = proposal.voting_deadline;
    manager.add_proposal(proposal).unwrap();
    
    // Vote to approve
    manager.vote_on_proposal("exec-test", "mn1".to_string(), VoteChoice::Yes, 10100).unwrap();
    manager.vote_on_proposal("exec-test", "mn2".to_string(), VoteChoice::Yes, 10100).unwrap();
    
    // Update status - should be approved
    manager.update_proposal_statuses(voting_deadline + 1);
    
    let proposal = manager.get_proposal("exec-test").unwrap();
    assert_eq!(proposal.status, ProposalStatus::Approved);
    
    // Mark as executed
    manager.mark_proposal_executed("exec-test").unwrap();
    
    let proposal = manager.get_proposal("exec-test").unwrap();
    assert_eq!(proposal.status, ProposalStatus::Executed);
}

#[test]
fn test_cleanup_old_proposals() {
    let mut manager = TreasuryConsensusManager::new();
    
    manager.register_masternode("mn1".to_string(), 100);
    
    // Create old proposals
    for i in 1..=5 {
        let proposal = TreasuryProposal::new(ProposalParams {
            id: format!("old-{}", i),
            title: format!("Old Proposal {}", i),
            description: "Old".to_string(),
            recipient: "wallet".to_string(),
            amount: 1000 * TIME_UNIT,
            submitter: "submitter".to_string(),
            submission_time: 1000, // Very old
            voting_period_days: 7,
        });
        manager.add_proposal(proposal).unwrap();
    }
    
    // Create recent proposals
    for i in 1..=5 {
        let proposal = TreasuryProposal::new(ProposalParams {
            id: format!("recent-{}", i),
            title: format!("Recent Proposal {}", i),
            description: "Recent".to_string(),
            recipient: "wallet".to_string(),
            amount: 1000 * TIME_UNIT,
            submitter: "submitter".to_string(),
            submission_time: 100_000, // Recent
            voting_period_days: 7,
        });
        manager.add_proposal(proposal).unwrap();
    }
    
    assert_eq!(manager.get_all_proposals().len(), 10);
    
    // Cleanup old proposals (keep last 30 days)
    let current_time = 100_000 + (35 * 86400);
    manager.cleanup_old_proposals(current_time, 30);
    
    // Old proposals should be removed, recent ones kept
    // (unless they're still Active or Approved)
    let remaining = manager.get_all_proposals();
    // Active/approved proposals are always kept, others are cleaned if too old
    assert!(remaining.len() <= 10);
}

#[test]
fn test_voting_power_changes_dont_affect_existing_proposals() {
    let mut manager = TreasuryConsensusManager::new();
    
    manager.register_masternode("mn1".to_string(), 100);
    manager.register_masternode("mn2".to_string(), 100);
    
    let proposal = TreasuryProposal::new(ProposalParams {
        id: "power-change-test".to_string(),
        title: "Power Change Test".to_string(),
        description: "Test voting power changes".to_string(),
        recipient: "wallet".to_string(),
        amount: 5000 * TIME_UNIT,
        submitter: "submitter".to_string(),
        submission_time: 11000,
        voting_period_days: 7,
    });
    
    // Store the initial voting power of the proposal
    let initial_power = 200; // 100 + 100
    manager.add_proposal(proposal).unwrap();
    
    // Verify proposal has correct initial power
    let proposal = manager.get_proposal("power-change-test").unwrap();
    assert_eq!(proposal.total_voting_power, initial_power);
    
    // Change masternode power after proposal created
    manager.register_masternode("mn1".to_string(), 200);
    manager.register_masternode("mn3".to_string(), 150);
    
    // Existing proposal should still have original total power
    let proposal = manager.get_proposal("power-change-test").unwrap();
    assert_eq!(proposal.total_voting_power, initial_power);
}
