# TIME Coin Proof-of-Time Configuration Guide

## Current Configuration (Testnet)

**Block Time:** 10 minutes  
**VDF Lock:** 2 minutes  
**Purpose:** Testing and development

### Security Properties

| Metric | Value |
|--------|-------|
| **Single block reorg** | 2 minutes minimum |
| **24-block reorg (4 hours)** | 48 minutes minimum |
| **144-block reorg (1 day)** | 288 minutes (4.8 hours) minimum |
| **1008-block reorg (1 week)** | 2,016 minutes (33.6 hours) minimum |

---

## Future Configuration (Mainnet)

**Block Time:** 1 hour  
**VDF Lock:** 5 minutes  
**Purpose:** Production deployment

### Security Properties

| Metric | Value |
|--------|-------|
| **Single block reorg** | 5 minutes minimum |
| **24-block reorg (1 day)** | 120 minutes (2 hours) minimum |
| **168-block reorg (1 week)** | 840 minutes (14 hours) minimum |
| **720-block reorg (1 month)** | 3,600 minutes (60 hours / 2.5 days) minimum |

---

## Why These Settings?

### Testnet (10-minute blocks, 2-minute VDF)

✅ **Fast iteration** - Test scenarios quickly  
✅ **Good UX** - Short confirmation times  
✅ **Adequate security** - 4-hour attack cost for 1-day reorg  
✅ **Easy debugging** - More blocks = more data  

### Mainnet (1-hour blocks, 5-minute VDF)

✅ **Excellent UX** - 1 hour max wait (faster than bank transfers)  
✅ **Strong security** - 2 hour minimum attack for daily reorg  
✅ **Time theme** - 24 blocks per day = 1 block per hour  
✅ **Sustainable** - Manageable block sizes, reasonable validator load  
✅ **Professional** - Industry-standard confirmation times  

---

## Comparison with Other Blockchains

| Blockchain | Block Time | Confirmation Time | Security Mechanism |
|------------|------------|-------------------|-------------------|
| **Bitcoin** | 10 min | 60 min (6 blocks) | Proof-of-Work (mining) |
| **Ethereum** | 12 sec | ~15 min (finality) | Proof-of-Stake + finality gadget |
| **TIME Coin (Testnet)** | 10 min | 10 min | Proof-of-Time (VDF) |
| **TIME Coin (Mainnet)** | 1 hour | 1 hour | Proof-of-Time (VDF) |

---

## Migration Path

### Phase 1: Testnet Launch (Now)
```rust
// Hardcoded in core/src/vdf.rs
DEFAULT_VDF_LOCK_SECONDS = TESTNET_VDF_LOCK_SECONDS  // 120 seconds
DEFAULT_TOTAL_ITERATIONS = TESTNET_TOTAL_ITERATIONS  // 12,000,000
```

**Timeline:** 3-6 months  
**Goal:** Test consensus, find bugs, build community

### Phase 2: Mainnet Genesis
```rust
// Change default in core/src/vdf.rs before mainnet launch
DEFAULT_VDF_LOCK_SECONDS = MAINNET_VDF_LOCK_SECONDS  // 300 seconds  
DEFAULT_TOTAL_ITERATIONS = MAINNET_TOTAL_ITERATIONS  // 30,000,000
```

**Timeline:** 6-12 months after testnet  
**Goal:** Launch production network with optimal settings

### Phase 3: Future Adjustments (Optional)
If needed, can adjust via hard fork:
- Increase VDF lock for more security
- Decrease VDF lock for faster sync
- Change block time based on network needs

---

## Configuration Constants

### Testnet
```rust
pub const TESTNET_BLOCK_TIME_SECONDS: u64 = 600;        // 10 minutes
pub const TESTNET_VDF_LOCK_SECONDS: u64 = 120;          // 2 minutes
pub const TESTNET_TOTAL_ITERATIONS: u64 = 12_000_000;   // 120 sec × 100k/sec
```

### Mainnet
```rust
pub const MAINNET_BLOCK_TIME_SECONDS: u64 = 3600;       // 1 hour
pub const MAINNET_VDF_LOCK_SECONDS: u64 = 300;          // 5 minutes
pub const MAINNET_TOTAL_ITERATIONS: u64 = 30_000_000;   // 300 sec × 100k/sec
```

### Current Active
```rust
pub const DEFAULT_VDF_LOCK_SECONDS: u64 = TESTNET_VDF_LOCK_SECONDS;
pub const DEFAULT_TOTAL_ITERATIONS: u64 = TESTNET_TOTAL_ITERATIONS;
```

---

## Verification Performance

With current hardware (modern CPU):

| Configuration | Computation Time | Verification Time |
|---------------|------------------|-------------------|
| **Testnet VDF** | ~2 minutes | ~400ms |
| **Mainnet VDF** | ~5 minutes | ~1 second |

**Implication:** Syncing 1000 blocks takes:
- Testnet: ~7 minutes verification
- Mainnet: ~17 minutes verification

Both are acceptable for blockchain sync.

---

## Attack Cost Examples

### Testnet Attack Costs

**Scenario:** Attacker wants to rewrite 1 day of history

| Current Block | Testnet Reorg Cost |
|---------------|-------------------|
| Block 144 | Rewrite 144 blocks × 2 min = 288 minutes (4.8 hours) |
| Block 1008 | Rewrite 1008 blocks × 2 min = 2,016 minutes (33.6 hours) |

**Conclusion:** Even on testnet, deep reorgs are expensive

### Mainnet Attack Costs

**Scenario:** Attacker wants to rewrite 1 day of history

| Current Block | Mainnet Reorg Cost |
|---------------|-------------------|
| Block 24 | Rewrite 24 blocks × 5 min = 120 minutes (2 hours) |
| Block 168 | Rewrite 168 blocks × 5 min = 840 minutes (14 hours) |
| Block 720 | Rewrite 720 blocks × 5 min = 3,600 minutes (60 hours) |

**Conclusion:** Production network has strong finality guarantees

---

## Recommendations

### For Development
- ✅ Use testnet settings (10-min blocks, 2-min VDF)
- ✅ Fast iteration, good testing experience
- ✅ Can always reset testnet if needed

### For Production Launch
- ✅ Use mainnet settings (1-hour blocks, 5-min VDF)
- ✅ Optimal balance of UX and security
- ✅ Professional positioning in market

### Don't Do
- ❌ Don't use 24-hour blocks (too slow for practical use)
- ❌ Don't use <1 minute VDF (insufficient security)
- ❌ Don't use VDF lock > block time (blocks would be delayed)

---

## Summary

**Current (Testnet):**
- 10-minute blocks
- 2-minute VDF lock
- Perfect for testing

**Future (Mainnet):**
- 1-hour blocks  
- 5-minute VDF lock
- Perfect for production

**Result:** Best-in-class security with excellent user experience! 🎯
