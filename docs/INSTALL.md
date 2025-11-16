# TIME Coin Installation Guide

## Installation Methods

### 1. System-Wide Installation (Recommended for Ubuntu/Linux)

Install binaries to `/usr/local/bin` for all users:

```bash
# Navigate to project root
cd /path/to/time-coin

# Install main daemon
sudo cargo install --path cli --root /usr/local

# This installs:
# - /usr/local/bin/timed (main daemon)
# - /usr/local/bin/time-cli (CLI tool)

# Verify installation
which timed
timed --version
```

### 2. Manual Build and Install

Build and copy binaries manually:

```bash
# Build all release binaries
cargo build --release --workspace

# Copy to /usr/local/bin (requires sudo)
sudo cp target/release/timed /usr/local/bin/
sudo cp target/release/time-cli /usr/local/bin/

# Optional: Install wallet GUI
sudo cp target/release/wallet-gui /usr/local/bin/

# Optional: Install tools
sudo cp target/release/tx-perf-test /usr/local/bin/

# Set permissions
sudo chmod +x /usr/local/bin/timed
sudo chmod +x /usr/local/bin/time-cli

# Verify
timed --version
time-cli --version
```

### 3. User-Only Installation

Install to `~/.cargo/bin` (no sudo required):

```bash
# Install for current user only
cargo install --path cli

# Ensure ~/.cargo/bin is in PATH
echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc

# Verify
which timed
timed --version
```

### 4. Custom Location

Install to a custom directory:

```bash
# Install to custom location
sudo cargo install --path cli --root /opt/timecoin

# This installs to:
# /opt/timecoin/bin/timed
# /opt/timecoin/bin/time-cli

# Add to PATH
echo 'export PATH="/opt/timecoin/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

## Post-Installation Setup

### 1. Create Configuration Directory

```bash
# Create config directory
sudo mkdir -p /etc/timecoin

# Create data directory
sudo mkdir -p /var/lib/timecoin

# Set permissions (adjust user/group as needed)
sudo chown -R $USER:$USER /var/lib/timecoin
```

### 2. Configure Environment Variables

```bash
# Edit /etc/environment (system-wide)
sudo nano /etc/environment

# Add these lines:
TIMECOIN_SEEDS="seed1:24100,seed2:24100"
NODE_PUBLIC_IP="your.public.ip"
```

Or for user-only:

```bash
# Edit ~/.bashrc
nano ~/.bashrc

# Add these lines:
export TIMECOIN_SEEDS="seed1:24100,seed2:24100"
export NODE_PUBLIC_IP="your.public.ip"

# Apply changes
source ~/.bashrc
```

### 3. Create Systemd Service

```bash
# Create service file
sudo nano /etc/systemd/system/timed.service
```

Add this content:

```ini
[Unit]
Description=TIME Coin Daemon
After=network.target

[Service]
Type=simple
User=timecoin
Group=timecoin
WorkingDirectory=/var/lib/timecoin
ExecStart=/usr/local/bin/timed
Restart=always
RestartSec=10

# Environment variables
Environment="TIMECOIN_SEEDS=seed1:24100,seed2:24100"
Environment="NODE_PUBLIC_IP=your.public.ip"
Environment="RUST_LOG=info"

# Security settings
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=full
ProtectHome=true

[Install]
WantedBy=multi-user.target
```

Enable and start the service:

```bash
# Create timecoin user
sudo useradd -r -s /bin/false timecoin

# Set permissions
sudo chown -R timecoin:timecoin /var/lib/timecoin

# Reload systemd
sudo systemctl daemon-reload

# Enable service (start on boot)
sudo systemctl enable timed

# Start service
sudo systemctl start timed

# Check status
sudo systemctl status timed

# View logs
sudo journalctl -u timed -f
```

## Quick Installation Script

Create a file `install.sh`:

```bash
#!/bin/bash
set -e

echo "Installing TIME Coin..."

# Build release binaries
echo "Building binaries..."
cargo build --release --workspace

