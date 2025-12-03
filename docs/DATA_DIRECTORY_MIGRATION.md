# Data Directory Migration Guide

## Overview

TIME Coin has migrated to industry-standard data directory locations, following Bitcoin's convention.

## New Directory Structure

### Linux/Mac
```
~/.timecoin/
├── config/
│   └── testnet.toml
├── data/
│   ├── blockchain/
│   ├── wallets/
│   └── genesis.json
└── logs/
    └── node.log
```

### Windows
```
%APPDATA%\timecoin\
├── config\
│   └── testnet.toml
├── data\
│   ├── blockchain\
│   ├── wallets\
│   └── genesis.json
└── logs\
    └── node.log
```

## Old Locations (Legacy)

### Linux
- `/var/lib/time-coin/` (system-wide)
- `~/time-coin-node/` (user-specific)

### Windows
- `%LOCALAPPDATA%\time-coin\`

## Migration Instructions

### Linux/Mac

If you have an existing installation, you can migrate your data:

```bash
# Stop the node
sudo systemctl stop timed

# Option 1: Move from /var/lib/time-coin
sudo mv /var/lib/time-coin ~/.timecoin
sudo chown -R $USER:$USER ~/.timecoin

# Option 2: Move from ~/time-coin-node
mv ~/time-coin-node ~/.timecoin

# Restart the node
sudo systemctl start timed
```

### Windows

```powershell
# Stop the node
Stop-Service timed

# Move from old location
Move-Item "$env:LOCALAPPDATA\time-coin" "$env:APPDATA\timecoin"

# Restart the node
Start-Service timed
```

## Environment Variable Override

You can use a custom location by setting the `TIME_COIN_DATA_DIR` environment variable:

### Linux/Mac
```bash
export TIME_COIN_DATA_DIR=/custom/path/.timecoin
```

### Windows
```powershell
$env:TIME_COIN_DATA_DIR = "C:\custom\path\timecoin"
```

## Backward Compatibility

The node will automatically detect and use legacy paths if they exist:

1. First checks: `$TIME_COIN_DATA_DIR` (if set)
2. Then checks: `~/.timecoin` (new standard)
3. Falls back to: `/var/lib/time-coin` (legacy)

A warning will be displayed if using legacy paths.

## New Installations

All new installations automatically use the Bitcoin-style paths:

```bash
# Fresh installation
cd ~/time-coin/scripts
sudo ./install-masternode.sh

# Data will be created at: ~/.timecoin/
```

## Why This Change?

### Industry Standard
- Bitcoin: `~/.bitcoin`
- Ethereum: `~/.ethereum`
- Litecoin: `~/.litecoin`
- TIME Coin: `~/.timecoin` ✅

### Benefits
1. **User-centric**: Data stays in user's home directory
2. **Portable**: Easy to backup and move
3. **Familiar**: Follows established conventions
4. **Secure**: User-owned, not system-wide
5. **Cross-platform**: Consistent approach (home dir on Unix, AppData on Windows)

## Updated Scripts

All scripts have been updated to support the new paths:

- ✅ `install-masternode.sh` - Creates `~/.timecoin`
- ✅ `setup-masternode.sh` - Uses `~/.timecoin`
- ✅ `reset-blockchain.sh` - Detects data directory automatically
- ✅ `timed` binary - Uses new default paths

## Documentation Updates

- ✅ `docs/PATHS.md` - Updated with new structure
- ✅ `config/testnet.example.toml` - Uses new paths
- ✅ `scripts/README.md` - Updated examples

## Support

If you encounter issues during migration:

1. Check that the data directory exists and has correct permissions
2. Verify the node can read/write to the directory
3. Check logs for any path-related errors
4. Set `TIME_COIN_DATA_DIR` environment variable if needed

For questions or issues, please open a GitHub issue.
