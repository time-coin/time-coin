# TIME Coin Wallet - Windows Installation Guide

## Version: v0.1.0-alpha

### System Requirements

- **Operating System:** Windows 10 or later (64-bit)
- **RAM:** 4 GB minimum, 8 GB recommended
- **Disk Space:** 100 MB for wallet + blockchain data
- **Internet Connection:** Required for syncing with the network

### Quick Start

1. **Extract** the ZIP file to your preferred location (e.g., `C:\Program Files\TimeCoin`)

2. **Run the Wallet**
   - Navigate to the `bin` folder
   - Double-click `time-coin-wallet.exe`

3. **First Launch**
   - If this is your first time, you'll be prompted to create a new wallet or restore from mnemonic
   - **IMPORTANT:** Write down your recovery phrase and store it safely!

### Features

- ✅ Create new wallet with 24-word recovery phrase
- ✅ Restore wallet from mnemonic
- ✅ Send and receive TIME coins
- ✅ View transaction history
- ✅ QR code generation for receiving
- ✅ Backup and restore wallet
- ✅ Testnet and Mainnet support
- ✅ Address book management

### Wallet Data Location

Your wallet data is stored in:
```
C:\Users\YourUsername\AppData\Local\time-coin-wallet\
```

This includes:
- Encrypted wallet file
- Transaction history
- Settings and preferences

### Network Selection

The wallet connects to:
- **Testnet:** `http://localhost:24100` (default for testing)
- **Mainnet:** Will be configured after mainnet launch

### Security Recommendations

1. **Backup Your Mnemonic**
   - Write it down on paper
   - Store in a secure location
   - Never share it with anyone
   - Never store it digitally (no screenshots, no cloud storage)

2. **Use a Strong Password**
   - Minimum 12 characters
   - Mix of uppercase, lowercase, numbers, and symbols

3. **Keep Software Updated**
   - Check for updates regularly
   - Updates include security patches

### Troubleshooting

#### Wallet won't start
- Check if Windows Defender is blocking the application
- Right-click wallet-gui.exe → Properties → Unblock

#### Can't connect to network
- Ensure you're running a TIME Coin node locally
- Check firewall settings
- Verify the node is running at `http://localhost:24100`

#### Transaction not showing
- Wait for network confirmation (1-2 minutes typical)
- Try refreshing the wallet
- Check if node is fully synced

### Getting Help

- **Documentation:** [GitHub Wiki](https://github.com/time-coin/time-coin/wiki)
- **Issues:** [GitHub Issues](https://github.com/time-coin/time-coin/issues)
- **Community:** [Discord/Telegram link]

### License

TIME Coin Wallet is dual-licensed under MIT and Apache 2.0 licenses.
See LICENSE-MIT and LICENSE-APACHE in the docs folder.

### Disclaimer

**THIS IS ALPHA SOFTWARE**

This wallet is in early development and should only be used on testnet.
Do not use with real funds until mainnet is officially launched.

The developers are not responsible for any loss of funds.
Always backup your recovery phrase!

---

**Version:** v0.1.0-alpha
**Build Date:** November 17, 2025
**Platform:** Windows x64
