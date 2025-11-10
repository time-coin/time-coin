use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Config {
    #[serde(default = "default_network")]
    pub network: String,

    #[serde(default = "default_data_dir")]
    pub data_dir: PathBuf,

    #[serde(default = "default_rpc_port")]
    pub rpc_port: u16,

    #[serde(default = "default_rpc_user")]
    pub rpc_user: String,

    #[serde(default = "default_rpc_password")]
    pub rpc_password: String,

    #[serde(default = "default_bootstrap_nodes")]
    pub bootstrap_nodes: Vec<String>,

    #[serde(default = "default_api_endpoint")]
    pub api_endpoint: String,
}

fn default_network() -> String {
    "testnet".to_string()
}

fn default_data_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("TimeCoin")
}

fn default_rpc_port() -> u16 {
    24101
}

fn default_rpc_user() -> String {
    "rpcuser".to_string()
}

fn default_rpc_password() -> String {
    "rpcpassword".to_string()
}

fn default_bootstrap_nodes() -> Vec<String> {
    vec![
        "161.35.129.70:24100".to_string(),
        "178.128.199.144:24100".to_string(),
        "165.232.154.150:24100".to_string(),
    ]
}

fn default_api_endpoint() -> String {
    "https://time-coin.io/api".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Config {
            network: default_network(),
            data_dir: default_data_dir(),
            rpc_port: default_rpc_port(),
            rpc_user: default_rpc_user(),
            rpc_password: default_rpc_password(),
            bootstrap_nodes: default_bootstrap_nodes(),
            api_endpoint: default_api_endpoint(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = Self::config_path();

        if config_path.exists() {
            let contents = fs::read_to_string(&config_path)?;
            let config: Config = serde_json::from_str(&contents)?;
            Ok(config)
        } else {
            let config = Config::default();
            config.save()?;
            Ok(config)
        }
    }

    pub fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = Self::config_path();

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, contents)?;

        Ok(())
    }

    pub fn config_path() -> PathBuf {
        default_data_dir().join("config.json")
    }

    pub fn wallet_dir(&self) -> PathBuf {
        let network_dir = if self.network == "testnet" {
            "testnet"
        } else {
            "mainnet"
        };
        self.data_dir.join(network_dir)
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WalletConfig {
    #[serde(default)]
    pub default_address: String,

    #[serde(default)]
    pub addresses: Vec<String>,
}

impl Default for WalletConfig {
    fn default() -> Self {
        WalletConfig {
            default_address: String::new(),
            addresses: Vec::new(),
        }
    }
}

impl WalletConfig {
    pub fn load(wallet_dir: &PathBuf) -> Result<Self, Box<dyn std::error::Error>> {
        let config_path = wallet_dir.join("wallet_config.json");

        if config_path.exists() {
            let contents = fs::read_to_string(&config_path)?;
            let config: WalletConfig = serde_json::from_str(&contents)?;
            Ok(config)
        } else {
            let config = WalletConfig::default();
            config.save(wallet_dir)?;
            Ok(config)
        }
    }

    pub fn save(&self, wallet_dir: &PathBuf) -> Result<(), Box<dyn std::error::Error>> {
        let config_path = wallet_dir.join("wallet_config.json");

        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }

        let contents = serde_json::to_string_pretty(self)?;
        fs::write(&config_path, contents)?;

        Ok(())
    }
}
