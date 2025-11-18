# Masternode Collateral Implementation Status

## Overview

TIME Coin supports both free and collateral-based masternodes. This allows new users to participate in the network without initial investment while also supporting serious operators with locked collateral.

## Tier Structure

### Free Tier (No Collateral)
- **Collateral:** 0 TIME
- **Reward Weight:** 1x
- **Voting Rights:** ❌ Cannot vote on proposals
- **Rewards:** ✅ Earns modest block rewards
- **Available:** Testnet and Mainnet
- **Purpose:** Onboarding new users, testing, small-scale participation

### Bronze Tier (1,000 TIME)
- **Collateral:** 1,000 TIME (locked UTXO)
- **Reward Weight:** 10x (10x more than Free tier)
- **Voting Rights:** ✅ Can vote (weight: 1x)
- **Est. APY:** 35-180% (varies with network size)
- **Purpose:** Entry-level masternode operators

### Silver Tier (10,000 TIME)
- **Collateral:** 10,000 TIME (locked UTXO)
- **Reward Weight:** 100x (10x more than Bronze)
- **Voting Rights:** ✅ Can vote (weight: 10x)
- **Est. APY:** 35-180% (varies with network size)
- **Purpose:** Serious masternode operators

### Gold Tier (100,000 TIME)
- **Collateral:** 100,000 TIME (locked UTXO)
- **Reward Weight:** 1000x (10x more than Silver)
- **Voting Rights:** ✅ Can vote (weight: 100x)
- **Est. APY:** 35-180% (varies with network size)
- **Purpose:** Premium masternode operators

## Implementation Status

### ✅ Completed Features

1. **Tier Definitions**
   - Free, Bronze, Silver, Gold tiers defined in `core/src/block.rs`
   - Collateral requirements: 0, 1k, 10k, 100k TIME
   - Voting rights: Free tier cannot vote, all others can

2. **CLI Commands**
   - `time-cli masternode genkey` - Generate masternode private key
   - `time-cli masternode outputs` - List available collateral UTXOs
   - `time-cli masternode list-conf` - View masternode.conf entries
   - `time-cli masternode add-conf` - Add masternode to config
   - `time-cli masternode start-alias` - Start specific masternode
   - `time-cli masternode register` - Register Free tier masternode

3. **Configuration System**
   - `masternode.conf` file support
   - Dash-style config format: `alias IP:port privkey txid vout`
   - Load/save configuration files

4. **Collateral Validation (Basic)**
   - API accepts collateral_txid and collateral_vout
   - State validates UTXO exists before registration
   - Checks collateral amount matches tier requirement

### 🚧 Partially Implemented

1. **UTXO Locking**
   - Basic validation exists
   - Need to prevent spending locked collateral
   - Should track locked UTXOs in state
   - Need unlock mechanism when masternode decommissions

2. **Masternode Start Protocol**
   - Config file infrastructure exists
   - Need to broadcast start message
   - Need to verify collateral before activation

### ❌ To Be Implemented

1. **Collateral Transaction Tracking**
   - Track which UTXOs are locked as collateral
   - Reject transactions attempting to spend locked collateral
   - Handle collateral unlock after cooldown period

2. **Masternode Private Key**
   - Generate proper private keys (currently placeholder)
   - Sign messages with masternode key
   - Verify masternode signatures

3. **Start Message Broadcasting**
   - Broadcast masternode start to network
   - Include collateral proof in start message
   - Network-wide verification of collateral

4. **Collateral Cooldown**
   - Implement cooldown period (e.g., 24 hours)
   - Allow collateral unlock after decommission
   - Return collateral to owner after cooldown

## Current Registration Flows

### Free Tier (Simple Registration)

```bash
# No collateral needed
time-cli masternode register \
    --node-ip 192.168.1.100 \
    --wallet-address TIME0abc... \
    --tier Free
```

### Collateral Tier (Dash-Style)

```bash
# Step 1: Generate masternode key
time-cli masternode genkey
# Output: MN93HaYBVUCYjEMeeH1Y4sBGLALQZE1Yc1K64xiqgX37tGBDQL8Xg

# Step 2: Send collateral to yourself
time-cli wallet send --to TIME0abc... --amount 1000

# Step 3: Find collateral UTXO
time-cli masternode outputs --min-conf 15
# Output: 2bcd3c84c84f87ea:0 (1000 TIME, Bronze tier)

# Step 4: Add to masternode.conf
time-cli masternode add-conf \
    mn1 \
    192.168.1.100:24000 \
    MN93HaYBVUCYjEMeeH1Y4sBGLALQZE1Yc1K64xiqgX37tGBDQL8Xg \
    2bcd3c84c84f87ea... \
    0

# Step 5: Start masternode (with collateral validation)
time-cli masternode start-alias mn1
```

## API Endpoints

### Register Masternode (Enhanced)

**Endpoint:** `POST /masternode/register`

**Request (Free Tier):**
```json
{
  "node_ip": "192.168.1.100",
  "wallet_address": "TIME0abc...",
  "tier": "Free"
}
```

**Request (Collateral Tier):**
```json
{
  "node_ip": "192.168.1.100",
  "wallet_address": "TIME0abc...",
  "tier": "Bronze",
  "collateral_txid": "2bcd3c84c84f87ea...",
  "collateral_vout": 0
}
```

**Response:**
```json
{
  "success": true,
  "message": "Masternode registered successfully as Bronze tier",
  "node_ip": "192.168.1.100",
  "wallet_address": "TIME0abc...",
  "tier": "Bronze"
}
```

## Next Steps

1. **Implement UTXO Locking System**
   - Add locked_utxos tracking to BlockchainState
   - Reject transactions spending locked UTXOs
   - Unlock UTXOs after masternode decommission + cooldown

2. **Complete Masternode Key System**
   - Implement proper key generation (secp256k1)
   - Add signing and verification
   - Integrate with start protocol

3. **Add Start Message Protocol**
   - Define start message format
   - Broadcast to network
   - Verify collateral across network

4. **Testing**
   - Test Free tier on mainnet
   - Test collateral tier registration
   - Test collateral locking/unlocking
   - Test reward distribution across tiers

## Security Considerations

1. **Collateral Protection**
   - Locked UTXOs cannot be spent
   - Only masternode owner can unlock after cooldown
   - Network validates collateral exists

2. **Key Management**
   - Masternode private key separate from wallet key
   - Hot/cold wallet separation supported
   - Keys stored securely in masternode.conf

3. **Voting Rights**
   - Free tier explicitly cannot vote
   - Prevents sybil attacks via free nodes
   - Voting power scales with collateral

## Documentation

- [Dash-Style Setup Guide](./dash-style-setup.md)
- [Masternode Tiers](./TIERS.md)
- [Collateral Tiers Details](./collateral-tiers.md)
- [Setup Guide](./setup-guide.md)

## References

- Dash Masternode Documentation
- TIME Coin Whitepaper
- BFT Consensus Implementation
