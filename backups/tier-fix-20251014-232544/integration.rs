use governance::*;

#[test]
fn test_masternode_tiers() {
    assert_eq!(
        MasternodeTier::from_collateral(1_000 * TIME_UNIT),
        Some(MasternodeTier::Bronze)
    );
    
    assert_eq!(
        MasternodeTier::from_collateral(100_000 * TIME_UNIT),
        Some(MasternodeTier::Diamond)
    );
}

#[test]
fn test_voting_power() {
    let bronze = MasternodeTier::Bronze;
    let diamond = MasternodeTier::Diamond;
    
    assert_eq!(bronze.voting_power(), 1);
    assert_eq!(diamond.voting_power(), 100);
}

#[test]
fn test_proposal_creation() {
    let proposal = Proposal::new(
        "prop-1".to_string(),
        "Test Proposal".to_string(),
        ProposalType::DevelopmentGrant,
        "submitter".to_string(),
        10_000 * TIME_UNIT,
        "Description".to_string(),
    );
    
    assert_eq!(proposal.status, ProposalStatus::Draft);
    assert_eq!(proposal.requested_amount, 10_000 * TIME_UNIT);
}

#[test]
fn test_voting_result() {
    let mut result = VotingResult::new("prop-1".to_string());
    
    let vote1 = Vote {
        voter: "mn1".to_string(),
        proposal_id: "prop-1".to_string(),
        choice: VoteChoice::Yes,
        voting_power: 100,
        timestamp: 1000,
    };
    
    let vote2 = Vote {
        voter: "mn2".to_string(),
        proposal_id: "prop-1".to_string(),
        choice: VoteChoice::No,
        voting_power: 50,
        timestamp: 1001,
    };
    
    result.add_vote(vote1);
    result.add_vote(vote2);
    
    assert_eq!(result.total_power, 150);
    assert_eq!(result.yes_power, 100);
    assert_eq!(result.no_power, 50);
    assert_eq!(result.approval_percentage(), 66); // 100/150 * 100
}
