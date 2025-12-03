//! Simple HTTP client for masternode communication
//!
//! This is a thin client that delegates all blockchain operations to masternodes.
//! The wallet only handles key management and transaction signing locally.

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct MasternodeClient {
    endpoint: String,
    client: Client,
}

impl MasternodeClient {
    pub fn new(endpoint: String) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        log::info!("📡 Masternode client initialized: {}", endpoint);
        Self { endpoint, client }
    }

    /// Get balance for an xpub
    pub async fn get_balance(&self, xpub: &str) -> Result<Balance, ClientError> {
        let url = format!("{}/wallet/balance", self.endpoint);
        log::debug!("→ GET {}", url);

        let response = self
            .client
            .get(&url)
            .query(&[("xpub", xpub)])
            .send()
            .await?;

        if !response.status().is_success() {
            log::error!("❌ Balance fetch failed: {}", response.status());
            return Err(ClientError::http(response.status().as_u16()));
        }

        let balance = response.json().await?;
        log::info!("✅ Balance retrieved: {:?}", balance);
        Ok(balance)
    }

    /// Get transaction history for an xpub
    pub async fn get_transactions(
        &self,
        xpub: &str,
        limit: u32,
    ) -> Result<Vec<TransactionRecord>, ClientError> {
        let url = format!("{}/wallet/transactions", self.endpoint);
        log::debug!("→ GET {} (limit: {})", url, limit);

        let response = self
            .client
            .get(&url)
            .query(&[("xpub", xpub), ("limit", &limit.to_string())])
            .send()
            .await?;

        if !response.status().is_success() {
            log::error!("❌ Transaction fetch failed: {}", response.status());
            return Err(ClientError::http(response.status().as_u16()));
        }

        let transactions: Vec<TransactionRecord> = response.json().await?;
        log::info!("✅ Retrieved {} transactions", transactions.len());
        Ok(transactions)
    }

    /// Get UTXOs for an xpub
    pub async fn get_utxos(&self, xpub: &str) -> Result<Vec<Utxo>, ClientError> {
        let url = format!("{}/wallet/utxos", self.endpoint);
        log::debug!("→ GET {}", url);

        let response = self
            .client
            .get(&url)
            .query(&[("xpub", xpub)])
            .send()
            .await?;

        if !response.status().is_success() {
            log::error!("❌ UTXO fetch failed: {}", response.status());
            return Err(ClientError::http(response.status().as_u16()));
        }

        let utxos: Vec<Utxo> = response.json().await?;
        log::info!("✅ Retrieved {} UTXOs", utxos.len());
        Ok(utxos)
    }

    /// Broadcast a signed transaction
    pub async fn broadcast_transaction(&self, tx_hex: &str) -> Result<String, ClientError> {
        let url = format!("{}/transaction/broadcast", self.endpoint);
        log::debug!("→ POST {}", url);

        let body = serde_json::json!({ "tx": tx_hex });

        let response = self.client.post(&url).json(&body).send().await?;

        if !response.status().is_success() {
            log::error!("❌ Transaction broadcast failed: {}", response.status());
            let error_text = response
                .text()
                .await
                .unwrap_or_else(|_| "Unknown error".to_string());
            return Err(ClientError::BroadcastFailed(error_text));
        }

        let result: BroadcastResponse = response.json().await?;
        log::info!("✅ Transaction broadcast: {}", result.txid);
        Ok(result.txid)
    }

    /// Get address information
    pub async fn get_address_info(&self, address: &str) -> Result<AddressInfo, ClientError> {
        let url = format!("{}/address/{}", self.endpoint, address);
        log::debug!("→ GET {}", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            log::error!("❌ Address info fetch failed: {}", response.status());
            return Err(ClientError::http(response.status().as_u16()));
        }

        let info = response.json().await?;
        log::debug!("✅ Address info retrieved: {:?}", info);
        Ok(info)
    }

    /// Check if masternode is reachable
    pub async fn health_check(&self) -> Result<HealthStatus, ClientError> {
        let url = format!("{}/health", self.endpoint);
        log::debug!("→ GET {}", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            log::warn!("⚠️ Health check failed: {}", response.status());
            return Err(ClientError::http(response.status().as_u16()));
        }

        let status: HealthStatus = response.json().await?;
        log::info!("✅ Masternode healthy: {:?}", status);
        Ok(status)
    }

    /// Get current blockchain height
    pub async fn get_block_height(&self) -> Result<u64, ClientError> {
        let url = format!("{}/blockchain/height", self.endpoint);
        log::debug!("→ GET {}", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ClientError::http(response.status().as_u16()));
        }

        let result: BlockHeightResponse = response.json().await?;
        Ok(result.height)
    }

    /// Validate an address
    pub async fn validate_address(&self, address: &str) -> Result<bool, ClientError> {
        let url = format!("{}/address/validate/{}", self.endpoint, address);
        log::debug!("→ GET {}", url);

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(ClientError::http(response.status().as_u16()));
        }

        let result: AddressValidation = response.json().await?;
        Ok(result.valid)
    }
}

// ============================================================================
// Data Structures
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub confirmed: u64,
    pub pending: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    pub txid: String,
    pub from: Vec<String>,
    pub to: Vec<String>,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: i64,
    pub confirmations: u32,
    pub status: TransactionStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Utxo {
    pub txid: String,
    pub vout: u32,
    pub amount: u64,
    pub address: String,
    pub confirmations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressInfo {
    pub address: String,
    pub has_transactions: bool,
    pub balance: u64,
    pub transaction_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub block_height: u64,
    pub peer_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
struct BroadcastResponse {
    txid: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct BlockHeightResponse {
    height: u64,
}

#[derive(Debug, Serialize, Deserialize)]
struct AddressValidation {
    valid: bool,
}

// ============================================================================
// Error Handling
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("HTTP error {0}: {1}")]
    Http(u16, String),

    #[error("Request failed: {0}")]
    Request(#[from] reqwest::Error),

    #[error("Network timeout")]
    Timeout,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),

    #[error("Transaction broadcast failed: {0}")]
    BroadcastFailed(String),

    #[error("Masternode unavailable")]
    Unavailable,
}

// Fix the HTTP error construction
impl ClientError {
    pub fn http(status: u16) -> Self {
        let message = match status {
            400 => "Bad Request",
            404 => "Not Found",
            500 => "Internal Server Error",
            503 => "Service Unavailable",
            _ => "Unknown Error",
        };
        Self::Http(status, message.to_string())
    }
}

// Update the usage in the impl block
impl MasternodeClient {
    // Helper method to handle HTTP errors consistently
    fn handle_error_response(status: u16) -> ClientError {
        ClientError::http(status)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = MasternodeClient::new("https://testnet.time-coin.io".to_string());
        assert_eq!(client.endpoint, "https://testnet.time-coin.io");
    }

    #[test]
    fn test_balance_serialization() {
        let balance = Balance {
            confirmed: 1000,
            pending: 500,
            total: 1500,
        };

        let json = serde_json::to_string(&balance).unwrap();
        let deserialized: Balance = serde_json::from_str(&json).unwrap();

        assert_eq!(balance.total, deserialized.total);
    }

    #[test]
    fn test_transaction_status() {
        let status = TransactionStatus::Confirmed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""confirmed""#);
    }
}
