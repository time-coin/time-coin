# Quick Build Commands Reference

## ✅ Correct Package Names

The package name is **`time-masternode`**, not `masternode`!

## 🚀 One-Liners for Linux Servers

### Minimal (Masternode + CLI only)
```bash
cargo build --release -p time-masternode -p time-cli
```

### Everything except GUI
```bash
cargo build --release --workspace --exclude wallet-gui
```

### With .cargo/config.toml optimization
```bash
# One-time setup
mkdir -p .cargo && cat > .cargo/config.toml << 'EOF'
[net]
git-fetch-with-cli = true

[profile.release]
strip = true
debug = false
lto = "thin"
EOF

# Then just build
cargo build --release -p time-masternode -p time-cli
```

## 📦 All Package Names

```bash
# Binaries
time-masternode       # Masternode binary
time-cli              # CLI tool
time-api              # API server (if needed standalone)
wallet-gui            # GUI wallet (skip on servers)

# Libraries (auto-included as dependencies)
time-core
time-consensus
time-network
time-mempool
time-storage
time-crypto
wallet
time-treasury
```

## 🎯 Common Scenarios

### Fresh Production Server
```bash
git clone https://github.com/time-coin/time-coin.git
cd time-coin
cargo build --release -p time-masternode -p time-cli
./target/release/time-masternode
```

### Build with API Server
```bash
cargo build --release -p time-masternode -p time-cli -p time-api
```

### Development/Testing (all tools)
```bash
cargo build --release --workspace --exclude wallet-gui
```

## 📊 Binary Locations

After building:
- Masternode: `target/release/time-masternode`
- CLI: `target/release/time-cli`
- API: `target/release/time-api`

## ⚡ Performance

| Command | Build Time | Disk Space |
|---------|------------|------------|
| `-p time-masternode -p time-cli` | ~3 min | ~800 MB |
| `--workspace --exclude wallet-gui` | ~5 min | ~1.2 GB |
| Full workspace | ~10 min | ~2.5 GB |

---

**Remember**: Use `time-masternode`, not `masternode`! 🎯
