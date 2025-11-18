# Wallet File Protection - Fix for time-wallet.dat Deletion

**Date**: November 18, 2025  
**Issue**: time-wallet.dat file was being deleted/overwritten without backup

---

## Problem

The wallet file `time-wallet.dat` was being accidentally deleted or overwritten because:

1. **No atomic writes**: Crash during save could corrupt the file
2. **No existence check**: Creating a new wallet would silently overwrite existing one
3. **No backup on replace**: Replacing wallet would lose old wallet permanently

---

## Solution: Keep It Simple

**Key Insight**: `time-wallet.dat` contains ONLY derived data from the mnemonic:
- Encrypted mnemonic phrase
- xpub (derived from mnemonic)
- Master key (derived from mnemonic)

**Therefore**: The file should be written **ONLY ONCE** during wallet creation, never modified after.

### Changes Made

#### 1. Atomic Write for Safety

**File**: `wallet-gui/src/wallet_dat.rs`

```rust
pub fn save(&self) -> Result<(), WalletDatError> {
    // Write to temporary file first
    let temp_path = path.with_extension("dat.tmp");
    fs::write(&temp_path, &data)?;

    // Atomic rename (crash-safe)
    fs::rename(&temp_path, &path)?;
}
```

**Benefits**:
- ✅ Atomic operation (all-or-nothing)
- ✅ Crash-safe (no partial writes)
- ✅ Simple and reliable

#### 2. Existence Check Prevents Accidents

**File**: `wallet-gui/src/wallet_manager.rs`

```rust
pub fn create_from_mnemonic(...) -> Result<Self, WalletDatError> {
    // Check if wallet already exists
    if wallet_path.exists() {
        log::warn!("Wallet already exists. Loading existing wallet instead.");
        return Self::load(network);
    }
    
    // Only create if doesn't exist
    wallet_dat.save()?;  // Saved ONCE
}
```

**Benefits**:
- ✅ Won't accidentally overwrite
- ✅ Loads existing wallet instead
- ✅ Clear warning in logs

#### 3. Explicit Replace with Backup

**File**: `wallet-gui/src/wallet_manager.rs`

```rust
pub fn replace_from_mnemonic(...) -> Result<Self, WalletDatError> {
    // Create backup before replacing
    if wallet_path.exists() {
        let backup_path = wallet_path.with_extension("dat.old");
        fs::copy(&wallet_path, &backup_path)?;
        log::warn!("Old wallet backed up to: {}", backup_path.display());
    }
    
    // Save new wallet
    wallet_dat.save()?;  // Saved ONCE
}
```

**Benefits**:
- ✅ Explicit intent to replace
- ✅ Old wallet saved as `.dat.old`
- ✅ User knows replacement occurred

---

## When Does wallet.dat Get Written?

**ONLY in these 2 scenarios:**

1. **First-time wallet creation**: `create_from_mnemonic()` → saves once
2. **Wallet replacement**: `replace_from_mnemonic()` → backs up old, saves new once

**NEVER during:**
- ❌ Normal wallet operations
- ❌ Receiving transactions
- ❌ Sending transactions
- ❌ Address generation
- ❌ Balance updates

All transaction/address data goes in `wallet.db` (separate SQLite database).

---

## File Locations

| File | Purpose | Modified? |
|------|---------|-----------|
| `time-wallet.dat` | Mnemonic + keys | **Once at creation** |
| `time-wallet.dat.old` | Old wallet backup | Only on replace |
| `time-wallet.dat.tmp` | Atomic write temp | Deleted after save |
| `wallet.db` | Transactions, addresses | Frequently updated |

**Default Locations**:
- **Linux**: `~/.local/share/time-coin/mainnet/`
- **macOS**: `~/Library/Application Support/time-coin/mainnet/`
- **Windows**: `C:\Users\<username>\AppData\Roaming\time-coin\mainnet\`

---

## Recovery Instructions

### If time-wallet.dat is missing:

**Option 1: Restore from backup**
```bash
# Check for .dat.old backup (from replacement)
ls ~/.local/share/time-coin/mainnet/time-wallet.dat.old

# Restore it
cp time-wallet.dat.old time-wallet.dat
```

**Option 2: Restore from mnemonic**
- You MUST have your 12-word mnemonic phrase
- Use "Restore from Mnemonic" in wallet GUI
- Or: `WalletManager::replace_from_mnemonic(network, mnemonic)`

**ALWAYS KEEP YOUR MNEMONIC PHRASE SAFE!**

---

## Protection Summary

### What's Protected:

1. ✅ **Atomic writes**: Crash during save won't corrupt file
2. ✅ **Existence check**: Won't accidentally overwrite existing wallet
3. ✅ **Backup on replace**: Old wallet saved before replacement
4. ✅ **Simple design**: Written once, read many times

### What's NOT Protected:

- ❌ User manually deleting files (need mnemonic to restore)
- ❌ Disk failure (need off-site backup of mnemonic)
- ❌ Ransomware (need cold storage of mnemonic)

**Bottom line**: Keep your 12-word mnemonic phrase safe. That's your ultimate backup.

---

## Changes Summary

**Files Modified**: 3

1. `wallet-gui/src/wallet_dat.rs`
   - Simplified save() to atomic write only
   - Removed unnecessary backup logic
   - Added comment about one-time save

2. `wallet-gui/src/wallet_manager.rs`
   - Added existence check in create_from_mnemonic()
   - Added replace_from_mnemonic() with .dat.old backup
   - Added fs import

3. `wallet-gui/src/main.rs`
   - Changed to use replace_from_mnemonic() when appropriate

**Total Lines**: ~30 added, ~10 modified

---

## Design Philosophy

**Keep it simple**: 
- Wallet.dat = immutable derived data from mnemonic
- Write once, read many times
- Mnemonic phrase is the source of truth
- Everything else can be regenerated

**Result**: No complex backup schemes needed. File is protected by:
1. Atomic writes (crash-safe)
2. Existence checks (accident-proof)  
3. Explicit replace with backup (user-controlled)

---

**Fixed by**: GitHub Copilot CLI  
**Date**: November 18, 2025  
**Status**: Complete - Simplified approach
