# Protocol-Managed Treasury Architecture

## Overview

The TIME Coin treasury is a **protocol-managed fund** that receives a portion of block rewards and transaction fees to support ecosystem development, marketing, security audits, and community initiatives. Unlike traditional cryptocurrency treasuries that rely on multi-signature wallets, TIME Coin's treasury has **no private keys or wallet addresses**—it is managed entirely through on-chain state and governed by masternode consensus.

## Key Features

### 🔒 Security-First Design
- **No private keys** - Treasury exists only as protocol state
- **No wallet address** - Funds tracked in blockchain state, not UTXOs
- **Consensus-driven** - All spending requires 2/3+ masternode approval
- **Fully auditable** - Complete transaction history on-chain
- **Immutable rules** - Protocol enforces all governance decisions

### 🗳️ Decentralized Governance
- **Masternode voting** - Only active masternodes can vote
- **Weighted power** - Bronze=1, Silver=10, Gold=100 voting power
- **Supermajority required** - 67% (2/3+) YES votes needed for approval
- **Time-bound voting** - 14-day default voting period
- **Execution deadline** - 30-day window to execute approved proposals

### 💰 Automated Funding
- **Block rewards** - 5% of each block reward (5 TIME out of 100 TIME)
- **Transaction fees** - 50% of all transaction fees
- **Transparent allocation** - Every deposit recorded with source
- **Real-time balance** - Current balance always available

## Architecture

### Data Flow

```
┌─────────────────┐
│  Block Creation │
└────────┬────────┘
         │
         ├─> Block Reward (100 TIME)
         │   ├─> 5 TIME → Treasury (5%)
         │   └─> 95 TIME → Masternodes (95%)
         │
         └─> Transaction Fees
             ├─> 50% → Treasury
             └─> 50% → Masternodes

┌────────────────┐
│ Treasury State │ ◄── Tracked in BlockchainState
│  Balance: 1,234,567 TIME
│  Total Allocated: 2,345,678 TIME
│  Total Distributed: 1,111,111 TIME
└────────────────┘
         │
         ├─> Proposal Created
         │   │  ID: "prop-001"
         │   │  Amount: 10,000 TIME
         │   │  Recipient: "addr123"
         │   │  Voting Ends: Block 12,345
         │   │
         │   ├─> Masternodes Vote
         │   │   │  MN1 (Gold): YES (100 power)
         │   │   │  MN2 (Silver): YES (10 power)
         │   │   │  MN3 (Bronze): NO (1 power)
         │   │   └─> Total: 110 YES, 1 NO = 99% approval ✓
         │   │
         │   ├─> Proposal Approved (>= 67% YES)
         │   │
         │   └─> Funds Distributed
         │       │  10,000 TIME → addr123
         │       └─> Recorded in withdrawal history

┌──────────────────┐
│  Audit Trail     │
│  - All deposits  │
│  - All proposals │
│  - All votes     │
│  - All distributions
└──────────────────┘
```

### Components

#### 1. Treasury State (`core/src/state.rs`)
```rust
pub struct Treasury {
    balance: u64,                                      // Current balance
    total_allocated: u64,                              // Lifetime deposits
    total_distributed: u64,                            // Lifetime withdrawals
    allocations: Vec<TreasuryAllocation>,              // Deposit history
    withdrawals: Vec<TreasuryWithdrawal>,              // Distribution history
    approved_proposals: HashMap<String, u64>,          // Approved amounts
    block_reward_percentage: u64,                      // 5% of rewards
    fee_percentage: u64,                               // 50% of fees
}
```

**Key Methods:**
- `allocate_from_block_reward()` - Add funds from block rewards
- `allocate_from_fees()` - Add funds from transaction fees
- `approve_proposal()` - Approve a proposal for spending
- `distribute()` - Execute approved distribution
- `get_stats()` - Get treasury statistics

