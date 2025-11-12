# Fork Detection and Recovery Procedures

## Overview

TIME Coin implements comprehensive fork detection and quarantine mechanisms to maintain network integrity and prevent consensus attacks. This document outlines the detection mechanisms, recovery procedures, and best practices for node operators.

## Fork Detection Mechanisms

### 1. Genesis Block Validation

**What it does:**
- Every peer connection validates the genesis block hash during handshake
- Nodes with different genesis blocks are immediately rejected
- Genesis mismatch results in automatic peer quarantine

**When it triggers:**
- During initial peer connection (handshake phase)
- When downloading genesis block from peers during sync
- During periodic fork detection checks

**Detection criteria:**
```
Our Genesis:   00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048
Peer Genesis:  00000000000000000000000000000000000000000000000000000000deadbeef
Result:        GENESIS MISMATCH - Peer quarantined
```

### 2. Height Validation

**What it does:**
- Validates peer block heights against expected maximum based on time since genesis
- TIME Coin uses 24-hour blocks, so height should never exceed days since genesis + tolerance
- Rejects peers with suspiciously high heights

**Formula:**
```
max_expected_height = days_since_genesis + 10 (tolerance)
```

**When it triggers:**
- During chain sync operations
- When querying peer heights
- Before accepting blocks from peers

**Example:**
```
Genesis time:      Nov 1, 2024 00:00:00 UTC
Current time:      Nov 11, 2024 00:00:00 UTC
Days elapsed:      10
Max expected:      20 (10 + 10 tolerance)
Peer height:       1000
Result:            SUSPICIOUS HEIGHT - Peer quarantined
```

### 3. Fork Detection at Same Height

**What it does:**
- Compares our block hash with peer block hashes at the same height
- Detects when multiple competing blocks exist at the same height
- Implements consensus rules to select the winning block

**When it triggers:**
- During periodic sync checks (every 5 minutes)
- When multiple blocks are detected at same height
- During chain synchronization

**Resolution strategy:**
1. **Timestamp priority**: Earlier timestamp wins
2. **Tier weight**: Higher masternode tier gets bonus weight
3. **Deterministic tiebreaker**: Lexicographically smaller hash wins

### 4. Consensus Validation

**What it does:**
- Ensures BFT consensus rules are followed
- Requires supermajority (67%) agreement for block finalization
- Tracks validator participation and voting patterns

**When it triggers:**
- During block production
- When validating blocks in consensus mode
- Before entering BFT mode

## Quarantine System

### Quarantine Reasons

1. **Genesis Mismatch**: Different genesis block detected
2. **Fork Detected**: Competing blocks at same height
3. **Suspicious Height**: Height exceeds reasonable maximum
4. **Consensus Violation**: Invalid consensus participation

### Quarantine Duration

- Default: 1 hour
- Configurable per deployment
- Automatically expires after duration
- Can be manually released by operators

### Quarantined Peer Restrictions

Quarantined peers are:
- Excluded from consensus voting
- Not used for block downloads
- Not considered for sync operations
- Logged for monitoring and investigation

## Recovery Procedures

### Scenario 1: Genesis Block Changed

**Situation:** Node operator intentionally changes genesis block configuration.

**What happens:**
1. Node detects genesis mismatch on startup
2. Automatically clears existing blockchain database
3. Rebuilds chain from new genesis block
4. All old blocks are discarded

**Operator actions:**
```bash
# 1. Stop the node
sudo systemctl stop timed

# 2. Update genesis configuration
# Edit config/genesis-testnet.json with new genesis block

# 3. Restart the node (automatic cleanup happens)
sudo systemctl start timed

# 4. Monitor logs for successful rebuild
sudo journalctl -u timed -f
```

**Expected log output:**
```
⚠️  Genesis block mismatch detected!
   Expected: 00000000839a8e68...
   Found:    00000000000000000...
   Rebuilding blockchain from new genesis block...
✅ Genesis block initialized: 00000000839a8e68...
```

### Scenario 2: Fork Detected at Current Height

**Situation:** Multiple nodes produce blocks at the same height.

**What happens:**
1. Fork detection identifies competing blocks
2. All blocks are compared using consensus rules
3. Winning block is selected deterministically
4. Losing blocks are reverted
5. Nodes update to winning chain

**Operator actions:**
- No manual intervention required
- Monitor logs to verify correct resolution
- Check that node converges with network

**Expected log output:**
```
⚠️  FORK DETECTED at height 42!
   Found 2 competing blocks
   📊 Block comparison:
      ✓ WINNER self - Timestamp: 2024-11-11 00:00:01, Hash: abc123...
      ✗ peer_192.168.1.100 - Timestamp: 2024-11-11 00:00:02, Hash: def456...
   ✓ Our block won - no action needed
```

