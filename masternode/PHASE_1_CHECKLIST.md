# Phase 1b-1e Implementation Checklist

This checklist guides you through completing the remaining Phase 1 foundation improvements.

## Prerequisites

- [x] Phase 1a complete (unified errors, safety lints, documentation)
- [x] All tests passing (80/80)
- [x] Build successful

## Phase 1b: Replace String Errors (Est: 4-6 hours)

### Step 1: Find All String Errors
```bash
cd C:\Users\wmcor\projects\time-coin\masternode
grep -r "Result<.*String>" src/ --include="*.rs" > string_errors.txt
```

### Step 2: Fix Each File

#### `src/detector.rs`
- [ ] Line scan: `grep -n "Result<.*String>" src/detector.rs`
- [ ] Replace with `Result<T>` (uses `MasternodeError`)
- [ ] Update error messages to use proper error variants
- [ ] Example:
  ```rust
  // BEFORE
  fn check_violation(&self) -> Result<(), String> {
      Err("Violation detected".to_string())
  }
  
  // AFTER
  fn check_violation(&self) -> Result<()> {
      Err(MasternodeError::ViolationDetected("details".to_string()))
  }
  ```

#### `src/utxo_tracker.rs`
- [ ] Scan for string errors
- [ ] Replace with `MasternodeError::UtxoError`
- [ ] Test UTXO tracking after changes

#### `src/wallet_manager.rs`
- [ ] Scan for string errors
- [ ] Use existing `WalletDatError` → automatic conversion via `From` impl
- [ ] Test wallet operations

#### `src/utxo_integration.rs`
- [ ] Scan for string errors
- [ ] Use appropriate error variants (Network, Connection, UTXO)
- [ ] Test integration flow

### Step 3: Verify
```bash
# Should find zero string errors
grep -r "Result<.*String>" src/ --include="*.rs"

# Rebuild
cargo check -p time-masternode

# Run tests
cargo test -p time-masternode
```

---

## Phase 1c: Remove Unwrap/Panic Patterns (Est: 3-4 hours)

### Step 1: Audit All Unwraps
```bash
# Find all unwrap calls
grep -rn "\.unwrap()" src/ --include="*.rs" > unwraps.txt

# Find all expect calls
grep -rn "\.expect(" src/ --include="*.rs" > expects.txt

# Find all panic calls
grep -rn "panic!" src/ --include="*.rs" > panics.txt
```

### Step 2: Categorize and Fix

For each occurrence, choose the appropriate fix:

#### Option A: Propagate Error (Preferred)
```rust
// BEFORE
let json = serde_json::to_string(&data).unwrap();

// AFTER
let json = serde_json::to_string(&data)?;
```

#### Option B: Provide Default (Safe Fallback)
```rust
// BEFORE
let value = map.get("key").unwrap();

// AFTER
let value = map.get("key").unwrap_or(&default_value);
```

#### Option C: Handle Explicitly (Complex Cases)
```rust
// BEFORE
let result = operation().unwrap();

// AFTER
let result = match operation() {
    Ok(r) => r,
    Err(e) => {
        tracing::error!("Operation failed: {}", e);
        return Err(MasternodeError::from(e));
    }
};
```

### Step 3: Known Issues

#### `detector.rs:184`
```rust
// Current code (line ~184)
let evidence_data = serde_json::to_string(&ds).unwrap_or_default();

// Fix option 1: Return empty on error (current behavior)
let evidence_data = serde_json::to_string(&ds)
    .unwrap_or_else(|e| {
        tracing::warn!("Failed to serialize evidence: {}", e);
        String::new()
    });

// Fix option 2: Propagate error (better)
let evidence_data = serde_json::to_string(&ds)
    .map_err(|e| MasternodeError::SerializationError(e.to_string()))?;
```

### Step 4: Test Critical Paths
```bash
# Run full test suite
cargo test -p time-masternode

# Run specific tests for changed files
cargo test -p time-masternode detector
cargo test -p time-masternode violations
```

---

## Phase 1d: Consolidate Type Definitions (Est: 2-3 hours)

### Step 1: Add Deprecation Warning

Edit `src/types.rs`:
```rust
#[deprecated(
    since = "0.2.0",
    note = "Use node::Masternode instead. This type will be removed in 0.3.0"
)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Masternode {
    // ... existing fields
}
```

### Step 2: Update lib.rs Exports

Edit `src/lib.rs`:
```rust
// Prefer the node.rs version
pub use node::Masternode;
pub use node::MasternodeStatus;

// For backward compatibility (deprecated)
#[deprecated(since = "0.2.0", note = "Use node::Masternode instead")]
pub use types::Masternode as LegacyMasternode;
```

### Step 3: Update Internal Imports

Find all usages:
```bash
grep -rn "use crate::types::Masternode" src/
grep -rn "types::Masternode" src/
```

Replace with:
```rust
use crate::node::Masternode;
```

### Step 4: Create Migration Guide

Add to `MIGRATION.md`:
```markdown
# Type Migration Guide

## Masternode Type

**Old:** `time_masternode::types::Masternode`
**New:** `time_masternode::node::Masternode` or `time_masternode::Masternode`

**Changes:**
- Added `reputation: Reputation` field
- Removed `network_info: NetworkInfo` (fields flattened)
- Added `blocks_validated: u64` field
- Added `total_rewards: u64` field

**Migration:**
```rust
// Old code
use time_masternode::types::Masternode;

// New code
use time_masternode::Masternode;
// or explicitly:
use time_masternode::node::Masternode;
```
```

### Step 5: Verify
```bash
# Check for compiler warnings about deprecation
cargo check -p time-masternode 2>&1 | grep deprecated

# Run tests
cargo test -p time-masternode
```

---

