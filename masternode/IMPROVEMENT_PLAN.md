# Masternode Improvement Implementation Plan

This document tracks the implementation of improvements based on the comprehensive audit.

## Status Key
- ✅ Completed
- 🚧 In Progress
- ⏳ Planned
- ⏸️ Deferred

## Phase 1: Foundation (Weeks 1-2) - PRIORITY

### 1.1 Unified Error Handling 🚧
**Status:** In Progress
**Files:** `src/error.rs`, `src/config.rs`, `src/wallet_dat.rs`
**Goal:** Single unified error type replacing all `Result<T, String>`

**Changes:**
- [x] Audit existing error types
- [ ] Extend `MasternodeError` with all error variants
- [ ] Add `From<ConfigError>` and `From<WalletError>` conversions
- [ ] Replace all `Result<T, String>` with `Result<T, MasternodeError>`
- [ ] Update all `.map_err(|e| format!("..."))` to use proper error variants

**Implementation Notes:**
Current error types found:
- `MasternodeError` (error.rs) - Main type, well-structured with thiserror
- `MasternodeConfigError` (config.rs) - Separate type for config parsing
- `WalletDatError` (wallet_dat.rs) - Separate type for wallet operations

Strategy: Keep specialized error types but make them convertible to `MasternodeError`

### 1.2 Consolidate Masternode Type Definitions ⏳
**Status:** Planned
**Files:** `src/lib.rs`, `src/types.rs`, `src/node.rs`
**Goal:** Single source of truth for `Masternode` struct

**Current State:**
- `lib.rs` line 36-117: Has `CollateralTier` enum (NOT a Masternode struct)
- `types.rs` line 26-36: Has `Masternode` struct with `NetworkInfo`
- `node.rs` line 36-59: Has `Masternode` struct with `Reputation`

**Decision:** 
- Keep `node.rs` as canonical implementation (most complete)
- Deprecate `types.rs` version
- Update all imports

**Migration Steps:**
1. Add deprecation warning to `types.rs::Masternode`
2. Update all references to use `node::Masternode`
3. Create migration guide for any external users
4. Remove deprecated type in next major version

### 1.3 Consolidate Status Enums ⏳
**Status:** Planned
**Files:** `src/types.rs`, `src/node.rs`

**Current State:**
- `types.rs::MasternodeStatus`: Registered, Active, Inactive, Banned
- `node.rs::MasternodeStatus`: Pending, Active, Offline, Slashed, Deregistered

**Decision:** Use `node.rs` version (more granular states needed)
- Map deprecated states: Registered→Pending, Inactive→Offline, Banned→Slashed

### 1.4 Module Structure Reorganization ⏳
**Status:** Planned
**Goal:** Clean separation of concerns

**Proposed Structure:**
```
src/
├── lib.rs                    # ONLY re-exports and module declarations
├── types/
│   ├── mod.rs               # Re-exports all types
│   ├── masternode.rs        # Re-export from node.rs
│   ├── status.rs            # Re-export from node.rs
│   └── tier.rs              # Move from lib.rs
├── core/
│   ├── mod.rs
│   ├── node.rs              # Canonical Masternode implementation
│   ├── registry.rs          # Masternode registry
│   └── network.rs           # Network management (or deprecate)
├── validation/
│   ├── mod.rs
│   ├── detector.rs          # Violation detection
│   ├── violations.rs        # Violation types
│   └── slashing.rs          # Slashing logic
├── wallet/
│   ├── mod.rs
│   ├── manager.rs
│   ├── dat.rs
│   └── integration.rs
└── ...existing modules...
```

## Phase 2: Performance & Stability (Weeks 3-4)

### 2.1 Optimize Detector Data Structures ⏳
**Status:** Planned
**File:** `src/detector.rs`

**Changes:**
```rust
// BEFORE: O(n) signature lookup
block_signatures: HashMap<u64, Vec<BlockSignature>>

// AFTER: O(1) signature lookup
block_signatures: HashMap<u64, HashMap<String, BlockSignature>>
```

