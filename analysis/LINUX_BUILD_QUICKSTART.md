# TIME Coin - Quick Build Guide for Linux Servers

## 🚀 Simple One-Liner (Copy & Paste)

```bash
cargo build --release -p time-masternode -p time-cli
```

That's it! This builds:
- ✅ Masternode binary
- ✅ CLI tool
- ✅ Only necessary dependencies
- ❌ Skips GUI and dev tools

## 📦 Binaries Location

After build completes (~3-5 minutes):
```bash
./target/release/time-masternode    # Masternode
./target/release/time-cli            # CLI tool
```

## ⚡ Even Faster (One-Time Setup)

```bash
# Create optimized config (do this once)
mkdir -p .cargo && cat > .cargo/config.toml << 'EOF'
[net]
git-fetch-with-cli = true

[profile.release]
strip = true
debug = false
lto = "thin"
EOF

# Then build is the same
cargo build --release -p time-masternode -p time-cli
```

## 🎯 Alternative: Build Everything Except GUI

```bash
cargo build --release --workspace --exclude wallet-gui
```

This builds all tools and utilities (useful for development).

## 📊 Build Times

| Command | Time | Disk |
|---------|------|------|
| -p time-masternode -p time-cli | 3-5 min | ~1 GB |
| --workspace --exclude wallet-gui | 5-8 min | ~1.5 GB |

---

**Note**: First build downloads dependencies (slower). Subsequent builds are much faster!