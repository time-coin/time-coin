# TIME Coin Protocol Specification v1.0

**UTXO-Based Instant Finality with Masternode BFT Consensus**

---

## Table of Contents

1. [Abstract](#abstract)
2. [Introduction](#introduction)
3. [Protocol Architecture](#protocol-architecture)
4. [UTXO Model](#utxo-model)
5. [Masternode BFT Consensus](#masternode-bft-consensus)
6. [Instant Finality Mechanism](#instant-finality-mechanism)
7. [State Transition Model](#state-transition-model)
8. [Network Protocol](#network-protocol)
9. [Security Analysis](#security-analysis)
10. [Implementation Requirements](#implementation-requirements)
11. [Appendix](#appendix)

---

## 1. Abstract

The TIME Coin Protocol is a novel cryptocurrency protocol that achieves instant transaction finality (<3 seconds) while maintaining Bitcoin's proven UTXO (Unspent Transaction Output) accounting model. It combines:

- **UTXO-based accounting** for simplicity and parallel transaction processing
- **Masternode BFT consensus** for instant finality
- **Real-time state tracking** for double-spend prevention
- **Push notifications** for immediate transaction awareness

The protocol tolerates up to 33% Byzantine faults (malicious nodes) and achieves 1000+ transactions per second throughput while maintaining instant finality guarantees.

---

## 2. Introduction

### 2.1 Problem Statement

Traditional UTXO-based cryptocurrencies (Bitcoin) require multiple block confirmations for transaction safety, resulting in 30-60 minute finality times. Account-based systems (Ethereum) achieve faster finality but introduce state complexity and gas race conditions.

### 2.2 Solution Overview

The TIME Coin Protocol introduces a **UTXO state machine** where every UTXO transitions through defined states, validated by masternode consensus. This enables:

1. **Instant UTXO locking** to prevent double-spends
2. **Parallel BFT voting** on transaction validity
3. **Sub-3-second finality** when 67%+ consensus reached
4. **Bitcoin compatibility** through UTXO model

### 2.3 Design Goals

- **Instant Finality**: <3 second transaction confirmation
- **Byzantine Fault Tolerance**: Tolerate up to 33% malicious nodes
- **UTXO Simplicity**: Maintain Bitcoin's proven accounting model
- **High Throughput**: Support 1000+ TPS
- **Real-Time Notifications**: Push state updates to all subscribers
- **Deterministic**: Same inputs produce same outputs

---

## 3. Protocol Architecture

### 3.1 System Components

```
┌─────────────────────────────────────────────────────────────┐
│                   TIME Coin Protocol                         │
├─────────────────────────────────────────────────────────────┤
│                                                              │
│  ┌──────────────────┐         ┌──────────────────┐         │
│  │  UTXO State      │         │   Masternode     │         │
│  │  Manager         │◄───────►│   Consensus      │         │
│  │  - State tracking│         │   - BFT voting   │         │
│  │  - Locking       │         │   - Finality     │         │
│  └────────┬─────────┘         └────────┬─────────┘         │
│           │                            │                    │
│           │        ┌──────────────────┐│                   │
│           └───────►│  Network Layer   ││                   │
│                    │  - P2P protocol  ││                   │
│                    │  - State sync    ││                   │
│                    └──────────────────┘│                   │
│                                        │                    │
│           ┌────────────────────────────▼─────────┐         │
│           │         Blockchain                    │         │
│           │         (24-hour blocks)              │         │
│           └───────────────────────────────────────┘         │
│                                                              │
└─────────────────────────────────────────────────────────────┘
```

### 3.2 Layer Architecture

| Layer | Component | Responsibility |
|-------|-----------|----------------|
| **Application** | Wallets, Exchanges | User-facing applications |
| **Protocol** | TIME Coin Protocol | UTXO state management, instant finality |
| **Consensus** | Masternode BFT | Transaction/block validation |
| **Network** | P2P Protocol | Message propagation, peer discovery |
| **Storage** | Blockchain | Immutable transaction ledger |

### 3.3 Design Principles

1. **Separation of Concerns**: Transaction finality separate from block production
2. **Optimistic Execution**: Lock UTXOs immediately, validate in parallel
3. **Byzantine Fault Tolerance**: Assume up to 33% of nodes are malicious
4. **Real-Time State**: All nodes maintain consistent UTXO state
5. **Deterministic Finality**: 67%+ votes = irreversible transaction

---

## 4. UTXO Model

### 4.1 UTXO Definition

A UTXO (Unspent Transaction Output) is defined as:

```rust
struct UTXO {
    outpoint: OutPoint,      // (txid, vout) identifier
    value: u64,              // Amount in smallest unit
    script_pubkey: Vec<u8>,  // Locking script
    address: String,         // Owner address
}

struct OutPoint {
    txid: Hash256,  // Transaction ID
    vout: u32,      // Output index
}
```

### 4.2 UTXO Properties

- **Immutable**: Once created, UTXO data cannot change
- **Atomic**: UTXOs are either unspent or spent, no partial spending
- **Parallel**: Multiple UTXOs can be processed independently
- **Deterministic**: UTXO existence can be verified cryptographically

### 4.3 Transaction Structure

```rust
struct Transaction {
    version: u32,
    inputs: Vec<TxInput>,
    outputs: Vec<TxOutput>,
    lock_time: u32,
    timestamp: i64,
}

struct TxInput {
    previous_output: OutPoint,  // UTXO being spent
    script_sig: Vec<u8>,        // Unlocking script
    sequence: u32,              // Lock time support
}

struct TxOutput {
    value: u64,           // Amount
    script_pubkey: Vec<u8>,  // Locking script
}
```

### 4.4 Transaction Rules

1. **Input Validation**: All inputs must reference existing, unspent UTXOs
2. **Value Conservation**: Sum(inputs) ≥ Sum(outputs) + fee
3. **Signature Verification**: All inputs must have valid signatures
4. **Double-Spend Prevention**: Each UTXO can only be spent once
5. **Lock Time**: Transactions can be time-locked for future spending

### 4.5 UTXO Set Management

The **UTXO Set** is the set of all unspent transaction outputs:

```
UTXOSet = { (outpoint, utxo) | utxo is unspent }
```

**Operations**:

- `add_utxo(outpoint, utxo)`: Add new UTXO to set
- `remove_utxo(outpoint)`: Remove spent UTXO from set
- `get_utxo(outpoint)`: Retrieve UTXO by outpoint
- `get_balance(address)`: Sum all UTXOs for address

**Invariants**:
- No duplicate outpoints
- Total supply = sum of all UTXO values
- UTXO set reflects blockchain state

---

## 5. Masternode BFT Consensus

### 5.1 Masternode Network

**Definition**: A masternode is a collateralized full node with enhanced responsibilities:

```rust
struct Masternode {
    address: String,           // Network address
    collateral: u64,          // Locked TIME coins
    tier: MasternodeTier,     // Bronze/Silver/Gold
    public_key: PublicKey,    // For vote signing
    registered_at: u64,       // Block height
}

enum MasternodeTier {
    Bronze = 1000,    // 1,000 TIME
    Silver = 10000,   // 10,000 TIME
    Gold = 100000,    // 100,000 TIME
}
```

### 5.2 BFT Consensus Properties

**Byzantine Fault Tolerance**:
- **f = ⌊(n-1)/3⌋**: Maximum Byzantine faults tolerated
- **n ≥ 3f + 1**: Minimum nodes for consensus
- **Quorum = ⌈2n/3⌉**: Required votes for finality

**Safety**: At most one transaction can achieve finality
**Liveness**: Valid transactions eventually achieve finality

### 5.3 Consensus Algorithm

```
Algorithm: BFT_Transaction_Consensus
Input: Transaction tx, Masternode set M
Output: Finality decision (Approved/Rejected)

1. Phase 1: Validation
   for each masternode m in M:
     validated[m] ← validate_transaction(tx, utxo_set)
     
2. Phase 2: Voting
   for each masternode m in M:
     vote[m] ← sign(validated[m], m.private_key)
     broadcast(vote[m])
     
3. Phase 3: Aggregation
   approvals ← count(vote[m] = true for m in M)
   rejections ← count(vote[m] = false for m in M)
   
4. Phase 4: Finality
   quorum ← ceil(2 * |M| / 3)
   if approvals ≥ quorum:
     return Approved
   else if rejections > |M| - quorum:
     return Rejected
   else:
     return Pending
```

### 5.4 Vote Structure

```rust
struct Vote {
    txid: Hash256,           // Transaction being voted on
    voter: String,           // Masternode address
    approve: bool,           // Vote decision
    timestamp: i64,          // Vote time
    signature: Signature,    // Cryptographic signature
}
```

**Vote Validity Rules**:
1. Voter must be registered masternode
2. Signature must verify against voter's public key
3. Cannot vote twice on same transaction
4. Timestamp must be reasonable (within 5 minutes)

### 5.5 Consensus Guarantees

**Theorem 1 (Safety)**: If two transactions spending the same UTXO both achieve finality, then more than 33% of masternodes are Byzantine.

**Proof**: For both tx1 and tx2 to achieve finality:
- tx1 needs ≥ ⌈2n/3⌉ approvals
- tx2 needs ≥ ⌈2n/3⌉ approvals
- Combined: ≥ ⌈4n/3⌉ approvals
- But total nodes = n
- Therefore ≥ ⌈n/3⌉ nodes voted twice (Byzantine)
- This violates the f < n/3 assumption. □

**Theorem 2 (Liveness)**: If ≥ ⌈2n/3⌉ honest nodes approve a valid transaction, it will achieve finality.

**Proof**: Honest nodes follow protocol and vote consistently. With ≥ ⌈2n/3⌉ honest approvals, quorum is reached. □

---

## 6. Instant Finality Mechanism

### 6.1 UTXO State Machine

Each UTXO transitions through defined states:

```
          ┌─────────┐
    ┌────►│ Unspent │◄─────┐
    │     └────┬────┘      │
    │          │           │ unlock
    │          │ lock      │
    │          ▼           │
    │     ┌─────────┐      │
    │     │ Locked  │──────┘
    │     └────┬────┘
    │          │ broadcast
    │          ▼
    │     ┌─────────────┐
    │     │SpentPending │
    │     └──────┬──────┘
    │            │ 67%+ votes
    │            ▼
    │     ┌──────────────┐
    │     │SpentFinalized│
    │     └──────┬───────┘
    │            │ block inclusion
    │            ▼
    │     ┌───────────┐
    └─────│ Confirmed │
          └───────────┘
```

### 6.2 State Definitions

```rust
enum UTXOState {
    Unspent,
    Locked {
        txid: Hash256,
        locked_at: i64,
    },
    SpentPending {
        txid: Hash256,
        votes: u32,
        total_nodes: u32,
        spent_at: i64,
    },
    SpentFinalized {
        txid: Hash256,
        finalized_at: i64,
        votes: u32,
    },
    Confirmed {
        txid: Hash256,
        block_height: u64,
        confirmed_at: i64,
    },
}
```

### 6.3 State Transition Rules

**Rule 1: Unspent → Locked**
```
Precondition: 
  - state = Unspent
  - Transaction tx references UTXO
  
Action:
  - state ← Locked(tx.txid, now())
  - Notify all subscribers
  
Effect:
  - UTXO cannot be spent by other transactions
```

**Rule 2: Locked → SpentPending**
```
Precondition:
  - state = Locked(txid, _)
  - Transaction tx broadcast to network
  
Action:
  - state ← SpentPending(txid, 0, n, now())
  - Masternodes begin voting
  
Effect:
  - Vote collection begins
```

**Rule 3: SpentPending → SpentFinalized**
```
Precondition:
  - state = SpentPending(txid, votes, total_nodes, _)
  - votes ≥ ceil(2 * total_nodes / 3)
  
Action:
  - state ← SpentFinalized(txid, now(), votes)
  - Notify all subscribers
  
Effect:
  - Transaction is irreversible (instant finality achieved)
```

**Rule 4: SpentFinalized → Confirmed**
```
Precondition:
  - state = SpentFinalized(txid, _, _)
  - Transaction included in block at height h
  
Action:
  - state ← Confirmed(txid, h, block.timestamp)
  - Notify all subscribers
  
Effect:
  - Transaction permanently recorded on blockchain
```

**Rule 5: Locked → Unspent (Failure)**
```
Precondition:
  - state = Locked(txid, locked_at)
  - Transaction rejected OR timeout (> 60 seconds)
  
Action:
  - state ← Unspent
  - Notify all subscribers
  
Effect:
  - UTXO can be spent by other transactions
```

### 6.4 Instant Finality Algorithm

```
Algorithm: Process_Transaction_With_Instant_Finality
Input: Transaction tx, Masternode set M
Output: Finality status

1. UTXO Locking Phase
   for each input in tx.inputs:
     if utxo_state[input.previous_output] ≠ Unspent:
       return Error("UTXO already spent/locked")
     utxo_state[input.previous_output] ← Locked(tx.txid, now())
   
2. Broadcast Phase
   broadcast_to_network(tx)
   for each input in tx.inputs:
     utxo_state[input.previous_output] ← SpentPending(tx.txid, 0, |M|, now())
   
3. Voting Phase (Parallel)
   for each masternode m in M (parallel):
     is_valid ← validate(tx, utxo_set)
     vote[m] ← Vote(tx.txid, m.address, is_valid, now(), sign(m))
     broadcast(vote[m])
   
4. Aggregation Phase
   approvals ← 0
   while approvals < ceil(2 * |M| / 3) and time < timeout:
     wait for votes
     approvals ← count(vote[m].approve = true)
     for each input in tx.inputs:
       utxo_state[input.previous_output].votes ← approvals
   
5. Finality Phase
   if approvals ≥ ceil(2 * |M| / 3):
     for each input in tx.inputs:
       utxo_state[input.previous_output] ← SpentFinalized(tx.txid, now(), approvals)
     for each output in tx.outputs:
       utxo_set.add(output.outpoint, output)
       utxo_state[output.outpoint] ← Unspent
     return Finalized
   else:
     for each input in tx.inputs:
       utxo_state[input.previous_output] ← Unspent
     return Rejected
```

### 6.5 Time Complexity Analysis

- **UTXO Locking**: O(n) where n = number of inputs
- **Validation**: O(n) per masternode (parallel)
- **Vote Aggregation**: O(m) where m = number of masternodes
- **Total Time**: O(network_latency + validation_time)
  - Network latency: 50-200ms (P2P propagation)
  - Validation time: 10-50ms per node
  - Parallel voting: All nodes vote simultaneously
  - **Expected time to finality: <3 seconds**

---

## 7. State Transition Model

### 7.1 Global State

The global state S is defined as:

```
S = (UTXOSet, UTXOState, MasternodeSet, BlockchainState)

where:
  UTXOSet = set of all unspent transaction outputs
  UTXOState = map from OutPoint to UTXOState
  MasternodeSet = set of registered masternodes
  BlockchainState = ordered list of blocks
```

### 7.2 State Transitions

```
State Transition Function: δ(S, event) → S'

Events:
  - NewTransaction(tx): New transaction broadcast
  - Vote(txid, voter, approve): Masternode vote received
  - NewBlock(block): New block mined
  - RegisterMasternode(mn): Masternode registration
```

### 7.3 State Invariants

**Invariant 1 (UTXO Uniqueness)**:
```
∀ outpoint ∈ UTXOSet: count(outpoint) = 1
```

**Invariant 2 (Value Conservation)**:
```
sum(utxo.value for utxo in UTXOSet) + total_fees = initial_supply + mined_supply
```

**Invariant 3 (State Consistency)**:
```
∀ outpoint: 
  (outpoint ∈ UTXOSet) ⟺ (UTXOState[outpoint] ∈ {Unspent, Locked, SpentPending})
```

**Invariant 4 (Finality Safety)**:
```
∀ tx1, tx2 spending same UTXO:
  ¬(finalized(tx1) ∧ finalized(tx2))
```

### 7.4 Consistency Model

The TIME Coin Protocol uses **eventual consistency** with **instant finality**:

1. **Immediate Consistency**: UTXO locks propagate immediately
2. **Eventual Consistency**: All nodes eventually agree on UTXO set
3. **Instant Finality**: Once finalized, transaction is irreversible
4. **Causal Consistency**: State transitions respect causality

---

## 8. Network Protocol

### 8.1 Message Types

```rust
enum NetworkMessage {
    // Transaction messages
    TransactionBroadcast(Transaction),
    TransactionVoteRequest(Hash256),
    TransactionVote(Vote),
    
    // UTXO state messages
    UTXOStateQuery(Vec<OutPoint>),
    UTXOStateResponse(Vec<(OutPoint, UTXOState)>),
    UTXOStateNotification(UTXOStateChange),
    
    // Subscription messages
    Subscribe(Subscription),
    Unsubscribe(String),
    
    // Block messages
    BlockProposal(Block),
    BlockVote(Vote),
    BlockAnnouncement(BlockHeader),
    
    // Sync messages
    GetUTXOSet,
    UTXOSetResponse(Vec<UTXO>),
    GetBlocks(u64, u64),
    BlocksResponse(Vec<Block>),
}
```

### 8.2 Protocol Flow

**Transaction Submission**:
```
1. Client → Node: TransactionBroadcast(tx)
2. Node: Lock UTXOs, change state to Locked
3. Node → Network: TransactionBroadcast(tx)
4. Node → Masternodes: TransactionVoteRequest(tx.txid)
5. Masternodes: Validate transaction
6. Masternodes → Network: TransactionVote(vote)
7. All Nodes: Aggregate votes
8. If quorum reached:
   Node → Network: UTXOStateNotification(SpentFinalized)
```

**State Synchronization**:
```
1. New Node → Peer: GetUTXOSet
2. Peer → New Node: UTXOSetResponse(utxo_set)
3. New Node → Peer: GetBlocks(0, latest)
4. Peer → New Node: BlocksResponse(blocks)
5. New Node: Verify and apply state
```

### 8.3 Network Security

**Authentication**: All votes must be signed by registered masternodes

**Encryption**: Optional TLS for P2P connections

**Rate Limiting**: 
- Max 1000 transactions per second per node
- Max 100 UTXO state queries per second
- Max 10 subscription requests per minute

**DDoS Protection**:
- Connection limits per IP
- Proof-of-work for initial connection
- Reputation-based throttling

### 8.4 Network Topology

**Peer Discovery**:
1. DNS seeds (primary)
2. Hardcoded peer list (fallback)
3. Peer exchange protocol (ongoing)

**Connection Management**:
- Target: 8-125 outbound connections
- Max: 125 inbound connections
- Prefer masternode connections
- Periodic connection rotation

---

## 9. Security Analysis

### 9.1 Threat Model

**Assumptions**:
- Up to 33% of masternodes may be Byzantine (malicious)
- Network may experience partitions
- Adversary has polynomial-time computational power
- Cryptographic primitives (signatures, hashes) are secure

**Attack Vectors**:
1. Double-spend attempts
2. UTXO state manipulation
3. Vote manipulation
4. Network partition attacks
5. Eclipse attacks
6. Sybil attacks (via masternode registration)

### 9.2 Double-Spend Prevention

**Attack**: Adversary tries to spend same UTXO twice

**Defense**:
1. **UTXO Locking**: First transaction locks UTXO immediately
2. **Atomic State Transition**: State changes are atomic
3. **BFT Consensus**: Requires 67%+ honest nodes
4. **Finality Guarantee**: Once finalized, irreversible

**Proof of Safety**:
```
Let tx1 and tx2 both spend UTXO u
Assume both achieve finality

tx1 achieves finality ⟹ ≥ ⌈2n/3⌉ honest nodes approved tx1
tx2 achieves finality ⟹ ≥ ⌈2n/3⌉ honest nodes approved tx2

Honest nodes never approve conflicting transactions
⟹ At least ⌈n/3⌉ dishonest nodes
⟹ Contradiction with f < n/3 assumption

Therefore, double finality is impossible.
```

### 9.3 Network Partition Tolerance

**Scenario**: Network splits into partitions P1 and P2

**Behavior**:
- If |P1| ≥ ⌈2n/3⌉: P1 achieves finality
- If |P2| ≥ ⌈2n/3⌉: P2 achieves finality
- If both ≥ ⌈2n/3⌉: Impossible (requires > n nodes)

**Recovery**:
1. Network partition heals
2. Nodes sync state
3. Partition with more nodes wins
4. Minority partition reverts conflicting transactions

### 9.4 Sybil Resistance

**Mechanism**: Collateral-based masternode registration

**Requirements**:
- Bronze: 1,000 TIME locked
- Silver: 10,000 TIME locked
- Gold: 100,000 TIME locked

**Cost of Attack**:
- To control 33% of network: ≥ 33% of total collateral
- Economic incentive: Honest behavior earns rewards
- Malicious behavior: Lose collateral + rewards

### 9.5 Cryptographic Security

**Hash Function**: SHA-256
- Collision resistance: 2^256 operations
- Pre-image resistance: 2^256 operations

**Signature Scheme**: Ed25519
- Public key: 32 bytes
- Signature: 64 bytes
- Security: ~128-bit

**Transaction ID**: SHA-256(transaction data)
- Unique identifier
- Commitment to transaction content

---

## 10. Implementation Requirements

### 10.1 Node Requirements

**Minimum Specifications**:
- CPU: 2 cores, 2.0 GHz
- RAM: 4 GB
- Storage: 100 GB SSD
- Network: 10 Mbps up/down
- OS: Linux, macOS, Windows

**Masternode Requirements**:
- CPU: 4 cores, 2.5 GHz
- RAM: 8 GB
- Storage: 200 GB SSD
- Network: 50 Mbps up/down
- Uptime: 99%+ recommended

### 10.2 Software Components

**Required Modules**:
1. **UTXO State Manager**: Track all UTXO states
2. **Consensus Engine**: BFT voting and aggregation
3. **Network Layer**: P2P communication
4. **Storage Layer**: Blockchain and UTXO set persistence
5. **API Layer**: RPC and REST interfaces

**Language**: Rust (for safety and performance)

**Dependencies**:
- tokio: Async runtime
- serde: Serialization
- ed25519-dalek: Signatures
- sha2/sha3: Hashing
- sled: Embedded database

### 10.3 Performance Targets

| Metric | Target | Measurement |
|--------|--------|-------------|
| Time to Finality | <3 seconds | 95th percentile |
| Throughput | 1000+ TPS | Sustained load |
| UTXO Lock Latency | <100 ms | Average |
| State Sync Time | <5 minutes | Full node sync |
| Memory Usage | <4 GB | Typical operation |
| Storage Growth | <10 GB/month | Blockchain data |

### 10.4 Monitoring and Metrics

**Key Metrics**:
- Transactions per second (TPS)
- Average time to finality (ATF)
- UTXO set size
- Active masternodes count
- Vote participation rate
- Network latency (P2P)
- Block production rate

**Alerting**:
- Finality time > 5 seconds
- Masternode participation < 67%
- UTXO set inconsistency detected
- Network partition detected

---

## 11. Appendix

### 11.1 Glossary

- **UTXO**: Unspent Transaction Output, fundamental unit of value
- **OutPoint**: Unique identifier for a UTXO (txid + vout)
- **BFT**: Byzantine Fault Tolerance, consensus with malicious nodes
- **Quorum**: Minimum votes required (⌈2n/3⌉)
- **Finality**: Irreversible transaction state
- **Masternode**: Collateralized validator node
- **State Machine**: Formal model of UTXO lifecycle

### 11.2 Mathematical Notation

- **⌈x⌉**: Ceiling function (smallest integer ≥ x)
- **⌊x⌋**: Floor function (largest integer ≤ x)
- **∀**: Universal quantifier (for all)
- **∃**: Existential quantifier (there exists)
- **∧**: Logical AND
- **∨**: Logical OR
- **¬**: Logical NOT
- **⟹**: Logical implication
- **⟺**: Logical equivalence

### 11.3 References

1. **Bitcoin: A Peer-to-Peer Electronic Cash System** - Satoshi Nakamoto (2008)
2. **Practical Byzantine Fault Tolerance** - Castro & Liskov (1999)
3. **The Byzantine Generals Problem** - Lamport, Shostak, Pease (1982)
4. **UTXO Model** - Bitcoin Core Documentation
5. **Ed25519: High-speed high-security signatures** - Bernstein et al. (2011)

### 11.4 Change Log

| Version | Date | Changes |
|---------|------|---------|
| 1.0 | 2025-11-18 | Initial specification |

### 11.5 Contributors

- TIME Coin Core Development Team

---

**END OF SPECIFICATION**

**Document Status**: Final  
**Version**: 1.0  
**Last Updated**: 2025-11-18  
**License**: MIT

For implementation details, see:
- [TIME Coin Protocol Implementation](../consensus/src/utxo_state_protocol.rs)
- [TIME Coin Protocol Overview](../TIME_COIN_PROTOCOL.md)
- [Quick Start Guide](../TIME_COIN_PROTOCOL_QUICKSTART.md)
