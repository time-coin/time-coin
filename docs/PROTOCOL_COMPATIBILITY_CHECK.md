# Protocol Compatibility Investigation - Nov 26, 2025

## Current Protocol Configuration

### Protocol Version
```rust
pub const PROTOCOL_VERSION: u32 = 1;
```

### Magic Bytes
```rust
// Testnet (what your nodes should use)
pub const TESTNET: [u8; 4] = [0x7E, 0x57, 0x7E, 0x4D]; // "TEST TIME"

// Mainnet (for reference)
pub const MAINNET: [u8; 4] = [0xC0, 0x1D, 0x7E, 0x4D]; // "COLD TIME"
```

## Handshake Flow

### What Should Happen
```
Node A                                    Node B
   │                                         │
   │─────── Magic Bytes (4 bytes) ─────────>│
   │─────── Length (4 bytes) ──────────────>│
   │─────── Handshake JSON ────────────────>│
   │                                         │
   │<────── Magic Bytes (4 bytes) ──────────│
   │<────── Length (4 bytes) ───────────────│
   │<────── Handshake JSON ─────────────────│
   │                                         │
   ✅ Connection established
```

### Handshake Message Structure
```json
{
  "node_id": "50.28.104.50",
  "version": "0.1.0-345b616",
  "protocol_version": 1,
  "network": "testnet",
  "blockchain_height": 45,
  "genesis_hash": "9a81c7599d8eed97...",
  "timestamp": 1732578000
}
```

## What's Breaking Consensus

### The Problem
From the logs, we see:
- **Broken pipe** errors
- **Connection refused** errors  
- Only 1/6 votes received despite all nodes creating same block

This indicates **network connectivity issues**, not protocol mismatch.

### Why Votes Aren't Being Exchanged

#### Symptom 1: Broken Pipe
```
✗ 69.167.168.176 did NOT respond to TCP ping: 
  Failed to send via stored connection: 
  Failed to write length: Broken pipe (os error 32)
```

**Cause:** Connection was established but then died (node crashed, restarted, or connection timeout)

#### Symptom 2: Connection Refused
```
✗ Failed to reconnect to 165.232.154.150:55352: 
  Connect failed: Connection refused (os error 111)
```

**Cause:** 
- Node is down
- Firewall blocking
- Wrong port
- Node not listening on that interface

## Node Version Status

Based on logs, nodes are running **OLD versions**:

| Node IP | Current Version | Status |
|---------|----------------|--------|
| 134.199.175.106 | ? | Unknown |
| 161.35.129.70 | b5c8c14 | ❌ OLD |
| 165.232.154.150 | b5c8c14 | ❌ OLD |
| 178.128.199.144 | ? | Unknown |
| 50.28.104.50 | b5c8c14 | ❌ OLD |
| 69.167.168.176 | ? | Unknown |

**Required version:** `345b616` or later

## Protocol Compatibility Check

### Check If Nodes Have Same Protocol

Run this to verify protocol compatibility:
```bash
./scripts/verify-protocol.sh
```

Or manually check each node:
```bash
# Check node protocol version
curl -s http://134.199.175.106:24101/rpc/getinfo | jq

# Should show:
# {
#   "version": "0.1.0-345b616",  ← Latest commit
#   "protocol_version": 1,        ← Must be 1
#   "network": "testnet",         ← Must be testnet
#   ...
# }
```

## What To Check

### 1. Version Mismatch
```bash
# On each node:
time-cli --api http://localhost:24101 rpc getinfo | grep version
```

**Expected:** All nodes show `0.1.0-345b616` or later

### 2. Network Mismatch
```bash
# Check network type
time-cli --api http://localhost:24101 rpc getinfo | grep network
```

**Expected:** All nodes show `"network": "testnet"`

### 3. Firewall Issues
```bash
# Test connectivity from each node to others
nc -zv 69.167.168.176 24100
nc -zv 165.232.154.150 24100
nc -zv 161.35.129.70 24100
nc -zv 50.28.104.50 24100
nc -zv 178.128.199.144 24100
nc -zv 134.199.175.106 24100
```

**Expected:** All connections should succeed

### 4. Check Listening Ports
```bash
# On each node:
sudo netstat -tlnp | grep 24100
```

**Expected:** 
```
tcp  0  0  0.0.0.0:24100  0.0.0.0:*  LISTEN  1234/timed
```

## Most Likely Root Causes

### 1. ✅ OLD CODE (Most Likely)
**Evidence:**
- Logs show nodes running `b5c8c14` (before consensus fixes)
- Latest is `345b616` (with all fixes)
- Old code has emergency fallback that causes chain splits

**Fix:**
```bash
# On each node:
cd ~/time-coin-node
sudo ./scripts/update-node.sh
```

### 2. Network Partitions
**Evidence:**
- "Broken pipe" errors
- "Connection refused" errors
- Only 1/6 votes received

**Fix:**
```bash
# Check firewall on each node
sudo ufw status
sudo ufw allow 24100/tcp

# Restart nodes after updating
sudo systemctl restart timed
```

### 3. Nodes Behind NAT
**Evidence:**
- Some nodes might be behind firewalls
- Port forwarding not configured

**Fix:**
- Ensure port 24100 is forwarded to each node
- Or use VPN/tunnel for direct connectivity

## Quick Fix Steps

1. **Update All Nodes** (CRITICAL)
```bash
# SSH to each node and run:
cd ~/time-coin-node
sudo ./scripts/update-node.sh
```

2. **Verify Versions Match**
```bash
./scripts/verify-protocol.sh
```

3. **Check Network Connectivity**
```bash
# From any node, test others:
nc -zv 69.167.168.176 24100
nc -zv 165.232.154.150 24100
# etc...
```

4. **Monitor Consensus**
```bash
# Watch logs on one node:
sudo journalctl -u timed -f | grep -E "votes|consensus|FALLBACK"
```

**Expected after fix:**
```
📊 Final tally: 4/6 votes (needed 4)  ← Or 5/6, 6/6
✅ CONSENSUS REACHED
```

## Summary

The **protocol itself is fine** (version 1, proper magic bytes).

The **problem is**:
1. ❌ Nodes running old code (b5c8c14 instead of 345b616)
2. ❌ Network connectivity issues (broken pipes, connection refused)
3. ❌ Emergency fallback causing chain splits (removed in 76a94d8)

**Solution:**
1. Deploy latest code (`345b616`) to ALL nodes
2. Fix network connectivity (firewall, NAT, ports)
3. Monitor consensus voting

Once all nodes are updated and can communicate, consensus will work properly! 🚀
