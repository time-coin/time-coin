//! Masternode configuration file (masternode.conf) support
//!
//! Format: alias IP:port masternodeprivkey collateral_txid collateral_output_index
//!
//! Example:
//! mn1 192.168.1.100:24000 93HaYBVUCYjEMeeH1Y4sBGLALQZE1Yc1K64xiqgX37tGBDQL8Xg 2bcd3c84c84f87eaa86e4e56834c92927a07f9e18718810b92e0d0324456a67c 0

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum MasternodeConfigError {
    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Parse error at line {line}: {message}")]
    ParseError { line: usize, message: String },

    #[error("Invalid format: {0}")]
    InvalidFormat(String),

    #[error("Duplicate alias: {0}")]
    DuplicateAlias(String),

    #[error("Masternode not found: {0}")]
    MasternodeNotFound(String),
}

/// Single masternode configuration entry
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MasternodeConfigEntry {
    /// Alias/name for this masternode
    pub alias: String,

    /// IP address and port (e.g., "192.168.1.100:24000")
    pub ip_port: String,

    /// Masternode private key (for signing messages)
    pub masternode_privkey: String,

    /// Collateral transaction hash
    pub collateral_txid: String,

    /// Collateral output index
    pub collateral_output_index: u32,
}

impl MasternodeConfigEntry {
    /// Parse a single line from masternode.conf
    pub fn parse_line(line: &str, line_num: usize) -> Result<Self, MasternodeConfigError> {
        let line = line.trim();

        // Skip empty lines and comments
        if line.is_empty() || line.starts_with('#') {
            return Err(MasternodeConfigError::ParseError {
                line: line_num,
                message: "Empty or comment line".to_string(),
            });
        }

        // Split by whitespace
        let parts: Vec<&str> = line.split_whitespace().collect();

        if parts.len() != 5 {
            return Err(MasternodeConfigError::ParseError {
                line: line_num,
                message: format!(
                    "Expected 5 fields, got {}. Format: alias IP:port privkey txid index",
                    parts.len()
                ),
            });
        }

        let alias = parts[0].to_string();
        let ip_port = parts[1].to_string();
        let masternode_privkey = parts[2].to_string();
        let collateral_txid = parts[3].to_string();

        let collateral_output_index =
            parts[4]
                .parse::<u32>()
                .map_err(|_| MasternodeConfigError::ParseError {
                    line: line_num,
                    message: format!("Invalid output index: {}", parts[4]),
                })?;

        // Validate IP:port format
        if !ip_port.contains(':') {
            return Err(MasternodeConfigError::ParseError {
                line: line_num,
                message: format!("Invalid IP:port format: {}", ip_port),
            });
        }

        // Validate txid format (should be hex string)
        if collateral_txid.len() != 64 || !collateral_txid.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(MasternodeConfigError::ParseError {
                line: line_num,
                message: format!("Invalid transaction ID format: {}", collateral_txid),
            });
        }

        Ok(Self {
            alias,
            ip_port,
            masternode_privkey,
            collateral_txid,
            collateral_output_index,
        })
    }

    /// Format as a line for masternode.conf
    pub fn to_line(&self) -> String {
        format!(
            "{} {} {} {} {}",
            self.alias,
            self.ip_port,
            self.masternode_privkey,
            self.collateral_txid,
            self.collateral_output_index
        )
    }

    /// Validate the configuration entry
    pub fn validate(&self) -> Result<(), MasternodeConfigError> {
        // Validate alias
        if self.alias.is_empty() || self.alias.contains(char::is_whitespace) {
            return Err(MasternodeConfigError::InvalidFormat(
                "Invalid alias".to_string(),
            ));
        }

        // Validate IP:port
        if !self.ip_port.contains(':') {
            return Err(MasternodeConfigError::InvalidFormat(
                "Invalid IP:port format".to_string(),
            ));
        }

        // Validate private key (should be hex string)
        if self.masternode_privkey.is_empty() {
            return Err(MasternodeConfigError::InvalidFormat(
                "Empty private key".to_string(),
            ));
        }

        // Validate txid (should be 64-char hex string)
        if self.collateral_txid.len() != 64
            || !self.collateral_txid.chars().all(|c| c.is_ascii_hexdigit())
        {
            return Err(MasternodeConfigError::InvalidFormat(
                "Invalid transaction ID".to_string(),
            ));
        }

        Ok(())
    }
}

/// Masternode configuration file (masternode.conf)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MasternodeConfig {
    entries: Vec<MasternodeConfigEntry>,
}

