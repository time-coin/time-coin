# Treasury Consensus and Governance Guide

## Overview

TIME Coin's treasury uses a robust consensus mechanism to ensure all spending decisions require 2/3+ (67%) masternode approval. This guide covers the complete consensus integration, voting procedures, and governance best practices.

## Consensus Architecture

### Key Components

#### 1. TreasuryConsensusManager
Central component that manages all treasury proposals and voting:
```rust
pub struct TreasuryConsensusManager {
    proposals: HashMap<String, TreasuryProposal>,
    total_voting_power: u64,
    masternode_power: HashMap<String, u64>,
}
```

**Responsibilities:**
- Register and track masternodes with their voting power
- Manage proposal lifecycle (creation, voting, approval, execution)
- Calculate consensus based on weighted votes
- Handle proposal expiration

#### 2. TreasuryProposal
Individual funding proposal with complete voting state:
```rust
pub struct TreasuryProposal {
    pub id: String,
    pub title: String,
    pub description: String,
    pub recipient: String,
    pub amount: u64,
    pub submitter: String,
    pub submission_time: u64,
    pub voting_deadline: u64,
    pub execution_deadline: u64,
    pub status: ProposalStatus,
    pub votes: HashMap<String, Vote>,
    pub total_voting_power: u64,
}
```

**Lifecycle States:**
- **Active**: Accepting votes during voting period
- **Approved**: Reached 2/3+ consensus, awaiting execution
- **Rejected**: Failed to reach consensus or majority voted NO
- **Executed**: Funds distributed to recipient
- **Expired**: Approved but not executed within deadline

#### 3. Voting System
Weighted voting based on masternode tiers:
- **Bronze Tier**: 1 voting power
- **Silver Tier**: 10 voting power  
- **Gold Tier**: 100 voting power

## Voting Procedures

### For Proposal Submitters

#### 1. Create a Proposal
```rust
use treasury::*;

// Initialize consensus manager
let mut manager = TreasuryConsensusManager::new();

// Create proposal
let proposal = TreasuryProposal::new(ProposalParams {
    id: "dev-grant-001".to_string(),
    title: "Mobile Wallet Development".to_string(),
    description: "Develop iOS and Android wallets for TIME Coin community".to_string(),
    recipient: "time1dev...wallet...address".to_string(),
    amount: 50_000 * TIME_UNIT, // 50,000 TIME
    submitter: "community-member-address".to_string(),
    submission_time: current_timestamp(),
    voting_period_days: 14, // 14-day voting period
});

// Submit to consensus system
manager.add_proposal(proposal)?;
```

**Best Practices:**
- Clear, detailed description of what funds will be used for
- Realistic budget with justification
- Specific deliverables and timeline
- Valid recipient address
- Appropriate voting period (7-30 days typical)

### For Masternode Operators

#### 1. Register Your Masternode
```rust
// Register with voting power based on tier
manager.register_masternode(
    "masternode-id".to_string(),
    100 // Gold tier = 100 voting power
);
```

#### 2. Cast Your Vote
```rust
use treasury::VoteChoice;

// Vote YES on a proposal
manager.vote_on_proposal(
    "dev-grant-001",                    // Proposal ID
    "masternode-id".to_string(),        // Your masternode ID
    VoteChoice::Yes,                    // YES, NO, or Abstain
    current_timestamp()                 // Timestamp
)?;
```

**Vote Choices:**
- **Yes**: Approve the proposal
- **No**: Reject the proposal
- **Abstain**: Participate but don't influence outcome

**Voting Rules:**
- One vote per masternode per proposal
- Votes cast during voting period only
- Cannot change vote once cast
- Must be registered masternode

#### 3. Check Voting Results
```rust
// Get current voting results
let results = manager.get_voting_results("dev-grant-001")?;

println!("YES votes: {} power", results.yes_power);
println!("NO votes: {} power", results.no_power);
println!("Approval: {}%", results.approval_percentage());
println!("Participation: {}%", results.participation_rate());

// Check if consensus reached
let has_consensus = manager.has_consensus("dev-grant-001")?;
```

### For Treasury Operators

#### 1. Monitor Proposal Status
```rust
// Update all proposal statuses based on current time
manager.update_proposal_statuses(current_timestamp());

// Get proposals by status
let active = manager.get_proposals_by_status(&ProposalStatus::Active);
let approved = manager.get_proposals_by_status(&ProposalStatus::Approved);

// Check for expired proposals
let expired = manager.expire_old_proposals(current_timestamp());
```