#### 2. Treasury Governance (`treasury/src/governance.rs`)
```rust
pub struct TreasuryProposal {
    id: String,
    title: String,
    description: String,
    recipient: String,
    amount: u64,
    submitter: String,
    submission_time: u64,
    voting_deadline: u64,
    execution_deadline: u64,
    status: ProposalStatus,
    votes: HashMap<String, Vote>,
    total_voting_power: u64,
}
```

**Proposal Lifecycle:**
1. **Draft** → Created but not yet submitted
2. **Active** → Accepting votes from masternodes
3. **Approved** → Passed with 67%+ YES votes
4. **Rejected** → Failed to reach approval threshold
5. **Executed** → Funds distributed to recipient
6. **Expired** → Approved but not executed in time

#### 3. Treasury Manager (`core/src/treasury_manager.rs`)
Unified interface that integrates Treasury state with Governance:

```rust
pub struct TreasuryManager {
    proposals: HashMap<String, TreasuryProposal>,
    treasury: Treasury,
    total_voting_power: u64,
}
```

**Key Methods:**
- `create_proposal()` - Submit new spending proposal
- `vote_on_proposal()` - Cast vote from masternode
- `update_proposals()` - Update statuses based on voting results
- `execute_proposal()` - Distribute funds for approved proposal
- `get_approved_proposals()` - Get proposals ready for execution

## Governance Process

### 1. Creating a Proposal

**Requirements:**
- Title and description
- Recipient address
- Requested amount
- Submitter identity

**Process:**
```rust
manager.create_proposal(
    "prop-001".to_string(),
    "Website Redesign".to_string(),
    "Redesign and modernize TIME Coin website".to_string(),
    "time1developer_address".to_string(),
    50_000 * TIME_UNIT,  // 50,000 TIME
    "submitter_address".to_string(),
    current_timestamp,
    14,  // 14-day voting period
)
```

### 2. Masternode Voting

**Voting Power by Tier:**
- 🥉 Bronze (1,000 TIME collateral) = 1 vote
- 🥈 Silver (10,000 TIME collateral) = 10 votes
- 🥇 Gold (100,000 TIME collateral) = 100 votes

**Voting Process:**
```rust
// Masternode casts vote
manager.vote_on_proposal(
    "prop-001",
    "masternode-gold-1".to_string(),
    VoteChoice::Yes,
    100,  // Voting power
    current_timestamp,
)
```

**Vote Choices:**
- **Yes** - Approve the proposal
- **No** - Reject the proposal
- **Abstain** - Participate without taking a position

### 3. Approval Calculation

**Formula:**
```
Approval % = (YES votes / Total votes cast) × 100
```

**Requirement:**
- Approval % **must be >= 67%** (2/3 supermajority)
- Only YES and NO votes count toward total
- Abstain votes don't affect percentage but show participation

**Examples:**
```
✅ PASSES:
   200 YES + 100 NO = 66.7% approval → PASSES (rounds to 67%)
   67 YES + 33 NO = 67.0% approval → PASSES
   100 YES + 0 NO = 100.0% approval → PASSES

❌ FAILS:
   66 YES + 34 NO = 66.0% approval → FAILS
   50 YES + 50 NO = 50.0% approval → FAILS
   0 YES + 100 NO = 0.0% approval → FAILS
```

### 4. Proposal Execution

**Requirements:**
- Proposal status must be **Approved**
- Current time must be before **execution_deadline**
- Treasury must have sufficient balance

**Process:**
```rust
// Execute approved proposal
manager.execute_proposal(
    "prop-001",
    block_number,
    current_timestamp,
)?;
```

**Result:**
- Funds transferred to recipient
- Balance deducted from treasury
- Withdrawal recorded in history
- Proposal marked as **Executed**

## API Endpoints

### GET /treasury/stats
Get current treasury statistics.

**Response:**
```json
{
  "balance": 1234567000000,
  "balance_time": 1234.567,
  "total_allocated": 2345678000000,
  "total_distributed": 1111111000000,
  "allocation_count": 15678,
  "withdrawal_count": 42,
  "pending_proposals": 3
}
```

