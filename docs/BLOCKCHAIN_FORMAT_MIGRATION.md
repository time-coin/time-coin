# Blockchain Format Migration Guide

## Issue
The BlockHeader format was changed to include `masternode_counts` field. Existing blocks stored with bincode cannot be deserialized with the new format.

## Solution Options

### Option 1: Clear and Resync (Simplest)
On the production server, clear the blockchain data and resync from scratch:

```bash
sudo systemctl stop timed
rm -rf /var/lib/time-coin/blockchain/*
sudo systemctl start timed
```

The node will download the full blockchain from peers with the new format.

### Option 2: Migration Script (Preserve Data)
Create a migration script that:
1. Reads old blocks with the old format
2. Converts to new format with default masternode_counts
3. Writes back with new format

### Option 3: Versioned Serialization
Implement version tags in serialization to handle multiple formats.

## Recommendation
**Use Option 1** - Clear and resync. This is the safest and simplest approach since:
- The node will sync quickly from other peers
- No risk of data corruption
- Clean migration to new format
- The blockchain is still relatively small

## Commands for Production Server

```bash
# Stop the daemon
sudo systemctl stop timed

# Backup (optional)
sudo cp -r /var/lib/time-coin/blockchain /var/lib/time-coin/blockchain.backup

# Clear blockchain data
sudo rm -rf /var/lib/time-coin/blockchain/*

# Start the daemon (will sync from network)
sudo systemctl start timed

# Watch the logs
sudo journalctl -u timed -f
```

The node will automatically sync from the network.