#### 2. Execute Approved Proposals
```rust
// After consensus reached and voting period ended
let proposal = manager.get_proposal("dev-grant-001").unwrap();

if proposal.status == ProposalStatus::Approved {
    // Distribute funds through treasury
    treasury.distribute(
        proposal.amount,
        &proposal.recipient,
        &format!("Treasury grant: {}", proposal.id)
    )?;
    
    // Mark as executed
    manager.mark_proposal_executed("dev-grant-001")?;
}
```

## Consensus Thresholds

### 2/3+ Approval Requirement

**Why 67%?**
- Requires strong majority support
- Prevents small groups from controlling treasury
- Balances decision-making speed with security
- Aligns with BFT consensus principles

**Calculation:**
```
YES votes must be >= 67% of all cast votes

Example with 3 equal masternodes:
- 3 YES, 0 NO = 100% ✅ APPROVED
- 2 YES, 1 NO = 67% ✅ APPROVED (exactly at threshold)
- 1 YES, 2 NO = 33% ❌ REJECTED

Example with weighted voting (Gold=100, Silver=10, Bronze=1):
Total power: 111
- Gold YES + Silver YES = 110/111 = 99% ✅ APPROVED
- Gold YES + Bronze NO = 100/101 = 99% ✅ APPROVED
- Silver YES + Bronze YES = 11/111 = 10% ❌ REJECTED (need Gold)
```

### Participation Requirements

**Minimum Participation:**
- No minimum required for decision
- Decision based on actual participating votes
- Low participation = rejected by default if no consensus

**Quorum Considerations:**
- Active masternodes should vote on all proposals
- Abstain votes count toward participation but not approval
- Regular participation expected from all masternodes

## Security Considerations

### 1. Protocol-Managed Security
✅ **No Private Keys**: Treasury exists only in blockchain state
✅ **No Wallet Address**: Funds not stored in UTXOs
✅ **No Single Point of Failure**: Requires distributed consensus
✅ **Immutable Rules**: Protocol enforces all governance decisions

### 2. Vote Integrity
✅ **One Vote Per Masternode**: Duplicate votes prevented
✅ **Time-Bound Voting**: Votes only during voting period
✅ **Immutable Votes**: Cannot change vote once cast
✅ **Verifiable Results**: All votes recorded on-chain

### 3. Proposal Integrity
✅ **Unique IDs**: Prevents duplicate proposals
✅ **Execution Deadlines**: Approved proposals expire if not executed (30 days default)
✅ **Status Tracking**: Clear lifecycle management
✅ **Audit Trail**: Complete history of all proposals and votes

### 4. Attack Resistance

**Sybil Attack Prevention:**
- Requires masternode collateral (1,000,000 TIME)
- Voting power weighted by tier
- New masternodes cannot instantly control treasury

**Vote Manipulation Prevention:**
- Votes locked after casting
- Deadline enforcement prevents late manipulation
- Transparent voting results

**Proposal Spam Prevention:**
- Each proposal has unique ID
- Active proposals tracked separately
- Old proposals cleaned up automatically

## Integration Examples

### Example 1: Community Development Grant

```rust
// Submit development grant proposal
let proposal = TreasuryProposal::new(ProposalParams {
    id: "community-dev-001".to_string(),
    title: "Community Developer Grants Q1 2024".to_string(),
    description: r#"
        Funding for 5 community developers to build tools and features:
        - Block explorer enhancements: 15,000 TIME
        - Wallet UI improvements: 10,000 TIME  
        - Documentation and guides: 5,000 TIME
        - Testing and QA: 5,000 TIME
        - Project management: 5,000 TIME
        
        Total: 40,000 TIME over 3 months
        Deliverables: Monthly progress reports, open source code
    "#.to_string(),
    recipient: "time1community...dev...address".to_string(),
    amount: 40_000 * TIME_UNIT,
    submitter: "community-lead-address".to_string(),
    submission_time: current_timestamp(),
    voting_period_days: 21, // 3 weeks for discussion
});

manager.add_proposal(proposal)?;
```

### Example 2: Marketing Campaign

```rust
let proposal = TreasuryProposal::new(ProposalParams {
    id: "marketing-q1-2024".to_string(),
    title: "Q1 Marketing and Conference Presence".to_string(),
    description: r#"
        Marketing budget for Q1 2024:
        - Social media advertising: 8,000 TIME
        - Conference booth and travel: 12,000 TIME
        - Content creation: 5,000 TIME
        - Influencer partnerships: 10,000 TIME
        - Contingency: 2,000 TIME
        
        Total: 37,000 TIME
        Expected reach: 100,000+ people
        ROI tracking: Social metrics, conference leads, community growth
    "#.to_string(),
    recipient: "time1marketing...team...address".to_string(),
    amount: 37_000 * TIME_UNIT,
    submitter: "marketing-lead".to_string(),
    submission_time: current_timestamp(),
    voting_period_days: 14,
});

manager.add_proposal(proposal)?;
```

