//! Integration tests for Treasury + Governance

#[cfg(test)]
mod tests {
    use treasury::*;
    use governance::*;

    #[test]
    fn test_proposal_to_treasury_flow() {
        // Create treasury
        let mut treasury_pool = TreasuryPool::new();
        
        // Add funds
        treasury_pool.deposit_block_reward(1, 1000).unwrap();
        
        // Create proposal
        let proposal = Proposal::new(
            "prop-1".to_string(),
            "Development Grant".to_string(),
            ProposalType::DevelopmentGrant,
            "dev-team".to_string(),
            1000 * TIME_UNIT,
            "Fund development".to_string(),
        );
        
        assert_eq!(proposal.status, ProposalStatus::Draft);
        assert!(treasury_pool.balance() > 0);
    }
}
