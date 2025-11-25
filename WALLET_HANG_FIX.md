# Wallet Peer Discovery Hanging - IMPORTANT

## Issue
Wallet hangs on: `Discovering peers via TCP from: 50.38.156.179:24100`

## Root Cause
The wallet and masternode are now using updated handshake protocol (magic bytes), but if the masternode hasn't been updated and restarted, there's a protocol mismatch causing the wallet to hang waiting for a response.

## Solution

### **Update Masternode First** (REQUIRED)

On the masternode server:

```bash
cd ~/time-coin
git pull
cargo build --release --bin timed
sudo systemctl restart timed
```

### Then Rebuild Wallet

```bash
cargo build --release -p wallet-gui
```

## Why This Happens

**Old masternode** (before protocol fixes):
- Doesn't send magic bytes in handshake
- Wallet waits forever for magic bytes that never come
- Connection hangs

**Updated masternode** (after protocol fixes):
- Sends proper handshake with magic bytes
- Wallet receives response correctly
- Connection works

## Verification

After updating masternode, check logs:
```bash
journalctl -u timed -f
```

You should see proper handshake exchanges without errors.

## Quick Fix (Temporary)

If you can't update the masternode right now, you can comment out the peer discovery in the wallet, but you'll lose automatic peer discovery features.

---

**Date**: 2025-11-25  
**Status**: Waiting for masternode update
