# TIME Coin Build and Installation Complete Guide

**Table of Contents**
- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Building from Source](#building-from-source)
- [Installation Methods](#installation-methods)
- [Build Optimization](#build-optimization)
- [Verification](#verification)
- [Troubleshooting](#troubleshooting)
- [Platform-Specific Notes](#platform-specific-notes)

---

## Overview

This guide covers building TIME Coin from source and installing it on Linux, macOS, and Windows. Follow the steps for your platform to get a working TIME Coin node.

---

## Prerequisites

### Ubuntu/Debian

```bash
# Update package list
sudo apt update

# Install build tools
sudo apt install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    clang \
    libclang-dev \
    git \
    curl

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Verify Rust installation
rustc --version
cargo --version
```

### macOS

```bash
# Install Homebrew (if not installed)
/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"

# Install dependencies
brew install llvm openssl pkg-config

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# Set LLVM path
export PATH="/usr/local/opt/llvm/bin:$PATH"
```

### Windows

**Requirements**:
1. [Visual Studio Build Tools](https://visualstudio.microsoft.com/downloads/)
   - Select "Desktop development with C++"
   - Include Windows 10/11 SDK

2. [Rust](https://rustup.rs/)
   - Download and run `rustup-init.exe`
   - Follow installer prompts

3. Git (optional but recommended)
   - [Git for Windows](https://git-scm.com/download/win)

**PowerShell Setup**:
```powershell
# Verify installation
rustc --version
cargo --version
```

---

## Building from Source

### Clone Repository

```bash
# HTTPS
git clone https://github.com/time-coin/time-coin.git
cd time-coin

# SSH (if you have GitHub SSH keys)
git clone git@github.com:time-coin/time-coin.git
cd time-coin
```

### Quick Build (All Binaries)

```bash
# Release build (optimized, recommended)
cargo build --release --workspace

# Debug build (faster compilation, slower runtime)
cargo build --workspace

# Binaries will be in:
# target/release/ (or target/debug/)
```

### Build Individual Components

```bash
# Main node daemon
cargo build --release --bin timed

# CLI tool
cargo build --release --bin time-cli

# Dashboard
cargo build --release --bin time-dashboard

# Wallet GUI
cargo build --release --bin wallet-gui

# Core library only
cargo build --release --package time-core
```

### Fast Masternode Build

For quick masternode deployment:

```bash
# Build only what's needed for a masternode
cargo build --release --bin timed --bin time-cli

# Skip tests and docs
cargo build --release --bin timed --bin time-cli --no-default-features

# Parallel build (adjust based on CPU cores)
cargo build --release --bin timed --bin time-cli -j 4
```

---

## Installation Methods

### Method 1: System-Wide Installation (Recommended)

Install to `/usr/local/bin` for all users:

```bash
# Build and install in one step
sudo cargo install --path cli --root /usr/local

# This installs:
# - /usr/local/bin/timed
# - /usr/local/bin/time-cli

# Verify
which timed
timed --version
```

### Method 2: Manual Binary Copy

Build then manually copy:

```bash
# Build release binaries
cargo build --release --workspace

# Copy to system location (Linux/macOS)
sudo cp target/release/timed /usr/local/bin/
sudo cp target/release/time-cli /usr/local/bin/
sudo cp target/release/time-dashboard /usr/local/bin/

# Set executable permissions
sudo chmod +x /usr/local/bin/timed
sudo chmod +x /usr/local/bin/time-cli
sudo chmod +x /usr/local/bin/time-dashboard

# Verify
timed --version
```

### Method 3: User-Only Installation

Install to `~/.cargo/bin` (no sudo required):

```bash
# Install for current user
cargo install --path cli

# Ensure ~/.cargo/bin is in PATH
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Verify
timed --version
```

### Method 4: Automated Masternode Installation

Use the automated script for fresh Ubuntu servers:

```bash
# Download and run installation script
wget -O install-masternode.sh https://raw.githubusercontent.com/time-coin/time-coin/main/scripts/install-masternode.sh

# Make executable
chmod +x install-masternode.sh

# Run as root
sudo ./install-masternode.sh
```

This script will:
- Install all dependencies
- Build binaries
- Create systemd service
- Configure masternode
- Apply for testnet grant
- Activate masternode

---

## Build Optimization

### Faster Builds

#### 1. Use Faster Linker

**Linux (mold or lld)**:
```bash
# Install mold (fastest)
sudo apt install mold

# Or install lld
sudo apt install lld

# Configure in .cargo/config.toml
mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[target.x86_64-unknown-linux-gnu]
linker = "clang"
rustflags = ["-C", "link-arg=-fuse-ld=mold"]
EOF
```

**macOS (zld)**:
```bash
brew install michaeleisel/zld/zld

# Configure in .cargo/config.toml
mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[target.x86_64-apple-darwin]
rustflags = ["-C", "link-arg=-fuse-ld=/usr/local/bin/zld"]
EOF
```

#### 2. Increase Parallel Jobs

```bash
# Use all CPU cores
cargo build --release -j $(nproc)

# Or set permanently
export CARGO_BUILD_JOBS=$(nproc)
```

#### 3. Enable Incremental Compilation

```bash
# Set environment variable
export CARGO_INCREMENTAL=1

# Or in .cargo/config.toml
[build]
incremental = true
```

#### 4. Use System Libraries

```bash
# Use system zstd instead of building from source
export ZSTD_SYS_USE_PKG_CONFIG=1
sudo apt install libzstd-dev

cargo build --release
```

### Build Time Expectations

| Component | First Build | Rebuild |
|-----------|-------------|---------|
| Core | 5-10 min | 30 sec |
| Full workspace | 15-25 min | 1-2 min |
| Single binary | 10-15 min | 30 sec |

**Note**: RocksDB dependency takes 5-10 minutes on first build.

---

## Verification

### Test Installation

```bash
# Check binary locations
which timed
which time-cli

# Check versions
timed --version
time-cli --version

# Test help
timed --help
time-cli --help
```

### Run Tests

```bash
# Run all tests
cargo test --workspace

# Run specific module tests
cargo test --package time-core
cargo test --package time-consensus

# Run with output
cargo test --workspace -- --nocapture

# Quick test (skip slow tests)
cargo test --workspace --lib
```

### Build Verification

```bash
# Check binary sizes
ls -lh target/release/timed
ls -lh target/release/time-cli

# Typical sizes:
# timed: 50-100 MB (release)
# time-cli: 30-50 MB (release)

# Check dependencies
ldd target/release/timed  # Linux
otool -L target/release/timed  # macOS
```

---

## Troubleshooting

### Common Build Errors

#### Error: "libclang not found"

**Linux**:
```bash
sudo apt install -y libclang-dev

# If still fails, set path explicitly
export LIBCLANG_PATH=/usr/lib/llvm-14/lib
cargo build --release
```

**macOS**:
```bash
brew install llvm
export PATH="/usr/local/opt/llvm/bin:$PATH"
```

#### Error: "zstd-sys build failed"

```bash
# Use system zstd
sudo apt install -y libzstd-dev
export ZSTD_SYS_USE_PKG_CONFIG=1
cargo clean
cargo build --release
```

#### Error: "RocksDB build takes forever"

This is normal! First build of RocksDB takes 5-15 minutes.

**Speed it up**:
```bash
# Use more cores
cargo build --release -j $(nproc)

# Or install system RocksDB
sudo apt install -y librocksdb-dev
```

#### Error: "openssl-sys build failed"

**Linux**:
```bash
sudo apt install -y libssl-dev pkg-config
```

**macOS**:
```bash
brew install openssl
export OPENSSL_DIR=/usr/local/opt/openssl
```

**Windows**:
```powershell
# Use vcpkg
vcpkg install openssl:x64-windows
```

#### Error: "linking with `cc` failed"

**Linux**:
```bash
# Install build essentials
sudo apt install -y build-essential

# Or use clang
sudo apt install -y clang
export CC=clang
export CXX=clang++
```

#### Error: "Out of memory"

```bash
# Reduce parallel jobs
cargo build --release -j 2

# Or add swap space
sudo fallocate -l 4G /swapfile
sudo chmod 600 /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile
```

### Platform-Specific Issues

#### Ubuntu 18.04/20.04

```bash
# Need newer Rust
rustup update stable

# Need newer LLVM
sudo apt install -y llvm-14 libclang-14-dev
export LIBCLANG_PATH=/usr/lib/llvm-14/lib
```

#### Windows Antivirus

Windows Defender may slow builds dramatically:

```powershell
# Add exclusion for project directory
Add-MpPreference -ExclusionPath "C:\path\to\time-coin"

# Add exclusion for cargo
Add-MpPreference -ExclusionPath "$env:USERPROFILE\.cargo"
```

#### macOS M1/M2 (ARM)

```bash
# Ensure using ARM Rust
rustup default stable-aarch64-apple-darwin

# Some dependencies may need Rosetta
softwareupdate --install-rosetta
```

---

## Platform-Specific Notes

### Linux

#### Systemd Service

After installation, set up as systemd service:

```bash
# Create service file
sudo nano /etc/systemd/system/timed.service

[Unit]
Description=TIME Coin Node
After=network.target

[Service]
Type=simple
User=timecoin
ExecStart=/usr/local/bin/timed --config ~/.timecoin/config/testnet.toml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target

# Enable and start
sudo systemctl daemon-reload
sudo systemctl enable timed
sudo systemctl start timed
```

### macOS

#### LaunchAgent

Create launch agent for automatic startup:

```bash
# Create plist
nano ~/Library/LaunchAgents/io.time-coin.timed.plist

<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
    <key>Label</key>
    <string>io.time-coin.timed</string>
    <key>ProgramArguments</key>
    <array>
        <string>/usr/local/bin/timed</string>
    </array>
    <key>RunAtLoad</key>
    <true/>
    <key>KeepAlive</key>
    <true/>
</dict>
</plist>

# Load agent
launchctl load ~/Library/LaunchAgents/io.time-coin.timed.plist
```

### Windows

#### Windows Service

Use NSSM (Non-Sucking Service Manager):

```powershell
# Download NSSM
# https://nssm.cc/download

# Install service
nssm install timed "C:\path\to\timed.exe"
nssm set timed AppDirectory "C:\Users\YourUser\.timecoin"

# Start service
nssm start timed
```

---

## Next Steps

After successful build and installation:

1. **Configure Node**: Create configuration file
2. **Start Node**: Run timed daemon
3. **Check Status**: Use dashboard or time-cli
4. **Join Network**: Connect to peers
5. **Run Masternode**: Apply for grant and activate

See related guides:
- [Masternode Guide](MASTERNODE_GUIDE.md)
- [Configuration](../config/README.md)
- [Dashboard Guide](DASHBOARD_GUIDE.md)

---

**Consolidated from**:
- BUILDING.md
- BUILD_COMMANDS.md
- BUILD_OPTIMIZATION.md
- BUILD-VERIFICATION.md
- INSTALL.md
- FAST_MASTERNODE_BUILD.md
