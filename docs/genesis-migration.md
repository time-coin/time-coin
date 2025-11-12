# Genesis Block Format Migration Guide

## Overview

The genesis block format has been updated to ensure all nodes in the network generate identical genesis block hashes. This prevents "genesis mismatch" errors that caused peers to be quarantined.

## What Changed

### Old Format
The old genesis format only stored basic transaction information:
```json
{
  "network": "testnet",
  "version": 1,
  "timestamp": 1760227200,
  "hash": "...",
  "transactions": [
    {
      "amount": 11653781624,
      "description": "genesis"
    }
  ]
}
```

**Problem**: Nodes would call `Block::new()` which used `Utc::now()`, creating different timestamps and hashes on each node.

### New Format
The new genesis format stores the complete block structure:
```json
{
  "network": "testnet",
  "version": 1,
  "message": "TIME Coin Testnet Launch - October 12, 2025 - 24 Hour Blocks, Instant Finality",
  "block": {
    "header": {
      "block_number": 0,
      "timestamp": "2025-10-12T00:00:00Z",
      "previous_hash": "0000000000000000000000000000000000000000000000000000000000000000",
      "merkle_root": "coinbase_0",
      "validator_signature": "genesis",
      "validator_address": "genesis"
    },
    "transactions": [
      {
        "txid": "coinbase_0",
        "version": 1,
        "inputs": [],
        "outputs": [
          {
            "amount": 11653781624,
            "address": "genesis"
          }
        ],
        "lock_time": 0,
        "timestamp": 1760227200
      }
    ],
    "hash": "9a81c7599d8eed9720282aa68dccbc76e92ac3770a1892a96e1d073f375d0aed"
  }
}
```

**Solution**: The complete block with fixed timestamp is deserialized directly from JSON, ensuring all nodes have identical genesis blocks.

## Genesis Block Details

- **Network**: testnet
- **Date**: October 12, 2025 00:00:00 UTC
- **Timestamp**: 1760227200 (Unix timestamp)
- **Hash**: `9a81c7599d8eed9720282aa68dccbc76e92ac3770a1892a96e1d073f375d0aed`
- **Total Supply**: 116.53781624 TIME
- **Recipient**: genesis

## Migration Instructions

### For Node Operators

If you're running a node with the old genesis format:

1. **Stop your node**:
   ```bash
   sudo systemctl stop timed
   ```

2. **Run the migration script**:
   ```bash
   cd /path/to/time-coin
   ./scripts/migrate_genesis.sh config/genesis-testnet.json
   ```

3. **Verify the migration**:
   ```bash
   cat config/genesis-testnet.json
   # Should show the new format with "block" field
   ```

4. **Start your node**:
   ```bash
   sudo systemctl start timed
   ```

5. **Verify genesis hash**:
   ```bash
   journalctl -u timed -f
   # Look for "✓ Genesis block hash verified"
   # Hash should be: 9a81c7599d8eed97...
   ```

### Manual Migration

If you prefer to manually update the file:

1. Backup your current genesis file
2. Replace the contents with the new format (see example above)
3. Ensure the hash matches: `9a81c7599d8eed9720282aa68dccbc76e92ac3770a1892a96e1d073f375d0aed`

### For Fresh Installations

No migration needed! The new genesis format is already in `config/genesis-testnet.json`.

## Verification

After migration, verify your node loads the genesis correctly:

```bash
# Check the logs
journalctl -u timed -n 100 | grep -A 10 "GENESIS BLOCK LOADED"
```

You should see:
```
╔══════════════════════════════════════╗
║         GENESIS BLOCK LOADED         ║
╚══════════════════════════════════════╝

Network: testnet
Block Hash: 9a81c7599d8eed97...
Timestamp: 2025-10-12 00:00:00 UTC
✓ Genesis block hash verified
```

## Troubleshooting

### "Genesis mismatch" errors persist

1. Ensure all nodes are using the **exact same genesis file**
2. Verify the hash matches on all nodes: `9a81c7599d8eed9720282aa68dccbc76e92ac3770a1892a96e1d073f375d0aed`
3. Clear quarantine if needed (will be auto-released after fix)

### Hash verification fails

If you see "⚠ Warning: Genesis block hash mismatch!", you may have:
- Modified the genesis file incorrectly
- Used a different genesis file

Solution: Re-run the migration script or copy the canonical genesis file from the repository.

### Node won't start after migration

1. Check the genesis file is valid JSON: `jq . config/genesis-testnet.json`
2. Verify file permissions: `ls -l config/genesis-testnet.json`
3. Check node logs: `journalctl -u timed -n 50`

## Technical Details

### Hash Calculation

The genesis block hash is calculated using double SHA3-256 over:
- Block number (0)
- Timestamp (RFC3339 format: "2025-10-12T00:00:00+00:00")
- Previous hash (64 zeros)
- Merkle root ("coinbase_0")
- Validator address ("genesis")

### Backward Compatibility

The new code will:
- Load new format genesis files directly
- Fall back to creating a default genesis block if no file is found
- Verify loaded genesis hashes match the calculated hash

Old format files are no longer supported and must be migrated.

## Support

If you encounter issues during migration:
1. Check your backup: `ls -la config/*.backup.*`
2. Review logs: `journalctl -u timed -n 100`
3. Report issues on GitHub with:
   - Your genesis file (redacted if needed)
   - Error messages from logs
   - Node version and commit hash