### 2.2 Add Memory Bounds ⏳
**Status:** Planned
**Files:** `src/detector.rs`, `src/utxo_tracker.rs`

**Changes:**
- Add maximum retention periods for all collections
- Implement cleanup methods
- Add periodic task scheduler for cleanup

### 2.3 Background Task for Blockchain Scanning ⏳
**Status:** Planned
**File:** `src/utxo_integration.rs`

**Changes:**
- Move `scan_blockchain()` to background task
- Return immediately with "scanning" status
- Add progress tracking mechanism

### 2.4 Remove All Panic Risks 🚧
**Status:** In Progress

**Audit Results:**
- [ ] `detector.rs:184` - `serde_json::to_string(&ds).unwrap_or_default()`
- [ ] Replace all `.unwrap()` with `?` operator
- [ ] Replace all `.expect()` with proper error handling

## Phase 3: Observability & Operations (Weeks 5-6)

### 3.1 Add Metrics Collection ⏳
**Status:** Planned

**Dependencies to Add:**
```toml
[dependencies]
prometheus = "0.13"
lazy_static = "1.4"
```

**Implementation:**
- Create `src/metrics.rs`
- Add counters: violations_detected, slashings_executed
- Add gauges: active_masternodes, utxo_tracked
- Add histograms: detection_duration_seconds

### 3.2 State Persistence Layer ⏳
**Status:** Planned

**Dependencies to Add:**
```toml
[dependencies]
rocksdb = "0.21"  # Or sled = "0.34" for pure Rust
```

**Implementation:**
- Create `src/storage/` module
- Trait-based design for swappable backends
- Persist: violations, signatures, reputation state

### 3.3 Unified Configuration ⏳
**Status:** Planned

**Goal:** Single `masternode.toml` configuration file

**Sections:**
- `[node]` - Node identification
- `[masternode]` - Collateral, address
- `[security]` - Rate limiting, thresholds
- `[violations]` - Detection parameters
- `[consensus]` - Voting parameters
- `[wallet]` - Wallet integration settings

## Phase 4: Security Hardening (Week 7+)

### 4.1 Encrypt Sensitive Data ⏳
**Status:** Planned
**File:** `src/wallet_dat.rs`

**Dependencies to Add:**
```toml
[dependencies]
chacha20poly1305 = "0.10"
argon2 = "0.5"
zeroize = "1.6"
```

**Changes:**
- Never store raw master keys
- Use Argon2 for key derivation from password
- Use ChaCha20-Poly1305 for encryption
- Zeroize sensitive data on drop

### 4.2 Input Validation ⏳
**Status:** Planned

**Changes:**
- Add validation for all configuration parameters
- Add bounds checking on all thresholds
- Parse and validate IP addresses
- Validate transaction IDs format

### 4.3 Replay Attack Protection ⏳
**Status:** Planned
**File:** `src/detector.rs`

**Changes:**
- Add timestamp validation to `Evidence`
- Implement nonce tracking with expiration
- Add freshness checks (max age: 1 hour)

## Quick Wins (Can Implement Immediately)

### QW1: Add `#![forbid(unsafe_code)]` ⏳
**File:** `src/lib.rs`
```rust
#![forbid(unsafe_code)]
```

### QW2: Add Comprehensive Clippy Lints ⏳
**File:** `Cargo.toml` or `.cargo/config.toml`
```toml
[lints.rust]
unsafe_code = "forbid"

[lints.clippy]
all = "warn"
pedantic = "warn"
unwrap_used = "warn"
expect_used = "warn"
panic = "warn"
```

### QW3: Pin Dependencies (Production Only) ⏸️
**Status:** Deferred until production release
**Note:** Keep flexible versions for development

### QW4: Add Module-Level Documentation ⏳
**Files:** All modules missing `//!` docs

**Priority Modules:**
- [ ] `src/node.rs`
- [ ] `src/reputation.rs`
- [ ] `src/security.rs`
- [ ] `src/detector.rs`

### QW5: Generate Documentation ✅
```bash
cargo doc --no-deps --open
```

## Testing Improvements

### Integration Tests Needed ⏳
**File:** `tests/integration_tests.rs`

