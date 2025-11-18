# Masternode Collateral System - Implementation Summary

## Changes Made

### 1. API Layer (`api/src/masternode_handlers.rs`)

**Enhanced RegisterMasternodeRequest:**
- Added `collateral_txid: Option<String>` - Transaction ID of collateral UTXO
- Added `collateral_vout: Option<u32>` - Output index of collateral UTXO
- Made these required for Bronze/Silver/Gold tiers
- Free tier doesn't need collateral fields

**Validation Logic:**
```rust
// For non-Free tiers, require collateral proof
if tier != MasternodeTier::Free {
    if req.collateral_txid.is_none() || req.collateral_vout.is_none() {
        return Err(ApiError::InvalidAddress(
            format!("{} tier requires collateral_txid and collateral_vout", req.tier)
        ));
    }
}
```

### 2. Core State Layer (`core/src/state.rs`)

**Enhanced register_masternode():**
- For non-Free tiers, validates collateral UTXO exists
- Checks UTXO amount meets tier requirements
- Uses OutPoint to lookup UTXO in the UTXO set
- Returns error if collateral is invalid

**Validation Logic:**
```rust
// Parse collateral format: "txid:vout"
let outpoint = OutPoint::new(txid, vout);
let utxo_valid = self.utxo_set.get(&outpoint)
    .map(|utxo| utxo.amount >= required_collateral)
    .unwrap_or(false);

if !utxo_valid {
    return Err(StateError::InvalidMasternodeCount);
}
```

### 3. Documentation

**Created COLLATERAL_IMPLEMENTATION.md:**
- Complete tier structure documentation
- Implementation status checklist  
- Current registration flows for Free and collateral tiers
- API endpoint documentation
- Next steps and TODO items
- Security considerations

## How It Works Now

### Free Tier Registration (No Changes Required)

```bash
# Simple registration - no collateral needed
curl -X POST http://localhost:24101/masternode/register \
  -H "Content-Type: application/json" \
  -d '{
    "node_ip": "192.168.1.100",
    "wallet_address": "TIME0abc...",
    "tier": "Free"
  }'
```

**What happens:**
1. API accepts request without collateral fields
2. State registers masternode with tier = Free
3. Masternode can earn rewards (weight: 1x)
4. Masternode CANNOT vote on proposals
5. No collateral validation required

### Bronze/Silver/Gold Tier Registration (New)

```bash
# Step 1: Generate masternode key
time-cli masternode genkey
# Output: MN93HaYBVUCYjEMeeH1Y4sBGLALQZE1Yc1K64xiqgX37tGBDQL8Xg

# Step 2: Send collateral to yourself
time-cli wallet send --to TIME0myaddress --amount 1000

# Step 3: Find the collateral UTXO
time-cli masternode outputs --min-conf 15
# Output: 2bcd3c84c84f87eaa86e4e56834c92927a07f9e18718810b92e0d0324456a67c:0

# Step 4: Register with collateral proof
curl -X POST http://localhost:24101/masternode/register \
  -H "Content-Type: application/json" \
  -d '{
    "node_ip": "192.168.1.100",
    "wallet_address": "TIME0myaddress",
    "tier": "Bronze",
    "collateral_txid": "2bcd3c84c84f87eaa86e4e56834c92927a07f9e18718810b92e0d0324456a67c",
    "collateral_vout": 0
  }'
```

**What happens:**
1. API validates collateral_txid and collateral_vout are provided
2. State creates OutPoint from txid:vout
3. State looks up UTXO in the UTXO set
4. State validates UTXO amount >= tier requirement (1,000 TIME for Bronze)
5. If valid, masternode is registered with collateral tier
6. Masternode can earn rewards (weight: 10x) AND vote on proposals

## Key Features

### ✅ Implemented

1. **Free tier for all networks** - Mainnet and testnet both support Free tier
2. **Voting restrictions** - Free tier cannot vote, already implemented in `block.rs`
3. **Collateral validation** - Bronze/Silver/Gold require valid UTXO
4. **UTXO verification** - Checks existence and amount before registration
5. **API validation** - Rejects collateral tiers without txid:vout
6. **CLI tools** - `genkey`, `outputs`, config management already exist

### 🚧 Partially Implemented

1. **UTXO locking** - Validation works, but collateral can still be spent
2. **Masternode keys** - genkey exists but needs crypto implementation

### ❌ Not Yet Implemented

1. **UTXO locking mechanism** - Prevent spending locked collateral
2. **Collateral cooldown** - Return collateral after decommission + waiting period
3. **Network-wide collateral verification** - Broadcast and verify across nodes
4. **Signature verification** - Verify messages signed with masternode key

