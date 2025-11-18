# TIME Coin Protocol Demo

This demo showcases the **TIME Coin Protocol** for instant finality - TIME Coin's innovative approach to real-time UTXO state tracking and consensus-based transaction validation.

## What It Demonstrates

1. **UTXO State Tracking** - Real-time monitoring of UTXO lifecycle
2. **Instant Finality** - Sub-3-second transaction confirmation
3. **Double-Spend Prevention** - UTXO locking mechanism
4. **State Notifications** - Push notifications for state changes
5. **Subscription Model** - Address and UTXO watching

## Running the Demo

```bash
cargo run --bin utxo-protocol-demo
```

## Demo Flow

### Step 1: Create Initial UTXOs
- Creates a genesis UTXO with 10,000 TIME for Alice

### Step 2: Set Up Subscriptions
- Wallet subscribes to Alice and Bob's addresses
- Receives real-time state change notifications

### Step 3: Create Transaction
- Alice sends 6,000 TIME to Bob
- 3,950 TIME returned as change
- 50 TIME transaction fee

### Step 4: Lock UTXOs
- Input UTXO locked immediately
- Prevents double-spend attacks

### Step 5: Masternode Voting
- 3 masternodes validate transaction
- 2 votes (67%) required for consensus
- Instant finality achieved in <3 seconds

### Step 6: Create New UTXOs
- Transaction outputs become new UTXOs
- Bob receives 6,000 TIME
- Alice receives 3,950 TIME change

### Step 7: Final State
- Display statistics
- Show updated balances
- Confirm instant finality

## Expected Output

```
═══════════════════════════════════════════════════════════
  TIME Coin - UTXO State Protocol Demo
  Instant Finality with Real-Time State Tracking
═══════════════════════════════════════════════════════════

📊 Network Configuration:
   Masternodes: 3
   Quorum: 67% (2 out of 3 votes)

🔔 UTXO State Change Notification:
   OutPoint: genesis_tx:0
   Previous State: Unspent
   New State: Locked { ... }

🎉 ════════════════════════════════════════════════════
🎉 INSTANT FINALITY ACHIEVED!
🎉 ════════════════════════════════════════════════════
   Votes: 2/3
   Percentage: 66.7%
   Time: <3 seconds
   Status: IRREVERSIBLE
```

## Key Features Demonstrated

- **Sub-3-second finality**: Faster than credit cards
- **Byzantine fault tolerance**: 67%+ consensus required
- **Double-spend prevention**: Lock-based protection
- **Real-time notifications**: Instant state updates
- **UTXO state tracking**: Complete lifecycle monitoring

## Architecture

```
Transaction Submission
        ↓
    UTXO Locked ─────────→ State Notification
        ↓
  Masternode Voting
        ↓
    Quorum Reached
        ↓
  Instant Finality ─────→ State Notification
        ↓
   Block Inclusion
        ↓
   Confirmed ─────────────→ State Notification
```

## Use Cases

- **Point of Sale**: Instant payment confirmation
- **Exchanges**: Fast deposit/withdrawal confirmation
- **Wallets**: Real-time balance updates
- **Payment Processors**: Sub-3-second settlement

## Learn More

- [TIME Coin Protocol Documentation](../../docs/time-coin-protocol.md)
- [TIME Coin Protocol Implementation](../../consensus/src/utxo_state_protocol.rs)
- [Instant Finality System](../../consensus/src/instant_finality.rs)
- [UTXO Set Management](../../core/src/utxo_set.rs)
