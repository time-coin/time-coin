//! Wallet configuration (simplified for thin client)
//!
//! In thin client mode, the wallet only needs to know:
//! - Which network (mainnet/testnet)
//! - Where to find the masternode
//! - Where to store local data
//!
//! Configuration is stored in `~/.time-wallet/time.conf` using Bitcoin-style
//! key=value syntax. Lines beginning with `#` are comments.
//!
//! ## Supported keys
//!
//! | Key | Default | Description |
//! |---|---|---|
//! | `testnet` | `0` | Set to `1` for testnet |
//! | `addnode` | — | Masternode peer (IP, IP:port, or URL). Repeatable. |
//! | `rpcuser` | — | RPC username for masternode auth |
//! | `rpcpassword` | — | RPC password for masternode auth |
//! | `maxconnections` | `0` | Max peer connections (0 = unlimited) |
//! | `wsendpoint` | — | Override WebSocket URL |
//! | `editor` | — | Path to text editor for opening config files |

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Network type ("mainnet" or "testnet")
    #[serde(default = "default_network")]
    pub network: String,

    /// Manually configured peer endpoints (e.g. ["64.91.241.10:24001"]).
    /// These are tried first, before peers discovered from the API.
    #[serde(default)]
    pub peers: Vec<String>,

    /// WebSocket endpoint for real-time notifications.
    /// Derived from the active peer if not set.
    #[serde(default)]
    pub ws_endpoint: Option<String>,

    /// RPC username for masternode authentication (from masternode's time.conf).
    /// If empty, the wallet will attempt to read the masternode's `.cookie` file.
    #[serde(default)]
    pub rpc_user: Option<String>,

    /// RPC password for masternode authentication (from masternode's time.conf).
    #[serde(default)]
    pub rpc_password: Option<String>,

    /// Maximum number of peers to track and display.
    /// Defaults to unlimited (usize::MAX). Set in config to cap the list.
    #[serde(default = "default_max_connections")]
    pub max_connections: usize,

    /// External editor command for opening config files.
    /// Empty or absent uses the OS default handler.
    #[serde(default)]
    pub editor: Option<String>,

    /// Local data directory (for wallet storage)
    #[serde(skip)]
    pub data_dir: Option<PathBuf>,

    /// The currently active masternode endpoint (set at runtime, not serialized).
    #[serde(skip)]
    pub active_endpoint: Option<String>,

    /// True when no config file existed on disk (first launch).
    #[serde(skip)]
    pub is_first_run: bool,
}

fn default_max_connections() -> usize {
    usize::MAX
}

fn default_network() -> String {
    "mainnet".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            network: default_network(),
            peers: Vec::new(),
            max_connections: default_max_connections(),
            ws_endpoint: None,
            rpc_user: None,
            rpc_password: None,
            editor: None,
            data_dir: None,
            active_endpoint: None,
            is_first_run: false,
        }
    }
}

impl Config {
    /// Load configuration from disk.
    ///
    /// Reads `~/.time-wallet/time.conf`. If that file doesn't exist but the
    /// legacy `config.toml` does, migrates it automatically.
    pub fn load() -> Result<Self, ConfigError> {
        let conf_path = Self::config_path()?;
        let data_dir = Self::data_dir()?;

        let mut config = if conf_path.exists() {
            log::info!("📁 Loading config from: {}", conf_path.display());
            let contents = fs::read_to_string(&conf_path)?;
            let mut c = Self::parse_time_conf(&contents);
            c.data_dir = Some(data_dir);
            c
        } else {
            // Migration: convert legacy config.toml → time.conf
            let toml_path = data_dir.join("config.toml");
            if toml_path.exists() {
                log::info!("🔄 Migrating config.toml → time.conf");
                let contents = fs::read_to_string(&toml_path)?;
                let mut c: Config = toml::from_str(&contents)?;
                c.data_dir = Some(data_dir);
                c.is_first_run = false;
                if c.editor.is_none() {
                    c.editor = detect_editor();
                }
                // Save in new format and keep old file as backup
                c.save()?;
                let backup = toml_path.with_extension("toml.bak");
                let _ = fs::rename(&toml_path, &backup);
                log::info!("✅ Migration complete — config.toml renamed to config.toml.bak");
                return Ok(c);
            }
            log::info!("📝 First run — no config file found, network selection required");
            Config {
                data_dir: Some(Self::data_dir()?),
                is_first_run: true,
                editor: detect_editor(),
                ..Config::default()
            }
        };

        config.is_first_run = false;
        if config.editor.is_none() {
            config.editor = detect_editor();
        }
        log::info!(
            "✅ Config loaded: network={}, {} manual peers",
            config.network,
            config.peers.len()
        );
        Ok(config)
    }

