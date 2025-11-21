# Testnet Reset Guide

## Overview
This guide explains how to reset the TIME Coin testnet to a clean state with a fresh genesis block.

## Prerequisites
- All nodes running version f7a26b9 or later
- Access to one "seed node" that will load genesis from file
- `genesis-testnet.json` file in the `config/` directory

## Steps

### 1. Stop All Nodes

On **all** testnet nodes, stop the daemon:

```bash
sudo systemctl stop timed
```

### 2. Clear Blockchain Data on All Nodes

On **all** nodes, remove the blockchain data:

```bash
# Clear blockchain data (preserves wallet)
sudo rm -rf /var/lib/time-coin/blockchain/*
```

### 3. Configure ONE Seed Node to Load Genesis

On **ONE** node (recommend the most stable/reliable node), ensure the genesis file path is set in the config:

```bash
# Edit testnet.toml
sudo nano /root/time-coin-node/config/testnet.toml
```

Verify this line exists and is uncommented:

```toml
[blockchain]
genesis_file = "/root/time-coin-node/config/genesis-testnet.json"
```

### 4. Start the Seed Node First

On the seed node:

```bash
sudo systemctl start timed

# Watch logs to confirm genesis loads
sudo journalctl -u timed -f
```

You should see:
```
📥 Loading genesis block from file...
✅ Genesis block loaded: 9a81c7599d8eed97...
```

### 5. Start All Other Nodes

Once the seed node has genesis loaded (height 0), start all other nodes:

```bash
# On each other node
sudo systemctl start timed
```

These nodes will:
1. Start with empty blockchain
2. Discover the seed node via peer discovery
3. Download genesis block from the seed node
4. Continue syncing normally

### 6. Verify Network Consensus

Check that all nodes reach consensus:

```bash
# On each node
curl -s http://localhost:24101/blockchain/info | jq
```

All nodes should report:
- `height: 0` initially
- Same `tip_hash` (genesis hash)
- Gradually increasing height as new blocks are created

## Troubleshooting

### Genesis file not found
- Check the path in `testnet.toml` is correct
- Ensure `genesis-testnet.json` exists in `/root/time-coin-node/config/`
- Check file permissions: `sudo chmod 644 /root/time-coin-node/config/genesis-testnet.json`

### Nodes can't sync from seed
- Verify seed node is reachable: `curl http://<seed-ip>:24101/blockchain/info`
- Check peer discovery is working: logs should show "Connected to X peers"
- Ensure firewall allows port 24100 (P2P) and 24101 (API)

### Fork detection after reset
- This is normal if not all nodes were cleared
- The network will auto-resolve forks once majority has clean state
- Worst case: stop, clear, and restart the problematic node

## Alternative: Comment Out Genesis File

If you want ALL nodes to download from network (no local genesis file):

```toml
[blockchain]
# genesis_file = "/root/time-coin-node/config/genesis-testnet.json"  # Commented out
```

This requires at least one node somewhere in the network to have genesis. Good for normal operation after initial setup.

## Expected Timeline

1. **Minute 0**: Seed node starts, loads genesis
2. **Minute 1-2**: Other nodes start, discover seed, download genesis
3. **Minute 3-5**: Network reaches BFT consensus (3+ masternodes)
4. **Next midnight UTC**: First block (height 1) is created via consensus

## Notes

- Genesis block has special hash: `9a81c7599d8eed97...`
- Genesis timestamp: 2025-10-12T00:00:00Z
- Initial supply: 116.53781624 TIME (in genesis address)
- Blocks are created every 24 hours at midnight UTC