### GET /treasury/allocations
Get allocation history (deposits into treasury).

**Response:**
```json
[
  {
    "block_number": 12345,
    "amount": 500000000,
    "source": "BlockReward",
    "timestamp": 1699123456
  },
  {
    "block_number": 12346,
    "amount": 250000000,
    "source": "TransactionFees",
    "timestamp": 1699123500
  }
]
```

### GET /treasury/withdrawals
Get withdrawal history (distributions from treasury).

**Response:**
```json
[
  {
    "proposal_id": "prop-001",
    "amount": 5000000000000,
    "recipient": "time1abc123...",
    "block_number": 12400,
    "timestamp": 1699200000
  }
]
```

### POST /treasury/approve
Approve a proposal (internal governance use).

**Request:**
```json
{
  "proposal_id": "prop-001",
  "amount": 5000000000000
}
```

**Response:**
```json
{
  "status": "success",
  "proposal_id": "prop-001",
  "approved_amount": 5000000000000,
  "message": "Proposal approved for treasury spending"
}
```

### POST /treasury/distribute
Distribute funds for approved proposal.

**Request:**
```json
{
  "proposal_id": "prop-001",
  "recipient": "time1abc123...",
  "amount": 5000000000000
}
```

**Response:**
```json
{
  "status": "success",
  "proposal_id": "prop-001",
  "recipient": "time1abc123...",
  "amount": 5000000000000,
  "message": "Treasury funds distributed successfully"
}
```

## CLI Commands

### Get Treasury Information
```bash
$ time-cli rpc gettreasury

Treasury Balance: 1,234.567 TIME
Total Allocated: 2,345.678 TIME
Total Distributed: 1,111.111 TIME
Pending Proposals: 3
```

### List Proposals
```bash
$ time-cli rpc listproposals

ID: prop-001
Title: Website Redesign
Amount: 50,000 TIME
Status: Active
Votes: 110 YES, 1 NO (99% approval)

ID: prop-002
Title: Security Audit
Amount: 25,000 TIME
Status: Approved
Votes: 220 YES, 50 NO (81% approval)
```

## Security Considerations

### Why No Private Keys?

Traditional cryptocurrency treasuries use multi-signature wallets that require private keys held by trusted parties. This introduces several risks:
- ❌ Key theft or loss
- ❌ Custodian collusion
- ❌ Single points of failure
- ❌ Trust in individuals

TIME Coin's approach eliminates these risks:
- ✅ **Protocol-managed** - Treasury is part of consensus rules
- ✅ **State-based** - Balance tracked in blockchain state
- ✅ **Decentralized** - No individual or group has control
- ✅ **Transparent** - All operations visible on-chain
- ✅ **Immutable** - Cannot be changed without network consensus

### Consensus Protection

All treasury operations are protected by masternode consensus:
- **Proposal approval** - Requires 67% masternode agreement
- **Fund distribution** - Only approved proposals can be executed
- **State changes** - All modifications validated by consensus
- **Double-spend prevention** - Protocol enforces balance checks

### Attack Resistance

**Scenario: Malicious Proposal**
- Attacker creates proposal to drain treasury
- Masternodes review and vote NO
- Proposal rejected, funds safe

**Scenario: Compromised Masternode**
- Single compromised masternode votes YES on malicious proposal
- Other masternodes vote NO
- 67% threshold not met, proposal fails

**Scenario: Network Fork**
- Fork creates alternate chain
- Treasury state diverges between chains
- Consensus determines canonical chain
- Treasury follows canonical state

## Implementation Example

### Complete Proposal Lifecycle