### Scenario 3: Peer on Different Chain

**Situation:** Peer is running a forked chain or imposter network.

**What happens:**
1. Genesis validation detects mismatch
2. Peer is quarantined immediately
3. No blocks are accepted from that peer
4. Peer is excluded from consensus

**Operator actions:**
```bash
# 1. Check quarantined peers via API
curl http://localhost:24101/network/quarantine

# 2. Investigate why peer has different genesis
# - Check if peer is on wrong network (mainnet vs testnet)
# - Verify genesis configuration
# - Contact peer operator if needed

# 3. Release from quarantine if it was a mistake (optional)
curl -X POST http://localhost:24101/network/quarantine/release \
  -H "Content-Type: application/json" \
  -d '{"peer_ip": "192.168.1.100"}'
```

### Scenario 4: Suspicious Height Detected

**Situation:** Peer reports block height much higher than expected.

**What happens:**
1. Height validation detects anomaly
2. Peer is quarantined
3. Alternative peers are used for sync
4. Attack or misconfiguration is logged

**Operator actions:**
```bash
# 1. Check logs for details
sudo journalctl -u timed | grep "SUSPICIOUS HEIGHT"

# 2. Verify your own node is on correct time
date -u

# 3. Check network consensus
curl http://localhost:24101/blockchain/info

# 4. Report persistent issues to development team
```

## Monitoring and Diagnostics

### Key Metrics to Monitor

1. **Genesis Hash**: Should remain constant across network
2. **Block Height**: Should increase by ~1 per day
3. **Fork Events**: Track frequency and resolution
4. **Quarantined Peers**: Monitor count and reasons
5. **Consensus Participation**: Verify BFT participation rate

### API Endpoints

```bash
# Get current blockchain state
curl http://localhost:24101/blockchain/info

# Get quarantined peers
curl http://localhost:24101/network/quarantine

# Get peer information
curl http://localhost:24101/network/peers

# Get consensus status
curl http://localhost:24101/consensus/status
```

### Log Patterns to Watch

**Normal operation:**
```
✅ Loaded 42 blocks from disk (genesis: 00000000839a8e68...)
🔗 Connected to peer: 192.168.1.100 | Version: 0.1.0-abc123
✓ Chain is up to date
```

**Warning signs:**
```
⚠️  FORK DETECTED at height 42!
⚠️  Peer height 1000 exceeds expected maximum 20
⚠️  Genesis block mismatch detected!
```

**Critical issues:**
```
⛔ GENESIS MISMATCH: Peer 192.168.1.100 on different chain!
🚫 Peer 192.168.1.100 quarantined: Genesis mismatch
```

## Prevention Best Practices

### For Node Operators

1. **Use Official Genesis**: Always use the official genesis block from repository
2. **Keep Time Synchronized**: Use NTP to keep system time accurate
3. **Monitor Regularly**: Check logs and metrics daily
4. **Stay Updated**: Keep node software up to date
5. **Backup Configuration**: Keep genesis and config files backed up

### For Network Administrators

1. **Document Genesis**: Maintain authoritative genesis block record
2. **Coordinate Updates**: Plan network-wide updates carefully
3. **Monitor Quarantines**: Track quarantine events across network
4. **Rapid Response**: Have procedures for handling network splits
5. **Communication**: Maintain operator communication channels

## Troubleshooting

### Problem: Node keeps resyncing from genesis

**Cause:** Genesis block configuration changed repeatedly

**Solution:**
1. Identify correct genesis block for your network
2. Update configuration once with correct genesis
3. Delete blockchain data directory
4. Restart node for clean sync

### Problem: All peers quarantined

**Cause:** Your node has different genesis than network

**Solution:**
1. Verify your genesis configuration
2. Compare with official genesis from repository
3. Update if different
4. Restart node

### Problem: Fork resolution takes too long

**Cause:** Network split with equal competing chains

**Solution:**
1. Check network partition hasn't occurred
2. Verify internet connectivity
3. Check if majority of nodes agree
4. Wait for more blocks to be produced
5. Manual intervention may be needed if persistent

## Security Considerations

1. **Genesis Trust**: The genesis block establishes chain identity - protect it
2. **Time Attacks**: Ensure system time is accurate to prevent timestamp manipulation
3. **Sybil Protection**: Quarantine prevents Sybil attacks via different chains
4. **Consensus Safety**: BFT requires 67% honest majority - monitor participation
5. **Network Monitoring**: Track quarantine events for attack detection

## References

- Issue #105: Genesis Block/Fork Inconsistency in Testnet
- Whitepaper: TIME Coin Technical Architecture
- Repository: github.com/time-coin/time-coin
