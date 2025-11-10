# TIME Coin Configuration

## Testnet Setup

1. **Copy config file:**
```bash
   mkdir -p ~/time-coin-node/config
   cp config/testnet.toml ~/time-coin-node/config/
   cp config/genesis-testnet.json ~/time-coin-node/config/
```

2. **Edit paths in testnet.toml:**
   - If running as root: Use `/root/time-coin-node/config/genesis-testnet.json`
   - If running as user: Use `/home/username/time-coin-node/config/genesis-testnet.json`
   - Or use `$HOME/time-coin-node/config/genesis-testnet.json` (recommended)

3. **Start node:**
```bash
   timed -c ~/time-coin-node/config/testnet.toml
```

## Path Expansion

The node expands these variables in config paths:
- `$HOME` - Your home directory
- `~` - Your home directory (at start of path)

## Genesis File

The genesis file must exist before starting the node. It defines:
- Network type (testnet/mainnet)
- Initial supply allocation
- Genesis timestamp
- Starting block hash
