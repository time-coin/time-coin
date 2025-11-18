//! TIME Coin Storage Layer - Simple File-Based Snapshots
//!
//! Designed for daily persistence model:
//! - State stays in memory all day
//! - Snapshot written once per 24 hours
//! - Quick load on startup

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Snapshot not found: {0}")]
    SnapshotNotFound(String),
}

/// Simple file-based storage for daily snapshots
pub struct Storage {
    data_dir: PathBuf,
}

impl Storage {
    /// Open storage directory
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let data_dir = path.as_ref().to_path_buf();

        // Create directory if it doesn't exist
        if !data_dir.exists() {
            fs::create_dir_all(&data_dir)?;
        }

        Ok(Self { data_dir })
    }

    /// Save a snapshot (JSON for readability, Bincode for speed)
    pub fn save_snapshot<T: Serialize>(&self, name: &str, data: &T) -> Result<(), StorageError> {
        let json_path = self.data_dir.join(format!("{}.json", name));
        let bin_path = self.data_dir.join(format!("{}.bin", name));

        // Save as JSON (human-readable backup)
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        fs::write(&json_path, json)?;

        // Save as Bincode (fast loading)
        let bin = bincode::serialize(data)
            .map_err(|e| StorageError::SerializationError(e.to_string()))?;
        fs::write(&bin_path, bin)?;

        Ok(())
    }

    /// Load a snapshot (tries Bincode first, falls back to JSON)
    pub fn load_snapshot<T: for<'de> Deserialize<'de>>(
        &self,
        name: &str,
    ) -> Result<T, StorageError> {
        let bin_path = self.data_dir.join(format!("{}.bin", name));
        let json_path = self.data_dir.join(format!("{}.json", name));

        // Try bincode first (faster)
        if bin_path.exists() {
            let data = fs::read(&bin_path)?;
            return bincode::deserialize(&data)
                .map_err(|e| StorageError::SerializationError(e.to_string()));
        }

        // Fall back to JSON
        if json_path.exists() {
            let data = fs::read_to_string(&json_path)?;
            return serde_json::from_str(&data)
                .map_err(|e| StorageError::SerializationError(e.to_string()));
        }

        Err(StorageError::SnapshotNotFound(name.to_string()))
    }

    /// Check if snapshot exists
    pub fn has_snapshot(&self, name: &str) -> bool {
        let bin_path = self.data_dir.join(format!("{}.bin", name));
        let json_path = self.data_dir.join(format!("{}.json", name));
        bin_path.exists() || json_path.exists()
    }

    /// List all snapshots
    pub fn list_snapshots(&self) -> Result<Vec<String>, StorageError> {
        let mut snapshots = Vec::new();

        for entry in fs::read_dir(&self.data_dir)? {
            let entry = entry?;
            let path = entry.path();

            if let Some(name) = path.file_stem() {
                if let Some(name_str) = name.to_str() {
                    if !snapshots.contains(&name_str.to_string()) {
                        snapshots.push(name_str.to_string());
                    }
                }
            }
        }

        Ok(snapshots)
    }

    /// Delete a snapshot
    pub fn delete_snapshot(&self, name: &str) -> Result<(), StorageError> {
        let bin_path = self.data_dir.join(format!("{}.bin", name));
        let json_path = self.data_dir.join(format!("{}.json", name));

        if bin_path.exists() {
            fs::remove_file(bin_path)?;
        }
        if json_path.exists() {
            fs::remove_file(json_path)?;
        }

        Ok(())
    }

    /// Get storage directory path
    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    use tempfile::tempdir;

    #[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
    struct TestData {
        value: u64,
        name: String,
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempdir().unwrap();
        let storage = Storage::open(dir.path()).unwrap();

        let data = TestData {
            value: 12345,
            name: "test".to_string(),
        };

        storage.save_snapshot("test", &data).unwrap();
        let loaded: TestData = storage.load_snapshot("test").unwrap();

        assert_eq!(data, loaded);
    }

    #[test]
    fn test_has_snapshot() {
        let dir = tempdir().unwrap();
        let storage = Storage::open(dir.path()).unwrap();

        assert!(!storage.has_snapshot("test"));

        let data = TestData {
            value: 123,
            name: "test".to_string(),
        };
        storage.save_snapshot("test", &data).unwrap();

        assert!(storage.has_snapshot("test"));
    }

    #[test]
    fn test_list_snapshots() {
        let dir = tempdir().unwrap();
        let storage = Storage::open(dir.path()).unwrap();

        let data1 = TestData {
            value: 1,
            name: "one".to_string(),
        };
        let data2 = TestData {
            value: 2,
            name: "two".to_string(),
        };

        storage.save_snapshot("snapshot1", &data1).unwrap();
        storage.save_snapshot("snapshot2", &data2).unwrap();

        let snapshots = storage.list_snapshots().unwrap();
        assert_eq!(snapshots.len(), 2);
    }
}
