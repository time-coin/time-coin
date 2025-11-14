# Treasury Governance Flow - Detailed Process Documentation

## Table of Contents
1. [Overview](#overview)
2. [Complete Lifecycle Flow](#complete-lifecycle-flow)
3. [Consensus Mechanism](#consensus-mechanism)
4. [Data Structures](#data-structures)
5. [State Transitions](#state-transitions)
6. [Security Guarantees](#security-guarantees)
7. [Example Scenarios](#example-scenarios)

## Overview

TIME Coin's treasury is a **protocol-managed, state-only** system with no private keys or wallet addresses. All treasury operations are governed by masternode consensus requiring a 2/3+ supermajority (67%+) for approval.

### Key Principles
- ✅ **State-Only**: Treasury balance tracked in blockchain state
- ✅ **No Private Keys**: Eliminates key theft/loss risks
- ✅ **Consensus-Driven**: All spending requires masternode approval
- ✅ **Time-Bound**: Proposals have voting and execution deadlines
- ✅ **Fully Auditable**: Complete on-chain history

## Complete Lifecycle Flow

### Phase 1: Treasury Funding

```
┌─────────────────────────────────────────────────────────────────┐
│                        BLOCK CREATION                            │
│                                                                  │
│  Block #12345 created at timestamp T                            │
│  Block Reward: 100 TIME                                         │
│  Transaction Fees: 10 TIME                                      │
└────────────────┬────────────────────────────────────────────────┘
                 │
                 ├──> Block Reward Distribution
                 │    │
                 │    ├──> 95 TIME → Masternodes (95%)
                 │    └──> 5 TIME → Treasury (5%)
                 │
                 └──> Transaction Fee Distribution
                      │
                      ├──> 5 TIME → Masternodes (50%)
                      └──> 5 TIME → Treasury (50%)
                      
┌─────────────────────────────────────────────────────────────────┐
│                   TREASURY STATE UPDATE                          │
│                                                                  │
│  treasury.allocate_from_block_reward(5 TIME, block #12345)     │
│  treasury.allocate_from_fees(5 TIME, block #12345)             │
│                                                                  │
│  New Balance: Previous + 10 TIME                                │
│  Total Allocated: Lifetime + 10 TIME                            │
│  Allocation History: [..., new_allocation]                      │
└─────────────────────────────────────────────────────────────────┘
```

**Code Flow:**
```rust
// In block validation (core/src/state.rs)
impl BlockchainState {
    pub fn add_block(&mut self, block: Block) -> Result<()> {
        // ... validate block ...
        
        // Allocate treasury funds
        let treasury_reward = TREASURY_BLOCK_REWARD; // 5 TIME
        self.treasury.allocate_from_block_reward(
            treasury_reward,
            block.height,
            block.timestamp
        )?;
        
        let treasury_fees = total_fees * TREASURY_FEE_PERCENTAGE / 100;
        self.treasury.allocate_from_fees(
            treasury_fees,
            block.height,
            block.timestamp
        )?;
        
        // ... rest of block processing ...
    }
}
```

### Phase 2: Proposal Creation

```
┌─────────────────────────────────────────────────────────────────┐
│                    PROPOSAL SUBMISSION                           │
│                                                                  │
│  Submitter: time1proposer_address                               │
│  Timestamp: 1699000000 (Nov 3, 2023 12:00:00 UTC)              │
└────────────────┬────────────────────────────────────────────────┘
                 │
                 v
       ┌─────────────────────┐
       │  Create Proposal    │
       │                     │
       │  ID: "prop-2024-01" │
       │  Title: "Mobile App"│
       │  Amount: 50,000 TIME│
       │  Recipient: time1...│
       └─────────┬───────────┘
                 │
                 v
       ┌─────────────────────────────────────────┐
       │  Calculate Deadlines                    │
       │                                         │
       │  Voting Period: 14 days                 │
       │  Voting Deadline: T + 1,209,600 sec     │
       │  = Nov 17, 2023 12:00:00 UTC            │
       │                                         │
       │  Execution Window: 30 days              │
       │  Execution Deadline: Dec 17, 2023       │
       └─────────┬───────────────────────────────┘
                 │
                 v
       ┌─────────────────────────────────────────┐
       │  Initialize Proposal State              │
       │                                         │
       │  Status: Active                         │
       │  Votes: {} (empty)                      │
       │  Total Voting Power: 0                  │
       └─────────┬───────────────────────────────┘
                 │
                 v
┌─────────────────────────────────────────────────────────────────┐
│              PROPOSAL STORED IN TREASURY MANAGER                 │
│                                                                  │
│  manager.proposals.insert("prop-2024-01", proposal)             │
│  Status: Active, awaiting votes                                 │
└─────────────────────────────────────────────────────────────────┘
```

**Code Flow:**
```rust
// In treasury manager (core/src/treasury_manager.rs)
impl TreasuryManager {
    pub fn create_proposal(&mut self, params: CreateProposalParams) -> Result<()> {
        // Validate proposal doesn't exist
        if self.proposals.contains_key(&params.id) {
            return Err(StateError::IoError("Proposal already exists".into()));
        }
        
        // Calculate deadlines
        let voting_deadline = params.submission_time + (params.voting_period_days * 86400);
        let execution_deadline = voting_deadline + (30 * 86400);
        
        // Create proposal
        let proposal = TreasuryProposal {
            id: params.id.clone(),
            title: params.title,
            description: params.description,
            recipient: params.recipient,
            amount: params.amount,
            submitter: params.submitter,
            submission_time: params.submission_time,
            voting_deadline,
            execution_deadline,
            status: ProposalStatus::Active,
            votes: HashMap::new(),
            total_voting_power: 0,
        };
        
        // Store proposal
        self.proposals.insert(params.id, proposal);
        Ok(())
    }
}
```

### Phase 3: Masternode Voting

```
┌─────────────────────────────────────────────────────────────────┐
│                    VOTING PERIOD (14 DAYS)                       │
│                                                                  │
│  Proposal "prop-2024-01" is Active                              │
│  Current Time: Within voting period                             │
└────────────────┬────────────────────────────────────────────────┘
                 │
                 ├──> VOTE #1: Gold Masternode
                 │    │  MN ID: "mn-gold-alpha"
                 │    │  Choice: YES
                 │    │  Power: 100
                 │    │  Time: Nov 5, 2023 10:00
                 │    │
                 │    └──> proposal.add_vote(mn_id, YES, 100, timestamp)
                 │         │
                 │         ├──> Check: Not already voted ✓
                 │         ├──> Check: Before deadline ✓
                 │         └──> votes["mn-gold-alpha"] = Vote{YES, 100}
                 │
                 ├──> VOTE #2: Silver Masternode
                 │    │  MN ID: "mn-silver-beta"
                 │    │  Choice: YES
                 │    │  Power: 10
                 │    │
                 │    └──> votes["mn-silver-beta"] = Vote{YES, 10}
                 │
                 ├──> VOTE #3: Bronze Masternode
                 │    │  MN ID: "mn-bronze-gamma"
                 │    │  Choice: NO
                 │    │  Power: 1
                 │    │
                 │    └──> votes["mn-bronze-gamma"] = Vote{NO, 1}
                 │
                 └──> VOTE #4: Gold Masternode
                      │  MN ID: "mn-gold-delta"
                      │  Choice: ABSTAIN
                      │  Power: 100
                      │
                      └──> votes["mn-gold-delta"] = Vote{ABSTAIN, 100}

┌─────────────────────────────────────────────────────────────────┐
│                   VOTING POWER SUMMARY                           │
│                                                                  │
│  Total Votes Cast: 4 masternodes                                │
│  Total Power: 211 (100 + 10 + 1 + 100)                         │
│                                                                  │
│  YES Votes: 110 power (mn-gold-alpha + mn-silver-beta)         │
│  NO Votes: 1 power (mn-bronze-gamma)                           │
│  ABSTAIN: 100 power (mn-gold-delta)                            │
│                                                                  │
│  Note: Abstain votes don't count toward approval calculation    │
└─────────────────────────────────────────────────────────────────┘
```

**Code Flow:**
```rust
// In treasury manager (core/src/treasury_manager.rs)
impl TreasuryManager {
    pub fn vote_on_proposal(
        &mut self,
        proposal_id: &str,
        masternode_id: String,
        vote_choice: VoteChoice,
        voting_power: u64,
        timestamp: u64,
    ) -> Result<()> {
        let proposal = self.proposals.get_mut(proposal_id)
            .ok_or(StateError::IoError("Proposal not found".into()))?;
        
        // Validate proposal is active
        if proposal.status != ProposalStatus::Active {
            return Err(StateError::IoError("Proposal not active".into()));
        }
        
        // Check voting deadline
        if timestamp > proposal.voting_deadline {
            return Err(StateError::IoError("Voting period ended".into()));
        }
        
        // Check for duplicate vote
        if proposal.votes.contains_key(&masternode_id) {
            return Err(StateError::IoError("Already voted".into()));
        }
        
        // Record vote
        let vote = Vote {
            masternode_id: masternode_id.clone(),
            vote_choice,
            voting_power,
            timestamp,
        };
        
        proposal.votes.insert(masternode_id, vote);
        Ok(())
    }
}
```

### Phase 4: Approval Calculation

```
┌─────────────────────────────────────────────────────────────────┐
│              VOTING DEADLINE REACHED (Nov 17, 2023)              │
│                                                                  │
│  Trigger: update_proposals(current_timestamp)                   │
└────────────────┬────────────────────────────────────────────────┘
                 │
                 v
       ┌─────────────────────────────────────────────┐
       │  Calculate Voting Results                   │
       │                                             │
       │  For each vote in proposal.votes:           │
       │    if vote.choice == YES:                   │
       │       yes_power += vote.voting_power        │
       │    else if vote.choice == NO:               │
       │       no_power += vote.voting_power         │
       │    else: // ABSTAIN                         │
       │       abstain_power += vote.voting_power    │
       └─────────┬───────────────────────────────────┘
                 │
                 v
       ┌─────────────────────────────────────────────┐
       │  Results for "prop-2024-01"                 │
       │                                             │
       │  YES Power: 110                             │
       │  NO Power: 1                                │
       │  ABSTAIN Power: 100                         │
       │                                             │
       │  Total Participating: 111 (YES + NO only)   │
       └─────────┬───────────────────────────────────┘
                 │
                 v
       ┌─────────────────────────────────────────────┐
       │  Calculate Approval Percentage              │
       │                                             │
       │  Formula: (YES / (YES + NO)) × 100          │
       │                                             │
       │  Calculation: (110 / 111) × 100             │
       │             = 99.09%                        │
       └─────────┬───────────────────────────────────┘
                 │
                 v
       ┌─────────────────────────────────────────────┐
       │  Compare Against Threshold                  │
       │                                             │
       │  Required: 67% (2/3 supermajority)          │
       │  Actual: 99.09%                             │
       │                                             │
       │  Result: 99.09% >= 67% ✓ APPROVED           │
       └─────────┬───────────────────────────────────┘
                 │
                 v
┌─────────────────────────────────────────────────────────────────┐
│                  UPDATE PROPOSAL STATUS                          │
│                                                                  │
│  proposal.status = ProposalStatus::Approved                     │
│  treasury.approve_proposal(proposal_id, amount)                 │
│                                                                  │
│  Execution Window: Nov 17 - Dec 17, 2023 (30 days)             │
└─────────────────────────────────────────────────────────────────┘
```

**Code Flow:**
```rust
// In treasury manager (core/src/treasury_manager.rs)
impl TreasuryManager {
    pub fn update_proposals(&mut self, current_time: u64) -> Result<()> {
        for proposal in self.proposals.values_mut() {
            if proposal.status != ProposalStatus::Active {
                continue;
            }
            
            // Check if voting period ended
            if current_time > proposal.voting_deadline {
                // Calculate results
                let results = proposal.calculate_results();
                
                // Check approval threshold (67%)
                if results.approval_percentage() >= 67.0 {
                    proposal.status = ProposalStatus::Approved;
                    
                    // Register with treasury
                    self.treasury.approve_proposal(
                        &proposal.id,
                        proposal.amount
                    )?;
                } else {
                    proposal.status = ProposalStatus::Rejected;
                }
            }
        }
        Ok(())
    }
}

impl TreasuryProposal {
    pub fn calculate_results(&self) -> VotingResults {
        let mut yes_power = 0u64;
        let mut no_power = 0u64;
        let mut abstain_power = 0u64;
        
        for vote in self.votes.values() {
            match vote.vote_choice {
                VoteChoice::Yes => yes_power += vote.voting_power,
                VoteChoice::No => no_power += vote.voting_power,
                VoteChoice::Abstain => abstain_power += vote.voting_power,
            }
        }
        
        VotingResults {
            yes_power,
            no_power,
            abstain_power,
        }
    }
}

impl VotingResults {
    pub fn approval_percentage(&self) -> f64 {
        let total_participating = self.yes_power + self.no_power;
        if total_participating == 0 {
            return 0.0;
        }
        (self.yes_power as f64 / total_participating as f64) * 100.0
    }
}
```

### Phase 5: Proposal Execution

```
┌─────────────────────────────────────────────────────────────────┐
│            APPROVED PROPOSAL (Within 30-day window)              │
│                                                                  │
│  Proposal: "prop-2024-01"                                       │
│  Status: Approved                                               │
│  Execution Window: Nov 17 - Dec 17, 2023                        │
│  Current Time: Nov 20, 2023 (within window ✓)                  │
└────────────────┬────────────────────────────────────────────────┘
                 │
                 v
       ┌─────────────────────────────────────────────┐
       │  Execute Proposal                           │
       │                                             │
       │  manager.execute_proposal(                  │
       │    "prop-2024-01",                         │
       │    block_number: 12500,                    │
       │    timestamp: 1700500000                   │
       │  )                                          │
       └─────────┬───────────────────────────────────┘
                 │
                 v
       ┌─────────────────────────────────────────────┐
       │  Pre-Execution Validation                   │
       │                                             │
       │  ✓ Status is Approved                       │
       │  ✓ Before execution deadline                │
       │  ✓ Treasury balance sufficient              │
       │    (Balance: 150,000 TIME >= 50,000 TIME)   │
       └─────────┬───────────────────────────────────┘
                 │
                 v
       ┌─────────────────────────────────────────────┐
       │  Treasury Fund Distribution                 │
       │                                             │
       │  treasury.distribute(                       │
       │    proposal_id: "prop-2024-01",            │
       │    recipient: "time1developer...",         │
       │    amount: 50,000 TIME,                    │
       │    block_number: 12500,                    │
       │    timestamp: 1700500000                   │
       │  )                                          │
       └─────────┬───────────────────────────────────┘
                 │
                 v
       ┌─────────────────────────────────────────────┐
       │  Update Treasury State                      │
       │                                             │
       │  balance: 150,000 - 50,000 = 100,000 TIME   │
       │  total_distributed += 50,000 TIME           │
       │                                             │
       │  Add to withdrawal history:                 │
       │  {                                          │
       │    proposal_id: "prop-2024-01",            │
       │    amount: 50,000 TIME,                    │
       │    recipient: "time1developer...",         │
       │    block_number: 12500,                    │
       │    timestamp: 1700500000                   │
       │  }                                          │
       └─────────┬───────────────────────────────────┘
                 │
                 v
       ┌─────────────────────────────────────────────┐
       │  Update Proposal Status                     │
       │                                             │
       │  proposal.status = ProposalStatus::Executed │
       └─────────┬───────────────────────────────────┘
                 │
                 v
┌─────────────────────────────────────────────────────────────────┐
│                   EXECUTION COMPLETE                             │
│                                                                  │
│  ✓ Funds transferred to recipient                               │
│  ✓ Treasury balance updated                                     │
│  ✓ Withdrawal recorded in history                               │
│  ✓ Proposal marked as Executed                                  │
│  ✓ Fully auditable on-chain                                     │
└─────────────────────────────────────────────────────────────────┘
```

**Code Flow:**
```rust
// In treasury manager (core/src/treasury_manager.rs)
impl TreasuryManager {
    pub fn execute_proposal(
        &mut self,
        proposal_id: &str,
        block_number: u64,
        timestamp: u64,
    ) -> Result<()> {
        let proposal = self.proposals.get_mut(proposal_id)
            .ok_or(StateError::IoError("Proposal not found".into()))?;
        
        // Validate status
        if proposal.status != ProposalStatus::Approved {
            return Err(StateError::IoError("Proposal not approved".into()));
        }
        
        // Check execution deadline
        if timestamp > proposal.execution_deadline {
            proposal.status = ProposalStatus::Expired;
            return Err(StateError::IoError("Execution deadline passed".into()));
        }
        
        // Distribute funds via treasury
        self.treasury.distribute(
            &proposal.id,
            &proposal.recipient,
            proposal.amount,
            block_number,
            timestamp,
        )?;
        
        // Mark as executed
        proposal.status = ProposalStatus::Executed;
        
        Ok(())
    }
}

// In treasury state (core/src/state.rs)
impl Treasury {
    pub fn distribute(
        &mut self,
        proposal_id: &str,
        recipient: &str,
        amount: u64,
        block_number: u64,
        timestamp: u64,
    ) -> Result<()> {
        // Check sufficient balance
        if self.balance < amount {
            return Err(StateError::IoError("Insufficient treasury balance".into()));
        }
        
        // Deduct from balance
        self.balance -= amount;
        
        // Update distributed total
        self.total_distributed += amount;
        
        // Record withdrawal
        let withdrawal = TreasuryWithdrawal {
            proposal_id: proposal_id.to_string(),
            amount,
            recipient: recipient.to_string(),
            block_number,
            timestamp,
        };
        
        self.withdrawals.push(withdrawal);
        
        Ok(())
    }
}
```

## Consensus Mechanism

### Byzantine Fault Tolerance (BFT) Properties

TIME Coin's treasury consensus is designed to be Byzantine Fault Tolerant, meaning it can tolerate up to 1/3 of masternodes acting maliciously or failing.

```
┌─────────────────────────────────────────────────────────────────┐
│                    CONSENSUS PARAMETERS                          │
│                                                                  │
│  Approval Threshold: 67% (2/3 + ε)                             │
│  Maximum Byzantine Nodes: 33% (1/3)                            │
│  Minimum Honest Nodes: 67% (2/3)                               │
│                                                                  │
│  Voting Weight: By collateral tier                             │
│    - Gold (100,000 TIME):   100x weight                        │
│    - Silver (10,000 TIME):  10x weight                         │
│    - Bronze (1,000 TIME):   1x weight                          │
└─────────────────────────────────────────────────────────────────┘
```

### Attack Resistance Analysis

#### Attack Scenario 1: Malicious Proposal Drain

**Attack:** Adversary submits proposal to drain entire treasury
```
┌─────────────────────────────────────────────────────────────────┐
│  ATTACK: Drain Treasury                                         │
│                                                                  │
│  Attacker creates proposal:                                     │
│    Amount: 10,000,000 TIME (entire balance)                    │
│    Recipient: attacker's address                                │
│                                                                  │
│  Network Response:                                              │
│    Masternodes review proposal                                  │
│    Recognize malicious intent                                   │
│    Vote NO                                                      │
│                                                                  │
│  Result:                                                        │
│    YES: 0 votes                                                 │
│    NO: 100% of votes                                            │
│    Proposal REJECTED ✓                                          │
│    Treasury funds safe ✓                                        │
└─────────────────────────────────────────────────────────────────┘
```

**Protection:**
- Open voting process allows review
- 67% threshold requires broad consensus
- Community oversight
- Masternode operators have incentive to protect network value

#### Attack Scenario 2: Compromised Masternode Voting

**Attack:** Adversary compromises multiple masternodes
```
┌─────────────────────────────────────────────────────────────────┐
│  ATTACK: Compromise Masternodes                                 │
│                                                                  │
│  Total Voting Power: 1000                                       │
│  Compromised: 300 (30% - below threshold)                       │
│  Honest: 700 (70%)                                              │
│                                                                  │
│  Malicious Proposal Voting:                                     │
│    Compromised nodes vote YES: 300                              │
│    Honest nodes vote NO: 700                                    │
│                                                                  │
│  Result:                                                        │
│    Approval: 300/(300+700) = 30% < 67% ✗                       │
│    Proposal REJECTED ✓                                          │
│                                                                  │
│  Required for Attack Success:                                   │
│    Compromised > 67% of voting power                            │
│    = High economic cost (need 670+ of 1000 power)              │
└─────────────────────────────────────────────────────────────────┘
```

**Protection:**
- 67% threshold prevents minority attacks
- Economic cost of acquiring 67%+ voting power is prohibitive
- Distributed masternode ownership
- Collateral at risk for bad actors

#### Attack Scenario 3: Time-Based Exploitation

**Attack:** Wait until execution deadline and claim "expired" funds
```
┌─────────────────────────────────────────────────────────────────┐
│  ATTACK: Exploit Expired Proposals                              │
│                                                                  │
│  Scenario:                                                      │
│    Proposal approved on Nov 17, 2023                            │
│    Execution deadline: Dec 17, 2023                             │
│    Attacker waits until Dec 18, 2023                            │
│    Attempts to claim "abandoned" funds                          │
│                                                                  │
│  Protocol Response:                                             │
│    Proposal status checked: Approved                            │
│    Current time: Dec 18 > Deadline (Dec 17)                     │
│    Status updated: Approved → Expired                           │
│                                                                  │
│  Result:                                                        │
│    Execution attempt REJECTED ✗                                 │
│    Status: Expired                                              │
│    Funds remain in treasury ✓                                   │
│    Approval invalidated ✓                                       │
│                                                                  │
│  Note: Funds don't automatically go anywhere                    │
│        New proposal needed to access funds                      │
└─────────────────────────────────────────────────────────────────┘
```

**Protection:**
- Automatic expiration after deadline
- Expired proposals cannot be executed
- Funds return to general treasury pool
- New proposal + voting required to access

## Data Structures

### Core Data Types

#### Treasury (State Container)
```rust
pub struct Treasury {
    /// Current available balance (in smallest unit)
    balance: u64,
    
    /// Lifetime total allocated to treasury
    total_allocated: u64,
    
    /// Lifetime total distributed from treasury
    total_distributed: u64,
    
    /// History of all deposits
    allocations: Vec<TreasuryAllocation>,
    
    /// History of all withdrawals
    withdrawals: Vec<TreasuryWithdrawal>,
    
    /// Approved proposals awaiting execution
    approved_proposals: HashMap<String, u64>,
    
    /// Percentage of block rewards (5% = 5 TIME per 100 TIME block)
    block_reward_percentage: u64,
    
    /// Percentage of transaction fees (50%)
    fee_percentage: u64,
}
```

**Field Details:**
- `balance`: Current spendable amount in smallest unit (1 TIME = 100,000,000 units)
- `total_allocated`: Cumulative deposits since genesis (never decreases)
- `total_distributed`: Cumulative withdrawals since genesis (never decreases)
- `allocations`: Append-only log of deposits (block rewards + fees)
- `withdrawals`: Append-only log of distributions (executed proposals)
- `approved_proposals`: Map of proposal_id → approved_amount
- `block_reward_percentage`: Immutable constant (5%)
- `fee_percentage`: Immutable constant (50%)

#### TreasuryAllocation (Deposit Record)
```rust
pub struct TreasuryAllocation {
    /// Block number where allocation occurred
    block_number: u64,
    
    /// Amount allocated (in smallest unit)
    amount: u64,
    
    /// Source of allocation
    source: AllocationSource,
    
    /// Timestamp of allocation
    timestamp: i64,
}

pub enum AllocationSource {
    /// From block reward (5% of 100 TIME = 5 TIME)
    BlockReward,
    
    /// From transaction fees (50% of fees)
    TransactionFees,
}
```

**Usage:**
```rust
// Example allocation from block reward
TreasuryAllocation {
    block_number: 12345,
    amount: 500_000_000,  // 5 TIME
    source: AllocationSource::BlockReward,
    timestamp: 1699000000,
}

// Example allocation from fees
TreasuryAllocation {
    block_number: 12345,
    amount: 250_000_000,  // 2.5 TIME (50% of 5 TIME fees)
    source: AllocationSource::TransactionFees,
    timestamp: 1699000000,
}
```

#### TreasuryWithdrawal (Distribution Record)
```rust
pub struct TreasuryWithdrawal {
    /// Associated proposal ID
    proposal_id: String,
    
    /// Amount distributed (in smallest unit)
    amount: u64,
    
    /// Recipient address
    recipient: String,
    
    /// Block number where distribution occurred
    block_number: u64,
    
    /// Timestamp of distribution
    timestamp: i64,
}
```

**Usage:**
```rust
// Example withdrawal for executed proposal
TreasuryWithdrawal {
    proposal_id: "prop-2024-01".to_string(),
    amount: 5_000_000_000_000,  // 50,000 TIME
    recipient: "time1developer_address_here...".to_string(),
    block_number: 12500,
    timestamp: 1700500000,
}
```

#### TreasuryProposal (Governance Record)
```rust
pub struct TreasuryProposal {
    /// Unique proposal identifier
    id: String,
    
    /// Human-readable title
    title: String,
    
    /// Detailed description
    description: String,
    
    /// Recipient address for funds
    recipient: String,
    
    /// Requested amount (in smallest unit)
    amount: u64,
    
    /// Address of proposal submitter
    submitter: String,
    
    /// Timestamp when submitted
    submission_time: u64,
    
    /// Deadline for voting (submission + 14 days)
    voting_deadline: u64,
    
    /// Deadline for execution (voting_deadline + 30 days)
    execution_deadline: u64,
    
    /// Current status
    status: ProposalStatus,
    
    /// Map of masternode_id → Vote
    votes: HashMap<String, Vote>,
    
    /// Total voting power cast
    total_voting_power: u64,
}
```

**Status Transitions:**
```
Active → Approved (if ≥67% YES after voting deadline)
Active → Rejected (if <67% YES after voting deadline)
Approved → Executed (when execute_proposal() called)
Approved → Expired (if execution deadline passes)
```

#### Vote (Individual Vote Record)
```rust
pub struct Vote {
    /// Masternode identifier
    masternode_id: String,
    
    /// Vote choice (Yes/No/Abstain)
    vote_choice: VoteChoice,
    
    /// Voting power based on tier
    voting_power: u64,
    
    /// When vote was cast
    timestamp: u64,
}

pub enum VoteChoice {
    Yes,      // Support the proposal
    No,       // Oppose the proposal
    Abstain,  // Participate without taking a position
}
```

**Voting Power by Tier:**
```rust
match masternode_tier {
    MasternodeTier::Bronze => 1,    // 1,000 TIME collateral
    MasternodeTier::Silver => 10,   // 10,000 TIME collateral
    MasternodeTier::Gold => 100,    // 100,000 TIME collateral
}
```

## State Transitions

### Complete State Machine

```
┌──────────────────────────────────────────────────────────────────┐
│                    PROPOSAL STATE MACHINE                         │
└──────────────────────────────────────────────────────────────────┘

                    [Created]
                       │
                       │ create_proposal()
                       │
                       v
                  ┌─────────┐
                  │ Active  │ <─── Initial State
                  └────┬────┘
                       │
           ┌───────────┼───────────┐
           │                       │
           │ voting_deadline       │ voting_deadline
           │ reached               │ reached
           │                       │
           │ approval ≥ 67%        │ approval < 67%
           │                       │
           v                       v
      ┌──────────┐            ┌──────────┐
      │ Approved │            │ Rejected │ <─── Terminal State
      └────┬─────┘            └──────────┘
           │
           ├─────────────────┬──────────────────┐
           │                 │                  │
           │ execute()       │ execution_       │
           │ called          │ deadline         │
           │                 │ passes           │
           v                 v                  v
      ┌──────────┐      ┌──────────┐
      │ Executed │      │ Expired  │ <─── Terminal States
      └──────────┘      └──────────┘
```

### Transition Details

#### Active → Approved
**Trigger:** `update_proposals(current_time)` called after voting_deadline
**Condition:** `approval_percentage() >= 67.0`
**Actions:**
1. Calculate voting results
2. Check approval threshold
3. Update proposal.status = Approved
4. Register with treasury: `treasury.approve_proposal(id, amount)`

**Code:**
```rust
if current_time > proposal.voting_deadline {
    let results = proposal.calculate_results();
    if results.approval_percentage() >= 67.0 {
        proposal.status = ProposalStatus::Approved;
        self.treasury.approve_proposal(&proposal.id, proposal.amount)?;
    }
}
```

#### Active → Rejected
**Trigger:** `update_proposals(current_time)` called after voting_deadline
**Condition:** `approval_percentage() < 67.0`
**Actions:**
1. Calculate voting results
2. Check approval threshold
3. Update proposal.status = Rejected

**Code:**
```rust
if current_time > proposal.voting_deadline {
    let results = proposal.calculate_results();
    if results.approval_percentage() < 67.0 {
        proposal.status = ProposalStatus::Rejected;
    }
}
```

#### Approved → Executed
**Trigger:** `execute_proposal(id, block, time)` called
**Condition:** `timestamp <= execution_deadline` AND `treasury.balance >= amount`
**Actions:**
1. Validate proposal status is Approved
2. Check execution deadline
3. Distribute funds: `treasury.distribute()`
4. Update proposal.status = Executed

**Code:**
```rust
if proposal.status == ProposalStatus::Approved {
    if timestamp <= proposal.execution_deadline {
        if self.treasury.balance >= proposal.amount {
            self.treasury.distribute(
                &proposal.id,
                &proposal.recipient,
                proposal.amount,
                block_number,
                timestamp
            )?;
            proposal.status = ProposalStatus::Executed;
        }
    }
}
```

#### Approved → Expired
**Trigger:** `execute_proposal(id, block, time)` called OR `update_proposals()` called
**Condition:** `timestamp > execution_deadline`
**Actions:**
1. Validate proposal status is Approved
2. Check execution deadline passed
3. Update proposal.status = Expired
4. Invalidate approval in treasury

**Code:**
```rust
if proposal.status == ProposalStatus::Approved {
    if timestamp > proposal.execution_deadline {
        proposal.status = ProposalStatus::Expired;
        // Approval remains in treasury but cannot be executed
    }
}
```

## Security Guarantees

### Cryptographic Properties

#### 1. No Private Key Exposure
**Guarantee:** Treasury has zero private keys in memory, storage, or transmission.

**Implementation:**
```rust
// Treasury is pure state
pub struct Treasury {
    balance: u64,  // Just a number, not a wallet
    // NO private_key field
    // NO signing capability
    // NO key derivation
}
```

**Benefit:** Eliminates entire class of attacks:
- ✓ No key theft possible
- ✓ No key loss possible
- ✓ No key compromise possible
- ✓ No signing oracle attacks
- ✓ No timing attacks on signatures

#### 2. Consensus-Enforced Spending
**Guarantee:** All spending requires 67%+ masternode approval.

**Proof:** 
```
Let T = Total voting power
Let M = Malicious nodes voting power
Let H = Honest nodes voting power

For malicious proposal to pass:
  M/(M+H) ≥ 0.67
  M ≥ 0.67(M+H)
  M ≥ 0.67M + 0.67H
  0.33M ≥ 0.67H
  M/H ≥ 2.03
  
Therefore: Attacker needs DOUBLE the honest voting power
```

**Economic Cost:**
- If honest nodes have 100,000 TIME in collateral
- Attacker needs 200,000 TIME in collateral
- At $1/TIME, that's $200,000+ at risk

#### 3. Time-Bound Operations
**Guarantee:** All proposals have explicit deadlines that cannot be bypassed.

**Implementation:**
```rust
pub struct TreasuryProposal {
    voting_deadline: u64,      // Immutable after creation
    execution_deadline: u64,   // Immutable after creation
}

// Voting enforcement
if timestamp > proposal.voting_deadline {
    return Err("Voting period ended");
}

// Execution enforcement
if timestamp > proposal.execution_deadline {
    proposal.status = ProposalStatus::Expired;
    return Err("Execution deadline passed");
}
```

**Benefit:**
- No indefinite proposals
- Expired approvals cannot be revived
- Predictable state transitions

#### 4. Immutable History
**Guarantee:** All treasury operations are recorded immutably.

**Implementation:**
```rust
pub struct Treasury {
    allocations: Vec<TreasuryAllocation>,   // Append-only
    withdrawals: Vec<TreasuryWithdrawal>,   // Append-only
}

// Only operation: Push new records
self.allocations.push(allocation);
self.withdrawals.push(withdrawal);

// No operations to:
// - Remove records
// - Modify past records
// - Rewrite history
```

**Benefit:**
- Complete audit trail
- Forensic analysis possible
- Accountability enforced
- Tampering detectable

### Attack Surface Analysis

#### Zero Attack Surface
**What Cannot Be Attacked:**
- ❌ Private keys (don't exist)
- ❌ Signing operations (not implemented)
- ❌ Key storage (no keys to store)
- ❌ Wallet addresses (no addresses)
- ❌ External dependencies (pure state)

#### Minimal Attack Surface
**What Can Be Attacked (and how it's defended):**

1. **Proposal Submission Spam**
   - Attack: Submit thousands of proposals
   - Defense: Proposal IDs must be unique
   - Defense: Masternodes can ignore spam
   - Impact: Minimal (just storage bloat)

2. **Vote Manipulation**
   - Attack: Vote multiple times
   - Defense: One vote per masternode per proposal
   - Defense: Votes are cryptographically signed by MN keys
   - Impact: None (prevented by protocol)

3. **Timestamp Manipulation**
   - Attack: Fake timestamps to bypass deadlines
   - Defense: Timestamps validated by consensus
   - Defense: BFT ensures majority agreement
   - Impact: None if <33% Byzantine nodes

4. **Balance Manipulation**
   - Attack: Directly modify treasury.balance
   - Defense: Only callable through consensus-validated methods
   - Defense: All changes recorded in history
   - Impact: None (caught by validation)

## Example Scenarios

### Scenario 1: Successful Developer Grant

**Context:** Developer wants funding for mobile app development

**Step 1: Proposal Submission**
```rust
manager.create_proposal(CreateProposalParams {
    id: "mobile-wallet-2024".to_string(),
    title: "iOS and Android Wallet Development".to_string(),
    description: r#"
        Develop native mobile wallets for iOS and Android platforms.
        
        Deliverables:
        - iOS app (Swift, iOS 15+)
        - Android app (Kotlin, Android 10+)
        - Biometric authentication
        - QR code scanning
        - Transaction history
        - Push notifications
        
        Timeline: 6 months
        Budget: 75,000 TIME
        
        Team: 3 developers, 1 designer
        Previous work: bitcoin.com wallet contributors
    "#.to_string(),
    recipient: "time1dev_team_wallet...".to_string(),
    amount: 75_000 * 100_000_000,  // 75,000 TIME
    submitter: "time1dev_lead...".to_string(),
    submission_time: 1699000000,
    voting_period_days: 14,
})?;
```

**Step 2: Community Discussion** (Off-chain, 7-14 days)
- Discord/Forum discussion
- Developer answers questions
- Team credentials verified
- Milestone plan reviewed
- Community sentiment: Positive

**Step 3: Masternode Voting** (14 days)
```rust
// Gold masternodes (high collateral, trusted operators)
manager.vote_on_proposal("mobile-wallet-2024", "mn-gold-1", VoteChoice::Yes, 100, T+1)?;
manager.vote_on_proposal("mobile-wallet-2024", "mn-gold-2", VoteChoice::Yes, 100, T+2)?;
manager.vote_on_proposal("mobile-wallet-2024", "mn-gold-3", VoteChoice::Yes, 100, T+3)?;
manager.vote_on_proposal("mobile-wallet-2024", "mn-gold-4", VoteChoice::Yes, 100, T+4)?;
manager.vote_on_proposal("mobile-wallet-2024", "mn-gold-5", VoteChoice::No, 100, T+5)?;

// Silver masternodes
for i in 0..20 {
    manager.vote_on_proposal(
        "mobile-wallet-2024",
        &format!("mn-silver-{}", i),
        VoteChoice::Yes,
        10,
        T + 100000 + i*1000
    )?;
}

// Bronze masternodes
for i in 0..50 {
    let choice = if i < 40 { VoteChoice::Yes } else { VoteChoice::No };
    manager.vote_on_proposal(
        "mobile-wallet-2024",
        &format!("mn-bronze-{}", i),
        choice,
        1,
        T + 200000 + i*1000
    )?;
}
```

**Step 4: Voting Results**
```
Total Voting Power:
  Gold: 5 × 100 = 500
  Silver: 20 × 10 = 200  
  Bronze: 50 × 1 = 50
  TOTAL: 750

YES Votes:
  Gold: 4 × 100 = 400
  Silver: 20 × 10 = 200
  Bronze: 40 × 1 = 40
  TOTAL YES: 640

NO Votes:
  Gold: 1 × 100 = 100
  Silver: 0 × 10 = 0
  Bronze: 10 × 1 = 10
  TOTAL NO: 110

Approval: 640/(640+110) = 85.3% ✓ APPROVED
```

**Step 5: Execution**
```rust
// After voting deadline
manager.update_proposals(T + (14*86400))?;
// Proposal status: Active → Approved

// Developer executes within 30 days
manager.execute_proposal(
    "mobile-wallet-2024",
    12600,  // Block number
    T + (20*86400)  // 20 days after voting ended
)?;

// Funds distributed:
// Treasury: 1,000,000 TIME → 925,000 TIME
// Developer: 0 TIME → 75,000 TIME
// Proposal status: Approved → Executed
```

### Scenario 2: Rejected Malicious Proposal

**Context:** Bad actor attempts to steal funds

**Step 1: Malicious Proposal**
```rust
manager.create_proposal(CreateProposalParams {
    id: "urgent-security-fix".to_string(),
    title: "URGENT: Critical Security Vulnerability Fix".to_string(),
    description: "Critical 0-day exploit discovered. Funds needed immediately for patch deployment.".to_string(),
    recipient: "time1attacker_address...".to_string(),  // Attacker's address
    amount: 500_000 * 100_000_000,  // 500,000 TIME (huge amount)
    submitter: "time1anon...".to_string(),
    submission_time: 1699000000,
    voting_period_days: 14,
})?;
```

**Step 2: Community Response** (Immediate)
- Experienced devs: "No known vulnerability"
- Proposer: Anonymous, no credentials
- No technical details provided
- Amount excessive
- Community sentiment: Suspicious 🚩

**Step 3: Masternode Voting** (Clear rejection)
```rust
// All responsible masternodes vote NO
for i in 0..10 {
    manager.vote_on_proposal(
        "urgent-security-fix",
        &format!("mn-gold-{}", i),
        VoteChoice::No,  // Clear NO
        100,
        T + 10000 + i*1000
    )?;
}

// Only a few compromised/naive nodes vote YES
manager.vote_on_proposal("urgent-security-fix", "mn-bronze-1", VoteChoice::Yes, 1, T+5000)?;
manager.vote_on_proposal("urgent-security-fix", "mn-bronze-2", VoteChoice::Yes, 1, T+6000)?;
```

**Step 4: Results**
```
Voting Power:
  YES: 2 (2 bronze nodes)
  NO: 1000 (10 gold nodes)
  
Approval: 2/(2+1000) = 0.2% ✗ REJECTED

Proposal status: Active → Rejected
Treasury funds: Safe ✓
```

### Scenario 3: Expired Approval

**Context:** Approved proposal not executed in time

**Step 1-3:** Proposal submitted and approved (67%+ YES votes)

**Step 4: Execution Deadline Approaches**
```
Voting ended: Nov 17, 2023
Execution deadline: Dec 17, 2023 (30 days)
Current date: Dec 10, 2023 (7 days remaining)
```

**Step 5: Developer Fails to Execute**
```
Dec 10: Proposal still Approved, not Executed
Dec 15: 2 days remaining - still not executed
Dec 17: Execution deadline passes
Dec 18: Proposal automatically expires
```

**Code Behavior:**
```rust
// On Dec 18, any execution attempt fails
manager.execute_proposal(
    "expired-proposal",
    12800,
    1702900000  // Dec 18, 2023
)?;

// Result: Error!
// proposal.status automatically updated: Approved → Expired
// Funds remain in treasury
// New proposal needed if still want funding
```

**Outcome:**
- Treasury protected ✓
- No automatic fund release ✓  
- Developer lost opportunity (own fault)
- New proposal can be submitted
- Community can reconsider

---

**Document Version:** 1.0  
**Last Updated:** November 2024  
**Status:** Active
