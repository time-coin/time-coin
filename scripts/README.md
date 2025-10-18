# TIME Coin Masternode Setup Scripts

## Quick Start

To set up a masternode with a 1000 TIME grant:

```bash
# Download the script
curl -O https://raw.githubusercontent.com/time-coin/time-coin/main/scripts/setup-masternode.sh

# Make it executable
chmod +x setup-masternode.sh

# Run it
./setup-masternode.sh
```

The script will:
1. Apply for a grant with your email
2. Verify your email automatically
3. Generate your masternode keypair
4. Activate your masternode with 1000 TIME
5. Update your node configuration
6. Restart your node
7. Save your credentials securely

## What You Need

- Ubuntu/Linux server
- TIME Coin node running
- Email address

## What You Get

- 1000 TIME locked to your masternode
- Entry tier masternode status
- Credentials saved to `~/time-coin-node/masternode-credentials.txt`
- Auto-configured node

## After Setup

Check your masternode:
```bash
# View credentials
cat ~/time-coin-node/masternode-credentials.txt

# Check balance
curl http://localhost:24101/balance/YOUR_ADDRESS

# View logs
tail -f ~/time-coin-node/logs/node.log
```

## Security

⚠️ **IMPORTANT:**
- Backup your credentials file immediately
- Never share your private key
- Keep credentials file secure (chmod 600)
- Store offline backup in safe location

## Support

- Documentation: `docs/GRANT_SYSTEM.md`
- Testing Guide: `docs/GRANT_TESTING.md`
- Issues: https://github.com/time-coin/time-coin/issues
