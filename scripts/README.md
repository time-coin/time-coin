# TIME Coin Masternode Setup Scripts

## Complete Masternode Installation (Recommended)

### install-masternode.sh
Complete automated installation script for fresh Ubuntu servers. This single script handles everything from dependency installation to masternode activation.

**Features:**
- Clones TIME Coin repository (if not present)
- Installs all system dependencies
- Installs Rust toolchain
- Builds and installs binaries
- Creates systemd service
- Applies for testnet grant
- Generates masternode keypair
- Activates masternode with collateral
- Creates complete configuration

**Usage:**

```bash
# On a fresh Ubuntu server (22.04+ or 25.04):
wget -O install-masternode.sh https://raw.githubusercontent.com/your-org/time-coin/main/scripts/install-masternode.sh
sudo bash install-masternode.sh
```

Or if you already have the repository:

```bash
cd ~/time-coin/scripts
sudo ./install-masternode.sh
```

**What happens:**
1. ✅ Checks for root permissions
2. ✅ Clones repository (if needed)
3. ✅ Installs dependencies (build tools, Rust, etc.)
4. ✅ Builds timed and time-cli binaries
5. ✅ Installs binaries to /usr/local/bin
6. ✅ Creates node configuration
7. ✅ Sets up systemd service
8. ✅ Starts the node
9. ✅ Prompts for your email
10. ✅ Applies for testnet grant
11. ✅ Verifies email automatically
12. ✅ Generates masternode keypair
13. ✅ Activates masternode with 1000 TIME
14. ✅ Updates config with credentials
15. ✅ Restarts node with new configuration
16. ✅ Saves credentials securely

**Requirements:**
- Fresh Ubuntu server (22.04+)
- Internet connection
- Email address (for testnet grant)

**What you get:**
- Fully operational masternode
- 1000 TIME testnet collateral
- Complete configuration at `~/.timecoin/` (Bitcoin-style)
- Credentials saved to `~/.timecoin/masternode-credentials.txt`
- Systemd service running as `timed`

**After installation:**

```bash
# Check service status
sudo systemctl status timed

# View live logs
sudo journalctl -u timed -f

# Check node status
time-cli status

# View your credentials
cat ~/.timecoin/masternode-credentials.txt

# Check balance
time-cli balance YOUR_ADDRESS
```

---

## Other Scripts

### setup-masternode.sh
Legacy script for setting up masternode on an already-running node. Use `install-masternode.sh` instead for new installations.

### reset-blockchain.sh
Resets the blockchain to a fresh state with the new genesis block (October 12, 2024).

**WARNING:** This will erase all blockchain data! Wallet data will be preserved.

```bash
# Run with confirmation prompt
sudo ./scripts/reset-blockchain.sh

# Run without confirmation (automated)
sudo ./scripts/reset-blockchain.sh --yes
```

The script will:
1. Stop the TIME Coin node service
2. Create a backup of existing data
3. Clear the blockchain database
4. Remove the old genesis file
5. Install the new genesis block
6. Preserve wallet data and logs
7. Restart the node service

**What's Preserved:**
- Wallet data (`~/.timecoin/data/wallets`)
- Logs (`~/.timecoin/logs`)

**What's Removed:**
- Blockchain database (`~/.timecoin/data/blockchain`)
- Genesis file (`~/.timecoin/data/genesis.json`)

**Backups:** Created automatically in `/var/backups/time-coin-YYYYMMDD-HHMMSS/`

**Note:** The script automatically detects your data directory:
- New installations: `~/.timecoin` (Bitcoin-style)
- Legacy installations: `/var/lib/time-coin` (still supported)

---

## Security Notes

⚠️ **IMPORTANT:**
- Backup your credentials file immediately
- Never share your private key
- Keep credentials file secure (chmod 600)
- Store offline backup in safe location

## Support

- Documentation: `docs/GRANT_SYSTEM.md`
- Testing Guide: `docs/GRANT_TESTING.md`
- Issues: https://github.com/time-coin/time-coin/issues
