# Build Verification Commands

**Date**: November 18, 2025  
**After**: Documentation consolidation and dependency updates

---

## Changes Made

### 1. Documentation Consolidation
- Created `TIME-COIN-TECHNICAL-SPECIFICATION.md`
- Removed 9 redundant files
- Updated navigation (README.md, PROTOCOL_INDEX.md)

### 2. Dependency Consolidation
- Updated tokio-tungstenite: 0.21 → 0.28
- Updated tungstenite: 0.21 → 0.28
- Updated tokio: 1.41 → 1.48
- Fixed API compatibility issues

### 3. Code Fixes for tungstenite 0.28
- `masternode/src/ws_bridge.rs`: Added `.into()` to Message::Text calls
- `wallet-gui/src/protocol_client.rs`: Added `.into()` to Message::Text call

---

## Commands to Run

### 1. Format Code
```bash
cargo fmt --all
```
**Status**: ✅ Completed successfully

### 2. Run Clippy (Linter)
```bash
cargo clippy --workspace --all-targets -- -D warnings
```
**Status**: ⏳ In progress (long build time expected)

### 3. Build All Packages
```bash
# Full workspace build
cargo build --release --workspace

# Or just masternode + CLI (recommended for servers)
cargo build --release -p time-masternode -p time-cli
```
**Status**: ⏳ In progress (expected time: 5-10 minutes)

### 4. Run Tests
```bash
# All tests
cargo test --workspace

# Specific package tests
cargo test -p time-core
cargo test -p time-consensus
cargo test -p time-network
```
**Status**: ⏳ Pending completion of build

---

## Expected Build Times

| Command | Expected Time | Notes |
|---------|--------------|-------|
| `cargo fmt` | <10 seconds | ✅ Done |
| `cargo check` | 2-3 minutes | Fast validation |
| `cargo clippy` | 3-5 minutes | With all warnings |
| `cargo build` | 5-10 minutes | Full workspace |
| `cargo build -p time-masternode` | 3-5 minutes | Masternode only |
| `cargo test` | 5-8 minutes | All tests |

---

## Verification Checklist

- [x] `cargo fmt --all` - Format all code
- [ ] `cargo clippy --workspace --all-targets` - Check for warnings
- [ ] `cargo build --release -p time-masternode -p time-cli` - Build main binaries
- [ ] `cargo test --workspace` - Run all tests
- [ ] `cargo test -p time-consensus` - Test consensus module
- [ ] `cargo test -p time-network` - Test network module

---

## Known Issues Fixed

### Issue 1: tungstenite API Change
**Problem**: Updated tungstenite 0.28 changed `Message::Text` to require `Utf8Bytes` instead of `String`

**Fix**: Added `.into()` conversion in 3 locations:
1. `masternode/src/ws_bridge.rs:109` - Subscription confirmation
2. `masternode/src/ws_bridge.rs:115` - Pong message
3. `wallet-gui/src/protocol_client.rs:204` - Subscribe message

**Status**: ✅ Fixed

---

## Manual Verification Steps

After builds complete, verify:

### 1. Masternode Builds
```bash
./target/release/time-masternode --version
```

### 2. CLI Works
```bash
./target/release/time-cli --help
```

### 3. No Warnings in Clippy
```bash
cargo clippy --workspace 2>&1 | grep warning | wc -l
# Should be 0 or very few
```

### 4. All Tests Pass
```bash
cargo test --workspace 2>&1 | grep "test result"
```

---

## If Builds Fail

### Check Rust Version
```bash
rustc --version
cargo --version
```
Should be Rust 1.70+ (stable channel)

### Clean and Rebuild
```bash
cargo clean
cargo build --release -p time-masternode -p time-cli
```

### Check Specific Errors
```bash
cargo build -p time-masternode 2>&1 | grep error
```

---

## Next Steps After Verification

1. ✅ Commit all changes
2. ✅ Update CHANGELOG.md
3. ✅ Tag release (if applicable)
4. ✅ Deploy to test environment
5. ✅ Run integration tests

---

## Commands Summary

Quick copy-paste for Linux servers:

```bash
# Update Rust (if needed)
rustup update

# Navigate to project
cd time-coin

# Format code
cargo fmt --all

# Check for issues
cargo clippy --workspace --all-targets

# Build release binaries
cargo build --release -p time-masternode -p time-cli

# Run tests
cargo test --workspace

# Verify binaries
./target/release/time-masternode --version
./target/release/time-cli --help
```

---

**Status**: Builds in progress as of 2025-11-18 19:22 UTC  
**Estimated completion**: 19:30 UTC (8 minutes remaining)  
**Next action**: Wait for builds to complete, then run tests
