# Genesis Blocks

## Overview

The genesis block is the first block in the TIME Coin blockchain. It defines the initial state and distribution of coins.

## Testnet Genesis

- **Network:** Testnet
- **Timestamp:** November 1, 2025 00:00:00 UTC (1730419200)
- **Message:** "TIME Coin Testnet - 24 Hour Blocks, Instant Finality"

### Initial Allocations

1. **Testnet Faucet:** 5,000,000 TIME (for testing purposes)

### Treasury Model

The treasury no longer uses a pre-allocated burn address. Instead:

- **Block Rewards**: All block rewards are minted on-demand and distributed directly to masternodes via coinbase transactions
- **Treasury Budget**: Treasury funds are tracked off-chain in the TreasuryPool as a budget authority
- **Governance Spending**: When governance approves spending, coins are minted directly to recipients in future coinbase transactions
- **No Burn Address**: This eliminates the problem of inaccessible funds in burn addresses

Block rewards are distributed as follows:
- Masternodes receive rewards based on their tier (Free, Bronze, Silver, Gold)
- Treasury budget is maintained separately and does not require pre-allocated coins
- All rewards go directly to masternode addresses (no burn address)

## Mainnet Genesis

- **Total Supply:** Coins minted on-demand via block rewards
- **Network:** Mainnet
- **Timestamp:** November 1, 2025 00:00:00 UTC (1730419200)
- **Message:** "TIME Coin - Where Every Second Counts"

### Initial Allocations

Minimal initial allocations for bootstrap purposes only.

### Treasury Model

Same as testnet - treasury operates as a budget authority with on-demand minting through governance-approved coinbase transactions.

## Port Configuration

### Official TIME Coin Ports (24-Hour Theme)

- **Mainnet:** 
  - P2P: 24000
  - RPC: 24001
  
- **Testnet:**
  - P2P: 24100
  - RPC: 24101

## Loading Genesis Block

Your node will automatically load the genesis block from the path specified in your config:

```toml
[blockchain]
genesis_file = "$HOME/time-coin-node/data/genesis-testnet.json"
```

The genesis block is verified on startup to ensure integrity.