```rust
use time_core::treasury_manager::{TreasuryManager, VoteChoice};

// 1. Create treasury manager
let mut manager = TreasuryManager::new();
manager.set_total_voting_power(300); // Total of all masternodes

// 2. Create proposal
manager.create_proposal(
    "prop-security-audit".to_string(),
    "Q4 2024 Security Audit".to_string(),
    "Comprehensive security audit by Trail of Bits".to_string(),
    "time1auditor_address".to_string(),
    25_000 * 100_000_000,  // 25,000 TIME
    "time1proposer_address".to_string(),
    1699000000,  // Submission time
    14,  // 14-day voting period
)?;

// 3. Masternodes vote
manager.vote_on_proposal(
    "prop-security-audit",
    "mn-gold-1".to_string(),
    VoteChoice::Yes,
    100,  // Gold voting power
    1699100000,
)?;

manager.vote_on_proposal(
    "prop-security-audit",
    "mn-silver-1".to_string(),
    VoteChoice::Yes,
    10,  // Silver voting power
    1699110000,
)?;

manager.vote_on_proposal(
    "prop-security-audit",
    "mn-bronze-1".to_string(),
    VoteChoice::No,
    1,  // Bronze voting power
    1699120000,
)?;

// 4. Update proposal status after voting period
manager.update_proposals(1699200000);  // After deadline

// 5. Check if approved
let proposal = manager.get_proposal("prop-security-audit").unwrap();
assert_eq!(proposal.status, ProposalStatus::Approved);
// Votes: 110 YES, 1 NO = 99% approval ✓

// 6. Execute approved proposal
manager.execute_proposal(
    "prop-security-audit",
    12500,  // Block number
    1699300000,  // Execution time
)?;

// 7. Verify execution
let proposal = manager.get_proposal("prop-security-audit").unwrap();
assert_eq!(proposal.status, ProposalStatus::Executed);
assert_eq!(manager.balance(), original_balance - 25_000 * 100_000_000);
```

## Testing

### Unit Tests
```bash
# Test treasury module
cargo test --package treasury

# Test treasury manager
cargo test --package time-core --lib treasury_manager

# Test governance integration
cargo test --package time-core --lib state
```

### Integration Tests
```bash
# Full treasury lifecycle
cargo test --package treasury --test integration

# End-to-end proposal workflow
cargo test --package time-core treasury_manager::tests::test_proposal_approval
```

### Test Scenarios
1. ✅ Funding from block rewards
2. ✅ Funding from transaction fees
3. ✅ Proposal creation and validation
4. ✅ Masternode voting with weighted power
5. ✅ Approval with 67% threshold
6. ✅ Rejection below 67% threshold
7. ✅ Proposal expiration
8. ✅ Insufficient funds handling
9. ✅ Double-vote prevention
10. ✅ Execution after approval

## Future Enhancements

### Planned Features
- **Milestone-based funding** - Release funds incrementally
- **Proposal amendments** - Modify proposals during discussion
- **Voting delegation** - Allow masternodes to delegate votes
- **Treasury bonds** - Lock funds for guaranteed returns
- **Automated grants** - Recurring payments for ongoing projects

### Governance Improvements
- **Discussion period** - 7-day discussion before voting
- **Quorum requirements** - Minimum participation rate
- **Emergency proposals** - Fast-track critical issues
- **Proposal deposits** - Require stake to submit proposals
- **Vote incentives** - Reward participation

## Resources

### Code References
- **Treasury State**: `core/src/state.rs`
- **Treasury Governance**: `treasury/src/governance.rs`
- **Treasury Manager**: `core/src/treasury_manager.rs`
- **Treasury Pool**: `treasury/src/pool.rs`
- **API Handlers**: `api/src/treasury_handlers.rs`
- **CLI Commands**: `cli/src/bin/time-cli.rs`

### Documentation
- **Whitepaper**: `/docs/whitepaper.md`
- **API Reference**: `/docs/api.md`
- **CLI Guide**: `/docs/cli.md`
- **Governance Guide**: This document

### Support
- **GitHub Issues**: https://github.com/time-coin/time-coin/issues
- **Discord**: https://discord.gg/timecoin
- **Forum**: https://forum.timecoin.org

---

**Last Updated**: November 2024  
**Version**: 1.0  
**Status**: Active
