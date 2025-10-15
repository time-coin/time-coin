# TIME Coin: A Community-Governed Cryptocurrency with Fair Launch Economics

**Version 1.3**
**October 2025**

---

## Abstract

TIME Coin introduces a revolutionary approach to cryptocurrency through purchase-based minting, community-governed treasury management, and a masternode network that provides both security and democratic governance. Unlike traditional cryptocurrencies with pre-mines or venture capital allocation, TIME Coin ensures fair distribution through direct purchase. With 24-hour blocks and instant transaction verification via a modified Byzantine Fault Tolerant (BFT) consensus mechanism with dynamic quorum selection, TIME Coin achieves immediate transaction finality (<3 seconds), long-term security, and unlimited scalability (100k+ masternodes) while maintaining decentralization. The three-tier masternode system with longevity multipliers and dynamic block rewards ensures sustainable operator incentives across all network growth phases, achieving 14-42% APY for new nodes scaling to 42-56% with transaction fees for 4+ year veterans.

---

## Table of Contents

1. [Introduction](#1-introduction)
2. [Problem Statement](#2-problem-statement)
3. [The TIME Coin Solution](#3-the-time-coin-solution)
4. [Technical Architecture](#4-technical-architecture)
5. [Economic Model](#5-economic-model)
6. [Treasury System](#6-treasury-system)
7. [Governance Framework](#7-governance-framework)
8. [Masternode Network](#8-masternode-network)
9. [Security & Consensus](#9-security--consensus)
10. [Accessibility Features](#10-accessibility-features)
11. [Tokenomics](#11-tokenomics)
12. [Roadmap](#12-roadmap)
13. [Conclusion](#13-conclusion)

---

## 1. Introduction

### 1.1 Vision

TIME Coin represents a paradigm shift in cryptocurrency design, emphasizing community ownership, fair distribution, and democratic governance. By eliminating pre-mines, initial coin offerings, and venture capital allocation, TIME Coin ensures that every participant starts on equal footing.

### 1.2 Core Principles

- **Fair Launch**: No pre-mine, no VCs, no insider allocation
- **Community Governance**: All major decisions voted on by masternode operators
- **Self-Funding**: Treasury receives 50% of fees + 5% of dynamic block rewards
- **Sustainable Economics**: Dynamic block rewards (100-500 TIME/day) maintain operator incentives while capping inflation
- **Instant Finality**: Transactions verified instantly via BFT consensus
- **Daily Settlement**: One block per day for efficient long-term security
- **Accessibility**: Multi-channel access (SMS, Email, Web, Mobile)
- **Transparency**: All treasury spending and governance on-chain
- **Long-Term Incentives**: Longevity multiplier rewards commitment (up to 3.0×)

### 1.3 Key Innovations

1. **Purchase-Based Minting**: Coins created only when purchased, ensuring organic growth
2. **24-Hour Block System**: One block per day combining efficiency with security
3. **BFT Instant Verification**: Modified Byzantine Fault Tolerant consensus for immediate transaction finality
4. **Dynamic Quorum Selection**: Scalable consensus that maintains <3 second finality even with 100k+ masternodes
5. **Three-Tier Weighted System**: Bronze (1k) → Silver (10k) → Gold (100k) with linear scaling
6. **Longevity Multiplier**: Rewards long-term commitment (1.0× to 3.0× over 4 years)
7. **Dynamic Block Rewards**: Scales with network size (100-500 TIME/day) to maintain operator incentives while capping inflation
8. **Self-Funding Ecosystem**: Treasury automatically funded without runaway inflation
9. **Milestone-Based Grants**: Transparent, auditable project funding

---

## 2. Problem Statement

### 2.1 Cryptocurrency Distribution Problems

**Pre-mines and ICOs:**

- Create massive wealth concentration
- Enable insider manipulation
- Often violate securities laws
- Provide exit liquidity for founders at retail expense

**Venture Capital Funding:**

- Prioritizes VC returns over community benefit
- Creates misaligned incentives
- Leads to centralized decision-making
- Often results in token unlocks that crash prices

**Mining Centralization:**

- ASIC manufacturers dominate
- Geographic concentration (cheap electricity)
- Environmental concerns
- Barrier to entry for regular users

### 2.2 Blockchain Speed vs. Security Trade-off

Most blockchains face a fundamental dilemma:

- **Fast blocks** (Solana, Avalanche): High throughput but security concerns, state bloat
- **Slow blocks** (Bitcoin): Very secure but painfully slow transactions
- **Middleground** (Ethereum): Still too slow for payments, high fees during congestion

### 2.3 Governance Challenges

Most cryptocurrencies lack effective governance:

- Bitcoin: Slow, contentious upgrades
- Ethereum: Foundation-driven centralization
- DeFi protocols: Whale domination, low participation

### 2.4 Accessibility Barriers

Existing cryptocurrencies require:

- Technical knowledge (wallets, keys, addresses)
- Smartphone or computer
- Internet connectivity
- Understanding of complex interfaces

---

## 3. The TIME Coin Solution

### 3.1 Fair Distribution Model

**Purchase-Based Minting:**

```
User Purchases $100 → Receives TIME equivalent
                    → Coins minted at purchase
                    → No pre-existing supply
```

**Benefits:**

- Price discovery from day one
- No insider allocation
- Organic, demand-driven growth
- Immediate liquidity

### 3.2 Revolutionary Architecture

**24-Hour Blocks + Instant Verification:**

```
Transaction Flow:
1. User submits transaction
2. Masternodes verify instantly (BFT consensus)
3. Transaction confirmed in <2 seconds
4. Included in next daily block
5. Permanent settlement every 24 hours
```

**Benefits:**

- Instant user experience (BFT verification)
- Efficient blockchain growth (1 block/day)
- Long-term security and scalability
- Low storage requirements

### 3.3 Community Governance

**Masternode Voting System:**

- 3 tiers based on collateral (1,000 - 100,000 TIME)
- Weighted voting power (1× - 100×)
- Longevity multiplier amplifies rewards (1.0× - 3.0×)
- Vote on treasury proposals, protocol changes, upgrades
- Participation incentives (5% reward bonus)

### 3.4 Self-Funding Treasury

**Automatic Funding:**

```
Transaction Fees → 50% Treasury, 50% Masternodes
Block Rewards → 5 TIME Treasury, 95 TIME Masternodes
```

**Uses:**

- Development grants
- Marketing initiatives
- Security audits
- Infrastructure improvements
- Research projects
- Community programs

---

## 4. Technical Architecture

### 4.1 Blockchain Design

**Core Specifications:**

```
Block Time:              24 hours (one block per day)
Transaction Finality:    <2 seconds (instant BFT verification)
Consensus:               Modified Byzantine Fault Tolerant (BFT)
Block Reward:            100 TIME per block (daily)
Treasury Allocation:     5 TIME per block
Masternode Allocation:   95 TIME per block
Transaction Fees:        Dynamic (market-based)
Max Transactions/Block:  Unlimited (practical limit: millions)
```

**Why 24-Hour Blocks?**

Traditional blockchains create blocks every few seconds or minutes, leading to:

- Massive blockchain size (hundreds of GB)
- State bloat
- High bandwidth requirements
- Synchronization challenges

TIME Coin's approach:

- **Instant verification** via BFT consensus (user experience)
- **Daily settlement** via blocks (efficiency and security)
- **Dynamic quorum selection** (scalability to 100k+ masternodes)
- **Best of all worlds**: Fast transactions + manageable blockchain + unlimited scale

**Architecture Overview:**

```
┌───────────────────────────────────────────────────────┐
│                   Application Layer                      │
│  (Web, Mobile, SMS, Email Interfaces)                   │
└──────────────────┬────────────────────────────────────┘
                   │
┌──────────────────▼────────────────────────────────────┐
│                      API Layer                           │
│        (REST API, RPC, WebSocket)                       │
└──────────────────┬────────────────────────────────────┘
                   │
┌──────────────────▼────────────────────────────────────┐
│                   Business Logic                         │
│  ┌─────────────┬──────────────┬───────────────────      │
│  │  Treasury   │  Governance  │    Economics    │      │
│  └─────────────┴──────────────┴───────────────────      │
└──────────────────┬────────────────────────────────────┘
                   │
┌──────────────────▼────────────────────────────────────┐
│           BFT Consensus Layer (Real-time)                │
│  Instant transaction verification by masternodes         │
│  Byzantine Fault Tolerant modified protocol              │
└──────────────────┬────────────────────────────────────┘
                   │
┌──────────────────▼────────────────────────────────────┐
│         Block Formation Layer (Daily)                    │
│  Aggregate verified transactions into daily block        │
│  Masternode signatures and finalization                  │
└──────────────────┬────────────────────────────────────┘
                   │
┌──────────────────▼────────────────────────────────────┐
│              Blockchain Storage Layer                    │
│         (Blocks, State, Transaction Index)              │
└──────────────────┬────────────────────────────────────┘
                   │
┌──────────────────▼────────────────────────────────────┐
│                 Network Layer (P2P)                      │
│              Masternode Network                          │
└───────────────────────────────────────────────────────┘
```

### 4.2 Block Structure

```rust
Block {
    block_number: u64,              // Day number since genesis
    timestamp: u64,                 // Unix timestamp (start of day)
    previous_hash: Hash,            // Previous block hash
    state_root: Hash,               // Merkle root of state
    transactions_root: Hash,        // Merkle root of all txs
    transaction_count: u64,         // Number of transactions in block
    transactions: Vec<Transaction>, // All verified transactions from the day
    masternode_signatures: Vec<Signature>, // BFT signatures
    total_fees: u64,               // Sum of all transaction fees
}
```

**Block Size:**

- Not limited by block time (24 hours gives ample room)
- Practical limit: Millions of transactions per block
- Average expected: 10,000 - 1,000,000 transactions/day initially

### 4.3 Transaction Types

**Transfer Transaction:**

```rust
Transfer {
    from: Address,
    to: Address,
    amount: u64,
    fee: u64,
    nonce: u64,
    timestamp: u64,          // When submitted
    bft_signatures: Vec<Signature>, // Instant verification
    signature: Signature,    // User's signature
}
```

**Mint Transaction (Purchase):**

```rust
Mint {
    recipient: Address,
    amount: u64,
    purchase_proof: PaymentProof,
    timestamp: u64,
    bft_signatures: Vec<Signature>,
}
```

**Treasury Transaction:**

```rust
TreasuryWithdrawal {
    proposal_id: String,
    milestone_id: String,
    recipient: Address,
    amount: u64,
    multisig_signatures: Vec<Signature>,
    bft_signatures: Vec<Signature>,
}
```

### 4.4 Modified BFT Consensus

**Byzantine Fault Tolerant Basics:**

- Consensus algorithm that tolerates up to 1/3 malicious nodes
- Used by Stellar, Ripple, and other instant-finality chains
- Requires validator communication and voting

**TIME Coin's Modifications:**

**1. Weighted Voting:**

```
Traditional BFT: One node = one vote
TIME Coin BFT: Voting power based on tier × longevity

Example with 3-tier system:
- 100 new Bronze nodes: 100 voting power (100 × 1 × 1.0)
- 10 new Silver nodes: 100 voting power (10 × 10 × 1.0)
- 1 new Gold node: 100 voting power (1 × 100 × 1.0)
- 1 veteran Gold (4yr): 300 voting power (1 × 100 × 3.0)
```

**2. Transaction Verification Process:**

```
Step 1: Transaction Broadcast
  User → Submits signed transaction → Network

Step 2: Initial Validation
  Masternodes → Check signature, balance, nonce → Valid?

Step 3: BFT Voting Round
  Masternodes → Vote to accept/reject → 67% threshold

Step 4: Instant Confirmation
  If 67% agree → Transaction confirmed instantly
  User sees confirmation in <2 seconds

Step 5: Daily Settlement
  Confirmed transactions → Included in next block
  Block created at midnight UTC
```

**3. Quorum Requirements:**

```
For transaction confirmation (Dynamic Quorum):
  - Quorum selection: Weighted random sampling
  - Quorum size: Logarithmic scaling with network size
    * 100 nodes → 50 node quorum
    * 1,000 nodes → 150 node quorum  
    * 10,000 nodes → 300 node quorum
    * 100,000 nodes → 500 node quorum
  - Voting threshold: 67% of selected quorum must agree
  - Selection rotates per transaction (VRF-based)
  
For block finalization (Full Network):
  - All active masternodes participate
  - Minimum 80% of total voting power must sign
  - At least 67% of active masternodes must participate
  - Happens once per day (more time = full participation feasible)
```

**4. Security Properties:**

```
Safety: Cannot confirm conflicting transactions
  - 67% threshold ensures overlap in any two quorums

Liveness: Always makes progress
  - As long as >67% of network honest and online

Finality: Immediate
  - No possibility of transaction reversal after confirmation
```

### 4.5 Daily Block Creation

**Block Formation Process:**

**Time: 00:00:00 UTC (midnight - day boundary)**

```
Day N ends, Day N+1 begins
All transactions from Day N are now complete (00:00:00 to 23:59:59)
```

**Time: 00:00:01 UTC (1 second after midnight)**

```
1. Block Proposer Selection
   - Random masternode selected (weighted by tier × longevity)
   - Uses verifiable random function (VRF)
   - Selected node begins building block for Day N

2. Transaction Aggregation
   - Proposer collects ALL verified transactions from Day N (00:00:00 to 23:59:59)
   - Orders by timestamp
   - Calculates merkle roots
   - Computes new state root

3. Block Proposal
   - Proposer broadcasts block candidate for Day N
   - Includes all metadata and signatures
   - Target completion: 00:03 UTC
```

**Time: 00:03 UTC (3 minutes for validation)**

```
4. Block Validation
   - All masternodes independently verify:
     * Transaction validity
     * State transitions
     * Fee calculations
     * Reward distribution
   - Masternodes have until 00:05 UTC to validate
```

**Time: 00:05 UTC (2 minutes for signing)**

```
5. Block Signing Round
   - Masternodes vote to accept/reject
   - Sign block if valid
   - 80% threshold required
   - Signing window: 00:05-00:07 UTC
```

**Time: 00:07 UTC (block finalization)**

```
6. Block Finalization
   - If 80% signed → Block for Day N confirmed
   - Block added to blockchain
   - State updated
   - Rewards distributed
   - Network continues processing Day N+1 transactions
```

**Failure Handling:**

```
If block not finalized by 00:10 UTC:
  - Emergency round triggered
  - New proposer selected
  - 5-minute emergency finalization window
  - Requires 90% approval (higher bar)
  - Must finalize by 00:15 UTC
```

**Key Design Benefits:**

```
1. Complete Day Coverage
   - Block for Day N includes ALL transactions from 00:00:00 to 23:59:59
   - No transactions missed or excluded
   - Clean day boundaries

2. Real-Time Processing Continues
   - While Day N block is being finalized, Day N+1 transactions are already being verified via BFT
   - Users experience no interruption
   - Instant confirmations continue 24/7

3. Predictable Schedule
   - Block finalization happens in first ~7 minutes of new day
   - Consistent, reliable timing
   - Easy to monitor and audit
```

---

## 5. Economic Model

### 5.1 Supply Dynamics

**No Fixed Supply:**

- Coins minted only when purchased
- Organic, demand-driven creation
- No inflation beyond purchases and daily rewards
- Penalties and failed proposal deposits return to treasury

**Minting Formula:**

```
Purchase $X USD → Mint (X / CURRENT_PRICE) TIME
CURRENT_PRICE = Market-determined price
```

### 5.2 Block Rewards

**Dynamic Block Reward System:**

TIME Coin implements a dynamic block reward that scales with network size to maintain operator incentives while capping long-term inflation.

**Formula:**
```
Total_Block_Reward = min(MAX_REWARD, BASE_REWARD + (Active_Masternodes × SCALE_FACTOR))

Where:
  BASE_REWARD = 100 TIME
  SCALE_FACTOR = 0.04 TIME per masternode
  MAX_REWARD = 500 TIME (inflation cap)

Distribution (always 95%/5% split):
  Masternodes: 95% of total
  Treasury: 5% of total
```

**Reward Scaling Examples:**

| Active Nodes | Total Daily Reward | Masternode Pool | Treasury Pool | Annual Total |
|--------------|-------------------|-----------------|---------------|--------------|
| 100 | 104 TIME | 98.8 TIME | 5.2 TIME | 37,960 TIME |
| 500 | 120 TIME | 114 TIME | 6 TIME | 43,800 TIME |
| 1,000 | 140 TIME | 133 TIME | 7 TIME | 51,100 TIME |
| 2,500 | 200 TIME | 190 TIME | 10 TIME | 73,000 TIME |
| 5,000 | 300 TIME | 285 TIME | 15 TIME | 109,500 TIME |
| 10,000 | 500 TIME (capped) | 475 TIME | 25 TIME | 182,500 TIME |
| 20,000 | 500 TIME (capped) | 475 TIME | 25 TIME | 182,500 TIME |

**Key Benefits:**

```
1. Maintains Operator Incentives
   - Per-node rewards decrease slower than network growth
   - Prevents node exodus as network scales
   - Sustainable long-term economics

2. Capped Inflation
   - Maximum 500 TIME/day = 182,500 TIME/year
   - Predictable long-term supply growth
   - Much lower than major cryptocurrencies

3. Self-Balancing
   - If rewards too low → nodes exit → rewards increase
   - If rewards too high → nodes join → rewards normalize
   - Natural equilibrium achieved

4. Transaction Fee Complement
   - By time cap is reached (10k+ nodes), fees are substantial
   - Primary income shifts from rewards to fees
   - Sustainable fee-based economy at maturity
```

**Comparison with Other Cryptocurrencies:**

```
Bitcoin: ~900 BTC/day (currently) = ~$40M+/day at scale
Ethereum: ~1,700 ETH/day (post-merge) = ~$3M+/day at scale
TIME Coin: 104-500 TIME/day (capped) = Sustainable at any price

Advantage: Much lower inflation, scales with network needs
```

**Long-Term Inflation Model:**

```
Year 1 (Avg 200 nodes): ~120 TIME/day → 43,800 TIME/year
Year 3 (Avg 1,000 nodes): ~140 TIME/day → 51,100 TIME/year
Year 5 (Avg 5,000 nodes): ~300 TIME/day → 109,500 TIME/year
Year 10 (Avg 10,000+ nodes): 500 TIME/day → 182,500 TIME/year (capped)

Total 10-year inflation from rewards: ~1,000,000 TIME
Plus purchase-based minting (demand-driven)
```

**Why This Works:**

```
Early Network (100-1,000 nodes):
  - Higher per-node rewards attract operators
  - Low total inflation (43k-51k/year)
  - Bootstraps security

Growing Network (1,000-10,000 nodes):
  - Rewards scale with network
  - Maintains viability for operators
  - Transaction fees start contributing significantly

Mature Network (10,000+ nodes):
  - Reward cap reached (500/day)
  - Transaction fees become primary income
  - Fee-driven economy (like Bitcoin post-halving)
  - Sustainable long-term model
```

### 5.3 Fee Structure

**Transaction Fees:**

```
Base Fee: 0.001 TIME (100,000 satoshis)
Priority Fee: User-defined (optional)

Fee Distribution (per block):
  Treasury: 50% of total daily fees
  Masternodes: 50% of total daily fees
```

**Dynamic Fees:**

- Adjust based on network congestion
- Market-based pricing
- Prevents spam
- Incentivizes validators

**Example Fee Scenarios:**

```
Low Activity Day (1,000 transactions):
  Total fees: 1 TIME
  Treasury: 0.5 TIME
  Masternodes: 0.5 TIME

Medium Activity (100,000 transactions):
  Total fees: 100 TIME
  Treasury: 50 TIME
  Masternodes: 50 TIME

High Activity (1,000,000 transactions):
  Total fees: 1,000 TIME
  Treasury: 500 TIME
  Masternodes: 500 TIME
```

### 5.4 Masternode Economics

**Daily Reward Distribution:**

```
Masternode Pool: 95% of dynamic block reward
Treasury Pool: 5% of dynamic block reward

Distribution Formula:
  Node Reward = (Node Total Weight / Total Network Weight) × Daily Masternode Pool
  
Where:
  Node Total Weight = Tier Weight × Longevity Multiplier
  Daily Masternode Pool = 95% of total block reward (scales with network size)
```

**Three-Tier System with Longevity:**

| Tier | Collateral | Base Weight | Longevity Range | Weight Range |
|------|-----------|-------------|-----------------|--------------|
| Bronze | 1,000 TIME | 1× | 1.0× - 3.0× | 1 - 3 |
| Silver | 10,000 TIME | 10× | 1.0× - 3.0× | 10 - 30 |
| Gold | 100,000 TIME | 100× | 1.0× - 3.0× | 100 - 300 |

**Economic Balancing:**

The dynamic block reward system ensures sustainable APY across network growth phases:

```
Small Network (100-500 nodes):
  - Higher per-node rewards (more TIME to distribute)
  - Attracts early operators
  - Builds initial security
  - Example: 100 nodes → 98.8 TIME/day pool

Medium Network (1,000-5,000 nodes):
  - Moderate per-node rewards
  - Transaction fees starting to contribute
  - Healthy competition
  - Example: 1,000 nodes → 133 TIME/day pool

Large Network (10,000+ nodes):
  - Reward cap reached (475 TIME/day pool)
  - Transaction fees are primary income
  - Mature fee-based economy
  - Example: 10,000 nodes → 475 TIME/day pool + substantial fees
```

**Target ROI Examples (Various Network Sizes):**

*Network: 1,000 nodes (140 TIME total daily reward, 133 TIME to masternodes)*

| Tier | Age | Daily Reward | Annual Reward | APY |
|------|-----|--------------|---------------|-----|
| **Bronze (1,000 TIME)** | | | | |
| | New | ~0.54 TIME | ~197 TIME | ~20% |
| | 1 year | ~0.81 TIME | ~296 TIME | ~30% |
| | 2 years | ~1.08 TIME | ~394 TIME | ~39% |
| | 4+ years | ~1.63 TIME | ~595 TIME | ~60% |
| **Silver (10,000 TIME)** | | | | |
| | New | ~5.43 TIME | ~1,982 TIME | ~20% |
| | 1 year | ~8.15 TIME | ~2,975 TIME | ~30% |
| | 2 years | ~10.87 TIME | ~3,968 TIME | ~40% |
| | 4+ years | ~16.30 TIME | ~5,950 TIME | ~60% |
| **Gold (100,000 TIME)** | | | | |
| | New | ~54.3 TIME | ~19,820 TIME | ~20% |
| | 1 year | ~81.5 TIME | ~29,748 TIME | ~30% |
| | 2 years | ~108.7 TIME | ~39,681 TIME | ~40% |
| | 4+ years | ~163.0 TIME | ~59,495 TIME | ~60% |

*Network: 10,000 nodes (500 TIME capped, 475 TIME to masternodes)*

| Tier | Age | Daily Reward | Annual Reward | APY |
|------|-----|--------------|---------------|-----|
| **Bronze (1,000 TIME)** | | | | |
| | New | ~0.19 TIME | ~69 TIME | ~7% |
| | 1 year | ~0.29 TIME | ~106 TIME | ~11% |
| | 2 years | ~0.39 TIME | ~142 TIME | ~14% |
| | 4+ years | ~0.58 TIME | ~212 TIME | ~21% |

*Plus substantial transaction fees at this scale (expect 2-3× from fees)*

**ROI Summary:**

| Network Phase | New Nodes | Active (1-2yr) | Veterans (4+yr) | Fee Multiplier |
|---------------|-----------|----------------|-----------------|----------------|
| Early (100-500) | 18-25% | 30-40% | 50-65% | Low (1.1×) |
| Growth (1k-5k) | 15-20% | 25-35% | 40-55% | Medium (1.5×) |
| Mature (10k+) | 7-10% | 11-14% | 21-28% | High (2-3×) |
| **With Fees** | **14-20%** | **25-35%** | **42-56%** | - |

*At maturity, transaction fees become the primary income source, maintaining target APY range.*

**Additional Revenue: Transaction Fees**

- 50% of all transaction fees distributed to masternodes
- Proportional to total weight
- Scales with network adoption

**Example at 100,000 daily transactions (0.001 TIME avg fee):**
- Total fees: 100 TIME
- Masternode share: 50 TIME distributed proportionally

**Fee Impact:**
- Adds 2-10% additional APY depending on network activity and node weight
- High network adoption significantly boosts returns
- Transaction fees grow with TIME Coin usage
- Provides additional revenue beyond base block rewards

**Example Scenarios:**
- 10k daily tx: Adds ~1-2% APY across all tiers
- 100k daily tx: Adds ~5-7% APY across all tiers  
- 1M daily tx: Adds ~20-30% APY across all tiers

**Expected Equilibrium:**

- Target APY range: 14-42% based on longevity
  - New nodes: ~14% APY
  - Active nodes (6mo-2yr): 18-30% APY  
  - Veteran nodes (4+yr): up to 42% APY
- Sustainable long-term
- Adjusted by market forces (more nodes = lower rewards per node)
- Fee revenue increases with adoption
- Veterans earn premium returns rewarding commitment

---

## 6. Treasury System

### 6.1 Funding Sources

**Automatic Funding:**

```
1. Dynamic Block Rewards:
   5% of total block reward (scales with network)
   
   Network Size → Daily Treasury → Annual Treasury
   100 nodes: 5.2 TIME/day → 1,898 TIME/year
   1,000 nodes: 7 TIME/day → 2,555 TIME/year
   5,000 nodes: 15 TIME/day → 5,475 TIME/year
   10,000+ nodes: 25 TIME/day → 9,125 TIME/year (capped)

2. Transaction Fees:
   50% of all fees → Treasury

   Example scaling:
   Year 1 (10k avg daily tx): ~1,825 TIME/year from fees
   Year 3 (100k avg daily tx): ~18,250 TIME/year from fees
   Year 5 (1M avg daily tx): ~182,500 TIME/year from fees

3. Additional Sources:
   - Community donations
   - Recovered funds (failed proposals)
   - Slashing penalties (malicious behavior)
   - Failed proposal deposits (<40% approval)
```

**Projected Treasury Growth:**

```
Year 1 (Conservative - avg 200 nodes):
  Block rewards: ~2,000 TIME (5.2-6 TIME/day avg)
  Transaction fees: ~1,825 TIME (10k daily tx)
  Total: ~3,825 TIME

Year 2 (Growth - avg 600 nodes):
  Block rewards: ~2,300 TIME (~6.3 TIME/day avg)
  Transaction fees: ~3,650 TIME (20k daily tx)
  Total: ~5,950 TIME/year (cumulative: ~9,775 TIME)

Year 5 (Mature - avg 5,000 nodes):
  Block rewards: ~5,475 TIME (15 TIME/day)
  Transaction fees: ~182,500 TIME (1M daily tx)
  Total: ~187,975 TIME/year

Year 10 (Large Scale - 10,000+ nodes):
  Block rewards: ~9,125 TIME (25 TIME/day capped)
  Transaction fees: ~500,000+ TIME (multi-million daily tx)
  Total: ~509,125+ TIME/year
```

### 6.2 Treasury Management

**Key Features:**

- On-chain transparency
- Multi-signature for large withdrawals (>10,000 TIME)
- Milestone-based payments
- Automatic financial reports
- Public dashboard

**Security Measures:**

```
Withdrawals > 10,000 TIME require:
  - 3 of 5 multi-sig approvals
  - Proposal approval (60%+ votes)
  - 30-day grace period
  - Milestone completion proof
```

### 6.3 Spending Categories

**Development Grants (40%):**

- Core protocol development
- Wallet implementations
- API development
- Infrastructure tools

**Marketing & Growth (25%):**

- Exchange listings
- Marketing campaigns
- Community building
- Educational content

**Security & Audits (20%):**

- Code audits
- Penetration testing
- Bug bounties
- Security research

**Infrastructure (10%):**

- Seed nodes
- Archive nodes
- RPC endpoints
- Monitoring systems

**Community Programs (5%):**

- Grants for community projects
- Educational initiatives
- Ecosystem development
- Research funding

### 6.4 Financial Controls

**Monthly Limits:**

- Maximum 20% of treasury balance per month
- 10% contingency buffer always maintained
- Emergency reserve: 5% of balance

**Reporting:**

- Daily balance updates (automatic)
- Monthly financial statements
- Quarterly audit reports
- Real-time dashboard
- Annual comprehensive review

---

## 7. Governance Framework

### 7.1 Proposal System

**Proposal Types:**

```
1. Development Grants
2. Marketing Initiatives
3. Security Audits
4. Infrastructure Improvements
5. Research Projects
6. Community Programs
7. Emergency Actions
8. Protocol Parameter Changes
```

**Submission Requirements:**

```
Standard Proposals:
  - Deposit: 100 TIME (returned if approved)
  - Discussion period: 7 days
  - Voting period: 14 days
  - Required approval: 60%
  - Required quorum: 60%

Emergency Proposals:
  - Deposit: 500 TIME
  - Discussion period: 2 days
  - Voting period: 5 days
  - Required approval: 75%
  - Higher deposit = skin in the game
```

### 7.2 Voting Mechanism

**Masternode Voting Power:**

| Tier | Collateral | Base Voting Power | With Longevity |
|------|-----------|------------------|----------------|
| Bronze | 1,000 TIME | 1× | 1× - 3× |
| Silver | 10,000 TIME | 10× | 10× - 30× |
| Gold | 100,000 TIME | 100× | 100× - 300× |

**Voting Process:**

```
1. Proposal Submission
   ↓
2. Discussion Period (7 days)
   - Community feedback
   - Clarifications
   - Amendments
   ↓
3. Voting Period (14 days)
   - Masternodes cast votes
   - Yes / No / Abstain
   ↓
4. Vote Counting (included in daily block)
   - Calculate weighted votes
   - Check approval threshold (60%)
   - Check quorum threshold (60%)
   ↓
5. Result
   - Approved → 30-day grace period → Execution
   - Rejected → Deposit returned to submitter
```

**Vote Choices:**

- **Yes**: Approve the proposal
- **No**: Reject the proposal
- **Abstain**: Counted for quorum, not for approval

### 7.3 Governance Incentives

**Participation Rewards:**

```
Active Voting Bonus: +5% masternode rewards
  - Must vote on >80% of proposals
  - Encourages participation
  - Applied to daily rewards

Proposal Bounty: 1,000 TIME
  - For approved proposals
  - Rewards quality submissions
  - Paid after successful completion
```

**Accountability:**

```
Failed Proposals:
  - Deposit returned if >40% approval
  - Deposit sent to treasury if <40% approval (spam prevention)

Milestone Failures:
  - Funding paused
  - Investigation period
  - Possible recovery action
```

### 7.4 Protocol Governance

**Adjustable Parameters:**

```
Via Governance Vote:
  - Transaction fee rates
  - Masternode collateral requirements
  - Reward distribution ratios
  - BFT consensus thresholds
  - Slashing penalties
  - Treasury spending limits
  - Longevity multiplier formula
```

**Upgrade Mechanism:**

```
Soft Forks:
  - 80% masternode approval
  - Backward compatible changes
  - 30-day activation period

Hard Forks:
  - 90% masternode approval
  - Breaking changes
  - 90-day preparation period
  - Multiple block validations

Emergency Upgrades:
  - 75% approval + 3/5 emergency committee
  - Critical security issues only
  - 7-day ratification
  - Post-upgrade review required
```

---

## 8. Masternode Network

### 8.1 Masternode Tiers

**Three-Tier Weighted System:**

**Bronze Tier:**
- Collateral: 1,000 TIME
- Tier Weight: 1×
- Voting Power: 1×
- Entry-level masternode tier
- Target ROI: 14% (new) to 42% (4+ years) APY

**Silver Tier:**
- Collateral: 10,000 TIME (10× Bronze)
- Tier Weight: 10×
- Voting Power: 10×
- Mid-tier commitment
- Target ROI: 14% (new) to 42% (4+ years) APY

**Gold Tier:**
- Collateral: 100,000 TIME (100× Bronze)
- Tier Weight: 100×
- Voting Power: 100×
- Maximum tier and influence
- Target ROI: 14% (new) to 42% (4+ years) APY

**Design Philosophy:**
- Simple, clear tier structure
- Linear scaling (1×, 10×, 100×)
- Accessible entry point (1,000 TIME)
- Combined with longevity multiplier for fair long-term rewards

### 8.2 Longevity Multiplier System

**Purpose:**
The longevity multiplier rewards masternode operators for long-term commitment and network stability.

**Formula:**
```
Longevity Multiplier = 1 + (Days_Active ÷ 365) × 0.5
Maximum: 3.0× (after 4+ years)
```

**Multiplier Schedule:**

| Duration | Days Active | Multiplier | Bonus | Example: Bronze Daily |
|----------|-------------|-----------|-------|---------------------|
| New Node | 0-30 | 1.0× | 0% | 0.388 TIME |
| 6 Months | ~180 | 1.25× | +25% | 0.485 TIME |
| 1 Year | 365 | 1.5× | +50% | 0.582 TIME |
| 2 Years | 730 | 2.0× | +100% | 0.776 TIME |
| 3 Years | 1,095 | 2.5× | +150% | 0.970 TIME |
| 4+ Years | 1,460+ | 3.0× | +200% | 1.164 TIME |

**Key Characteristics:**

1. **Continuous Growth:** Multiplier increases smoothly with each day of operation
2. **Fair Start:** All new nodes begin at 1.0× regardless of tier
3. **Maximum Cap:** Caps at 3.0× to prevent excessive concentration
4. **Reset Mechanism:** >72 hours of downtime resets multiplier to 1.0×
5. **Transparent:** All calculations on-chain and publicly verifiable

**Total Weight Calculation:**
```
Total Weight = Tier Weight × Longevity Multiplier

Examples:
- New Bronze: 1 × 1.0 = 1 total weight
- Veteran Bronze (4yr): 1 × 3.0 = 3 total weight
- New Gold: 100 × 1.0 = 100 total weight
- Veteran Gold (4yr): 100 × 3.0 = 300 total weight
```

**Impact on Network:**
- Veteran Gold node (4yr) = equivalent to 300 new Bronze nodes
- Encourages long-term participation and network stability
- New nodes remain competitive with meaningful rewards (14% APY)
- Active nodes earn in target range (18-30% APY for 6mo-2yr operators)
- Veterans earn premium returns (up to 42% APY for 4+ years)

**Reset Conditions:**

⚠️ **Downtime Penalty:**
- **>72 hours offline:** Longevity multiplier resets to 1.0×
- Must rebuild time commitment from scratch
- Encourages reliable operation and uptime

**Strategic Implications:**
- Tier determines base power
- Longevity amplifies that power over time
- Both small long-term operators and large new operators can be competitive
- Balanced system that rewards both capital and commitment

### 8.3 Masternode Functions

**BFT Consensus Participation:**

- Verify transactions in real-time
- Participate in BFT voting rounds
- Sign transaction confirmations
- Achieve 67% quorum for finality

**Daily Block Creation:**

- Propose blocks (when selected)
- Validate proposed blocks
- Sign finalized blocks
- Distribute rewards

**Governance:**

- Vote on proposals
- Protocol parameter changes
- Treasury fund allocation
- Network upgrades

**Network Services:**

- Transaction relay
- Block propagation
- State synchronization
- API endpoints

**Security:**

- Double-spend prevention
- Network monitoring
- Attack mitigation
- Maintain consensus

### 8.4 Rewards Distribution

**Daily Reward Calculation:**

```
Base Pool: 95 TIME per day

Distribution Formula:
  Node Reward = (Node Total Weight / Total Network Weight) × 95 TIME × Bonus

Where:
  Total Weight = Tier Weight × Longevity Multiplier

Bonus Multipliers:
  - Active voting (>80% participation): 1.05×
  - Uptime >99.5%: 1.0× (standard)
  - Uptime <95%: 0.9× (penalty)
```

**Transaction Fee Rewards:**

```
Daily Fee Pool: 50% of all transaction fees

Distribution:
  Same proportional distribution as block rewards

Example:
  If 100,000 transactions @ 0.001 TIME fee:
  Total fees: 100 TIME
  Masternode pool: 50 TIME

  Each masternode gets proportional share based on total weight
  New Bronze (1/245 power): 0.204 TIME
  Gold 4yr (300/245 power): 61.2 TIME
```

### 8.5 Setup Requirements

**Hardware:**

```
Minimum:
  - 2 CPU cores
  - 4 GB RAM
  - 50 GB SSD (1 block/day = very light!)
  - 50 Mbps internet

Recommended:
  - 4 CPU cores
  - 8 GB RAM
  - 100 GB SSD
  - 100 Mbps internet
```

**Software:**

```
Operating System:
  - Linux (Ubuntu 22.04+ recommended)
  - Docker support

Dependencies:
  - TIME node software
  - Wallet with collateral
  - Automated monitoring
```

**Network:**

```
Requirements:
  - Static IP or dynamic DNS
  - Open ports (8080, 8081)
  - 99%+ uptime recommended
  - Low latency preferred (<100ms to peers)
```

### 8.6 Slashing & Penalties

**Offline Penalties:**

```
Downtime > 2 hours:
  - Warning issued
  - Excluded from BFT consensus
  - Miss rewards during downtime

Downtime > 24 hours:
  - Masternode deactivated
  - Miss rewards until reactivated
  - No collateral slashing (first occurrence)

Downtime > 72 hours:
  - Longevity multiplier reset to 1.0×
  - Must rebuild time commitment
  - Significant impact on long-term rewards

Repeated Issues (3+ times/month):
  - 1% collateral penalty → Treasury
  - Must resolve issues
  - Continued issues may result in higher penalties
```

**Malicious Behavior:**

```
Double-signing in BFT:
  - 10% collateral slashed → Treasury
  - Masternode banned for 30 days
  - Longevity reset

Invalid block proposals:
  - 5% collateral slashed → Treasury
  - Temporary suspension
  - Longevity reset

Governance manipulation:
  - 5% collateral slashed → Treasury
  - Voting rights suspended
  - Longevity reset

Network attacks:
  - 100% collateral slashed → Treasury
  - Permanent ban
```

---

## 9. Security & Consensus

### 9.1 Modified BFT Consensus

**Core BFT Properties:**

```
Byzantine Fault Tolerance:
  - Tolerates up to 33% malicious nodes
  - Provides instant finality
  - No possibility of forks
  - Deterministic outcomes

TIME Coin Modifications:
  - Weighted voting by tier × longevity
  - Tiered quorum requirements
  - Integration with daily blocks
  - Economic security (collateral)
```

**Transaction Verification:**

```
Phase 1: Proposal
  User broadcasts signed transaction

Phase 2: Pre-validation
  Nodes check: signature, balance, nonce
  Invalid transactions immediately rejected

Phase 3: BFT Voting
  Valid transactions enter voting round
  Nodes vote based on their total weight
  67% weighted agreement required

Phase 4: Confirmation
  Transaction immediately confirmed
  Irreversible finality achieved
  User notified (typically <2 seconds)

Phase 5: Block Inclusion
  Confirmed transactions batched
  Included in next daily block
  Permanent on-chain record
```

**Block Finalization:**

```
Phase 1: Day Boundary (00:00:00 UTC)
  Previous day ends (Day N: 00:00:00 to 23:59:59)
  New day begins (Day N+1)
  All Day N transactions complete

Phase 2: Aggregation (00:00:01 UTC)
  Collect all confirmed transactions from Day N
  Build block structure
  Calculate merkle roots

Phase 3: Proposal (00:03 UTC)
  Selected node proposes block for Day N
  Broadcast to all masternodes

Phase 4: Validation (00:03-00:05 UTC)
  Each node independently validates
  Check all state transitions
  Verify reward calculations

Phase 5: Signing (00:05-00:07 UTC)
  Nodes vote to accept/reject
  Sign if valid
  80% threshold required

Phase 6: Finalization (00:07 UTC)
  Block for Day N permanently added
  State updated
  Rewards distributed
  Day N+1 continues (already processing new transactions)
```

### 9.2 Scalability & Performance

**Challenge: BFT Consensus at Scale**

Traditional BFT systems face communication overhead that grows with network size:
- More nodes = more messages
- Communication complexity: O(n²) in basic implementations
- Risk: Slower finality as network grows

**TIME Coin's Solution: Dynamic Quorum Selection**

```
Key Innovation: Not all masternodes validate every transaction

Instead:
  - Random weighted subset selected per transaction
  - Quorum size grows logarithmically (not linearly)
  - Maintains <3 second finality even at 100k+ masternodes
```

**Quorum Scaling Model:**

| Network Size | Quorum Size | % of Network | Target Finality |
|--------------|-------------|--------------|-----------------|
| 100 | 50 | 50% | <1 second |
| 500 | 100 | 20% | <2 seconds |
| 1,000 | 150 | 15% | <2 seconds |
| 5,000 | 250 | 5% | <2 seconds |
| 10,000 | 300 | 3% | <2 seconds |
| 50,000 | 450 | 0.9% | <3 seconds |
| 100,000 | 500 | 0.5% | <3 seconds |

**Selection Algorithm:**

```rust
fn select_quorum(transaction: Transaction) -> Vec<Masternode> {
    // 1. Calculate quorum size
    let total_nodes = active_masternodes.len();
    let quorum_size = calculate_quorum_size(total_nodes); // Logarithmic
    
    // 2. Use VRF for randomness
    let seed = hash(transaction.id + previous_block.hash);
    let vrf = VerifiableRandomFunction::new(seed);
    
    // 3. Weighted selection (tier × longevity)
    let selected = weighted_sample(
        masternodes,
        weights: tier_weight * longevity_multiplier,
        count: quorum_size,
        rng: vrf
    );
    
    return selected;
}

fn calculate_quorum_size(n: usize) -> usize {
    // Logarithmic scaling: log₂(n) × 50
    // Minimum: 50 nodes
    // Maximum: 500 nodes (caps at 100k network)
    max(50, min(500, (n as f64).log2() * 50.0) as usize)
}
```

**Security Properties:**

```
1. Probabilistic Security
   - Attacker needs 67% of QUORUM (not total network)
   - BUT: Quorum randomly selected each time
   - Expected attack success if attacker has 40% total weight:
     * With 150 node quorum: <0.001% chance
     * With 300 node quorum: <0.0001% chance

2. Weighted Selection
   - Veteran high-tier nodes selected more often
   - Difficult for attacker to game selection
   - VRF ensures verifiable randomness

3. Full Network for Blocks
   - Daily blocks require full network (80% threshold)
   - Cannot manipulate block creation
   - Ultimate security checkpoint
```

**Performance Benefits:**

```
1. Constant Time Complexity
   - Transaction confirmation: O(log n) not O(n)
   - Network growth doesn't significantly impact speed
   - 100× more nodes = only ~2× quorum size

2. Parallel Processing
   - Different quorums for simultaneous transactions
   - High throughput: 1000+ TPS possible
   - No bottleneck from consensus

3. Network Efficiency
   - Less bandwidth per node
   - Lower message overhead
   - Geographic optimization possible
```

**Why This Works:**

```
Transaction Verification (Every 2 seconds):
  - High frequency = use quorum
  - Acceptable risk (daily block validates all)
  - Speed critical for user experience

Block Finalization (Once per day):
  - Low frequency = use full network
  - 10 minutes available for consensus
  - Security maximized
  - Validates all transactions from quorums
```

**Comparison:**

```
Traditional BFT (Tendermint, PBFT):
  - All validators for every transaction
  - 100 nodes: ~100 messages per transaction
  - 10,000 nodes: ~10,000 messages per transaction
  - Doesn't scale well

TIME Coin BFT:
  - Quorum for each transaction
  - 100 nodes: ~50 messages per transaction
  - 10,000 nodes: ~300 messages per transaction
  - Scales to 100k+ nodes
```

### 9.3 Attack Vectors & Mitigations

**51% Attack (or 67% in BFT):**

```
Traditional PoW: Requires 51% hash power
TIME Coin BFT Quorum: Requires 67% of randomly selected quorum

Attack Scenarios:

Scenario 1: Attack Transaction Confirmation
  - Need to control 67% of a specific quorum
  - Quorum randomly selected per transaction
  - Even with 40% of total network weight:
    * Probability of controlling 67% of 150-node quorum: <0.001%
    * Would need to attack thousands of transactions to succeed once
  
Scenario 2: Attack Block Finalization  
  - Need 80% of TOTAL network voting power
  - Much harder - requires majority of entire network
  - If 10M TIME locked: Need 8M+ TIME at stake
  - At $1/TIME: $8M+ attack cost

Cost to Attack:
  - Must acquire massive stake (40%+ for quorum attacks, 80%+ for blocks)
  - Longevity requirement: Can't just buy and attack immediately
  - Slashing risk: Lose all collateral if caught
  - Economic disincentive: Attacking devalues your own holdings

Mitigation:
  - Expensive to acquire supermajority
  - Random quorum selection makes targeted attacks impractical
  - Longevity requirement increases attack cost over time
  - Daily block validation catches any quorum attacks
  - Slashing punishes malicious behavior
  - Community can hard-fork if needed
```

**Long-Range Attack:**

```
Problem: Attacker creates alternate chain from old state

Mitigation:
  - Daily blocks with BFT signatures
  - Cannot rewrite past BFT consensus
  - New nodes sync from recent state
  - Checkpointing via daily blocks
  - Invalid chains detected and rejected
```

**Double-Spend Attack:**

```
Problem: Send same coins twice

Mitigation:
  - Instant finality via BFT (2 seconds)
  - 67% agreement required
  - Conflicting transactions rejected
  - Nonce prevents replay
  - Economic security (collateral at stake)
```

**Sybil Attack:**

```
Problem: Attacker creates many fake identities

Mitigation:
  - Collateral requirement (1,000+ TIME)
  - Economic barrier to entry
  - Identity tied to collateral
  - Slashing for malicious behavior
  - Voting power proportional to stake × longevity
  - Longevity requirement makes Sybil harder
```

**DDoS Attack:**

```
Problem: Flood network with requests

Mitigation:
  - Rate limiting on all endpoints
  - Transaction fees prevent spam
  - Multiple RPC endpoints
  - Geographic distribution of nodes
  - Automatic ban for abusive IPs
  - BFT continues with 67% active nodes
```

**Nothing-at-Stake:**

```
Problem: In PoS, validators can vote on multiple chains

Mitigation:
  - BFT provides deterministic finality
  - No chain selection, only one valid chain
  - Slashing for double-signing
  - Economic security via locked collateral
  - Longevity reset penalty
```

### 9.4 Cryptography

**Hash Function:**

```
Algorithm: SHA3-256 (Keccak)
  - NIST standard
  - Quantum-resistant candidate
  - Fast and secure
  - 256-bit output
```

**Signature Scheme:**

```
Algorithm: Ed25519 (EdDSA)
  - Fast verification
  - Small signatures (64 bytes)
  - Battle-tested
  - Widely supported
  - Deterministic
```

**Address Format:**

```
Format: Base58Check encoding
Example: TIME1a2B3c4D5e6F7g8H9i0J1k2L3m4N5o6P7

Structure:
  - Version byte (0x4D = 'TIME')
  - Public key hash (20 bytes)
  - Checksum (4 bytes)
```

**Merkle Trees:**

```
Usage:
  - Transaction sets in blocks
  - State representation
  - Efficient verification
  - Light client support
```

### 9.5 Network Security

**Peer Authentication:**

```
- TLS 1.3 for all connections
- Certificate pinning for known nodes
- Mutual authentication
- Regular key rotation
```

**Data Integrity:**

```
- Merkle trees for transaction sets
- State roots for global state
- Cryptographic hashes throughout
- Digital signatures on all actions
```

**Privacy Considerations:**

```
Current: Pseudonymous (like Bitcoin)
  - Addresses not directly linked to identity
  - Transaction graph analysis possible

Future Enhancements:
  - Optional privacy features
  - Confidential transactions
  - Ring signatures
  - Zero-knowledge proofs
```

---

## 10. Accessibility Features

### 10.1 Multi-Channel Access

**Web Interface:**

```
Features:
  - Browser-based wallet
  - No download required
  - Mobile-responsive
  - Progressive Web App (PWA)

Security:
  - Client-side key generation
  - Encrypted local storage
  - Hardware wallet support
  - 2FA optional
```

**Mobile Applications:**

```
Platforms: iOS, Android

Features:
  - Native app experience
  - Biometric authentication
  - Push notifications
  - QR code scanning
  - NFC payments
  - Instant transaction confirmation
```

**SMS Gateway:**

```
Usage: Text commands to send/receive TIME

Examples:
  "SEND 10 TIME to @alice"
  "BALANCE"
  "RECEIVE"

Benefits:
  - No smartphone required
  - No internet needed (SMS only)
  - Works on basic phones
  - Accessible in developing markets
  - Instant confirmation via SMS
```

**Email Gateway:**

```
Usage: Email commands for transactions

Examples:
  To: transactions@time-coin.io
  Subject: SEND
  Body: 10 TIME to alice@example.com

Benefits:
  - Familiar interface
  - Works on any device
  - Integrated with existing workflows
  - Good for business use
  - Email confirmation of transactions
```

### 10.2 User Experience

**Simple Addresses:**

```
Instead of: TIME1a2B3c4D5e6F7g8H9i0J1k2L3m4N5o6P7
Use: @alice or alice@time-coin.io

Features:
  - Human-readable names
  - DNS-like resolution
  - Name registration system
  - Alias support
```

**Transaction Templates:**

```
Common Actions:
  - "Pay monthly rent to @landlord"
  - "Send $50 worth of TIME to @bob"
  - "Scheduled payment: 100 TIME/week to @charity"

All confirmed instantly via BFT
```

**Multi-Language Support:**

```
Launch Languages:
  - English
  - Spanish
  - Mandarin Chinese
  - Hindi
  - Arabic

Future: 20+ languages
```

### 10.3 Education & Onboarding

**Interactive Tutorial:**

```
Steps:
1. Create wallet (guided)
2. Make test purchase
3. Send test transaction (see instant confirmation!)
4. Receive test transaction
5. Explore features
```

**Documentation:**

```
- Beginner's guide
- Video tutorials
- FAQ section
- Troubleshooting guides
- Best practices
- BFT consensus explained simply
```

**Community Support:**

```
Channels:
  - Discord server
  - Telegram groups
  - Forum (discourse)
  - Email support
  - Live chat (weekdays)
```

---

## 11. Tokenomics

### 11.1 Token Utility

**Primary Uses:**

```
1. Medium of Exchange
   - Peer-to-peer payments (instant!)
   - Merchant transactions
   - Cross-border transfers
   - Micropayments

2. Store of Value
   - Long-term holding
   - Inflation hedge
   - Portfolio diversification
   - Longevity rewards incentivize holding

3. Governance Rights
   - Vote on proposals (via masternode)
   - Protocol changes
   - Treasury allocation
   - Voting power scales with commitment

4. Network Staking
   - Masternode collateral
   - Earn rewards
   - Secure network
   - Participate in BFT consensus
   - Long-term commitment rewarded (up to 3×)

5. Fee Payment
   - Transaction fees
   - Service fees
   - Priority access
```

### 11.2 Token Metrics

**Initial State:**

```
Genesis Supply: 0 TIME
  - No pre-mine
  - No founder allocation
  - No VC rounds

Supply Growth: Demand-driven + Fixed daily rewards
  - Minted when purchased (variable)
  - 100 TIME per day from block rewards (constant)
  - Organic expansion
```

**Inflation Rate:**

```
Dynamic Block Rewards (scales with network):
Year 1 (avg 200 nodes): ~120 TIME/day = ~43,800 TIME/year
Year 2 (avg 400 nodes): ~116 TIME/day = ~42,340 TIME/year (cumulative: ~86,140)
Year 5 (avg 3,000 nodes): ~220 TIME/day = ~80,300 TIME/year (cumulative: ~300k+)
Year 10 (avg 10,000+ nodes): 500 TIME/day = 182,500 TIME/year (capped)

Plus purchased TIME (variable based on demand)

Inflation Rate Over Time:
  Year 1: High % (small base, ~44k from rewards)
  Year 5: Medium % (growing base, ~80k from rewards)
  Year 10: Lower % (large base, 182.5k from rewards - capped)
  Year 20: Very low % (asymptotically approaching 0%)

Maximum Annual Inflation (capped): 182,500 TIME/year
Total 10-year inflation from rewards: ~1,000,000 TIME (estimated)
```

**Projected Supply (5 Years):**

```
Assumptions:
  - $1M purchases Year 1
  - 100% YoY purchase growth
  - Average price $0.50

Year 1: 2M (purchases) + 36.5k (rewards) = 2.04M
Year 2: 4M + 36.5k = 4.04M (cumulative: 6.08M)
Year 3: 8M + 36.5k = 8.04M (cumulative: 14.12M)
Year 4: 16M + 36.5k = 16.04M (cumulative: 30.16M)
Year 5: 32M + 36.5k = 32.04M (cumulative: 62.2M)
```

### 11.3 Value Drivers

**Positive Factors:**

```
1. Increasing Demand
   - More users purchasing TIME
   - Network effects
   - Merchant adoption
   - Instant transactions attract users

2. Supply Limitations
   - Low daily inflation (100 TIME/day)
   - Purchase-only minting
   - Masternode lockup reduces circulating supply
   - Longevity incentives encourage long-term holding

3. Utility Growth
   - More use cases
   - DeFi integrations
   - Cross-chain bridges
   - Instant finality competitive advantage

4. Network Security
   - BFT provides instant finality
   - More masternodes = more security
   - Longevity requirement increases attack cost
   - Higher collateral requirements over time

5. Treasury Growth
   - Funds ecosystem development
   - Marketing and adoption
   - Continuous improvement
   - Sustainable model
```

**Deflationary Pressures:**

```
1. Masternode Lockup
   - Removes coins from circulation
   - Proportional to network value
   - Longevity incentivizes longer locks
   - Currently: All collateral locked

2. Treasury Accumulation
   - 50% of fees go to treasury
   - Removes coins from active circulation
   - Used for ecosystem development
   - Grows with network adoption

3. Lost Keys
   - Natural attrition
   - Irrecoverable wallets
   - Permanent removal
```

### 11.4 Economic Sustainability

**Long-Term Model:**

```
Revenue Sources:
  1. Transaction fees (sustainable, grows with usage)
  2. Network service fees (future)
  3. Treasury investments (conservative)

Cost Centers:
  1. Masternode rewards (predictable: 95 TIME/day)
  2. Treasury spending (governed)
  3. Infrastructure (efficient)

Balance:
  - Fees increase with usage
  - Rewards fixed in TIME
  - Self-sustaining model
  - Longevity system ensures stable operator base
```

**Sustainability Analysis:**

```
Daily Masternode Costs (scales with network):
  100 nodes: ~99 TIME/day (98.8 MN + 5.2 Treasury)
  1,000 nodes: ~133 TIME/day (133 MN + 7 Treasury)
  10,000+ nodes: 475 TIME/day (capped)

Break-even Scenarios (Fee-only sustainability):

Scenario A (1,000 node network):
  At 0.001 TIME average fee
  Daily MN cost: 133 TIME
  Need: ~266,000 transactions/day to be fee-sustainable
  (133 TIME × 2, since MN get 50% of fees)

Scenario B (10,000 node network at cap):
  At 0.001 TIME average fee  
  Daily MN cost: 475 TIME (capped)
  Need: ~950,000 transactions/day to be fee-sustainable
  (475 TIME × 2, since MN get 50% of fees)

Realistic Growth Path:
  Year 1: 10k tx/day → 5 TIME fees → Rewards cover 95%
  Year 3: 100k tx/day → 50 TIME fees → Rewards cover 73%
  Year 5: 500k tx/day → 250 TIME fees → Rewards cover 52%
  Year 10: 2M+ tx/day → 1,000+ TIME fees → Fees exceed rewards!

Conclusion:
  - Dynamic block rewards bootstrap network
  - Rewards scale with network size (maintaining incentives)
  - Fees increasingly cover costs as adoption grows
  - At maturity (10k+ nodes), fees become primary income
  - Capped inflation (max 182,500 TIME/year) is sustainable
  - Long-term fee-driven model (like Bitcoin)
```

---

## 12. Roadmap

### 12.1 Phase 1: Foundation (Q1 2025) - IN PROGRESS

**Development:**

- [x] Core blockchain architecture
- [x] Treasury system
- [x] Governance framework
- [x] BFT consensus design
- [x] Three-tier masternode design
- [x] Longevity multiplier system design
- [ ] Masternode implementation
- [ ] Wallet (basic)
- [ ] Network layer (P2P + BFT)

**Milestones:**

- Alpha testnet launch
- Internal BFT testing
- Documentation v1.3

### 12.2 Phase 2: Testnet (Q2 2025)

**Development:**

- [ ] Complete BFT consensus implementation
- [ ] Daily block creation system
- [ ] Longevity tracking system
- [ ] SMS/Email gateways
- [ ] Web interface
- [ ] Mobile apps (beta)
- [ ] Purchase system
- [ ] Security audit #1

**Milestones:**

- Public testnet launch
- Community testing program
- Bug bounty: $50k pool
- 50+ test masternodes
- First governance proposals
- BFT consensus stress testing
- Longevity multiplier testing

### 12.3 Phase 3: Security & Audits (Q3 2025)

**Security:**

- [ ] Professional audit (Trail of Bits or similar)
- [ ] BFT consensus security review
- [ ] Longevity system audit
- [ ] Penetration testing
- [ ] Economic model validation
- [ ] Load testing (100k+ transactions/day)
- [ ] BFT fault tolerance testing

**Infrastructure:**

- [ ] Seed node deployment (5+ locations)
- [ ] Monitoring systems
- [ ] Analytics dashboard
- [ ] Backup systems
- [ ] Disaster recovery plan

**Community:**

- [ ] Ambassador program
- [ ] Educational content (50+ articles)
- [ ] Video tutorials (20+ videos)
- [ ] Community grants ($100k equivalent TIME)

### 12.4 Phase 4: Mainnet Launch (Q4 2025)

**Launch Preparation:**

- [ ] Final security review
- [ ] Mainnet configuration
- [ ] Genesis block creation (Day 0)
- [ ] Initial masternode coordination (50+ nodes)
- [ ] Exchange discussions

**Go-Live:**

- [ ] Mainnet genesis (Block 1)
- [ ] 50+ masternodes active
- [ ] Purchase portal active
- [ ] Web/mobile apps live
- [ ] First daily block at midnight UTC
- [ ] First treasury proposals
- [ ] Longevity tracking begins

**Post-Launch (First 30 Days):**

- [ ] 24/7 monitoring
- [ ] Daily status updates
- [ ] Community support
- [ ] Bug fixes (if needed)
- [ ] First governance votes
- [ ] BFT performance monitoring

### 12.5 Phase 5: Growth (2026+)

**Q1 2026:**

- Exchange listings (3-5 tier-2 exchanges)
- Mobile apps (full release)
- SMS/Email full deployment
- First major treasury grants
- 100+ masternodes
- Demonstrate instant transaction advantage
- First longevity bonuses vest

**Q2 2026:**

- DeFi integrations
- Hardware wallet support
- Advanced trading features
- International expansion
- 500+ masternodes
- Payment processor partnerships

**Q3-Q4 2026:**

- Tier-1 exchange listings
- Cross-chain bridges
- Layer 2 exploration (if needed)
- Enterprise partnerships
- 1000+ masternodes
- Merchant adoption programs
- Significant veteran masternode presence

**2027+:**

- Privacy features
- Smart contracts (evaluation)
- Payment processor integrations
- Banking partnerships
- Global adoption
- 10,000+ masternodes
- Mature longevity distribution

---

## 13. Conclusion

### 13.1 Summary of Innovation

TIME Coin represents a new paradigm in cryptocurrency design:

**Fair Launch:**

- No pre-mine ensures equal opportunity
- Purchase-based minting aligns incentives
- Community-owned from day one

**Revolutionary Architecture:**

- 24-hour blocks for efficiency and scalability
- BFT consensus for instant transaction finality
- Best of both worlds: fast user experience + manageable blockchain
- Only 365 blocks per year vs. millions for other chains

**Effective Governance:**

- Masternode voting provides clear decision-making
- Three-tier weighted system balances participation and commitment
- Longevity multiplier rewards dedication
- Self-funding treasury enables sustainable growth

**Instant Finality:**

- <2 second transaction confirmation
- No waiting for block confirmations
- No possibility of reversal
- Competitive advantage over traditional blockchains

**Long-Term Sustainability:**

- Longevity multiplier encourages operator retention
- 14-42% APY sustainable based on longevity
- Economic model rewards both capital and commitment
- Balanced incentives for new and veteran operators

**Accessibility:**

- Multi-channel access (SMS, Email, Web, Mobile)
- User-friendly design
- Global reach potential

### 13.2 Competitive Advantages

**vs. Bitcoin:**

- Instant transactions (2s vs. 10min)
- Built-in governance (vs. contentious forks)
- Accessible to non-technical users
- Self-funding development
- Scalable blockchain (365 blocks/year vs. 52,560)
- Long-term operator incentives

**vs. Ethereum:**

- Simpler, focused design (payments first)
- Instant finality (vs. probabilistic)
- Lower fees (no gas wars)
- More democratic (no foundation control)
- Clearer economic model
- Better operator retention (longevity)

**vs. Fast Blockchains (Solana, etc.):**

- True instant finality via BFT (vs. probabilistic)
- Much lower storage requirements (1 block/day)
- More decentralized (no super-computers needed)
- Better security properties (BFT proven)
- Sustainable operator economics

**vs. Other BFT Chains (Stellar, Ripple):**

- Community-owned (vs. foundation-controlled)
- Fair launch (no pre-mine)
- Three-tier masternode system (more democratic)
- Longevity rewards (long-term alignment)
- Self-funding treasury (sustainable)
- Purchase-based minting (organic growth)
- Dynamic quorum (scales to 100k+ nodes vs. limited validator sets)

**vs. Other Masternodes (Dash, etc.):**

- BFT instant finality (vs. waiting for confirmations)
- Three-tier system (simpler, more accessible)
- Longevity multiplier (rewards commitment)
- Active governance (not passive)
- Daily blocks (vs. constant block creation)
- Modern architecture
- 14-42% APY based on longevity (competitive and sustainable)

### 13.3 Risk Factors

**Technical Risks:**

- Untested BFT implementation (mitigation: extensive testing, audits)
- Daily block novelty (mitigation: thorough analysis, testnet validation)
- Longevity tracking complexity (mitigation: simple formula, on-chain verification)
- Scalability unknowns (mitigation: BFT handles high TPS, daily blocks compress storage)

**Economic Risks:**

- Price volatility (mitigation: organic growth, no large unlocks)
- APY sustainability (mitigation: longevity reduces turnover, fees supplement)
- Adoption challenges (mitigation: instant finality advantage, accessibility features)
- Longevity gaming attempts (mitigation: 72-hour reset, slashing)

**Governance Risks:**

- Low participation (mitigation: voting incentives, 5% bonus)
- Veteran operator dominance (mitigation: linear tier scaling, 3× cap)
- Contentious proposals (mitigation: high thresholds, discussion periods)

**Consensus Risks:**

- 67% attack threshold (mitigation: economic security via collateral, longevity requirement, slashing)
- Network partitions (mitigation: BFT handles, requires 67% online)
- Coordinator failure (mitigation: decentralized proposer selection)

**Mitigation Strategy:**

- Phased rollout (testnet → mainnet)
- Conservative treasury spending
- Regular audits and reviews
- Active community engagement
- Transparent communication
- Professional security reviews of BFT implementation
- Continuous monitoring of longevity system

### 13.4 Long-Term Vision

**Years 1-2: Establishment**

- Build robust network
- Achieve critical mass (1000+ masternodes)
- Establish governance track record
- Begin treasury-funded projects
- Demonstrate BFT instant finality advantage
- First longevity bonuses vest

**Years 3-5: Growth**

- Major exchange listings
- International expansion
- DeFi ecosystem integration
- Merchant adoption programs
- 10,000+ masternodes
- Position as "instant settlement" leader
- Significant veteran masternode presence

**Years 5-10: Maturity**

- Global payment infrastructure
- Banking partnerships
- Enterprise adoption
- Cross-chain leadership
- 100,000+ masternodes
- Household name recognition
- Stable longevity distribution

**Years 10+: Evolution**

- Adapt to technological changes
- Community-driven feature additions
- Sustainable, self-funding ecosystem
- Financial system alternative
- Proven long-term model
- Maximum longevity multipliers common

### 13.5 Why TIME Coin Will Succeed

**Technical Excellence:**

- Novel 24-hour block architecture
- Proven BFT consensus (modified for our needs)
- Dynamic quorum selection (scales to 100k+ masternodes)
- Instant transaction finality (<3 seconds at any scale)
- Scalable and efficient

**Economic Sustainability:**

- Capped inflation (max 182,500 TIME/year at 10k+ nodes)
- Dynamic rewards scale with network needs
- Self-funding via fees (grows with usage)
- Fair distribution from day one
- No VC pressure or misaligned incentives
- Longevity system ensures operator retention
- 14-56% APY range across network phases (rewards + fees)
- Maintains operator incentives at all scales

**Community-First:**

- True decentralization
- Democratic governance
- Transparent treasury
- Active participation incentives
- Long-term commitment rewarded

**User Experience:**

- Instant confirmations (<2 seconds)
- Multi-channel accessibility
- Simple interfaces
- Real-world utility

**Market Opportunity:**

- Payments market is massive
- Instant finality is rare
- Fair launch coins are rare
- Longevity incentives are unique
- Combination is unprecedented

### 13.6 Call to Action

**For Users:**

- Experience instant transactions
- Join early for fair distribution
- Participate in governance
- Help build the community

**For Developers:**

- Contribute to open-source code
- Build on TIME Coin APIs
- Create tools and services
- Submit improvement proposals
- Help optimize BFT consensus

**For Masternodes:**

- Secure the network
- Participate in BFT consensus
- Vote in governance
- Earn sustainable rewards (14-42% APY based on longevity)
- Build long-term commitment (up to 3× multiplier)
- Shape the future

**For Everyone:**

- Time is money
- Own your financial future
- Support fair, community-governed money
- Experience true instant finality
- Benefit from long-term commitment
- Join the TIME Coin revolution

---

## Appendix A: Technical Specifications

### Constants

```rust
TIME_UNIT: 100_000_000 (8 decimals)
BLOCK_TIME: 86400 seconds (24 hours)
BLOCKS_PER_DAY: 1
BLOCKS_PER_YEAR: 365

// Dynamic Block Reward System
BLOCK_REWARD_BASE: 100 TIME
BLOCK_REWARD_SCALE_FACTOR: 0.04 TIME per masternode
BLOCK_REWARD_MAX: 500 TIME (inflation cap)

// Formula: min(MAX, BASE + (active_masternodes × SCALE_FACTOR))
// Split: 95% masternodes, 5% treasury

FEE_SPLIT: 50% treasury, 50% masternodes

BFT_CONFIRMATION_THRESHOLD: 67% (of selected quorum)
BFT_BLOCK_SIGNATURE_THRESHOLD: 80% (of total network)
TRANSACTION_FINALITY: <2 seconds (up to 10k nodes), <3 seconds (100k+ nodes)

QUORUM_SIZE_FORMULA: max(50, min(500, log₂(n) × 50))
QUORUM_MIN_SIZE: 50 masternodes
QUORUM_MAX_SIZE: 500 masternodes

LONGEVITY_MULTIPLIER_FORMULA: 1 + (days_active ÷ 365) × 0.5
LONGEVITY_MAX_MULTIPLIER: 3.0
LONGEVITY_RESET_THRESHOLD: 72 hours downtime
```

### Masternode Requirements

```
Bronze: 1,000 TIME,   1× voting power,  1.0×-3.0× longevity
Silver: 10,000 TIME,  10× voting power, 1.0×-3.0× longevity  
Gold:   100,000 TIME, 100× voting power, 1.0×-3.0× longevity
```

### Governance Parameters

```
Standard Proposal:
  Deposit: 100 TIME
  Discussion: 7 days
  Voting: 14 days
  Approval: 60%
  Quorum: 60%

Emergency Proposal:
  Deposit: 500 TIME
  Discussion: 2 days
  Voting: 5 days
  Approval: 75%
```

### BFT Consensus Parameters

```
Transaction Confirmation (Dynamic Quorum):
  - Quorum selection: Weighted random sampling (VRF-based)
  - Quorum size formula: max(50, min(500, log₂(n) × 50))
  - Examples:
    * 100 nodes → 50 node quorum (50%)
    * 1,000 nodes → 150 node quorum (15%)
    * 10,000 nodes → 300 node quorum (3%)
    * 100,000 nodes → 500 node quorum (0.5%)
  - Voting threshold: 67% of selected quorum
  - Selection weight: tier_weight × longevity_multiplier
  - Typical confirmation time: <2 seconds
  - Rotates every transaction

Block Finalization (Full Network):
  - All active masternodes participate
  - Signing threshold: 80% of total weighted voting power
  - Minimum participation: 67% of active masternodes
  - Block finalization window: 00:00:01 UTC - 00:10 UTC
  - Target finalization: 00:07 UTC
  - Emergency window: 00:10-00:15 UTC (if needed)
```

### Longevity Multiplier Parameters

```
Formula: 1 + (days_active ÷ 365) × 0.5
Minimum: 1.0× (new nodes)
Maximum: 3.0× (1,460+ days = 4+ years)
Reset condition: >72 hours downtime
Application: Rewards and block proposal selection
Tracking: On-chain, per masternode
```

---

## Appendix B: Glossary

**BFT (Byzantine Fault Tolerant):** Consensus algorithm that tolerates up to 1/3 malicious nodes

**Block Time:** Time between blocks (24 hours for TIME Coin)

**Collateral:** TIME tokens locked to operate a masternode

**Dynamic Block Reward:** TIME Coin's reward system that scales with network size (100-500 TIME/day) to maintain operator incentives while capping long-term inflation

**Dynamic Quorum:** Randomly selected subset of masternodes that validates each transaction, size scales logarithmically with network size to maintain speed

**Finality:** Point at which a transaction cannot be reversed (instant via BFT)

**Governance:** Decision-making process for protocol and treasury

**Longevity Multiplier:** Reward multiplier that increases with continuous masternode operation (1.0× to 3.0×)

**Masternode:** Network node with collateral that participates in BFT consensus

**Minting:** Creation of new TIME tokens through purchase

**Proposal:** Formal request for treasury funds or protocol changes

**Quorum:** Minimum participation required for valid vote or consensus; in TIME Coin, dynamically selected subset for transaction validation

**Slashing:** Penalty (loss of collateral sent to treasury) for malicious behavior

**Treasury:** Community-governed fund for ecosystem development

**VRF (Verifiable Random Function):** Cryptographic function that produces verifiable random output used for quorum selection

**Voting Power:** Influence in governance and BFT consensus based on masternode tier × longevity

**Weighted Voting:** Voting system where different participants have different voting power based on tier and longevity

---

## Appendix C: Comparison with Other Consensus Mechanisms

### Proof of Work (Bitcoin)

```
Block Time: 10 minutes
Finality: ~60 minutes (6 confirmations)
Energy: High
Scalability: Low (~7 TPS)
Decentralization: High (but centralizing to pools)

TIME Coin Advantage:
  - 300× faster finality (2s vs 60min)
  - Negligible energy use
  - Higher practical TPS
  - More accessible participation
  - Long-term operator incentives
```

### Proof of Stake (Ethereum)

```
Block Time: 12 seconds
Finality: ~15 minutes (2 epochs)
Energy: Low
Scalability: Medium (~15 TPS, higher with L2)
Decentralization: Medium (requires 32 ETH)

TIME Coin Advantage:
  - 450× faster finality (2s vs 15min)
  - Lower entry barrier (1,000 TIME)
  - 365 blocks/year vs 2.6M (99.98% less storage)
  - Simpler architecture
  - Longevity rewards commitment
```

### Traditional BFT (Stellar, Ripple)

```
Block Time: 5 seconds (Stellar)
Finality: Instant
Energy: Low
Scalability: High (1000+ TPS)
Decentralization: Low-Medium (Foundation-controlled)

TIME Coin Advantage:
  - True decentralization (community-owned)
  - Fair launch (no pre-mine)
  - 365 blocks/year vs 6.3M (99.99% less storage)
  - Democratic governance
  - Self-funding model
  - Long-term operator incentives (longevity)
```

### Other Masternodes (Dash)

```
Block Time: 2.5 minutes
Finality: ~15 minutes (6 blocks)
Energy: Medium (still PoW for block creation)
Scalability: Medium
Decentralization: Medium

TIME Coin Advantage:
  - 450× faster finality (2s vs 15min)
  - No PoW energy waste
  - 365 blocks/year vs 210K (99.8% less storage)
  - Modern BFT consensus
  - Three-tier system (more democratic)
  - Longevity multiplier (rewards commitment)
  - Higher sustainable APY (18-42%)
```

---

## Appendix D: References

1. Castro, M., Liskov, B. (1999). Practical Byzantine Fault Tolerance
2. Nakamoto, S. (2008). Bitcoin: A Peer-to-Peer Electronic Cash System
3. Mazières, D. (2015). The Stellar Consensus Protocol
4. Schwartz, D., Youngs, N., Britto, A. (2014). The Ripple Protocol Consensus Algorithm
5. Buterin, V. (2014). Ethereum: A Next-Generation Smart Contract Platform
6. Duffield, E., Diaz, D. (2018). Dash: A Payments-Focused Cryptocurrency
7. Wood, G. (2016). Polkadot: Vision for a Heterogeneous Multi-Chain Framework
8. Buchman, E. (2016). Tendermint: Byzantine Fault Tolerance in the Age of Blockchains

---

## Document Information

**Version:** 1.3 (Three-Tier + Longevity System)
**Date:** October 2025
**Status:** Official
**Authors:** TIME Coin Development Team
**License:** CC BY-SA 4.0

**Major Changes from v1.2:**

- Updated to three-tier masternode system (Bronze, Silver, Gold)
- Added longevity multiplier system (1.0× to 3.0× based on continuous uptime)
- **Implemented dynamic block reward system (100-500 TIME/day based on network size)**
- Updated ROI calculations to reflect dynamic rewards and network phases
- Removed Platinum and Diamond tiers
- Added dynamic quorum selection for scalability (scales to 100k+ masternodes)
- Updated all economic calculations and examples
- Revised Appendix A with new tier structure, dynamic reward formula, and BFT parameters
- Enhanced security analysis with longevity and quorum considerations
- Updated roadmap to include longevity tracking
- Fixed block timing (builds block after midnight, not before)
- All penalties and burns now go to treasury (not destroyed)
- Updated treasury projections to reflect dynamic rewards

**Contact:**

- Website: https://time-coin.io
- Email: info@time-coin.io
- Telegram: https://t.me/+CaN6EflYM-83OTY0
- Twitter: @TIMEcoin515010
- GitHub: https://github.com/time-coin/time-coin

---

**⏰ TIME is money.**

*This whitepaper describes TIME Coin as currently designed. Features and specifications may change during development. Always refer to the latest version at time-coin.io/whitepaper*