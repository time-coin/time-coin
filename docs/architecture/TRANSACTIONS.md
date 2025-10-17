# Transaction Processing

## Overview

TIME Coin transactions are instant-finality transfers validated by BFT consensus.

## Transaction Lifecycle

```
1. Create Transaction
   ├─ Sender address
   ├─ Recipient address
   ├─ Amount
   ├─ Fee
   └─ Nonce

2. Sign Transaction
   └─ Ed25519 signature with private key

3. Submit to Network
   └─ Broadcast to masternodes

4. BFT Consensus
   ├─ Select quorum (7-50 nodes)
   ├─ Masternodes validate
   ├─ Cast votes
   └─ 2/3+ approval required

5. Instant Confirmation
   └─ Added to mempool → Daily state

6. Daily Finalization
   └─ Included in 24-hour block
```

## Creating a Transaction

### Using TransactionBuilder

```rust
use time_core::TransactionBuilder;
use time_crypto::KeyPair;

// Generate or load keypair
let keypair = KeyPair::generate();
let from_address = keypair.public_key_hex();

// Build transaction
let mut tx = TransactionBuilder::new()
    .from(from_address.clone())
    .to("recipient_address".to_string())
    .amount(100_00000000)  // 100 TIME
    .fee(1_00000000)       // 1 TIME
    .nonce(0)
    .build();

// Sign transaction
let message = tx.signable_message();
let signature = keypair.sign(&message);
tx.sign(signature);
```

## Validation Rules

### Amount Validation
- Amount must be > 0
- Fee must be > 0
- Total (amount + fee) ≤ sender balance

### Address Validation
- Sender != Recipient (no self-transfers)
- Addresses must be valid format

### Signature Validation
- Must be valid Ed25519 signature
- Must sign correct transaction data
- Public key must match sender address

### Balance Validation
- Sender must have sufficient balance
- Balance checked against current state
- Prevents double-spending

## Transaction Fees

### Fee Structure
```
Minimum: 0.01 TIME (1_000_000 satoshis)
Recommended: 0.1% of amount
Priority: Higher fee = faster processing
```

### Fee Distribution
- 90% → Validating masternodes
- 10% → Treasury

## Mempool

### Purpose
Hold pending transactions until finalization

### Limits
- Maximum: 10,000 transactions
- FIFO processing order
- Expires after 24 hours

### Operations
```rust
use time_core::TransactionPool;

let mut mempool = TransactionPool::new();

// Add transaction
mempool.add(tx)?;

// Get next for processing
let next_tx = mempool.next();

// Remove specific transaction
mempool.remove(&txid);
```

## BFT Validation

### Process
1. Transaction enters mempool
2. Quorum selected (weighted by tier)
3. Masternodes validate:
   - Signature valid?
   - Balance sufficient?
   - No double-spend?
4. Cast votes (approve/reject)
5. Collect votes (need 2/3+)
6. Confirmed or rejected

### Timing
- Quorum selection: <1ms
- Vote collection: 100-500ms
- Total finality: <1 second

## Double-Spend Prevention

### In-Memory Protection
- Check current state before adding to mempool
- Track pending transactions per address
- Reject conflicting transactions

### Consensus Protection
- BFT voting ensures agreement
- 2/3+ masternodes must approve
- Malicious nodes cannot double-spend

## State Updates

### On Confirmation
```rust
// Update balances
state.balances[sender] -= (amount + fee);
state.balances[recipient] += amount;

// Add to confirmed transactions
state.transactions.push(tx);

// Remove from mempool
mempool.remove(&txid);
```

### At Midnight (Finalization)
```rust
// Create block with all daily transactions
let block = BlockFinalizer::finalize(&state, prev_hash);

// Save to disk
storage.save_block(&block)?;

// Clear daily state
state = DailyState::new(height + 1);
```

## Testing Transactions

```bash
# Run transaction tests
cargo test --package time-core transaction

# Run mempool tests
cargo test --package time-core mempool

# Run full validation
cargo test --all
```

## Example: Complete Flow

```rust
// 1. Create and sign
let keypair = KeyPair::generate();
let mut tx = TransactionBuilder::new()
    .from(keypair.public_key_hex())
    .to("recipient".to_string())
    .amount(100_00000000)
    .fee(1_00000000)
    .build();

let sig = keypair.sign(&tx.signable_message());
tx.sign(sig);

// 2. Validate
TransactionValidator::validate(&tx, &state)?;

// 3. Add to mempool
mempool.add(tx.clone())?;

// 4. BFT consensus (in real implementation)
let result = consensus_engine.validate_transaction(&tx, &masternodes)?;

if result.approved {
    // 5. Confirm transaction
    state.confirm_transaction(&tx.txid);
    
    println!("✓ Transaction confirmed!");
}
```

## Security Considerations

### Signature Security
- Ed25519 provides 128-bit security
- Private keys never transmitted
- Signatures are deterministic

### Balance Security
- All checks happen in BFT consensus
- Multiple masternodes verify
- Byzantine fault tolerant

### Replay Protection
- Nonces prevent transaction replay
- Timestamps ensure freshness
- Transaction IDs are unique

## Performance

### Throughput
- Parallel BFT quorums
- Thousands of TPS possible
- Limited by network, not validation

### Latency
- Validation: <1ms per transaction
- BFT consensus: 100-500ms
- Total confirmation: <1 second

## Future Enhancements

- Batch transactions
- Atomic swaps
- Multi-signature support
- Smart contract integration