    /// Save configuration to `~/.time-wallet/time.conf`.
    pub fn save(&self) -> Result<(), ConfigError> {
        let conf_path = Self::config_path()?;
        if let Some(parent) = conf_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&conf_path, self.to_time_conf())?;
        log::info!("💾 Config saved to: {}", conf_path.display());
        Ok(())
    }

    /// Get the wallet directory for current network.
    /// Mainnet: `~/.time-wallet/`  Testnet: `~/.time-wallet/testnet/`
    pub fn wallet_dir(&self) -> PathBuf {
        let base = self
            .data_dir
            .clone()
            .unwrap_or_else(|| Self::data_dir().unwrap_or_else(|_| PathBuf::from(".")));
        if self.is_testnet() {
            base.join("testnet")
        } else {
            base
        }
    }

    /// Get config file path (`~/.time-wallet/time.conf`)
    fn config_path() -> Result<PathBuf, ConfigError> {
        let mut path = Self::data_dir()?;
        path.push("time.conf");
        Ok(path)
    }

    /// Get base data directory
    pub fn data_dir() -> Result<PathBuf, ConfigError> {
        let home = dirs::home_dir().ok_or(ConfigError::NoHomeDir)?;
        let mut path = home;
        path.push(".time-wallet");
        Ok(path)
    }

    /// Parse a `time.conf` file (Bitcoin-style `key=value`, `#` comments).
    ///
    /// Unknown keys are silently ignored so future versions stay compatible.
    pub fn parse_time_conf(contents: &str) -> Self {
        let mut config = Config::default();

        for raw in contents.lines() {
            // Strip inline comments
            let line = raw.find('#').map_or(raw, |p| &raw[..p]).trim();
            if line.is_empty() {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            let key = key.trim();
            let value = value.trim();

            match key {
                "testnet" => {
                    config.network = if value == "1" {
                        "testnet".to_string()
                    } else {
                        "mainnet".to_string()
                    };
                }
                "addnode" if !value.is_empty() => {
                    config.peers.push(value.to_string());
                }
                "rpcuser" if !value.is_empty() => {
                    config.rpc_user = Some(value.to_string());
                }
                "rpcpassword" if !value.is_empty() => {
                    config.rpc_password = Some(value.to_string());
                }
                "maxconnections" => {
                    if let Ok(n) = value.parse::<usize>() {
                        config.max_connections = if n == 0 { usize::MAX } else { n };
                    }
                }
                "wsendpoint" if !value.is_empty() => {
                    config.ws_endpoint = Some(value.to_string());
                }
                "editor" if !value.is_empty() => {
                    config.editor = Some(value.to_string());
                }
                _ => {} // forward-compatible: ignore unknown keys
            }
        }

        config
    }

    /// Serialize config to `time.conf` format (Bitcoin-style key=value).
    pub fn to_time_conf(&self) -> String {
        let mut out = String::new();

        out.push_str("# TIME Coin Wallet Configuration\n");
        out.push_str("# Lines starting with # are comments.\n");
        out.push_str("# See https://github.com/time-coin for documentation.\n\n");

        out.push_str("# Network: 1=testnet, 0=mainnet\n");
        out.push_str(&format!(
            "testnet={}\n\n",
            if self.is_testnet() { 1 } else { 0 }
        ));

        out.push_str("# Masternode peers (IP, IP:port, or http://IP:port). Repeat for multiple.\n");
        if self.peers.is_empty() {
            out.push_str("#addnode=64.91.241.10:24001\n");
        } else {
            for peer in &self.peers {
                out.push_str(&format!("addnode={}\n", peer));
            }
        }
        out.push('\n');

        out.push_str("# RPC credentials (from the masternode's time.conf)\n");
        match (&self.rpc_user, &self.rpc_password) {
            (Some(u), Some(p)) => {
                out.push_str(&format!("rpcuser={}\n", u));
                out.push_str(&format!("rpcpassword={}\n", p));
            }
            _ => {
                out.push_str("#rpcuser=timecoinrpc\n");
                out.push_str("#rpcpassword=\n");
            }
        }
        out.push('\n');

        out.push_str("# Maximum peer connections (0 = unlimited)\n");
        let mc = if self.max_connections == usize::MAX {
            0
        } else {
            self.max_connections
        };
        out.push_str(&format!("maxconnections={}\n\n", mc));

        if let Some(ref ws) = self.ws_endpoint {
            out.push_str(&format!("wsendpoint={}\n\n", ws));
        }

        if let Some(ref ed) = self.editor {
            out.push_str(&format!("editor={}\n\n", ed));
        }

        out
    }

    /// Switch to mainnet
    pub fn use_mainnet(&mut self) {
        self.network = "mainnet".to_string();
    }

    /// Switch to testnet
    pub fn use_testnet(&mut self) {
        self.network = "testnet".to_string();
    }

    /// Validate configuration
    pub fn validate(&self) -> Result<(), ConfigError> {
        if self.network != "mainnet" && self.network != "testnet" {
            return Err(ConfigError::InvalidNetwork(self.network.clone()));
        }

        // Validate manually configured peer addresses
        for peer in &self.peers {
            if peer.is_empty() {
                return Err(ConfigError::InvalidPeer("empty peer address".to_string()));
            }
        }

        Ok(())
    }

    /// Get the WebSocket URL, deriving from the active endpoint if not explicitly set.
    pub fn ws_url(&self) -> String {
        if let Some(ref ws) = self.ws_endpoint {
            return ws.clone();
        }
        if let Some(ref endpoint) = self.active_endpoint {
            return Self::derive_ws_url(endpoint);
        }
        // No endpoint yet — return a placeholder that will fail gracefully
        "wss://127.0.0.1:0/ws".to_string()
    }

    /// Derive a WebSocket URL from an RPC endpoint.
    ///
    /// The masternode WS server listens on RPC port + 1, so we bump the port
    /// and swap the scheme: `http://host:24101` → `wss://host:24102`.
    pub fn derive_ws_url(endpoint: &str) -> String {
        let base = endpoint
            .replacen("https://", "wss://", 1)
            .replacen("http://", "wss://", 1);
        // Bump port: WS port = RPC port + 1
        if let Some(colon) = base.rfind(':') {
            if let Ok(rpc_port) = base[colon + 1..].parse::<u16>() {
                return format!("{}{}", &base[..colon + 1], rpc_port + 1);
            }
        }
        base
    }

    /// Get the RPC port for the current network.
    pub fn rpc_port(&self) -> u16 {
        if self.is_testnet() {
            24101
        } else {
            24001
        }
    }

    /// Build HTTP endpoint URLs from the manual peer list.
    /// Each entry can be `ip`, `ip:port`, or a full `http://...` URL.
    pub fn manual_endpoints(&self) -> Vec<String> {
        let port = self.rpc_port();
        self.peers
            .iter()
            .map(|p| {
                if p.starts_with("http://") || p.starts_with("https://") {
                    p.clone()
                } else if p.contains(':') {
                    format!("http://{}", p)
                } else {
                    format!("http://{}:{}", p, port)
                }
            })
            .collect()
    }

    /// Whether this is the testnet network.
    pub fn is_testnet(&self) -> bool {
        self.network == "testnet"
    }

    /// Get RPC credentials for masternode authentication.
    ///
    /// Returns `(user, password)` from, in priority order:
    /// 1. Explicit `rpc_user` / `rpc_password` in config.toml
    /// 2. The masternode's `.cookie` file (if running on the same machine)
    pub fn rpc_credentials(&self) -> Option<(String, String)> {
        // Explicit credentials take priority
        if let (Some(user), Some(pass)) = (&self.rpc_user, &self.rpc_password) {
            if !user.is_empty() && !pass.is_empty() {
                return Some((user.clone(), pass.clone()));
            }
        }

        // Try to read the masternode's .cookie file
        if let Some((user, pass)) = Self::read_cookie_file(self.is_testnet()) {
            log::info!("🍪 Auto-detected RPC credentials from masternode .cookie file");
            return Some((user, pass));
        }

        None
    }

    /// Read the masternode's `.cookie` file for auto-authentication.
    ///
    /// The masternode writes `user:password` to `<data_dir>/.cookie` on startup.
    fn read_cookie_file(testnet: bool) -> Option<(String, String)> {
        let cookie_path = Self::masternode_cookie_path(testnet)?;
        let content = std::fs::read_to_string(&cookie_path).ok()?;
        let content = content.trim();
        let (user, pass) = content.split_once(':')?;
        if user.is_empty() || pass.is_empty() {
            return None;
        }
        log::debug!("📁 Read .cookie from: {}", cookie_path.display());
        Some((user.to_string(), pass.to_string()))
    }

    /// Get the expected path of the masternode's `.cookie` file.
    fn masternode_cookie_path(testnet: bool) -> Option<std::path::PathBuf> {
        #[cfg(target_os = "windows")]
        let base = std::env::var("APPDATA").ok().map(std::path::PathBuf::from);
        #[cfg(not(target_os = "windows"))]
        let base = dirs::home_dir();

        let mut path = base?;
        #[cfg(target_os = "windows")]
        path.push("timecoin");
        #[cfg(not(target_os = "windows"))]
        path.push(".timecoin");

        if testnet {
            path.push("testnet");
        }
        path.push(".cookie");
        Some(path)
    }
}