**Test Cases:**
1. [ ] Violation detection → Slashing execution flow
2. [ ] Registry → Slashing integration
3. [ ] UTXO tracker with blockchain scanner
4. [ ] Wallet recovery scenarios
5. [ ] Concurrent violation detection (race conditions)

### Property-Based Testing ⏳
**Dependencies:**
```toml
[dev-dependencies]
proptest = "1.0"
quickcheck = "1.0"
```

**Test Modules:**
- [ ] `tests/property_evidence.rs` - Evidence verification
- [ ] `tests/property_reputation.rs` - Reputation calculations
- [ ] `tests/property_collateral.rs` - Tier calculations

## Code Quality Improvements

### CQ1: Consistent Error Handling Pattern ⏳
**Goal:** Use `?` operator everywhere, eliminate `.map_err(|e| format!())`

**Strategy:**
1. Implement `From<ExternalError>` for `MasternodeError`
2. Replace all `.map_err()` with `?`
3. Use `thiserror` for all error types

### CQ2: Consistent Logging ⏳
**Goal:** Use only `tracing` crate, remove `log!` macros

**Steps:**
1. Add `tracing-subscriber` to dependencies
2. Replace all `log::info!` with `tracing::info!`
3. Add span context to important operations

### CQ3: Remove TODOs ⏳
**Goal:** Convert all TODOs to GitHub issues

**Process:**
1. Audit all TODO comments: `grep -r "TODO" src/`
2. Create GitHub issues for each
3. Replace with issue links: `// See: #123`

## Metrics & Success Criteria

### Code Quality Metrics
- [ ] Zero `unwrap()` calls in production code paths
- [ ] Zero `Result<T, String>` in public APIs
- [ ] 100% of modules have top-level docs
- [ ] All public functions have doc comments
- [ ] `cargo clippy` passes with zero warnings

### Performance Metrics
- [ ] Violation detection O(1) signature lookup
- [ ] Memory bounded collections (max size enforced)
- [ ] Background tasks for I/O operations

### Security Metrics
- [ ] No sensitive data in logs
- [ ] Encrypted wallet storage
- [ ] Input validation on all external inputs
- [ ] Replay attack protection

### Testing Metrics
- [ ] >80% code coverage
- [ ] All critical paths have integration tests
- [ ] Property-based tests for core algorithms

## Implementation Timeline

### Week 1-2: Foundation
- Error handling unification
- Type consolidation
- Module reorganization

### Week 3-4: Performance
- Data structure optimization
- Memory bounds
- Background task migration

### Week 5-6: Operations
- Metrics collection
- State persistence
- Configuration unification

### Week 7+: Security
- Encryption implementation
- Input validation
- Security audit

## Notes & Decisions

### Decision Log

**2025-12-09:** Decided to keep separate error types (ConfigError, WalletError) but make them convertible to MasternodeError via `From` implementations. This maintains module independence while enabling unified error handling at higher levels.

**2025-12-09:** Canonical Masternode struct will be `node.rs` version. The `types.rs` version will be deprecated. This version includes Reputation tracking and more operational state.

**2025-12-09:** `MasternodeNetwork` in `lib.rs` appears to be a different abstraction than `MasternodeRegistry`. Need to investigate if they can be merged or if they serve different purposes.

### Open Questions

1. Should we merge `MasternodeNetwork` and `MasternodeRegistry` or keep them separate?
   - **Investigation needed:** Review usage patterns across codebase

2. What is the retention policy for violation records?
   - **Proposal:** Keep violations for 1 year, aggregate older data

3. Should wallet encryption be mandatory or optional?
   - **Recommendation:** Optional (graceful degradation), but warn users

4. What metrics backend to use? Prometheus vs custom?
   - **Recommendation:** Prometheus (industry standard, grafana integration)

## References

- Original audit document: `COMPREHENSIVE_IMPROVEMENT_RECOMMENDATIONS.md`
- Existing documentation: `README.md`, `VIOLATION_DETECTION.md`, `SLASHING.md`, `WALLET.md`
- Related issues: TBD (create GitHub issues)
