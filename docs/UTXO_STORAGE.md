# UTXO Storage Options

## Overview

TIME Coin provides two UTXO storage implementations to support different hardware configurations:

1. **`UTXOSet`** - In-memory HashMap (fast, limited by RAM)
2. **`DiskBackedUTXOSet`** - Disk storage with LRU cache (scalable, slightly slower)

## When to Use Each

### Use `UTXOSet` (In-Memory) When:
- Node has sufficient RAM (4GB+ recommended)
- UTXO count < 100,000
- Maximum performance is required
- Running on a system with fast RAM

### Use `DiskBackedUTXOSet` (Disk) When:
- Node has limited RAM (< 2GB)
- UTXO count > 100,000
- Need to support unlimited growth
- Running on resource-constrained hardware (Raspberry Pi, etc.)

## Configuration

### Environment Variables

Set these in your masternode configuration:

```bash
# Force disk-backed UTXO set
export UTXO_STORAGE_MODE=disk

# Or keep in-memory (default)
export UTXO_STORAGE_MODE=memory
```

### Adaptive Mode (Recommended)

The system can automatically switch based on thresholds:

```bash
# Switch to disk when UTXO count exceeds 50,000
export UTXO_THRESHOLD_COUNT=50000

# Or switch when memory usage exceeds 512MB
export UTXO_THRESHOLD_MEMORY_MB=512
```

## Performance Characteristics

| Operation | In-Memory | Disk-Backed (Cache Hit) | Disk-Backed (Cache Miss) |
|-----------|-----------|------------------------|--------------------------|
| Get UTXO  | ~100ns    | ~200ns                 | ~50μs                    |
| Add UTXO  | ~150ns    | ~60μs                  | ~60μs                    |
| Remove    | ~120ns    | ~55μs                  | ~55μs                    |
| Scan Address | O(n)    | O(n)                   | O(n)                     |

## Implementation Details

### DiskBackedUTXOSet Features:
- **LRU Cache**: Keeps 10,000 most-used UTXOs in memory
- **Sled Database**: Embedded key-value store for persistence
- **Automatic Flushing**: Ensures durability
- **Zero-Copy Reads**: When possible

### Memory Usage Estimates:
- **In-Memory**: ~200 bytes per UTXO
  - 50,000 UTXOs = ~10MB
  - 500,000 UTXOs = ~100MB
  - 5,000,000 UTXOs = ~1GB

- **Disk-Backed**: ~2MB + disk space
  - Cache: 10,000 UTXOs × 200 bytes = 2MB
  - Disk: ~150 bytes per UTXO (compressed)

## Migration

### Manual Migration

If you need to switch storage modes:

1. **Stop the masternode**
2. **Backup blockchain data**
   ```bash
   tar -czf blockchain-backup.tar.gz data/blockchain*
   ```
3. **Update configuration**
4. **Restart masternode**
   - First startup will be slower as it rebuilds UTXO set from blockchain

### No Automatic Migration

Currently, there is no automatic migration of existing UTXOs between storage modes.
When switching modes, the UTXO set will be rebuilt from the blockchain on startup.

## Future Improvements

- [ ] Hot migration between storage modes
- [ ] Compressed disk storage
- [ ] Bloom filters for faster lookups
- [ ] Parallel UTXO verification
- [ ] Memory-mapped file support

## Monitoring

Check UTXO storage status:

```bash
# View current mode and stats
curl http://localhost:24101/utxo/stats

# Response:
# {
#   "mode": "disk",
#   "utxo_count": 125000,
#   "total_supply": 1000000000000,
#   "cache_hit_rate": 0.95
# }
```

## Troubleshooting

### Out of Memory Errors
Switch to disk-backed storage:
```bash
export UTXO_STORAGE_MODE=disk
systemctl restart time-coin
```

### Slow UTXO Lookups
Increase cache size (requires code change in `utxo_disk_backed.rs`):
```rust
const UTXO_CACHE_SIZE: usize = 50_000; // Increase from 10,000
```

### Disk Space Issues
Monitor disk usage:
```bash
du -sh data/utxo_db/
```

Clean up if needed (requires rebuild):
```bash
rm -rf data/utxo_db/
# UTXO set will rebuild from blockchain
```
