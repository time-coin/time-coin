# GUI Wallet Address Derivation Bug - Root Cause Analysis

## Problem

GUI wallet transactions are not being found by masternode. Transactions sent to GUI wallet addresses don't appear even though they're in the blockchain.

## Root Cause: Incorrect BIP-44 Derivation Path

### Expected BIP-44 Path
```
m / 44' / coin_type' / account' / change / address_index
m / 44' / 0' / 0' / 0 / 0   (first receiving address)
m / 44' / 0' / 0' / 0 / 1   (second receiving address)
m / 44' / 0' / 0' / 1 / 0   (first change address)
```

Where:
- `44'` = BIP-44 constant (hardened)
- `0'` = Coin type (hardened, 0 for Bitcoin/testnet)
- `0'` = Account index (hardened, typically 0)
- `0` = Chain (0 = external/receiving, 1 = internal/change)
- `0` = Address index (0, 1, 2, ...)

### What GUI Wallet Does (WRONG)

**File:** `wallet-gui/src/wallet_dat.rs:111`
```rust
pub fn derive_keypair(&self, index: u32) -> Result<Keypair, WalletDatError> {
    let mnemonic = self.get_mnemonic()?;
    
    // ❌ WRONG: Uses account_index parameter as address index!
    let keypair = mnemonic_to_keypair_hd(&mnemonic, "", index)?;
    //                                                     ^^^^^
    //                                                  This is address index 0,1,2
    //                                              but goes to ACCOUNT level!
    Ok(keypair)
}
```

**File:** `wallet/src/mnemonic.rs:147`
```rust
pub fn mnemonic_to_keypair_hd(phrase: &str, passphrase: &str, account_index: u32) {
    // BIP-44 path: m/44'/0'/account'
    let path_str = format!("m/44'/0'/{}'", account_index);
    //                                     ^^^^^^^^^^^^^
    //                                  Hardened derivation at ACCOUNT level
}
```

**Result:** GUI wallet derives addresses at:
```
m / 44' / 0' / 0'    (index 0)
m / 44' / 0' / 1'    (index 1)
m / 44' / 0' / 2'    (index 2)
```

This is **wrong** - these are ACCOUNT paths, not ADDRESS paths!

### What Masternode Expects (CORRECT)

**File:** `wallet/src/mnemonic.rs:262`
```rust
pub fn xpub_to_address(xpub_str: &str, change: u32, index: u32, network: NetworkType) {
    let xpub: XPub = xpub_str.parse()?;  // xpub is already at m/44'/0'/0'
    
    // ✅ CORRECT: Derives change/{index} from xpub
    let change_key = xpub.derive_child(ChildNumber::new(change, false))?;
    let address_key = change_key.derive_child(ChildNumber::new(index, false))?;
    
    // Creates address from public key
    let address = Address::from_public_key(&address_key.public_key().to_bytes(), network)?;
}
```

**Result:** Masternode derives addresses at:
```
m / 44' / 0' / 0' / 0 / 0    (change=0, index=0)
m / 44' / 0' / 0' / 0 / 1    (change=0, index=1)
m / 44' / 0' / 0' / 0 / 2    (change=0, index=2)
```

This is **correct** BIP-44 standard!

---

## Why This Causes the Problem

### Scenario:
1. GUI wallet generates address #0
   - Derives at: `m/44'/0'/0'` ❌
   - Shows address: `TIME1abc...`

2. User receives transaction to `TIME1abc...`
   - Transaction recorded in blockchain ✅

3. GUI wallet registers xpub with masternode
   - Xpub is at level: `m/44'/0'/0'` ✅

4. Masternode scans blockchain for xpub addresses
   - Derives at: `m/44'/0'/0'/0/0` ✅ (correct BIP-44)
   - Generates address: `TIME1xyz...` ≠ `TIME1abc...` ❌

5. **MISMATCH!**
   - GUI address: derived from `m/44'/0'/0'`
   - Masternode address: derived from `m/44'/0'/0'/0/0`
   - **Different public keys = Different addresses!**

6. Masternode never finds the transaction because it's looking for `TIME1xyz...` but the transaction is sent to `TIME1abc...`

---

## Summary

**Root Cause:** GUI wallet uses `mnemonic_to_keypair_hd(index)` which derives at account level `m/44'/0'/{index}'` instead of address level `m/44'/0'/0'/0/{index}`.

**Impact:** GUI wallet and masternode generate different addresses from the same xpub, causing transaction sync to fail.

**Fix:** Update GUI wallet to use proper BIP-44 derivation at address level, not account level.