// ============================================================================
// Error Handling
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Only used during migration from legacy config.toml.
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("Home directory not found")]
    NoHomeDir,

    #[error("Invalid network: {0} (must be 'mainnet' or 'testnet')")]
    InvalidNetwork(String),

    #[error("Invalid endpoint: {0} (must start with http:// or https://)")]
    InvalidEndpoint(String),

    #[error("Invalid peer: {0}")]
    InvalidPeer(String),
}

/// Auto-detect an installed text editor, returning its path if found.
fn detect_editor() -> Option<String> {
    #[cfg(target_os = "windows")]
    {
        let candidates = [
            r"C:\Program Files\Notepad++\notepad++.exe",
            r"C:\Program Files (x86)\Notepad++\notepad++.exe",
            r"C:\Windows\System32\notepad.exe",
        ];
        for path in &candidates {
            if std::path::Path::new(path).exists() {
                log::info!("Auto-detected editor: {}", path);
                return Some(path.to_string());
            }
        }
    }

    #[cfg(target_os = "macos")]
    {
        if std::path::Path::new("/usr/bin/open").exists() {
            log::info!("Auto-detected editor: open -t (macOS default)");
            return Some("open".to_string());
        }
    }

    #[cfg(target_os = "linux")]
    {
        use std::process::Command;
        for editor in &["gedit", "kate", "xed", "nano", "vi"] {
            if Command::new("which")
                .arg(editor)
                .output()
                .is_ok_and(|o| o.status.success())
            {
                log::info!("Auto-detected editor: {}", editor);
                return Some(editor.to_string());
            }
        }
    }

    log::info!("No editor auto-detected, will use OS default");
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.network, "mainnet");
        assert!(config.peers.is_empty());
    }

    #[test]
    fn test_network_switch() {
        let mut config = Config::default();

        config.use_mainnet();
        assert_eq!(config.network, "mainnet");

        config.use_testnet();
        assert_eq!(config.network, "testnet");
    }

    #[test]
    fn test_validation() {
        let mut config = Config::default();
        assert!(config.validate().is_ok());

        config.network = "invalid".to_string();
        assert!(config.validate().is_err());

        config.network = "mainnet".to_string();
        config.peers = vec!["".to_string()];
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_serialization() {
        let config = Config {
            network: "testnet".to_string(),
            peers: vec!["64.91.241.10:24101".to_string()],
            rpc_user: Some("user".to_string()),
            rpc_password: Some("pass".to_string()),
            ..Config::default()
        };
        let conf = config.to_time_conf();
        let parsed = Config::parse_time_conf(&conf);
        assert_eq!(parsed.network, "testnet");
        assert_eq!(parsed.peers, vec!["64.91.241.10:24101"]);
        assert_eq!(parsed.rpc_user.as_deref(), Some("user"));
        assert_eq!(parsed.rpc_password.as_deref(), Some("pass"));
    }

    #[test]
    fn test_parse_time_conf_basics() {
        let conf = "\
testnet=1
addnode=1.2.3.4:24101
addnode=5.6.7.8:24101
rpcuser=alice
rpcpassword=secret
maxconnections=20
# this is a comment
editor=nano
";
        let c = Config::parse_time_conf(conf);
        assert_eq!(c.network, "testnet");
        assert_eq!(c.peers, vec!["1.2.3.4:24101", "5.6.7.8:24101"]);
        assert_eq!(c.rpc_user.as_deref(), Some("alice"));
        assert_eq!(c.rpc_password.as_deref(), Some("secret"));
        assert_eq!(c.max_connections, 20);
        assert_eq!(c.editor.as_deref(), Some("nano"));
    }

    #[test]
    fn test_parse_time_conf_maxconnections_zero() {
        let c = Config::parse_time_conf("maxconnections=0\n");
        assert_eq!(c.max_connections, usize::MAX);
    }

    #[test]
    fn test_parse_time_conf_ignores_unknown_keys() {
        // Should not panic on unknown keys
        let c = Config::parse_time_conf("unknownkey=value\ntestnet=0\n");
        assert_eq!(c.network, "mainnet");
    }

    #[test]
    fn test_ws_url_derived() {
        let config = Config {
            active_endpoint: Some("https://example.com:24001".to_string()),
            ws_endpoint: None,
            ..Default::default()
        };
        assert_eq!(config.ws_url(), "wss://example.com:24002");

        let config2 = Config {
            active_endpoint: Some("http://127.0.0.1:24101".to_string()),
            ws_endpoint: None,
            ..Default::default()
        };
        assert_eq!(config2.ws_url(), "wss://127.0.0.1:24102");
    }

    #[test]
    fn test_ws_url_explicit() {
        let config = Config {
            ws_endpoint: Some("ws://custom:9999/ws".to_string()),
            ..Config::default()
        };
        assert_eq!(config.ws_url(), "ws://custom:9999/ws");
    }

    #[test]
    fn test_manual_endpoints() {
        let config = Config {
            peers: vec![
                "64.91.241.10".to_string(),
                "50.28.104.50:24001".to_string(),
                "http://custom.host:24001".to_string(),
            ],
            ..Config::default()
        };
        let endpoints = config.manual_endpoints();
        assert_eq!(endpoints[0], "http://64.91.241.10:24001");
        assert_eq!(endpoints[1], "http://50.28.104.50:24001");
        assert_eq!(endpoints[2], "http://custom.host:24001");
    }

    #[test]
    fn test_manual_endpoints_testnet() {
        let mut config = Config::default();
        config.use_testnet();
        config.peers = vec!["64.91.241.10".to_string()];
        let endpoints = config.manual_endpoints();
        assert_eq!(endpoints[0], "http://64.91.241.10:24101");
    }
}
