//! Single-instance enforcement using an OS advisory file lock.
//!
//! Creates a `.lock` file inside the wallet's network-specific data directory
//! and acquires an exclusive advisory lock on it.  The OS releases the lock
//! automatically even if the process is killed, so stale locks are never an
//! issue.  Two instances targeting different networks (mainnet vs testnet)
//! use different directories and therefore different lock files, so they can
//! coexist normally.

use fs4::fs_std::FileExt;
use std::fs::{self, OpenOptions};
use std::path::PathBuf;

/// RAII guard that holds the exclusive advisory lock for the lifetime of the
/// process.  Dropping it releases the lock.
pub struct InstanceLock {
    _file: std::fs::File,
    path: PathBuf,
}

impl Drop for InstanceLock {
    fn drop(&mut self) {
        // Best-effort: unlock explicitly before the file handle closes.
        let _ = self._file.unlock();
        log::debug!("Instance lock released: {}", self.path.display());
    }
}

/// Try to acquire the single-instance lock for `wallet_dir`.
///
/// # Returns
///
/// - `Ok(InstanceLock)` — this is the only running instance; keep the guard
///   alive for the entire process lifetime.
/// - `Err(String)` — another instance already holds the lock; the message is
///   human-readable and suitable for display in an error dialog.
pub fn acquire(wallet_dir: &PathBuf) -> Result<InstanceLock, String> {
    // Ensure the directory exists before we try to create a file in it.
    fs::create_dir_all(wallet_dir).map_err(|e| {
        format!(
            "Cannot create wallet directory '{}': {e}",
            wallet_dir.display()
        )
    })?;

    let lock_path = wallet_dir.join(".lock");

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(|e| format!("Cannot open lock file '{}': {e}", lock_path.display()))?;

    file.try_lock_exclusive().map_err(|_| {
        "TIME Coin Wallet is already running.\n\n\
         Only one instance can be open at a time. \
         Close the existing window and try again."
            .to_string()
    })?;

    log::info!("Instance lock acquired: {}", lock_path.display());
    Ok(InstanceLock {
        _file: file,
        path: lock_path,
    })
}