# Install to /usr/local/bin
echo "Installing to /usr/local/bin..."
sudo cp target/release/timed /usr/local/bin/
sudo cp target/release/time-cli /usr/local/bin/
sudo chmod +x /usr/local/bin/timed
sudo chmod +x /usr/local/bin/time-cli

# Create directories
echo "Creating directories..."
sudo mkdir -p /etc/timecoin
sudo mkdir -p /var/lib/timecoin

# Create timecoin user
if ! id timecoin &>/dev/null; then
    echo "Creating timecoin user..."
    sudo useradd -r -s /bin/false timecoin
fi

# Set permissions
sudo chown -R timecoin:timecoin /var/lib/timecoin

echo ""
echo "✅ Installation complete!"
echo ""
echo "Next steps:"
echo "  1. Configure: sudo nano /etc/timecoin/config.toml"
echo "  2. Set seeds: export TIMECOIN_SEEDS='seed1:24100,seed2:24100'"
echo "  3. Start node: timed"
echo ""
echo "Or install as systemd service:"
echo "  sudo cp setup/timed.service /etc/systemd/system/"
echo "  sudo systemctl daemon-reload"
echo "  sudo systemctl enable timed"
echo "  sudo systemctl start timed"
```

Make it executable and run:

```bash
chmod +x install.sh
./install.sh
```

## Uninstallation

### Remove Binaries

```bash
sudo rm /usr/local/bin/timed
sudo rm /usr/local/bin/time-cli
sudo rm /usr/local/bin/wallet-gui
sudo rm /usr/local/bin/tx-perf-test
```

### Remove Service

```bash
sudo systemctl stop timed
sudo systemctl disable timed
sudo rm /etc/systemd/system/timed.service
sudo systemctl daemon-reload
```

### Remove Data (Optional)

```bash
sudo rm -rf /var/lib/timecoin
sudo rm -rf /etc/timecoin
sudo userdel timecoin
```

## Verification

After installation, verify everything works:

```bash
# Check binary location
which timed
which time-cli

# Check version
timed --version
time-cli --version

# Test run (Ctrl+C to stop)
timed

# If using systemd
sudo systemctl status timed
sudo journalctl -u timed -n 50
```

## Troubleshooting

### Binary not found

```bash
# Add /usr/local/bin to PATH if needed
echo 'export PATH="/usr/local/bin:$PATH"' >> ~/.bashrc
source ~/.bashrc
```

### Permission denied

```bash
# Fix permissions
sudo chmod +x /usr/local/bin/timed
sudo chmod +x /usr/local/bin/time-cli
```

### Service won't start

```bash
# Check logs
sudo journalctl -u timed -xe

# Check permissions
ls -la /usr/local/bin/timed
ls -la /var/lib/timecoin
```

## Platform-Specific Notes

### Ubuntu/Debian
- Default install location: `/usr/local/bin`
- Config location: `/etc/timecoin`
- Data location: `/var/lib/timecoin`

### Fedora/RHEL
- Same as Ubuntu/Debian
- Use `firewall-cmd` for firewall rules

### Arch Linux
- Can use `/usr/local/bin` or create AUR package
- Use `pacman` for dependencies

## Production Deployment

For production servers:

1. **Use systemd service**
2. **Set up monitoring** (journalctl, logging)
3. **Configure firewall** (allow port 24100 for testnet, 24000 for mainnet)
4. **Set up auto-updates** (systemd timer or cron)
5. **Configure backups** (wallet data, blockchain data)
6. **Set environment variables** (TIMECOIN_SEEDS, NODE_PUBLIC_IP)

## Summary

**Recommended installation for Ubuntu:**
```bash
# 1. Build
cargo build --release --workspace

# 2. Install
sudo cp target/release/timed /usr/local/bin/
sudo cp target/release/time-cli /usr/local/bin/

# 3. Verify
timed --version
```

**Or use cargo install:**
```bash
sudo cargo install --path cli --root /usr/local
```

Both methods install to `/usr/local/bin` which is the standard location for user-installed binaries on Linux systems.