## Phase 1e: Add Missing Documentation (Est: 2-3 hours)

### Step 1: List All Missing Docs
```bash
cargo rustdoc -p time-masternode -- -D missing-docs 2>&1 | grep "missing documentation"
```

### Step 2: Document Each Item

#### `COIN` Constant (lib.rs)
```rust
/// Satoshi-style denomination: 1 TIME = 100,000,000 base units
///
/// This constant defines the relationship between TIME coins and their
/// smallest indivisible unit (similar to satoshis in Bitcoin).
pub const COIN: u64 = 100_000_000;
```

#### `CollateralTier` Variants (lib.rs)
```rust
/// Masternode collateral tiers
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CollateralTier {
    /// Community tier: 1,000 TIME, 18% APY, 90% uptime required
    Community,
    
    /// Verified tier: 10,000 TIME, 24% APY, 95% uptime required
    Verified,
    
    /// Professional tier: 100,000 TIME, 30% APY, 98% uptime required
    Professional,
}
```

#### Methods Documentation Template
```rust
impl CollateralTier {
    /// Determines the appropriate collateral tier based on the amount staked
    ///
    /// # Arguments
    /// * `amount` - The collateral amount in base units (satoshis)
    ///
    /// # Returns
    /// * `Ok(CollateralTier)` - The matching tier
    /// * `Err(String)` - If amount is below minimum (1,000 TIME)
    ///
    /// # Examples
    /// ```
    /// use time_masternode::{CollateralTier, COIN};
    ///
    /// let tier = CollateralTier::from_amount(10_000 * COIN).unwrap();
    /// assert_eq!(tier, CollateralTier::Verified);
    /// ```
    pub fn from_amount(amount: u64) -> Result<Self, String> {
        // ... implementation
    }
    
    /// Returns the required collateral amount for this tier
    ///
    /// # Returns
    /// The collateral amount in base units
    pub fn required_collateral(&self) -> u64 {
        // ... implementation
    }
    
    /// Returns the base Annual Percentage Yield (APY) for this tier
    ///
    /// # Returns
    /// APY as a decimal (e.g., 0.18 = 18%)
    pub fn base_apy(&self) -> f64 {
        // ... implementation
    }
    
    // ... document other methods similarly
}
```

### Step 3: Verify Documentation
```bash
# Generate documentation
cargo doc -p time-masternode --no-deps --open

# Check for missing docs
cargo rustdoc -p time-masternode -- -D missing-docs
```

---

## Final Verification

### Checklist
- [ ] No `Result<T, String>` in codebase
- [ ] No `.unwrap()` in production code paths
- [ ] No `.expect()` in production code paths
- [ ] No `panic!()` in production code paths
- [ ] Types consolidated (deprecation warnings added)
- [ ] All public items documented
- [ ] All tests passing
- [ ] Documentation builds without errors
- [ ] Clippy passes without warnings

### Commands
```bash
# Full verification suite
cd C:\Users\wmcor\projects\time-coin\masternode

# 1. Check for string errors
echo "Checking for string errors..."
grep -r "Result<.*String>" src/ --include="*.rs"

# 2. Check for unwrap/expect/panic
echo "Checking for unwrap/expect/panic..."
grep -r "\.unwrap()\|\.expect(\|panic!" src/ --include="*.rs" | wc -l

# 3. Build check
echo "Building..."
cargo check -p time-masternode

# 4. Run tests
echo "Running tests..."
cargo test -p time-masternode

# 5. Run clippy
echo "Running clippy..."
cargo clippy -p time-masternode --all-targets -- -D warnings

# 6. Build documentation
echo "Building documentation..."
cargo doc -p time-masternode --no-deps

# 7. Check documentation coverage
cargo rustdoc -p time-masternode -- -D missing-docs

echo "✅ All checks complete!"
```

---

## Tips

### Working Efficiently
1. **Work file by file** - Complete one file entirely before moving to next
2. **Test incrementally** - Run tests after each file change
3. **Commit frequently** - Commit after each completed file
4. **Use IDE features** - Most IDEs can find unwrap/expect automatically

### Common Pitfalls
- **Don't change test code** - Tests can use `.unwrap()` (they should panic on error)
- **Don't break public APIs** - Keep backward compatibility
- **Check examples** - Make sure examples still compile
- **Update related docs** - If you change behavior, update docs

### When Stuck
1. Check `IMPLEMENTATION_SUMMARY.md` for context
2. Review `IMPROVEMENT_PLAN.md` for detailed explanations
3. Look at `src/error.rs` for available error types
4. Check existing code for patterns
5. Run tests to understand expected behavior

---

## Progress Tracking

Mark items complete as you finish them:

### Phase 1b: String Errors
- [ ] `detector.rs` converted
- [ ] `utxo_tracker.rs` converted
- [ ] `wallet_manager.rs` converted
- [ ] `utxo_integration.rs` converted
- [ ] Other files as needed
- [ ] Tests passing
- [ ] Zero string errors remaining

### Phase 1c: Unwrap/Panic
- [ ] All unwraps audited
- [ ] Critical unwraps fixed
- [ ] All expects audited
- [ ] Critical expects fixed
- [ ] All panics audited
- [ ] Tests passing

### Phase 1d: Type Consolidation
- [ ] Deprecation added to `types.rs`
- [ ] `lib.rs` exports updated
- [ ] Internal imports updated
- [ ] Migration guide created
- [ ] Tests passing

### Phase 1e: Documentation
- [ ] `COIN` documented
- [ ] `CollateralTier` variants documented
- [ ] All methods documented
- [ ] Examples added
- [ ] Docs build successfully
- [ ] No missing-docs warnings

---

**When complete, update `IMPLEMENTATION_SUMMARY.md` with completion date and move to Phase 2!**
