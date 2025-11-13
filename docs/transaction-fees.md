# Transaction Fee Collection and Distribution

## Overview

TIME Coin implements a comprehensive transaction fee system that collects fees from all transactions and distributes them to masternodes as part of the block reward. This document explains how transaction fees are calculated, collected, and distributed.

## How Transaction Fees Work

### Fee Calculation

Transaction fees in TIME Coin are calculated based on the difference between inputs and outputs:

```
Fee = Total Input Value - Total Output Value
```

**Example:**
- Alice has a UTXO worth 100 TIME
- She wants to send 80 TIME to Bob
- Transaction inputs: 100 TIME (from her UTXO)
- Transaction outputs: 80 TIME (to Bob) + 19.9 TIME (change to Alice)
- Transaction fee: 100 - (80 + 19.9) = **0.1 TIME**

### Fee Collection

Fees are collected automatically during block production:

1. **Block Producer**: Retrieves all transactions from the mempool
2. **Fee Calculation**: For each transaction, the fee is calculated using UTXO validation
3. **Aggregation**: All fees are summed to get `total_fees`
4. **Coinbase Inclusion**: Total fees are added to the coinbase transaction
5. **Distribution**: Fees are distributed to masternodes along with block rewards

## Implementation Details

### Block Producer Fee Collection

The block producer (`cli/src/block_producer.rs`) implements fee collection as follows:

```rust
// Calculate total transaction fees from mempool transactions
let mut total_fees: u64 = 0;
{
    let blockchain = self.blockchain.read().await;
    let utxo_map = blockchain.utxo_set().utxos();
    
    for tx in &transactions {
        match tx.fee(utxo_map) {
            Ok(fee) => {
                total_fees += fee;
                println!("📊 TX {} fee: {} satoshis", &tx.txid[..8], fee);
            }
            Err(e) => {
                println!("⚠️ Could not calculate fee for {}: {:?}", &tx.txid[..8], e);
                // Skip transaction if fee can't be calculated
            }
        }
    }
}

if total_fees > 0 {
    println!("💵 Total transaction fees: {} satoshis ({} TIME)", 
        total_fees, 
        total_fees as f64 / 100_000_000.0
    );
}
```

### Coinbase Transaction Creation

The coinbase transaction includes both block rewards and transaction fees:

```rust
let coinbase_tx = time_core::block::create_coinbase_transaction(
    block_num,
    &active_masternodes,
    &masternode_counts,
    total_fees,  // <-- Transaction fees included here
    block_timestamp,
);
```

### Fee Distribution

Transaction fees are distributed to the block producer (first masternode in sorted order):

```rust
// Transaction fees go to block producer (if any)
if transaction_fees > 0 && !masternode_list.is_empty() {
    // Give fees to the first masternode (block producer) after sorting
    if let Some((producer_address, _)) = masternode_list.first() {
        outputs.push(TxOutput::new(
            transaction_fees,
            producer_address.clone(),
        ));
    }
}
```

## Fee Structure

### Current Fee Model

TIME Coin uses a market-based fee model where users can set their own fees. Transactions with higher fees are prioritized by block producers.

**Recommended Fees:**
- **Standard Transaction**: 0.0001 TIME (10,000 satoshis)
- **Fast Transaction**: 0.001 TIME (100,000 satoshis)
- **Priority Transaction**: 0.01 TIME (1,000,000 satoshis)

### Minimum Fees

There is currently no enforced minimum fee, but transactions with zero fees may not be included in blocks as miners prioritize fee-paying transactions.

## Block Reward Composition

Each block's coinbase transaction contains:

1. **Masternode Rewards**: Calculated based on logarithmic scaling
2. **Transaction Fees**: Sum of all fees from included transactions

**Formula:**
```
Total Coinbase = Masternode Rewards + Transaction Fees
```

**Example Block:**
```
Block #1000
- Masternode Rewards: 2000 TIME
- Transaction Fees: 15.5 TIME (from 155 transactions)
- Total Coinbase: 2015.5 TIME
```

## Fee Distribution to Masternodes

### Standard Blocks

For regular blocks with mempool transactions:
- **Masternode Rewards**: Distributed to all active masternodes based on tier weight
- **Transaction Fees**: Given to the block producer

### Reward-Only Blocks

For blocks with no mempool transactions:
- **Masternode Rewards**: Distributed to all active masternodes
- **Transaction Fees**: 0 (no transactions to collect fees from)

## Monitoring Transaction Fees

### Via Node Logs

Transaction fees are logged during block production:

```
📋 5 mempool transactions
   📊 TX ab12cd34 fee: 10000 satoshis
   📊 TX ef56gh78 fee: 15000 satoshis
   📊 TX ij90kl12 fee: 20000 satoshis
   📊 TX mn34op56 fee: 12000 satoshis
   📊 TX qr78st90 fee: 18000 satoshis
   💵 Total transaction fees: 75000 satoshis (0.00075 TIME)
```

### Via API

Check block information to see fees:

```bash
# Get block by height
curl http://localhost:24101/blockchain/block/1000

# Response includes coinbase transaction with fees
{
  "block": {
    "header": { ... },
    "transactions": [
      {
        "txid": "coinbase_1000",
        "inputs": [],
        "outputs": [
          { "address": "masternode_1", "amount": 500000000 },
          { "address": "masternode_2", "amount": 500000000 },
          ...
          { "address": "masternode_1", "amount": 75000 }  // <-- Fees
        ]
      },
      ...
    ]
  }
}
```

