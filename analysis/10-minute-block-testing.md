# 10-Minute Block Interval Testing Configuration

**Date:** November 28, 2025  
**Purpose:** Enable rapid consensus testing

## Overview

Modified the block producer to support **10-minute block intervals** for testing the deterministic consensus implementation, instead of waiting 24 hours between blocks.

## Changes Made

### File: `cli/src/block_producer.rs`

#### 1. Added Testing Configuration Constants (Lines 17-23)

```rust
// ============================================================================
// TESTING CONFIGURATION  
// ============================================================================
// Change this to adjust block production interval for testing
// Production: 86400 seconds (24 hours)
// Testing: 600 seconds (10 minutes)
const BLOCK_INTERVAL_SECONDS: u64 = 600; // 10 minutes for testing
const IS_TESTING_MODE: bool = true;
```

**To switch back to production:** Set `IS_TESTING_MODE = false`

#### 2. Modified Block Production Loop (Lines 171-217)

**Before:** Only supported midnight UTC (24-hour intervals)

**After:** Supports both modes:
- **Testing Mode**: Rounds to next 10-minute interval
- **Production Mode**: Next midnight UTC (original behavior)

```rust
let next_block_time = if IS_TESTING_MODE {
    // Testing: round up to next 10-minute interval
    let current_seconds = now.timestamp();
    let next_interval = ((current_seconds / BLOCK_INTERVAL_SECONDS as i64) + 1) 
                        * BLOCK_INTERVAL_SECONDS as i64;
    Utc.timestamp_opt(next_interval, 0).unwrap()
} else {
    // Production: next midnight UTC
    let tomorrow = now.date_naive() + chrono::Duration::days(1);
    tomorrow.and_hms_opt(0, 0, 0).unwrap().and_utc()
};
```

#### 3. Updated Deterministic Timestamp Generation (Lines 951-962)

Ensures all nodes use the **same deterministic timestamp**:

```rust
let timestamp = if IS_TESTING_MODE {
    // Testing: round to 10-minute interval
    let current_seconds = now.timestamp();
    let interval_timestamp = (current_seconds / BLOCK_INTERVAL_SECONDS as i64) 
                            * BLOCK_INTERVAL_SECONDS as i64;
    Utc.timestamp_opt(interval_timestamp, 0).unwrap()
} else {
    // Production: midnight UTC
    now.date_naive().and_hms_opt(0, 0, 0).unwrap().and_utc()
};
```

#### 4. Updated Expected Height Calculation (Lines 277-288)

Calculates how many blocks should exist based on time since genesis:

```rust
let expected_height = if IS_TESTING_MODE {
    // Testing: blocks every 10 minutes
    let elapsed_seconds = now.timestamp() - genesis_timestamp;
    (elapsed_seconds / BLOCK_INTERVAL_SECONDS as i64) as u64
} else {
    // Production: blocks every 24 hours
    let days_since_genesis = (current_date - genesis_date).num_days();
    days_since_genesis as u64
};
```

#### 5. Updated Catch-Up Block Timestamps (Lines 793-803)

When recreating missed blocks, uses correct timestamp:

```rust
let timestamp = if IS_TESTING_MODE {
    // Testing: blocks every 10 minutes from genesis
    let block_timestamp = genesis_timestamp + (block_num as i64 * BLOCK_INTERVAL_SECONDS as i64);
    Utc.timestamp_opt(block_timestamp, 0).unwrap()
} else {
    // Production: blocks every 24 hours (midnight)
    let timestamp_date = genesis_date + chrono::Duration::days(block_num as i64);
    Utc.from_utc_datetime(&timestamp_date.and_hms_opt(0, 0, 0).unwrap())
};
```

## How It Works

### Genesis Timestamp
- **Loaded from file:** `config/genesis-testnet.json`
- **Timestamp:** `2025-10-12T00:00:00Z` (1760227200 Unix)
- **NOT hardcoded** - read dynamically from genesis block

### Testing Mode Block Schedule

Starting from genesis `2025-10-12T00:00:00Z`:

| Block # | Timestamp | Offset from Genesis |
|---------|-----------|---------------------|
| 0 (Genesis) | 2025-10-12 00:00:00 | 0 seconds |
| 1 | 2025-10-12 00:10:00 | 600 seconds |
| 2 | 2025-10-12 00:20:00 | 1200 seconds |
| 3 | 2025-10-12 00:30:00 | 1800 seconds |
| ... | ... | ... |
| 6 | 2025-10-12 01:00:00 | 3600 seconds (1 hour) |
| 144 | 2025-10-13 00:00:00 | 86400 seconds (24 hours) |