### Example 3: Security Audit

```rust
let proposal = TreasuryProposal::new(ProposalParams {
    id: "security-audit-2024".to_string(),
    title: "Independent Security Audit".to_string(),
    description: r#"
        Professional security audit by [Reputable Firm]:
        - Smart contract review
        - Consensus mechanism analysis
        - Network security assessment
        - Penetration testing
        - Final report and recommendations
        
        Cost: 100,000 TIME (fixed price contract)
        Timeline: 6-8 weeks
        Deliverable: Comprehensive security report
    "#.to_string(),
    recipient: "time1security...audit...escrow".to_string(),
    amount: 100_000 * TIME_UNIT,
    submitter: "core-dev-team".to_string(),
    submission_time: current_timestamp(),
    voting_period_days: 14,
});

manager.add_proposal(proposal)?;
```

## Monitoring and Maintenance

### Regular Tasks

**For Masternode Operators:**
1. Review new proposals daily during voting periods
2. Research and evaluate proposal merits
3. Cast informed votes before deadlines
4. Monitor treasury health and spending

**For Treasury Operators:**
1. Update proposal statuses regularly
2. Execute approved proposals promptly
3. Clean up old proposals monthly
4. Monitor for suspicious activity
5. Generate reports for community

### Cleanup Operations

```rust
// Clean up old proposals (keep last 90 days)
manager.cleanup_old_proposals(current_timestamp(), 90);

// Expire approved proposals past execution deadline
let expired = manager.expire_old_proposals(current_timestamp());
for id in expired {
    println!("Proposal {} expired without execution", id);
}
```

## Troubleshooting

### Common Issues

**Q: My vote was rejected**
- Check if you're a registered masternode
- Verify voting period hasn't ended
- Ensure you haven't already voted on this proposal

**Q: Proposal not reaching consensus**
- Check participation rate - need enough voters
- Review proposal details - may need revision
- Engage community for more votes

**Q: Can't execute approved proposal**
- Check if execution deadline passed (proposal expired)
- Verify treasury has sufficient balance
- Ensure recipient address is valid

**Q: Voting power seems wrong**
- Verify masternode tier registration
- Check if voting power was updated recently
- Remember: existing proposals keep original total power

## Best Practices

### For Proposal Authors
1. **Clear Communication**: Detailed, transparent descriptions
2. **Realistic Budgets**: Justified amounts with breakdown
3. **Community Engagement**: Discuss proposal before submission
4. **Follow-Through**: Deliver on promises if approved

### For Voters
1. **Due Diligence**: Research proposals before voting
2. **Active Participation**: Vote on all proposals
3. **Timely Voting**: Don't wait until last minute
4. **Informed Decisions**: Consider long-term treasury health

### For Treasury Managers
1. **Regular Monitoring**: Daily status checks
2. **Prompt Execution**: Execute approved proposals quickly
3. **Transparent Reporting**: Regular community updates
4. **Security First**: Verify all operations before execution

## API Reference

### TreasuryConsensusManager

```rust
// Create new manager
let mut manager = TreasuryConsensusManager::new();

// Register masternode
manager.register_masternode(id: String, power: u64);

// Add proposal
manager.add_proposal(proposal: TreasuryProposal) -> Result<()>;

// Vote on proposal
manager.vote_on_proposal(
    proposal_id: &str,
    masternode_id: String,
    vote_choice: VoteChoice,
    timestamp: u64
) -> Result<()>;

// Check consensus
manager.has_consensus(proposal_id: &str) -> Result<bool>;

// Update statuses
manager.update_proposal_statuses(current_time: u64);

// Get voting results
manager.get_voting_results(proposal_id: &str) -> Result<VotingResults>;

// Mark executed
manager.mark_proposal_executed(proposal_id: &str) -> Result<()>;

// Expire old proposals
manager.expire_old_proposals(current_time: u64) -> Vec<String>;

// Cleanup
manager.cleanup_old_proposals(current_time: u64, keep_days: u64);
```

## Conclusion

TIME Coin's treasury consensus system provides a secure, transparent, and democratic way to manage community funds. The 2/3+ approval threshold ensures broad support for all spending decisions while preventing centralization of control.

For questions or issues, refer to the main [Treasury Architecture](TREASURY_ARCHITECTURE.md) documentation or join the community discussion channels.

---

**Document Version**: 1.0  
**Last Updated**: November 2024  
**Status**: Production Ready
