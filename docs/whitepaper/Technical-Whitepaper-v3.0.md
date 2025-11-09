# TIME Coin: Complete Technical Specification

## Technical Whitepaper Version 3.0

**October 2025**

---

## Table of Contents

1. [Protocol Overview](#protocol-overview)
2. [Blockchain Architecture](#blockchain-architecture)
3. [Consensus Mechanism](#consensus-mechanism)
4. [Transaction Processing](#transaction-processing)
5. [Economic Model](#economic-model)
6. [Masternode System](#masternode-system)
7. [Governance Protocol](#governance-protocol)
8. [Treasury System](#treasury-system)
9. [Network Protocol](#network-protocol)
10. [Cryptographic Specifications](#cryptographic-specifications)
11. [State Management](#state-management)
12. [API Specifications](#api-specifications)
13. [Appendix: Constants & Parameters](#appendix-constants--parameters)

---

## Protocol Overview

### Design Philosophy

TIME Coin is designed around four core principles:

**1. Instant Finality**
- Modified Byzantine Fault Tolerant consensus
- Transactions confirmed in <3 seconds
- No probabilistic finality
- Deterministic and irreversible

**2. Efficient Scalability**
- 24-hour settlement blocks (365/year)
- Dynamic quorum selection
- Scales to 100,000+ masternodes
- Logarithmic communication complexity

**3. Democratic Governance**
- Community-controlled protocol
- Weighted masternode voting
- Transparent on-chain proposals
- No foundation control

**4. Sustainable Economics**
- Purchase-based minting
- Dynamic block rewards (100-500 TIME/day)
- Self-funding treasury
- Capped long-term inflation

### Layer Architecture

```
┌─────────────────────────────────────────────────────┐
│           Application Layer                          │
│  Web, Mobile, SMS, Email Interfaces                 │
└──────────────────┬──────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────┐
│              API Layer                               │
│  REST API, RPC, WebSocket                           │
└──────────────────┬──────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────┐
│          Transaction Layer                           │
│  Validation, Signing, Broadcasting                  │
└──────────────────┬──────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────┐
│      BFT Consensus Layer (Real-time)                │
│  Instant Validation, Dynamic Quorum                 │
└──────────────────┬──────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────┐
│    Block Formation Layer (Daily)                    │
│  Aggregate, Finalize, Distribute Rewards            │
└──────────────────┬──────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────┐
│         Blockchain Storage                           │
│  Blocks, State, Indices                             │
└──────────────────┬──────────────────────────────────┘
                   │
┌──────────────────▼──────────────────────────────────┐
│            Network Layer (P2P)                       │
│  Masternode Network, Gossip Protocol                │
└─────────────────────────────────────────────────────┘
```

---

## Blockchain Architecture

### Block Structure

#### Header

```rust
pub struct BlockHeader {
    /// Block number (days since genesis)
    pub block_number: u64,
    
    /// Unix timestamp (start of day, 00:00:00 UTC)
    pub timestamp: u64,
    
    /// Hash of previous block
    pub previous_hash: Hash,
    
    /// Merkle root of all transactions
    pub transactions_root: Hash,
    
    /// Merkle root of final state
    pub state_root: Hash,
    
    /// Number of transactions in block
    pub transaction_count: u64,
    
    /// Total fees collected
    pub total_fees: u64,
    
    /// Block proposer (selected masternode)
    pub proposer: NodeId,
    
    /// BFT signatures from masternodes
    pub signatures: Vec<Signature>,
}

impl BlockHeader {
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha3_256::new();
        hasher.update(&self.block_number.to_le_bytes());
        hasher.update(&self.timestamp.to_le_bytes());
        hasher.update(&self.previous_hash);
        hasher.update(&self.transactions_root);
        hasher.update(&self.state_root);
        Hash::from(hasher.finalize())
    }
}
```

#### Body

```rust
pub struct BlockBody {
    /// All transactions from the 24-hour period
    pub transactions: Vec<Transaction>,
    
    /// Reward distribution record
    pub rewards: RewardDistribution,
    
    /// State changes summary
    pub state_changes: StateChanges,
}

pub struct Block {
    pub header: BlockHeader,
    pub body: BlockBody,
}
```

### Block Creation Timeline

```
Day Boundary (00:00:00 UTC):
├─ Previous day ends (Day N: 00:00:00 to 23:59:59)
└─ New day begins (Day N+1: 00:00:00+)

00:00:01 UTC - Block Proposer Selection:
├─ VRF-based random selection
├─ Weighted by (tier × longevity)
└─ Selected node begins building block for Day N

00:00:01-00:03 UTC - Transaction Aggregation:
├─ Collect all validated transactions from Day N
├─ Order by timestamp
├─ Calculate merkle roots
├─ Compute new state root
└─ Build block structure

00:03:00 UTC - Block Proposal:
├─ Proposer broadcasts block candidate
└─ All masternodes receive

00:03:00-00:05:00 UTC - Block Validation:
├─ Each masternode independently verifies:
│  ├─ Transaction validity
│  ├─ State transitions
│  ├─ Fee calculations
│  └─ Reward distribution
└─ Masternodes prepare to vote

00:05:00-00:07:00 UTC - Block Signing:
├─ Masternodes vote to accept/reject
├─ Sign block if valid
├─ 80% threshold required (of total voting power)
└─ Signatures collected

00:07:00 UTC - Block Finalization:
├─ If 80% signed → Block confirmed
├─ Block added to blockchain
├─ State updated globally
├─ Rewards distributed
└─ Day N+1 continues (already processing)

If Block Not Finalized by 00:10 UTC:
├─ Emergency round triggered
├─ New proposer selected
├─ 5-minute emergency window
├─ Requires 90% approval (higher threshold)
└─ Must finalize by 00:15 UTC
```

### Storage Efficiency

**Projected Growth:**

```
Assumptions:
- Average 100,000 transactions per day
- Average transaction size: 250 bytes
- Block overhead: 1 KB

Daily Storage:
100,000 tx × 250 bytes = 25 MB
+ 1 KB overhead = ~25 MB per block

Annual Storage:
365 blocks × 25 MB = ~9.125 GB/year

10-Year Projection:
365 × 10 blocks × 25 MB = ~91.25 GB

100-Year Projection:
365 × 100 blocks × 25 MB = ~912.5 GB

Comparison:
- Bitcoin (14 years): ~500 GB
- Ethereum (8 years): ~1 TB
- TIME (100 years): ~912 GB
```

---

## Consensus Mechanism

### Modified Byzantine Fault Tolerant (BFT)

- Minimum of **3 active masternodes** required to initiate consensus (tolerates 0 Byzantine failures)
- **Recommended**: 4+ nodes for production (tolerates 1 Byzantine failure), 7+ for high security (tolerates 2 Byzantine failures)

**Current Implementation**: Simple one-node-one-vote counting with 67% threshold and deterministic round-robin quorum selection by block height.

**Future Enhancement**: Weighted voting based on tier, longevity, and reputation with VRF-based quorum selection.

#### Core Algorithm

**Consensus Phases:**

```
Phase 1: Pre-Vote
- Nodes receive transaction
- Validate signature, balance, nonce
- Broadcast PRE-VOTE message

Phase 2: Pre-Commit
- If receive 67%+ PRE-VOTES → Broadcast PRE-COMMIT
- Otherwise → Reject

Phase 3: Commit
- If receive 67%+ PRE-COMMITs → Transaction confirmed
- Update local state
- Broadcast state update

Phase 4: Finality
- Global state synchronized
- Transaction irreversibly confirmed
- User notified
```

**Implementation:**

```rust
pub struct BftConsensus {
    /// Current round number
    round: u64,
    
    /// Validator set for this round (dynamic quorum)
    validators: Vec<Validator>,
    
    /// Votes received
    pre_votes: HashMap<TransactionId, Vec<Signature>>,
    pre_commits: HashMap<TransactionId, Vec<Signature>>,
    
    /// Voting power threshold (67%)
    threshold: u64,
}

impl BftConsensus {
    pub fn process_transaction(&mut self, tx: Transaction) -> Result<()> {
        // Phase 1: Validate
        self.validate_transaction(&tx)?;
        
        // Phase 2: Collect pre-votes
        let pre_vote_weight = self.collect_pre_votes(&tx);
        if pre_vote_weight < self.threshold {
            return Err("Insufficient pre-votes");
        }
        
        // Phase 3: Collect pre-commits
        let pre_commit_weight = self.collect_pre_commits(&tx);
        if pre_commit_weight < self.threshold {
            return Err("Insufficient pre-commits");
        }
        
        // Phase 4: Finalize
        self.finalize_transaction(&tx);
        Ok(())
    }
}
```

### Dynamic Quorum Selection

**Current Implementation:**

Quorum size is calculated as: `(total_nodes * 2 / 3) + 1`

**Minimum**: 3 nodes required for BFT consensus

**Quorum Size Formula (Future Enhancement):**

```
quorum_size = max(50, min(500, log₂(total_nodes) × 50))

Examples:
- 100 nodes → 50 nodes (50%)
- 500 nodes → 112 nodes (22%)
- 1,000 nodes → 150 nodes (15%)
- 5,000 nodes → 237 nodes (4.7%)
- 10,000 nodes → 300 nodes (3%)
- 50,000 nodes → 450 nodes (0.9%)
- 100,000 nodes → 500 nodes (0.5%)
```

**Selection Algorithm (Future Enhancement):**

```rust
pub fn select_quorum(
    transaction: &Transaction,
    all_nodes: &[Masternode],
) -> Vec<Masternode> {
    // 1. Calculate quorum size
    let total_nodes = all_nodes.len();
    let quorum_size = calculate_quorum_size(total_nodes);
    
    // 2. Generate deterministic seed from transaction
    let seed = hash_transaction(transaction);
    
    // 3. Initialize VRF with seed
    let mut vrf = Vrf::new(seed);
    
    // 4. Calculate weights for each node
    let weights: Vec<u64> = all_nodes
        .iter()
        .map(|node| node.tier_weight() * node.longevity_multiplier())
        .collect();
    
    // 5. Weighted random selection
    let mut selected = Vec::new();
    let total_weight: u64 = weights.iter().sum();
    
    for _ in 0..quorum_size {
        let random_weight = vrf.next_u64() % total_weight;
        let mut cumulative = 0u64;
        
        for (i, &weight) in weights.iter().enumerate() {
            cumulative += weight;
            if random_weight < cumulative {
                selected.push(all_nodes[i].clone());
                break;
            }
        }
    }
    
    selected
}

fn calculate_quorum_size(n: usize) -> usize {
    let log_size = (n as f64).log2() * 50.0;
    std::cmp::max(50, std::cmp::min(500, log_size as usize))
}
```

### Voting Weight Calculation

**Note**: The following weighted voting system is planned for future implementation. Current implementation uses simple one-node-one-vote counting.

```rust
pub struct Masternode {
    pub address: Address,
    pub tier: Tier,
    pub registration_time: u64,
    pub last_active: u64,
}

impl Masternode {
    /// Calculate tier weight (Planned)
    pub fn tier_weight(&self) -> u64 {
        match self.tier {
            Tier::Free => 1,      // Free tier
            Tier::Bronze => 1,    // 1,000 TIME collateral
            Tier::Silver => 10,   // 10,000 TIME collateral
            Tier::Gold => 100,    // 100,000 TIME collateral
        }
    }
    
    /// Calculate longevity multiplier (Planned)
    pub fn longevity_multiplier(&self) -> f64 {
        let days_active = (current_time() - self.registration_time) / 86400;
        let years_active = days_active as f64 / 365.0;
        
        // Formula: 1 + (years × 0.5), max 3.0
        let multiplier = 1.0 + (years_active * 0.5);
        multiplier.min(3.0)
    }
    
    /// Total voting power (Planned)
    pub fn voting_power(&self) -> u64 {
        (self.tier_weight() as f64 * self.longevity_multiplier()) as u64
    }
}
```

### Consensus Guarantees

**Safety**: Cannot confirm conflicting transactions
```
Proof:
- Require 67% of quorum to confirm
- Any two quorums overlap by at least 34%
- Overlapping nodes see both transactions
- Will only vote for one (first seen)
- Impossible for both to reach 67%
```

**Liveness**: Always makes progress
```
Proof:
- As long as >67% of network online and honest
- Quorum can always be formed
- Transactions always processed
- Network never deadlocks
- Minimum 3 nodes required (tolerates 0 Byzantine failures)
```

**Finality**: Immediate and irreversible
```
Proof:
- Once 67% confirm → Irreversible by protocol
- No chain reorganizations possible
- Deterministic finality (not probabilistic)
- Cannot be undone except by hard fork
```

---

## Transaction Processing

### Transaction Types

#### Transfer Transaction

```rust
pub struct Transfer {
    /// Sender address
    pub from: Address,
    
    /// Recipient address
    pub to: Address,
    
    /// Amount in TIME (8 decimals)
    pub amount: u64,
    
    /// Transaction fee
    pub fee: u64,
    
    /// Sequential nonce (prevents double-spend)
    pub nonce: u64,
    
    /// Submission timestamp
    pub timestamp: u64,
    
    /// User's signature
    pub signature: Signature,
    
    /// BFT consensus signatures (added during processing)
    pub bft_signatures: Vec<Signature>,
}

impl Transfer {
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha3_256::new();
        hasher.update(self.from.as_bytes());
        hasher.update(self.to.as_bytes());
        hasher.update(&self.amount.to_le_bytes());
        hasher.update(&self.fee.to_le_bytes());
        hasher.update(&self.nonce.to_le_bytes());
        hasher.update(&self.timestamp.to_le_bytes());
        Hash::from(hasher.finalize())
    }
    
    pub fn verify_signature(&self) -> bool {
        verify_ed25519(
            &self.hash(),
            &self.signature,
            &self.from.to_public_key()
        )
    }
}
```

#### Mint Transaction

```rust
pub struct Mint {
    /// Recipient address
    pub recipient: Address,
    
    /// Amount to mint
    pub amount: u64,
    
    /// Payment verification proof
    pub payment_proof: PaymentProof,
    
    /// Gateway that verified payment
    pub gateway: GatewayId,
    
    /// Verification signatures (4 of 5 required)
    pub verifier_signatures: Vec<Signature>,
    
    /// Timestamp
    pub timestamp: u64,
}

pub struct PaymentProof {
    /// Gateway's signature
    pub gateway_signature: Signature,
    
    /// Payment receipt hash (bank or blockchain)
    pub receipt_hash: Hash,
    
    /// Amount in USD/crypto
    pub payment_amount: u64,
    
    /// Payment currency
    pub currency: Currency,
    
    /// Sequential nonce (anti-replay)
    pub nonce: u64,
}
```

#### Governance Vote Transaction

```rust
pub struct Vote {
    /// Voter's address (masternode)
    pub voter: Address,
    
    /// Proposal ID
    pub proposal_id: Hash,
    
    /// Vote choice
    pub choice: VoteChoice,
    
    /// Voter's signature
    pub signature: Signature,
    
    /// Timestamp
    pub timestamp: u64,
}

pub enum VoteChoice {
    Yes,
    No,
    Abstain,
}
```

### Transaction Validation

**Pre-Validation Checks:**

```rust
pub fn pre_validate(tx: &Transaction) -> Result<()> {
    // 1. Signature verification
    if !tx.verify_signature() {
        return Err("Invalid signature");
    }
    
    // 2. Nonce check
    let account_state = get_account_state(&tx.from)?;
    if tx.nonce != account_state.nonce + 1 {
        return Err("Invalid nonce");
    }
    
    // 3. Balance check
    let required = tx.amount + tx.fee;
    if account_state.balance < required {
        return Err("Insufficient balance");
    }
    
    // 4. Mempool conflict check
    if mempool.has_conflicting_tx(&tx) {
        return Err("Conflicting transaction in mempool");
    }
    
    // 5. Timestamp check (not too old, not too far in future)
    let now = current_time();
    if tx.timestamp < now - 3600 || tx.timestamp > now + 300 {
        return Err("Invalid timestamp");
    }
    
    Ok(())
}
```

**State Transition:**

```rust
pub fn execute_transaction(tx: &Transaction) -> Result<StateUpdate> {
    let mut state = get_current_state();
    
    // Update sender
    let sender = state.get_account_mut(&tx.from)?;
    sender.balance -= tx.amount + tx.fee;
    sender.nonce += 1;
    
    // Update recipient
    let recipient = state.get_account_mut(&tx.to)?;
    recipient.balance += tx.amount;
    
    // Distribute fee
    let masternode_fee = (tx.fee as f64 * 0.95) as u64;
    let treasury_fee = tx.fee - masternode_fee;
    
    state.treasury_balance += treasury_fee;
    state.pending_masternode_fees += masternode_fee;
    
    Ok(StateUpdate {
        accounts_changed: vec![tx.from, tx.to],
        new_state_root: state.merkle_root(),
    })
}
```

### Nonce System

**Purpose**: Prevent double-spending and transaction replay

**Implementation:**

```rust
pub struct AccountState {
    pub address: Address,
    pub balance: u64,
    pub nonce: u64,  // Transaction counter
}

pub fn validate_nonce(account: &AccountState, tx: &Transaction) -> bool {
    // Transaction must use next nonce in sequence
    tx.nonce == account.nonce + 1
}

pub fn update_nonce(account: &mut AccountState) {
    account.nonce += 1;
}
```

**Properties:**
- Each transaction increments nonce by 1
- Cannot skip nonces (must be sequential)
- Cannot reuse nonces (checked against current state)
- Provides total ordering of account transactions
- Makes double-spending cryptographically impossible

---

## Economic Model

### Supply Dynamics

**Token Creation:**

```
Purchase-Based Minting:
User pays $X → Gateway verifies → TIME minted

Formula:
TIME_minted = (Payment_USD / Current_Price_USD) × 0.90

Distribution:
- 90% to purchaser
- 8% to masternode fee pool
- 2% to development treasury

Example:
User pays $1,000, TIME price = $5
TIME minted = ($1,000 / $5) × 0.90 = 180 TIME
Fee pool receives = 200 × 0.08 = 16 TIME
Treasury receives = 200 × 0.02 = 4 TIME
Total created = 200 TIME
```

### Dynamic Block Rewards

**Formula:**

```rust
pub fn calculate_block_reward(active_masternodes: u64) -> u64 {
    const BASE_REWARD: u64 = 100 * TIME_UNIT;
    const SCALE_FACTOR: f64 = 0.04;
    const MAX_REWARD: u64 = 500 * TIME_UNIT;
    
    let scaled_reward = BASE_REWARD + 
        (active_masternodes as f64 * SCALE_FACTOR * TIME_UNIT as f64) as u64;
    
    std::cmp::min(scaled_reward, MAX_REWARD)
}

pub fn distribute_block_reward(total_reward: u64) -> (u64, u64) {
    let masternode_reward = (total_reward as f64 * 0.95) as u64;
    let treasury_reward = total_reward - masternode_reward;
    
    (masternode_reward, treasury_reward)
}
```

**Reward Schedule:**

```
Active Nodes | Total Daily | Masternode Pool | Treasury Pool | Annual Total
-------------|-------------|-----------------|---------------|-------------
100          | 104 TIME    | 98.8 TIME       | 5.2 TIME      | 37,960 TIME
500          | 120 TIME    | 114 TIME        | 6 TIME        | 43,800 TIME
1,000        | 140 TIME    | 133 TIME        | 7 TIME        | 51,100 TIME
2,500        | 200 TIME    | 190 TIME        | 10 TIME       | 73,000 TIME
5,000        | 300 TIME    | 285 TIME        | 15 TIME       | 109,500 TIME
10,000       | 500 TIME    | 475 TIME        | 25 TIME       | 182,500 TIME (capped)
20,000       | 500 TIME    | 475 TIME        | 25 TIME       | 182,500 TIME (capped)
```

### Fee Structure

**Transaction Fees:**

```rust
pub struct FeeCalculator;

impl FeeCalculator {
    /// Base fee (minimum)
    const BASE_FEE: u64 = 1_000_000; // 0.01 TIME
    
    /// Fee per byte
    const FEE_PER_BYTE: u64 = 1_000; // 0.00001 TIME
    
    /// Priority fee (optional, set by user)
    pub fn calculate_fee(tx_size: usize, priority: Priority) -> u64 {
        let size_fee = tx_size as u64 * Self::FEE_PER_BYTE;
        let priority_multiplier = match priority {
            Priority::Low => 1.0,
            Priority::Medium => 2.0,
            Priority::High => 5.0,
        };
        
        let base = Self::BASE_FEE + size_fee;
        (base as f64 * priority_multiplier) as u64
    }
}

pub fn distribute_fees(total_fees: u64) -> (u64, u64) {
    let masternode_share = (total_fees as f64 * 0.95) as u64;
    let treasury_share = total_fees - masternode_share;
    (masternode_share, treasury_share)
}
```

### Reward Distribution

**Per-Masternode Calculation:**

```rust
pub fn calculate_node_reward(
    node: &Masternode,
    total_pool: u64,
    total_network_weight: u64
) -> u64 {
    // Node's share of total weight
    let node_weight = node.voting_power();
    let share = node_weight as f64 / total_network_weight as f64;
    
    // Base reward
    let base_reward = (total_pool as f64 * share) as u64;
    
    // Voting bonus (5% if >80% participation)
    let voting_bonus = if node.voting_participation() > 0.80 {
        (base_reward as f64 * 0.05) as u64
    } else {
        0
    };
    
    base_reward + voting_bonus
}
```

**Example Distribution:**

```
Network State:
- Total nodes: 1,000
- Daily masternode pool: 133 TIME
- Total network weight: 1,650 (mixed tiers + longevity)

Node Examples:

1. Bronze Node (1 year old):
   Weight: 1 × 1.5 = 1.5
   Share: 1.5 / 1,650 = 0.0909%
   Daily reward: 133 × 0.0909% = 0.121 TIME
   Annual reward: 44.2 TIME (4.42% APY on 1,000 TIME)

2. Silver Node (2 years old):
   Weight: 10 × 2.0 = 20
   Share: 20 / 1,650 = 1.212%
   Daily reward: 133 × 1.212% = 1.61 TIME
   Annual reward: 588 TIME (5.88% APY on 10,000 TIME)

3. Gold Node (4 years old):
   Weight: 100 × 3.0 = 300
   Share: 300 / 1,650 = 18.18%
   Daily reward: 133 × 18.18% = 24.18 TIME
   Annual reward: 8,826 TIME (8.83% APY on 100,000 TIME)

Plus transaction fees distributed proportionally.
```

### Inflation Analysis

**Maximum Annual Inflation:**

```
Block Rewards (Capped):
- Maximum: 500 TIME/day
- Annual maximum: 182,500 TIME/year

Purchase-Based Minting:
- Demand-driven (not inflationary in traditional sense)
- Creates new tokens only when purchased
- Organic growth tied to adoption

Total Supply Growth:
Year 1: ~43,800 (rewards) + purchase minting
Year 5: ~109,500 (rewards) + purchase minting
Year 10+: 182,500 (rewards, capped) + purchase minting
```

**Deflationary Pressures:**

```
1. Masternode Lockup:
   - 1,000 nodes × 10,000 avg = 10M TIME locked
   - Not in circulation
   - Grows with network

2. Treasury Accumulation:
   - Holds funds for development
   - Spends only via governance
   - Accumulates faster than spends early on

3. Lost Keys:
   - Natural attrition
   - Irrecoverable wallets
   - Permanent removal
```

---

## Masternode System

### Collateral Requirements

```rust
pub enum Tier {
    Free,    // No collateral (limited features)
    Bronze,  // 1,000 TIME
    Silver,  // 10,000 TIME
    Gold,    // 100,000 TIME
}

pub struct MasternodeCollateral {
    pub tier: Tier,
    pub amount: u64,
    pub locked_at: u64,
    pub owner: Address,
}

impl MasternodeCollateral {
    pub fn required_amount(tier: Tier) -> u64 {
        match tier {
            Tier::Free => 0,
            Tier::Bronze => 1_000 * TIME_UNIT,
            Tier::Silver => 10_000 * TIME_UNIT,
            Tier::Gold => 100_000 * TIME_UNIT,
        }
    }
}
```

### Registration Process

```rust
pub struct MasternodeRegistration {
    pub operator_address: Address,
    pub tier: Tier,
    pub collateral_tx: Hash,
    pub node_public_key: PublicKey,
    pub ip_address: IpAddr,
    pub port: u16,
    pub signature: Signature,
}

pub fn register_masternode(reg: MasternodeRegistration) -> Result<NodeId> {
    // 1. Verify collateral transaction
    verify_collateral_tx(&reg.collateral_tx, &reg.tier)?;
    
    // 2. Verify signature
    verify_signature(&reg)?;
    
    // 3. Check no duplicate IP/key
    check_no_duplicates(&reg)?;
    
    // 4. Lock collateral in treasury
    lock_collateral(&reg.collateral_tx)?;
    
    // 5. Create masternode record
    let node_id = generate_node_id(&reg);
    let masternode = Masternode {
        id: node_id,
        address: reg.operator_address,
        tier: reg.tier,
        registration_time: current_time(),
        last_active: current_time(),
        public_key: reg.node_public_key,
    };
    
    // 6. Add to active set
    add_to_active_set(masternode)?;
    
    Ok(node_id)
}
```

### Longevity Tracking

```rust
pub struct LongevityTracker {
    pub registration_time: u64,
    pub total_uptime: u64,
    pub last_check: u64,
    pub downtime_events: Vec<DowntimeEvent>,
}

impl LongevityTracker {
    pub fn calculate_multiplier(&self) -> f64 {
        let days_active = self.total_uptime / 86400;
        let years_active = days_active as f64 / 365.0;
        
        // Formula: 1 + (years × 0.5), max 3.0
        let multiplier = 1.0 + (years_active * 0.5);
        multiplier.min(3.0)
    }
    
    pub fn check_downtime(&mut self, current_time: u64) -> bool {
        let downtime = current_time - self.last_check;
        
        // If offline > 72 hours, reset multiplier
        if downtime > 259200 {  // 72 hours
            self.total_uptime = 0;
            self.registration_time = current_time;
            self.downtime_events.push(DowntimeEvent {
                start: self.last_check,
                duration: downtime,
                reset: true,
            });
            true  // Multiplier reset
        } else {
            // Add to uptime
            self.total_uptime += downtime;
            self.last_check = current_time;
            false  // No reset
        }
    }
}
```

### Performance Monitoring

```rust
pub struct PerformanceMetrics {
    pub uptime_percentage: f64,
    pub transactions_validated: u64,
    pub blocks_signed: u64,
    pub governance_participation: f64,
    pub average_response_time: u64,  // milliseconds
}

pub fn calculate_performance_score(metrics: &PerformanceMetrics) -> f64 {
    let uptime_score = metrics.uptime_percentage;
    let participation_score = metrics.governance_participation * 100.0;
    let responsiveness_score = if metrics.average_response_time < 1000 {
        100.0
    } else if metrics.average_response_time < 3000 {
        75.0
    } else {
        50.0
    };
    
    // Weighted average
    (uptime_score * 0.5) + 
    (participation_score * 0.3) + 
    (responsiveness_score * 0.2)
}
```

### Slashing Mechanism

```rust
pub enum Violation {
    DoubleSigning { proof: DoubleSignProof },
    LongTermAbandonment { days_offline: u64 },
    DataWithholding { proof: WithholdingProof },
    NetworkAttack { proof: AttackProof },
    ConsensusManipulation { proof: ManipulationProof },
}

pub fn calculate_slash_amount(
    violation: &Violation,
    collateral: u64
) -> u64 {
    let percentage = match violation {
        Violation::DoubleSigning { .. } => 0.5,  // 50%
        Violation::LongTermAbandonment { days_offline } => {
            if *days_offline > 90 {
                0.2  // 20%
            } else if *days_offline > 60 {
                0.15  // 15%
            } else {
                0.1  // 10%
            }
        },
        Violation::DataWithholding { .. } => 0.25,  // 25%
        Violation::NetworkAttack { .. } => 1.0,  // 100%
        Violation::ConsensusManipulation { .. } => 0.7,  // 70%
    };
    
    (collateral as f64 * percentage) as u64
}

pub fn execute_slashing(
    node_id: NodeId,
    violation: Violation
) -> Result<SlashingRecord> {
    let node = get_masternode(node_id)?;
    let slash_amount = calculate_slash_amount(&violation, node.collateral);
    
    // Transfer to treasury
    transfer_to_treasury(slash_amount)?;
    
    // Update node record
    node.collateral -= slash_amount;
    node.slashing_history.push(violation);
    
    // Create on-chain record
    Ok(SlashingRecord {
        node_id,
        violation,
        amount: slash_amount,
        timestamp: current_time(),
        evidence: violation.evidence(),
    })
}
```

---

## Governance Protocol

### Proposal System

```rust
pub struct Proposal {
    pub id: Hash,
    pub proposer: Address,  // Must be Gold tier
    pub title: String,
    pub description: String,
    pub category: ProposalCategory,
    pub funding_amount: Option<u64>,
    pub recipient: Option<Address>,
    pub execution_plan: ExecutionPlan,
    pub deposit: u64,  // 100 or 500 TIME
    pub created_at: u64,
    pub voting_start: u64,
    pub voting_end: u64,
    pub status: ProposalStatus,
}

pub enum ProposalCategory {
    DevelopmentGrant,
    MarketingInitiative,
    SecurityAudit,
    InfrastructureImprovement,
    ResearchProject,
    CommunityProgram,
    EmergencyAction,
    ProtocolParameterChange,
}

pub enum ProposalStatus {
    Discussion,      // 7-14 days
    Voting,          // 7-14 days
    Approved,        // Passed thresholds
    Rejected,        // Failed thresholds
    Executed,        // Carried out
    Cancelled,       // Withdrawn by proposer
}
```

### Voting Mechanism

```rust
pub struct Vote {
    pub proposal_id: Hash,
    pub voter: Address,
    pub choice: VoteChoice,
    pub voting_power: u64,
    pub signature: Signature,
    pub timestamp: u64,
}

pub fn cast_vote(vote: Vote) -> Result<()> {
    // 1. Verify voter is masternode operator
    let node = get_masternode_by_address(&vote.voter)?;
    
    // 2. Verify proposal is in voting period
    let proposal = get_proposal(&vote.proposal_id)?;
    if proposal.status != ProposalStatus::Voting {
        return Err("Proposal not in voting period");
    }
    
    // 3. Calculate voting power
    let voting_power = node.voting_power();
    
    // 4. Record vote
    record_vote(vote)?;
    
    Ok(())
}

pub fn tally_votes(proposal_id: Hash) -> VoteResult {
    let votes = get_all_votes(proposal_id);
    let total_voting_power = calculate_total_network_power();
    
    let mut yes_power = 0u64;
    let mut no_power = 0u64;
    let mut abstain_power = 0u64;
    
    for vote in votes {
        match vote.choice {
            VoteChoice::Yes => yes_power += vote.voting_power,
            VoteChoice::No => no_power += vote.voting_power,
            VoteChoice::Abstain => abstain_power += vote.voting_power,
        }
    }
    
    let total_participated = yes_power + no_power + abstain_power;
    let participation_rate = total_participated as f64 / total_voting_power as f64;
    let approval_rate = yes_power as f64 / (yes_power + no_power) as f64;
    
    VoteResult {
        yes_power,
        no_power,
        abstain_power,
        participation_rate,
        approval_rate,
        passed: participation_rate >= 0.60 && approval_rate >= 0.60,
    }
}
```

### Execution Framework

```rust
pub struct ExecutionPlan {
    pub milestones: Vec<Milestone>,
    pub deliverables: Vec<Deliverable>,
    pub timeline: Timeline,
}

pub struct Milestone {
    pub id: u32,
    pub description: String,
    pub completion_criteria: Vec<Criterion>,
    pub funding_release: u64,
    pub deadline: u64,
}

pub fn execute_proposal(proposal_id: Hash) -> Result<()> {
    let proposal = get_proposal(proposal_id)?;
    let vote_result = tally_votes(proposal_id);
    
    if !vote_result.passed {
        return Err("Proposal did not pass");
    }
    
    match proposal.category {
        ProposalCategory::DevelopmentGrant => {
            execute_grant_proposal(&proposal)?;
        },
        ProposalCategory::ProtocolParameterChange => {
            execute_parameter_change(&proposal)?;
        },
        ProposalCategory::EmergencyAction => {
            execute_emergency_action(&proposal)?;
        },
        _ => {
            execute_standard_proposal(&proposal)?;
        }
    }
    
    Ok(())
}

pub fn execute_grant_proposal(proposal: &Proposal) -> Result<()> {
    // Milestone-based funding
    for milestone in &proposal.execution_plan.milestones {
        // Wait for milestone completion
        wait_for_milestone_completion(milestone)?;
        
        // Verify completion criteria
        verify_milestone_criteria(milestone)?;
        
        // Release funding
        release_funding(
            proposal.recipient.unwrap(),
            milestone.funding_release
        )?;
    }
    
    Ok(())
}
```

---

## Treasury System

### Treasury Structure

```rust
pub struct Treasury {
    /// Operating funds (spendable)
    pub operating_balance: u64,
    
    /// Collateral held in escrow (not spendable)
    pub collateral_balance: u64,
    
    /// Reserve fund (emergency)
    pub reserve_balance: u64,
    
    /// Historical income
    pub total_income: u64,
    
    /// Historical spending
    pub total_spending: u64,
    
    /// Active proposals
    pub active_proposals: Vec<Hash>,
}

impl Treasury {
    pub fn add_income(&mut self, amount: u64, source: IncomeSource) {
        match source {
            IncomeSource::BlockReward => {
                self.operating_balance += amount;
            },
            IncomeSource::TransactionFee => {
                self.operating_balance += amount;
            },
            IncomeSource::SlashingPenalty => {
                // 50% to operating, 50% to reserve
                let operating = amount / 2;
                let reserve = amount - operating;
                self.operating_balance += operating;
                self.reserve_balance += reserve;
            },
            IncomeSource::Collateral => {
                // Separate tracking (not spendable)
                self.collateral_balance += amount;
            }
        }
        self.total_income += amount;
    }
    
    pub fn spend(&mut self, amount: u64, category: SpendingCategory) -> Result<()> {
        if amount > self.operating_balance {
            return Err("Insufficient operating funds");
        }
        
        // Check rate limits
        check_spending_limits(amount, category)?;
        
        self.operating_balance -= amount;
        self.total_spending += amount;
        
        Ok(())
    }
}
```

### Income Sources

```rust
pub enum IncomeSource {
    BlockReward,
    TransactionFee,
    SlashingPenalty,
    Collateral,  // Escrow only, not spendable
}

pub fn process_daily_income() {
    let mut treasury = get_treasury();
    
    // 1. Block rewards (5%)
    let block_reward = calculate_block_reward(active_masternodes());
    let treasury_reward = (block_reward as f64 * 0.05) as u64;
    treasury.add_income(treasury_reward, IncomeSource::BlockReward);
    
    // 2. Transaction fees (5%)
    let daily_fees = get_daily_fees();
    let treasury_fees = (daily_fees as f64 * 0.05) as u64;
    treasury.add_income(treasury_fees, IncomeSource::TransactionFee);
    
    // 3. Any slashing penalties
    for penalty in get_daily_penalties() {
        treasury.add_income(penalty, IncomeSource::SlashingPenalty);
    }
}
```

### Spending Controls

```rust
pub struct SpendingLimits {
    pub daily_limit: u64,
    pub monthly_limit: u64,
    pub category_limits: HashMap<SpendingCategory, u64>,
}

pub fn check_spending_limits(
    amount: u64,
    category: SpendingCategory
) -> Result<()> {
    let limits = get_spending_limits();
    let today_spent = get_today_spending();
    let month_spent = get_month_spending();
    
    // Check daily limit
    if today_spent + amount > limits.daily_limit {
        return Err("Exceeds daily spending limit");
    }
    
    // Check monthly limit
    if month_spent + amount > limits.monthly_limit {
        return Err("Exceeds monthly spending limit");
    }
    
    // Check category limit
    if let Some(&category_limit) = limits.category_limits.get(&category) {
        let category_spent = get_category_spending(category);
        if category_spent + amount > category_limit {
            return Err("Exceeds category spending limit");
        }
    }
    
    Ok(())
}
```

### Threshold Signature Control

```rust
pub struct TreasuryOperation {
    pub operation_type: OperationType,
    pub amount: u64,
    pub recipient: Address,
    pub proposal_id: Option<Hash>,
    pub required_signatures: usize,
    pub collected_signatures: Vec<Signature>,
    pub time_lock: u64,
}

pub fn execute_treasury_operation(op: &mut TreasuryOperation) -> Result<()> {
    // 1. Check time lock
    if current_time() < op.time_lock {
        return Err("Time lock not expired");
    }
    
    // 2. Verify required signatures collected
    if op.collected_signatures.len() < op.required_signatures {
        return Err("Insufficient signatures");
    }
    
    // 3. Verify each signature
    for sig in &op.collected_signatures {
        verify_masternode_signature(sig)?;
    }
    
    // 4. Execute operation
    match op.operation_type {
        OperationType::Release => {
            release_funds(op.amount, op.recipient)?;
        },
        OperationType::Return => {
            return_collateral(op.amount, op.recipient)?;
        },
        OperationType::Slash => {
            slash_collateral(op.amount)?;
        }
    }
    
    Ok(())
}
```

---

## Network Protocol

### Peer-to-Peer Communication

```rust
pub struct P2PMessage {
    pub message_type: MessageType,
    pub payload: Vec<u8>,
    pub sender: NodeId,
    pub signature: Signature,
    pub timestamp: u64,
}

pub enum MessageType {
    TransactionBroadcast,
    BlockProposal,
    VoteMessage,
    StateUpdate,
    PeerDiscovery,
    Heartbeat,
}
```

### Gossip Protocol

```rust
pub struct GossipProtocol {
    pub peers: Vec<Peer>,
    pub message_cache: LruCache<Hash, P2PMessage>,
    pub fanout: usize,  // Number of peers to forward to
}

impl GossipProtocol {
    pub fn broadcast(&mut self, message: P2PMessage) {
        // 1. Add to local cache
        let msg_hash = hash_message(&message);
        self.message_cache.put(msg_hash, message.clone());
        
        // 2. Select random subset of peers
        let selected = self.select_random_peers(self.fanout);
        
        // 3. Forward to selected peers
        for peer in selected {
            peer.send(message.clone());
        }
    }
    
    pub fn receive(&mut self, message: P2PMessage) {
        let msg_hash = hash_message(&message);
        
        // Check if already seen
        if self.message_cache.contains(&msg_hash) {
            return;  // Already propagated
        }
        
        // Process message
        self.process_message(&message);
        
        // Forward to others
        self.broadcast(message);
    }
}
```

### State Synchronization

```rust
pub struct StateSync {
    pub current_state_root: Hash,
    pub pending_updates: Vec<StateUpdate>,
}

pub struct StateUpdate {
    pub transaction_id: Hash,
    pub accounts_modified: Vec<Address>,
    pub new_balances: HashMap<Address, u64>,
    pub new_nonces: HashMap<Address, u64>,
    pub timestamp: u64,
    pub signatures: Vec<Signature>,  // BFT signatures
}

impl StateSync {
    pub fn broadcast_state_update(&self, update: StateUpdate) {
        // Broadcast to all nodes via gossip
        let message = P2PMessage {
            message_type: MessageType::StateUpdate,
            payload: serialize(&update),
            sender: self.node_id,
            signature: sign(&update),
            timestamp: current_time(),
        };
        
        self.gossip.broadcast(message);
    }
    
    pub fn apply_state_update(&mut self, update: StateUpdate) -> Result<()> {
        // 1. Verify BFT signatures (67% threshold)
        verify_bft_signatures(&update)?;
        
        // 2. Apply changes atomically
        for (address, balance) in update.new_balances {
            set_account_balance(address, balance);
        }
        
        for (address, nonce) in update.new_nonces {
            set_account_nonce(address, nonce);
        }
        
        // 3. Update state root
        self.current_state_root = calculate_new_state_root();
        
        Ok(())
    }
}
```

---

## Cryptographic Specifications

### Hash Function

**Algorithm**: SHA3-256 (Keccak)

```rust
use sha3::{Sha3_256, Digest};

pub fn hash(data: &[u8]) -> Hash {
    let mut hasher = Sha3_256::new();
    hasher.update(data);
    Hash::from(hasher.finalize())
}

pub fn hash_transaction(tx: &Transaction) -> Hash {
    hash(&serialize(tx))
}

pub fn hash_block_header(header: &BlockHeader) -> Hash {
    let mut hasher = Sha3_256::new();
    hasher.update(&header.block_number.to_le_bytes());
    hasher.update(&header.timestamp.to_le_bytes());
    hasher.update(&header.previous_hash);
    hasher.update(&header.transactions_root);
    hasher.update(&header.state_root);
    Hash::from(hasher.finalize())
}
```

### Digital Signatures

**Algorithm**: Ed25519 (EdDSA)

```rust
use ed25519_dalek::{Keypair, Signature, PublicKey, SecretKey};

pub fn generate_keypair() -> Keypair {
    let mut csprng = OsRng;
    Keypair::generate(&mut csprng)
}

pub fn sign(message: &[u8], keypair: &Keypair) -> Signature {
    keypair.sign(message)
}

pub fn verify(
    message: &[u8],
    signature: &Signature,
    public_key: &PublicKey
) -> bool {
    public_key.verify(message, signature).is_ok()
}
```

### Verifiable Random Function (VRF)

```rust
pub struct Vrf {
    seed: Hash,
    counter: u64,
}

impl Vrf {
    pub fn new(seed: Hash) -> Self {
        Self { seed, counter: 0 }
    }
    
    pub fn next_u64(&mut self) -> u64 {
        let mut hasher = Sha3_256::new();
        hasher.update(&self.seed);
        hasher.update(&self.counter.to_le_bytes());
        self.counter += 1;
        
        let hash = hasher.finalize();
        u64::from_le_bytes(hash[0..8].try_into().unwrap())
    }
    
    pub fn verify(&self, output: u64, index: u64) -> bool {
        // Reconstruct hash at specific index
        let mut hasher = Sha3_256::new();
        hasher.update(&self.seed);
        hasher.update(&index.to_le_bytes());
        let hash = hasher.finalize();
        let expected = u64::from_le_bytes(hash[0..8].try_into().unwrap());
        output == expected
    }
}
```

---

## State Management

### State Representation

```rust
pub struct GlobalState {
    pub accounts: HashMap<Address, AccountState>,
    pub masternodes: HashMap<NodeId, Masternode>,
    pub treasury: Treasury,
    pub governance: GovernanceState,
}

pub struct AccountState {
    pub address: Address,
    pub balance: u64,
    pub nonce: u64,
}

impl GlobalState {
    pub fn merkle_root(&self) -> Hash {
        // Build Merkle tree of all account states
        let mut leaves: Vec<Hash> = self.accounts
            .iter()
            .map(|(addr, state)| hash_account(addr, state))
            .collect();
        
        build_merkle_tree(&mut leaves)
    }
}
```

### State Transitions

```rust
pub fn apply_transaction(
    state: &mut GlobalState,
    tx: &Transaction
) -> Result<StateUpdate> {
    match tx {
        Transaction::Transfer(transfer) => {
            apply_transfer(state, transfer)
        },
        Transaction::Mint(mint) => {
            apply_mint(state, mint)
        },
        Transaction::Vote(vote) => {
            apply_vote(state, vote)
        },
    }
}

fn apply_transfer(
    state: &mut GlobalState,
    transfer: &Transfer
) -> Result<StateUpdate> {
    // Get accounts
    let sender = state.accounts.get_mut(&transfer.from)
        .ok_or("Sender not found")?;
    let recipient = state.accounts.entry(transfer.to)
        .or_insert(AccountState {
            address: transfer.to,
            balance: 0,
            nonce: 0,
        });
    
    // Validate
    if sender.balance < transfer.amount + transfer.fee {
        return Err("Insufficient balance");
    }
    if sender.nonce + 1 != transfer.nonce {
        return Err("Invalid nonce");
    }
    
    // Update sender
    sender.balance -= transfer.amount + transfer.fee;
    sender.nonce += 1;
    
    // Update recipient
    recipient.balance += transfer.amount;
    
    // Distribute fee
    state.treasury.operating_balance += (transfer.fee as f64 * 0.05) as u64;
    // Remaining 95% to masternode pool (distributed later)
    
    Ok(StateUpdate {
        accounts_modified: vec![transfer.from, transfer.to],
        new_balances: vec![
            (transfer.from, sender.balance),
            (transfer.to, recipient.balance),
        ].into_iter().collect(),
        new_nonces: vec![
            (transfer.from, sender.nonce),
        ].into_iter().collect(),
        timestamp: current_time(),
        signatures: vec![],  // Filled by BFT
    })
}
```

---

## API Specifications

### REST API Endpoints

```
GET /api/v1/block/{block_number}
GET /api/v1/transaction/{tx_hash}
GET /api/v1/account/{address}
GET /api/v1/masternode/{node_id}
GET /api/v1/proposal/{proposal_id}
GET /api/v1/treasury

POST /api/v1/transaction/broadcast
POST /api/v1/proposal/create
POST /api/v1/vote/cast
```

### RPC Interface

```json
// Get account balance
{
  "jsonrpc": "2.0",
  "method": "getAccountBalance",
  "params": ["TIME1a2B3c4D5e6F7g8H9i0J1k2L3m4N5o6P7"],
  "id": 1
}

// Response
{
  "jsonrpc": "2.0",
  "result": {
    "address": "TIME1a2B3c4D5e6F7g8H9i0J1k2L3m4N5o6P7",
    "balance": "1000000000000",
    "nonce": 42
  },
  "id": 1
}

// Broadcast transaction
{
  "jsonrpc": "2.0",
  "method": "broadcastTransaction",
  "params": [{
    "from": "TIME1...",
    "to": "TIME2...",
    "amount": "1000000000",
    "fee": "1000000",
    "nonce": 43,
    "signature": "..."
  }],
  "id": 2
}
```

### WebSocket Subscriptions

```javascript
// Subscribe to new transactions
ws.send(JSON.stringify({
  method: "subscribe",
  params: ["transactions"]
}));

// Subscribe to new blocks
ws.send(JSON.stringify({
  method: "subscribe",
  params: ["blocks"]
}));

// Subscribe to specific address
ws.send(JSON.stringify({
  method: "subscribe",
  params: ["address", "TIME1a2B3c4D5e6F7g8H9i0J1k2L3m4N5o6P7"]
}));
```

---

## Appendix: Constants & Parameters

### Core Constants

```rust
/// TIME unit (8 decimal places)
pub const TIME_UNIT: u64 = 100_000_000;

/// Block time (24 hours)
pub const BLOCK_TIME: u64 = 86400;

/// Blocks per year
pub const BLOCKS_PER_YEAR: u64 = 365;

/// BFT consensus threshold (67%)
pub const BFT_THRESHOLD: f64 = 0.67;

/// Block signature threshold (80%)
pub const BLOCK_SIGNATURE_THRESHOLD: f64 = 0.80;

/// Maximum transaction age (1 hour)
pub const MAX_TX_AGE: u64 = 3600;

/// State sync target time (500ms)
pub const STATE_SYNC_TIME: u64 = 500;
```

### Economic Parameters

```rust
/// Dynamic block reward base
pub const BLOCK_REWARD_BASE: u64 = 100 * TIME_UNIT;

/// Block reward scale factor
pub const BLOCK_REWARD_SCALE: f64 = 0.04;

/// Maximum block reward (cap)
pub const BLOCK_REWARD_MAX: u64 = 500 * TIME_UNIT;

/// Masternode reward share (95%)
pub const MASTERNODE_REWARD_SHARE: f64 = 0.95;

/// Treasury reward share (5%)
pub const TREASURY_REWARD_SHARE: f64 = 0.05;

/// Transaction fee split (95% MN, 5% Treasury)
pub const FEE_MASTERNODE_SHARE: f64 = 0.95;
pub const FEE_TREASURY_SHARE: f64 = 0.05;
```

### Masternode Parameters

```rust
/// Tier collateral amounts
pub const FREE_COLLATERAL: u64 = 0;
pub const BRONZE_COLLATERAL: u64 = 1_000 * TIME_UNIT;
pub const SILVER_COLLATERAL: u64 = 10_000 * TIME_UNIT;
pub const GOLD_COLLATERAL: u64 = 100_000 * TIME_UNIT;

/// Tier voting weights (Planned for weighted voting)
pub const FREE_WEIGHT: u64 = 1;
pub const BRONZE_WEIGHT: u64 = 1;
pub const SILVER_WEIGHT: u64 = 10;
pub const GOLD_WEIGHT: u64 = 100;

/// Longevity multiplier parameters
pub const LONGEVITY_MULTIPLIER_MAX: f64 = 3.0;
pub const LONGEVITY_MULTIPLIER_RATE: f64 = 0.5;  // per year
pub const LONGEVITY_RESET_THRESHOLD: u64 = 259200;  // 72 hours

/// Slashing percentages
pub const SLASH_DOUBLE_SIGNING: f64 = 0.5;  // 50%
pub const SLASH_ABANDONMENT: f64 = 0.15;  // 15%
pub const SLASH_DATA_WITHHOLDING: f64 = 0.25;  // 25%
pub const SLASH_NETWORK_ATTACK: f64 = 1.0;  // 100%
```

### Governance Parameters

```rust
/// Proposal deposit (standard)
pub const PROPOSAL_DEPOSIT: u64 = 100 * TIME_UNIT;

/// Emergency proposal deposit
pub const EMERGENCY_DEPOSIT: u64 = 500 * TIME_UNIT;

/// Discussion period (7 days)
pub const DISCUSSION_PERIOD: u64 = 7 * 86400;

/// Voting period (14 days)
pub const VOTING_PERIOD: u64 = 14 * 86400;

/// Approval threshold (60%)
pub const APPROVAL_THRESHOLD: f64 = 0.60;

/// Quorum threshold (60%)
pub const QUORUM_THRESHOLD: f64 = 0.60;

/// Emergency threshold (75%)
pub const EMERGENCY_THRESHOLD: f64 = 0.75;
```

### Network Parameters

```rust
/// Minimum quorum size
pub const MIN_QUORUM_SIZE: usize = 50;

/// Maximum quorum size
pub const MAX_QUORUM_SIZE: usize = 500;

/// Quorum scale factor
pub const QUORUM_SCALE_FACTOR: f64 = 50.0;

/// Gossip fanout (peers to forward to)
pub const GOSSIP_FANOUT: usize = 8;

/// Maximum peers per node
pub const MAX_PEERS: usize = 50;

/// Minimum peers per node
pub const MIN_PEERS: usize = 8;
```

---

## Conclusion

This technical specification provides a complete reference for implementing the TIME Coin protocol. All algorithms, data structures, and parameters are specified in sufficient detail for independent implementation.

**Key Technical Achievements:**
- ⚡ <3 second transaction finality via BFT
- 📊 Scales to 100,000+ masternodes via dynamic quorum
- 💾 99.99% less blockchain bloat (365 vs 2.6M blocks/year)
- 🔒 Multiple layers of security (nonce, BFT, state sync, daily blocks)
- 🏛️ Democratic governance with weighted voting
- 💰 Sustainable economics with capped inflation

For implementation questions or clarifications, please refer to the reference implementation at github.com/time-coin or contact the development team.

---

*Version 3.0 - October 2025*  
*For general overview, see Overview Whitepaper*  
*For security analysis, see Security Architecture Whitepaper*