**Formula:** `block_timestamp = genesis_timestamp + (block_num × 600)`

### Current Time Calculation

When current time is `2025-11-28 02:50:00 UTC`:

```
Elapsed since genesis = 2025-11-28 02:50:00 - 2025-10-12 00:00:00
                      = 4,204,200 seconds
                      = 47 days, 2 hours, 50 minutes

Expected block = 4,204,200 ÷ 600 = 7,007 blocks
```

But if testnet just started, current height will be much lower (e.g., 46-47).

## Testing Benefits

### 1. **Rapid Consensus Validation**
- Test 144 blocks in 24 hours (vs 144 days in production)
- Identify consensus issues quickly

### 2. **Catch-Up Testing**
- Verify nodes can sync missed blocks
- Test reconciliation logic

### 3. **Network Stress Testing**
- More frequent consensus rounds
- Better network partition testing

### 4. **Determinism Verification**
- All nodes must generate identical blocks every 10 minutes
- Easy to verify timestamp alignment

## Console Output

### Testing Mode Enabled

```
⚠️  TESTING MODE: Block interval = 600 seconds (10 minutes)
Next block at 2025-11-28 03:00:00 UTC (in 0h 9m 32s)
⏰ Block interval reached - producing block...
```

### Production Mode

```
Block producer started (24-hour interval)
Next block at 2025-11-29 00:00:00 UTC (in 23h 58m 54s)
Midnight reached - producing block...
```

## Important Notes

### ⚠️ Determinism Requirements

For consensus to work, **ALL nodes must:**
1. Use the **same** `BLOCK_INTERVAL_SECONDS` value
2. Use the **same** `IS_TESTING_MODE` setting  
3. Have the **same** genesis timestamp

**If nodes have different settings, they will generate different blocks!**

### ⚠️ Database Compatibility

Blocks created in testing mode (10-min) **cannot** be used in production (24-hour) and vice versa. The timestamp calculation is fundamentally different.

### ⚠️ Network Requirements

All testnet nodes must be updated with this code **simultaneously** to avoid:
- Different expected heights
- Different block timestamps
- Consensus failures

## Switching Between Modes

### Enable Testing Mode (10 minutes)
```rust
const BLOCK_INTERVAL_SECONDS: u64 = 600;  
const IS_TESTING_MODE: bool = true;
```

### Enable Production Mode (24 hours)
```rust
const BLOCK_INTERVAL_SECONDS: u64 = 86400;  // Not used in prod, but keep consistent
const IS_TESTING_MODE: bool = false;
```

**Then rebuild:**
```bash
cargo build --release --bin timed
```

## Testing Checklist

- [ ] All testnet nodes updated with same configuration
- [ ] Genesis block deployed to all nodes
- [ ] Blockchain data cleared (if switching modes)
- [ ] Nodes synchronized to same starting height
- [ ] Monitor logs for "⚠️ TESTING MODE" message
- [ ] Verify blocks created every ~10 minutes
- [ ] Check consensus success rate (should be 99%+)
- [ ] Verify all nodes have matching block hashes
- [ ] Test catch-up mechanism (stop/restart nodes)
- [ ] Monitor for fork detection (should be minimal)

## Rollback Plan

If testing reveals issues:

1. **Stop all nodes**
2. **Set `IS_TESTING_MODE = false`**
3. **Rebuild:** `cargo build --release --bin timed`
4. **Clear blockchain data** (except genesis)
5. **Restart nodes**
6. **Resync from genesis**

## Performance Metrics to Monitor

| Metric | Expected Value |
|--------|----------------|
| Block creation time | <10 seconds |
| Consensus success rate | 99%+ |
| Blocks per hour | 6 blocks |
| Blocks per day | 144 blocks |
| Time to sync 100 blocks | <2 minutes |
| Fork resolution time | <30 seconds |

## Next Steps

1. **Deploy to testnet** (all nodes simultaneously)
2. **Monitor for 24 hours** (144 blocks)
3. **Analyze consensus logs**
4. **Verify determinism** (compare block hashes)
5. **Test node failures** (stop/restart random nodes)
6. **Validate catch-up** (nodes can sync missed blocks)
7. **Document any issues**
8. **Switch back to 24-hour mode** if stable

---

**Status:** ✅ Ready for testnet deployment  
**Compilation:** ✅ Successful (with warnings)  
**Risk Level:** Low (easy rollback)  
**Estimated Testing Duration:** 24-48 hours

**⚠️ IMPORTANT:** This is for testnet only. Do NOT enable on mainnet!