impl MasternodeConfig {
    /// Create a new empty configuration
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
        }
    }

    /// Load configuration from file
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self, MasternodeConfigError> {
        let content = fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Parse configuration from string
    pub fn parse(content: &str) -> Result<Self, MasternodeConfigError> {
        let mut entries = Vec::new();
        let mut aliases = std::collections::HashSet::new();

        for (line_num, line) in content.lines().enumerate() {
            // Try to parse the line
            match MasternodeConfigEntry::parse_line(line, line_num + 1) {
                Ok(entry) => {
                    // Check for duplicate aliases
                    if !aliases.insert(entry.alias.clone()) {
                        return Err(MasternodeConfigError::DuplicateAlias(entry.alias.clone()));
                    }
                    entries.push(entry);
                }
                Err(MasternodeConfigError::ParseError { message, .. })
                    if message.contains("Empty or comment") =>
                {
                    // Skip empty lines and comments
                    continue;
                }
                Err(e) => return Err(e),
            }
        }

        Ok(Self { entries })
    }

    /// Save configuration to file
    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<(), MasternodeConfigError> {
        let mut content = String::new();
        content.push_str("# TIME Coin Masternode Configuration\n");
        content.push_str(
            "# Format: alias IP:port masternodeprivkey collateral_txid collateral_output_index\n",
        );
        content.push_str("#\n");
        content.push_str("# Example:\n");
        content.push_str("# mn1 192.168.1.100:24000 93HaYBVUCYjEMeeH1Y4sBGLALQZE1Yc1K64xiqgX37tGBDQL8Xg 2bcd3c84c84f87eaa86e4e56834c92927a07f9e18718810b92e0d0324456a67c 0\n");
        content.push_str("#\n\n");

        for entry in &self.entries {
            content.push_str(&entry.to_line());
            content.push('\n');
        }

        fs::write(path, content)?;
        Ok(())
    }

    /// Add a masternode configuration entry
    pub fn add_entry(&mut self, entry: MasternodeConfigEntry) -> Result<(), MasternodeConfigError> {
        // Validate the entry
        entry.validate()?;

        // Check for duplicate alias
        if self.get_entry(&entry.alias).is_some() {
            return Err(MasternodeConfigError::DuplicateAlias(entry.alias.clone()));
        }

        self.entries.push(entry);
        Ok(())
    }

    /// Remove a masternode configuration entry by alias
    pub fn remove_entry(&mut self, alias: &str) -> Result<(), MasternodeConfigError> {
        let pos = self
            .entries
            .iter()
            .position(|e| e.alias == alias)
            .ok_or_else(|| MasternodeConfigError::MasternodeNotFound(alias.to_string()))?;

        self.entries.remove(pos);
        Ok(())
    }

    /// Get a masternode configuration entry by alias
    pub fn get_entry(&self, alias: &str) -> Option<&MasternodeConfigEntry> {
        self.entries.iter().find(|e| e.alias == alias)
    }

    /// Get all configuration entries
    pub fn entries(&self) -> &[MasternodeConfigEntry] {
        &self.entries
    }

    /// Get the number of configured masternodes
    pub fn count(&self) -> usize {
        self.entries.len()
    }

    /// Check if a masternode with the given alias exists
    pub fn has_alias(&self, alias: &str) -> bool {
        self.entries.iter().any(|e| e.alias == alias)
    }
}

impl Default for MasternodeConfig {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_line() {
        let line = "mn1 192.168.1.100:24000 93HaYBVUCYjEMeeH1Y4sBGLALQZE1Yc1K64xiqgX37tGBDQL8Xg 2bcd3c84c84f87eaa86e4e56834c92927a07f9e18718810b92e0d0324456a67c 0";
        let entry = MasternodeConfigEntry::parse_line(line, 1).unwrap();

        assert_eq!(entry.alias, "mn1");
        assert_eq!(entry.ip_port, "192.168.1.100:24000");
        assert_eq!(
            entry.masternode_privkey,
            "93HaYBVUCYjEMeeH1Y4sBGLALQZE1Yc1K64xiqgX37tGBDQL8Xg"
        );
        assert_eq!(
            entry.collateral_txid,
            "2bcd3c84c84f87eaa86e4e56834c92927a07f9e18718810b92e0d0324456a67c"
        );
        assert_eq!(entry.collateral_output_index, 0);
    }

    #[test]
    fn test_parse_invalid_line() {
        let line = "mn1 192.168.1.100:24000";
        let result = MasternodeConfigEntry::parse_line(line, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_comment() {
        let line = "# This is a comment";
        let result = MasternodeConfigEntry::parse_line(line, 1);
        assert!(result.is_err());
    }

    #[test]
    fn test_config_add_entry() {
        let mut config = MasternodeConfig::new();
        let entry = MasternodeConfigEntry {
            alias: "mn1".to_string(),
            ip_port: "192.168.1.100:24000".to_string(),
            masternode_privkey: "93HaYBVUCYjEMeeH1Y4sBGLALQZE1Yc1K64xiqgX37tGBDQL8Xg".to_string(),
            collateral_txid: "2bcd3c84c84f87eaa86e4e56834c92927a07f9e18718810b92e0d0324456a67c"
                .to_string(),
            collateral_output_index: 0,
        };

        assert!(config.add_entry(entry.clone()).is_ok());
        assert_eq!(config.count(), 1);

        // Try to add duplicate
        assert!(config.add_entry(entry).is_err());
    }

    #[test]
    fn test_config_parse() {
        let content = r#"
# Comment line
mn1 192.168.1.100:24000 93HaYBVUCYjEMeeH1Y4sBGLALQZE1Yc1K64xiqgX37tGBDQL8Xg 2bcd3c84c84f87eaa86e4e56834c92927a07f9e18718810b92e0d0324456a67c 0
mn2 192.168.1.101:24000 83HaYBVUCYjEMeeH1Y4sBGLALQZE1Yc1K64xiqgX37tGBDQL8Xh 3bcd3c84c84f87eaa86e4e56834c92927a07f9e18718810b92e0d0324456a67d 1

# Another comment
"#;

        let config = MasternodeConfig::parse(content).unwrap();
        assert_eq!(config.count(), 2);
        assert!(config.has_alias("mn1"));
        assert!(config.has_alias("mn2"));
    }
}