### Via RPC

```bash
# Get time block info (includes fee data)
curl -X POST http://localhost:24101/rpc/gettimeblockinfo \
  -H "Content-Type: application/json" \
  -d '{}'
```

## Fee Economics

### Impact on Masternodes

Transaction fees provide additional income to masternodes, incentivizing:
- ✅ **Network Participation**: More active masternodes
- ✅ **Transaction Validation**: Proper UTXO validation
- ✅ **Block Production**: Timely block creation
- ✅ **Network Security**: Honest behavior for fee rewards

### Impact on Users

Users benefit from the fee system through:
- ✅ **Priority Transactions**: Pay more for faster inclusion
- ✅ **Network Sustainability**: Fees support node operators
- ✅ **Fair Access**: Anyone can include transactions by paying fees
- ✅ **Spam Prevention**: Small fees deter network abuse

## Technical Specifications

### Fee Validation

Fees are validated during block validation:

```rust
// Calculate total fees from regular transactions
let mut total_fees = 0u64;
for tx in self.regular_transactions() {
    let fee = tx.fee(utxo_set.utxos())?;
    total_fees += fee;
}

// Coinbase should be masternode rewards + fees
let max_coinbase = total_masternode_reward + total_fees;
if coinbase_total > max_coinbase {
    return Err(BlockError::InvalidCoinbase);
}
```

### UTXO-Based Calculation

Fees are calculated using the UTXO set:

```rust
pub fn fee(
    &self,
    utxo_set: &std::collections::HashMap<OutPoint, TxOutput>,
) -> Result<u64, TransactionError> {
    let input_total = self.total_input(utxo_set)?;
    let output_total = self.total_output()?;
    
    input_total
        .checked_sub(output_total)
        .ok_or(TransactionError::InsufficientFunds)
}
```

## Best Practices

### For Users

1. **Include Appropriate Fees**: Use recommended fee levels for timely inclusion
2. **Check UTXO Values**: Ensure inputs cover outputs + fees
3. **Avoid Dust**: Don't create outputs that are smaller than typical fees
4. **Monitor Mempool**: Check mempool size to adjust fees accordingly

### For Block Producers

1. **Validate Fees**: Always validate transaction fees before inclusion
2. **Prioritize High Fees**: Include high-fee transactions first
3. **Log Fee Collection**: Monitor total fees for auditing
4. **Verify Coinbase**: Ensure coinbase amount doesn't exceed rewards + fees

### For Masternode Operators

1. **Track Fee Income**: Monitor additional income from fees
2. **Maintain Uptime**: Be available to produce blocks and earn fees
3. **Validate Transactions**: Proper validation ensures fee collection
4. **Monitor Network**: Watch for unusual fee patterns

## Troubleshooting

### Issue: "Could not calculate fee for transaction"

**Cause**: Transaction references UTXOs that don't exist

**Solutions:**
- Wait for previous transactions to confirm
- Check that input UTXOs are valid
- Verify the UTXO set is synchronized

### Issue: Low fee transactions not included

**Cause**: Block producer prioritizes higher fee transactions

**Solutions:**
- Increase transaction fee
- Wait for next block
- Check if mempool is full

### Issue: Fee calculation errors in logs

**Cause**: Invalid transaction inputs or missing UTXOs

**Solutions:**
- Investigate transaction structure
- Verify UTXO set integrity
- Check blockchain synchronization

## Examples

### Example 1: Simple Transaction with Fee

```rust
// User has 100 TIME UTXO
// Wants to send 50 TIME with 0.01 TIME fee

let input = TxInput::new(
    "previous_tx_id".to_string(),
    0,  // vout
    public_key,
    signature,
);

let outputs = vec![
    TxOutput::new(50_00_000_000, "recipient_address".to_string()),  // 50 TIME
    TxOutput::new(49_99_000_000, "sender_change_address".to_string()), // 49.99 TIME
];

// Total inputs: 100 TIME
// Total outputs: 50 + 49.99 = 99.99 TIME
// Fee: 100 - 99.99 = 0.01 TIME
```

### Example 2: Block with Multiple Transaction Fees

```
Block #500
├── Coinbase Transaction
│   ├── Masternode Rewards: 2000 TIME
│   └── Transaction Fees: 2.5 TIME
│       ├── TX 1 fee: 0.5 TIME
│       ├── TX 2 fee: 0.8 TIME
│       ├── TX 3 fee: 0.3 TIME
│       ├── TX 4 fee: 0.4 TIME
│       └── TX 5 fee: 0.5 TIME
└── Regular Transactions
    ├── Transaction 1 (0.5 TIME fee)
    ├── Transaction 2 (0.8 TIME fee)
    ├── Transaction 3 (0.3 TIME fee)
    ├── Transaction 4 (0.4 TIME fee)
    └── Transaction 5 (0.5 TIME fee)
```

## Related Documentation

- [Block Rewards Guide](block-rewards.md) - Understanding block rewards
- [UTXO Model](utxo-model.md) - How the UTXO set works
- [Masternode Economics](masternodes/economics.md) - Masternode reward structure
- [Transaction Guide](transactions.md) - Creating and sending transactions

## References

- **Issue #132**: Transaction Fees Collected and Added to Masternode Rewards Pool
- **Implementation**: `cli/src/block_producer.rs`
- **Core Logic**: `core/src/block.rs` and `core/src/transaction.rs`
- **Tests**: `core/src/block.rs` (test suite)