## Testing the Implementation

### Test Free Tier Registration

```bash
# Register a Free tier masternode (works on testnet and mainnet)
time-cli masternode register \
    --node-ip 192.168.1.100 \
    --wallet-address TIME0test123 \
    --tier Free

# Should succeed without any collateral
```

### Test Bronze Tier Registration (Will Validate Collateral)

```bash
# Create collateral transaction
time-cli wallet send --to TIME0myaddress --amount 1000

# Wait for confirmation (check with: time-cli wallet get-balance)

# Find the UTXO
time-cli masternode outputs --min-conf 1

# Try to register (will fail if UTXO doesn't exist or amount is wrong)
curl -X POST http://localhost:24101/masternode/register \
  -H "Content-Type: application/json" \
  -d '{
    "node_ip": "192.168.1.100",
    "wallet_address": "TIME0myaddress",
    "tier": "Bronze",
    "collateral_txid": "YOUR_TXID_HERE",
    "collateral_vout": 0
  }'
```

### Test Collateral Validation (Should Fail)

```bash
# Try Bronze tier without collateral (should fail)
curl -X POST http://localhost:24101/masternode/register \
  -H "Content-Type: application/json" \
  -d '{
    "node_ip": "192.168.1.100",
    "wallet_address": "TIME0test",
    "tier": "Bronze"
  }'
# Expected: Error "Bronze tier requires collateral_txid and collateral_vout"

# Try with fake collateral (should fail)
curl -X POST http://localhost:24101/masternode/register \
  -H "Content-Type: application/json" \
  -d '{
    "node_ip": "192.168.1.100",
    "wallet_address": "TIME0test",
    "tier": "Bronze",
    "collateral_txid": "0000000000000000000000000000000000000000000000000000000000000000",
    "collateral_vout": 0
  }'
# Expected: Error "InvalidMasternodeCount" (UTXO not found)
```

## Next Steps for Full Implementation

### 1. UTXO Locking System (High Priority)

Add to `core/src/state.rs`:
```rust
pub struct BlockchainState {
    // ... existing fields ...
    locked_utxos: HashSet<OutPoint>,  // Track locked collateral
}

impl BlockchainState {
    fn lock_collateral(&mut self, outpoint: OutPoint) {
        self.locked_utxos.insert(outpoint);
    }
    
    fn is_locked(&self, outpoint: &OutPoint) -> bool {
        self.locked_utxos.contains(outpoint)
    }
}
```

Update transaction validation to reject spending locked UTXOs.

### 2. Collateral Cooldown

Add decommission workflow:
1. Masternode owner requests decommission
2. Masternode status set to "Decommissioning"
3. After cooldown period (24 hours), collateral unlocked
4. Owner can spend collateral again

### 3. Network Verification

Broadcast collateral proof to network:
- All nodes verify collateral exists
- Network rejects invalid masternodes
- Periodic revalidation of collateral

### 4. Masternode Keys

Implement proper secp256k1 key generation:
- Generate private/public keypair
- Sign messages with private key
- Network verifies with public key

## Compatibility

- ✅ Backwards compatible with existing Free tier masternodes
- ✅ Works on testnet and mainnet
- ✅ Existing CLI commands still work
- ✅ Free tier can coexist with collateral tiers
- ✅ Voting restrictions already enforced

## Questions & Answers

**Q: Can Free tier masternodes exist on mainnet?**
A: Yes! Free tier works on both testnet and mainnet. They earn rewards but cannot vote.

**Q: What prevents someone from spending their collateral after registration?**
A: Currently nothing - this is the next feature to implement (UTXO locking).

**Q: Can a Free tier masternode upgrade to Bronze?**
A: Not automatically. Need to deregister and re-register with collateral.

**Q: How are rewards calculated?**
A: Weights scale 10x per tier. Free=1x, Bronze=10x, Silver=100x, Gold=1000x. All collateral tiers earn similar APY.

**Q: Who validates the collateral?**
A: Currently only the registering node validates. Need network-wide verification for production.

## Files Modified

1. `api/src/masternode_handlers.rs` - Enhanced API to accept collateral proof
2. `core/src/state.rs` - Added collateral validation logic
3. `docs/masternodes/COLLATERAL_IMPLEMENTATION.md` - Complete documentation
4. `docs/masternodes/IMPLEMENTATION_SUMMARY.md` - This file

## References

- [Dash Masternode System](https://docs.dash.org/en/stable/masternodes/setup.html)
- [TIME Coin Dash-Style Setup Guide](./dash-style-setup.md)
- [Masternode Tiers Documentation](./TIERS.md)
