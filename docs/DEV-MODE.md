# Dev Mode - Single Node Testing

## Overview

Dev mode allows you to run a single TIME Coin node for development and testing without requiring multiple masternodes for BFT consensus.

## Features

### In Dev Mode:
- ✅ Single node can validate transactions
- ✅ Auto-approves all valid transactions
- ✅ Bypasses BFT quorum requirements
- ✅ Perfect for development and testing
- ✅ Full transaction processing pipeline works
- ⚠️ **NOT for production use**

### In Production Mode:
- ✅ Full BFT consensus required
- ✅ Minimum 7 masternodes for quorum
- ✅ 2/3+ votes needed for approval
- ✅ Byzantine fault tolerance
- ✅ Secure and decentralized

## Enabling Dev Mode

### Method 1: Config File

Edit `config/testnet.toml`:

```toml
[node]
mode = "dev"  # Set to "production" for real consensus

[consensus]
dev_mode = true
auto_approve = true
```

### Method 2: Command Line Flag

```bash
timed --dev
```

### Method 3: Environment Variable

```bash
export TIME_NODE_DEV_MODE=true
timed
```

## What Dev Mode Does

### Transaction Flow

**Production Mode:**
```
TX → Validate → Select Quorum (7-50 nodes) → Vote → 2/3+ Approve → Confirm
                                               ↓
                                          Takes 100-500ms
```

**Dev Mode:**
```
TX → Validate → Auto-Approve → Confirm
                      ↓
                 < 1ms
```

### Consensus Behavior

| Feature | Production | Dev Mode |
|---------|-----------|----------|
| Minimum nodes | 7 | 1 |
| Voting required | Yes | No |
| BFT consensus | Yes | Bypassed |
| Auto-approve | No | Yes |
| Quorum selection | VRF-based | N/A |
| Byzantine tolerance | f < n/3 | None |

## Testing Transactions in Dev Mode

```bash
# 1. Start node in dev mode
cd ~/time-coin
cargo run --release --bin timed -- --dev

# 2. Create a transaction (in another terminal)
# This will auto-approve in dev mode

# 3. Check logs - you'll see:
#    "✓ Dev mode: Single-node consensus active"
#    "(dev mode)" in heartbeat messages
```

## Security Warnings

### ⚠️ Dev Mode is NOT secure for:
- Production deployments
- Real money / mainnet
- Multi-user environments
- Public networks
- Any critical applications

### ✅ Dev Mode is SAFE for:
- Local development
- Feature testing
- Integration testing
- Learning the system
- Debugging

## Switching Between Modes

### Dev → Production

```toml
# config/testnet.toml
[node]
mode = "production"  # Changed from "dev"

[consensus]
dev_mode = false     # Changed from true
```

Restart the node. It will now require real BFT consensus.

### Production → Dev

```toml
# config/testnet.toml
[node]
mode = "dev"

[consensus]
dev_mode = true
```

Restart the node. Transactions will auto-approve.

## Visual Indicators

When running in dev mode, you'll see:

```
TIME Coin Node v0.1.0
Config file: "testnet.toml"

⚠️  DEV MODE ENABLED
   Single-node testing - Auto-approving transactions

🚀 Starting TIME node...
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

✓ Genesis block verified
✓ Blockchain initialized
✓ Peer discovery started
✓ Masternode services starting
✓ Dev mode: Single-node consensus active

Node Status: ACTIVE
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

[2025-10-17 17:30:00] Node heartbeat - running... #1 (dev mode)
```

## FAQ

### Q: Can I test the full system with one node?
**A:** Yes! Dev mode lets you test everything except actual BFT voting.

### Q: Will transactions work in dev mode?
**A:** Yes! All transactions that pass validation will auto-approve.

### Q: Is dev mode safe for testnet?
**A:** Yes, testnet funds have no real value. Dev mode is perfect for testing.

### Q: Can I use dev mode on mainnet?
**A:** **NO!** Dev mode bypasses security. Only use on testnet/development.

### Q: How do I know if dev mode is enabled?
**A:** Look for yellow "DEV MODE ENABLED" message and "(dev mode)" in logs.

### Q: Can I switch modes without restarting?
**A:** No, you must restart the node after changing the mode.

## Examples

### Single Node Development

```bash
# Perfect for developing features
timed --dev

# Test transactions without other nodes
# All valid TXs auto-approve
```

### Multi-Node Testing (Still Dev Mode)

```bash
# Run 3 nodes for testing P2P
timed --dev --port 24100 --data-dir ./node1
timed --dev --port 24200 --data-dir ./node2
timed --dev --port 24300 --data-dir ./node3

# Each auto-approves, tests networking
```

### Production Deployment

```bash
# Disable dev mode
# Ensure min 7 masternodes running
# Full BFT consensus active
timed --config production.toml
```
