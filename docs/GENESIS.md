# Genesis Blocks

## Overview

The genesis block is the first block in the TIME Coin blockchain. It defines the initial state and distribution of coins.

## Testnet Genesis

- **Total Supply:** 1,000,000 TIME
- **Network:** Testnet
- **Timestamp:** October 17, 2025 00:00:00 UTC
- **Message:** "TIME Coin Testnet - Testing the Future"

### Allocations

1. **Treasury:** 100,000 TIME (10%)
2. **Development:** 50,000 TIME (5%)
3. **Masternode Pool:** 850,000 TIME (85%)

## Mainnet Genesis

- **Total Supply:** 21,000,000 TIME
- **Network:** Mainnet
- **Timestamp:** October 17, 2025 00:00:00 UTC
- **Message:** "TIME Coin - Where Every Second Counts"

### Allocations

1. **Treasury:** 2,100,000 TIME (10%)
2. **Core Development:** 1,050,000 TIME (5%, 48-month vesting)
3. **Marketing & Growth:** 420,000 TIME (2%, 36-month vesting)
4. **Masternode Pool:** 17,430,000 TIME (83%)

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
