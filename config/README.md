# TIME Coin Configuration Files

## Testnet Setup

### 1. Create Genesis File
```bash
# Option A: Use the script
./scripts/create-genesis.sh ~/time-coin-node/config

# Option B: Copy from repo
cp config/genesis-testnet.json ~/time-coin-node/config/
```

### 2. Update Config Path

Edit `testnet.toml` and update the genesis path:
```toml
[blockchain]
# Use absolute path (recommended for systemd services)
genesis_file = "/root/time-coin-node/config/genesis-testnet.json"

# Or use $HOME variable (works in most cases)
genesis_file = "$HOME/time-coin-node/config/genesis-testnet.json"
```

### 3. Verify Genesis Loads

After starting the node, check logs:
```bash
sudo journalctl -u time-node -n 50 --no-pager | grep -A 10 "Genesis"
```

You should see:
```
╔═══════════════════════════════════════════════════╗
║         GENESIS BLOCK LOADED                      ║
╚═══════════════════════════════════════════════════╝
```

## Files

- `genesis-testnet.json` - Testnet genesis block (20M TIME supply)
- `testnet.toml` - Example testnet configuration
- `README.md` - This file

## Troubleshooting

**Genesis not found error:**
- Make sure path in `testnet.toml` uses absolute path or `$HOME` (not `~`)
- Verify file exists: `ls -la /root/time-coin-node/config/genesis-testnet.json`
- Check config: `cat ~/time-coin-node/config/testnet.toml | grep genesis_file`
