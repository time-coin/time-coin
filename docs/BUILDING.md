# Building TIME Coin

## Prerequisites

### Ubuntu/Debian
```bash
sudo apt update
sudo apt install -y build-essential pkg-config libssl-dev clang libclang-dev
```

### macOS
```bash
brew install llvm
```

### Windows
- Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/)
- Install [Rust](https://rustup.rs/)

## Build Commands

### Full Build (All Modules)
```bash
cargo build --all --release
```

### Individual Modules
```bash
# Core only
cargo build --package time-core --release

# Node binary
cargo build --package timed --release

# Consensus
cargo build --package time-consensus --release
```

## Common Issues

### libclang not found
```bash
# Ubuntu/Debian
sudo apt install -y libclang-dev

# Then set path if needed
export LIBCLANG_PATH=/usr/lib/llvm-14/lib
```

### RocksDB build slow
This is normal on first build (5-15 minutes). Subsequent builds are fast.

### zstd-sys errors
```bash
# Use system zstd
sudo apt install -y libzstd-dev
export ZSTD_SYS_USE_PKG_CONFIG=1
cargo build --all --release
```

## Testing

```bash
# Run all tests
cargo test --all

# Run specific module tests
cargo test --package time-consensus

# Run with output
cargo test --all -- --nocapture
```

## Installation

After building:
```bash
sudo cp target/release/timed /usr/local/bin/
sudo chmod +x /usr/local/bin/timed
timed --version
```
