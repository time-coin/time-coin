# Build Fix: RocksDB Dependency

**Issue:** Build failing on Linux with `zstd-sys` requiring `libclang`

**Fix:** Commented out `rocksdb` dependency (Phase 2 feature not needed yet)

**Files Changed:**
- `Cargo.toml` - Commented out workspace dependency
- `core/Cargo.toml` - Commented out rocksdb usage

**Why This Works:**
Phase 1 only needs:
- ✅ `flate2` for compression (works fine)
- ✅ `rayon` for parallel processing (works fine)
- 🚧 `rocksdb` is for Phase 2 (hot/cold storage)

**Re-enabling RocksDB (Phase 2):**

```bash
# Install dependencies first
sudo apt-get install clang libclang-dev llvm-dev

# Then uncomment in Cargo.toml:
rocksdb = "0.21"

# And in core/Cargo.toml:
rocksdb = { workspace = true }
```

**Alternative (system zstd):**
```bash
sudo apt-get install libzstd-dev
export ZSTD_SYS_USE_PKG_CONFIG=1
cargo build
```

**Commits:**
- `1d388cf` - Disable rocksdb in workspace
- `5832231` - Remove from core/Cargo.toml
