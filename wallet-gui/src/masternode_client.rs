//! Masternode JSON-RPC client
//!
//! Communicates with masternodes using JSON-RPC 2.0 over HTTP.
//! The masternode exposes an axum-based HTTP server on the RPC port
//! (24101 for testnet, 24001 for mainnet).

use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

static REQUEST_ID: AtomicU64 = AtomicU64::new(1);

/// Parse a JSON numeric value to satoshis (1 TIME = 100_000_000 satoshis).
/// Handles plain decimal strings ("12.34567890") and scientific notation
/// ("1e-8", "1.5e2") that serde_json may emit for very small/large floats.
pub fn json_to_satoshis(val: &serde_json::Value) -> u64 {
    let s = match val {
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        _ => return 0,
    };

    let s = s.trim();
    let negative = s.starts_with('-');
    let s = s.trim_start_matches('-');

    if negative {
        return 0;
    }

    parse_time_string_to_satoshis(s)
}

/// Like `json_to_satoshis` but returns the absolute value (for amounts/fees
/// that may be negative in the RPC response).
fn json_to_satoshis_abs(val: &serde_json::Value) -> u64 {
    let s = match val {
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        _ => return 0,
    };

    parse_time_string_to_satoshis(s.trim().trim_start_matches('-'))
}

/// Convert a non-negative decimal or scientific-notation string to satoshis.
/// Falls back to f64 parsing when the string contains 'e'/'E'.
fn parse_time_string_to_satoshis(s: &str) -> u64 {
    // Fast path: plain decimal (no scientific notation)
    if !s.contains('e') && !s.contains('E') {
        let (whole, frac) = if let Some(dot) = s.find('.') {
            (&s[..dot], &s[dot + 1..])
        } else {
            (s, "")
        };
        let whole_val: u64 = whole.parse().unwrap_or(0);
        let frac_padded = format!("{:0<8}", &frac[..frac.len().min(8)]);
        let frac_val: u64 = frac_padded[..8].parse().unwrap_or(0);
        return whole_val
            .saturating_mul(100_000_000)
            .saturating_add(frac_val);
    }

    // Fallback: scientific notation — use f64 arithmetic
    match s.parse::<f64>() {
        Ok(f) if f >= 0.0 => (f * 100_000_000.0).round() as u64,
        _ => 0,
    }
}

#[derive(Debug, Clone)]
pub struct MasternodeClient {
    rpc_endpoint: String,
    client: Client,
    /// Optional HTTP Basic Auth credentials (user, password)
    rpc_credentials: Option<(String, String)>,
}

/// JSON-RPC 2.0 request
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: String,
    method: String,
    params: serde_json::Value,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Deserialize)]
struct JsonRpcResponse {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    #[allow(dead_code)]
    id: Option<serde_json::Value>,
    result: Option<serde_json::Value>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    code: i64,
    message: String,
}

impl MasternodeClient {
    pub fn new(endpoint: String, credentials: Option<(String, String)>) -> Self {
        // Preserve http:// if explicitly provided (for nodes without TLS configured).
        // Add https:// prefix only for bare hostnames with no scheme.
        let rpc_endpoint = if endpoint.starts_with("http://") || endpoint.starts_with("https://") {
            endpoint
        } else {
            format!("http://{}", endpoint)
        };

        // Accept self-signed certificates — masternodes use auto-generated certs (TOFU model)
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .danger_accept_invalid_certs(true)
            .build()
            .expect("Failed to create HTTP client");

        if credentials.is_some() {
            log::info!(
                "📡 Masternode JSON-RPC client initialized: {} (with auth)",
                rpc_endpoint
            );
        } else {
            log::info!(
                "📡 Masternode JSON-RPC client initialized: {} (no auth)",
                rpc_endpoint
            );
        }

        Self {
            rpc_endpoint,
            client,
            rpc_credentials: credentials,
        }
    }

    pub fn endpoint(&self) -> &str {
        &self.rpc_endpoint
    }

    /// Send a JSON-RPC 2.0 request and return the result
    async fn rpc_call(
        &self,
        method: &str,
        params: serde_json::Value,
    ) -> Result<serde_json::Value, ClientError> {
        let id = REQUEST_ID.fetch_add(1, Ordering::Relaxed);
        let request = JsonRpcRequest {
            jsonrpc: "2.0",
            id: id.to_string(),
            method: method.to_string(),
            params,
        };

        log::debug!("→ RPC {}: {:?}", method, request.params);

        let mut req = self.client.post(&self.rpc_endpoint).json(&request);

        if let Some((ref user, ref pass)) = self.rpc_credentials {
            req = req.basic_auth(user, Some(pass));
        }

        let response = req.send().await?;

        if !response.status().is_success() {
            return Err(ClientError::http(response.status().as_u16()));
        }

        let rpc_response: JsonRpcResponse = response.json().await.map_err(|e| {
            ClientError::InvalidResponse(format!("Failed to parse JSON-RPC response: {}", e))
        })?;

        if let Some(error) = rpc_response.error {
            return Err(ClientError::RpcError(error.code, error.message));
        }

        rpc_response
            .result
            .ok_or_else(|| ClientError::InvalidResponse("No result in JSON-RPC response".into()))
    }

    /// Get balance for an address
    pub async fn get_balance(&self, address: &str) -> Result<Balance, ClientError> {
        let result = self
            .rpc_call("getbalance", serde_json::json!([address]))
            .await?;

        // Masternode returns {balance, locked, available} in TIME
        let confirmed = result.get("available").map(json_to_satoshis).unwrap_or(0);
        let total = result.get("balance").map(json_to_satoshis).unwrap_or(0);

        let balance = Balance {
            confirmed,
            pending: 0,
            total,
        };
        log::info!("✅ Balance: {} sats (available: {} sats)", total, confirmed);
        Ok(balance)
    }

    /// Get combined balance across multiple addresses (batch query for HD wallets)
    pub async fn get_balances(&self, addresses: &[String]) -> Result<Balance, ClientError> {
        let result = self
            .rpc_call("getbalances", serde_json::json!([addresses]))
            .await?;

        let confirmed = result.get("available").map(json_to_satoshis).unwrap_or(0);
        let total = result.get("balance").map(json_to_satoshis).unwrap_or(0);

        let balance = Balance {
            confirmed,
            pending: 0,
            total,
        };

        let addr_count = result
            .get("address_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(0);
        log::info!(
            "✅ Batch balance ({} addresses): {} sats (available: {} sats)",
            addr_count,
            total,
            confirmed
        );
        Ok(balance)
    }

    /// Get transaction history for a single address
    pub async fn get_transactions(
        &self,
        address: &str,
        limit: u32,
    ) -> Result<Vec<TransactionRecord>, ClientError> {
        let result = self
            .rpc_call("listtransactions", serde_json::json!([address, limit]))
            .await?;

        Self::parse_transaction_list(result)
    }

    /// Get transaction history across multiple addresses (batch query for HD wallets)
    pub async fn get_transactions_multi(
        &self,
        addresses: &[String],
        limit: u32,
    ) -> Result<Vec<TransactionRecord>, ClientError> {
        let result = self
            .rpc_call(
                "listtransactionsmulti",
                serde_json::json!([addresses, limit]),
            )
            .await?;

        Self::parse_transaction_list(result)
    }

    /// Parse a JSON array of transaction objects into TransactionRecords
    fn parse_transaction_list(
        result: serde_json::Value,
    ) -> Result<Vec<TransactionRecord>, ClientError> {
        let txs: Vec<serde_json::Value> = serde_json::from_value(result).unwrap_or_default();

        let records: Vec<TransactionRecord> = txs
            .into_iter()
            .filter_map(|tx| {
                let txid = tx.get("txid")?.as_str()?.to_string();
                let category = tx.get("category")?.as_str().unwrap_or("unknown");
                let amount = tx.get("amount").map(json_to_satoshis_abs).unwrap_or(0);
                let fee = tx.get("fee").map(json_to_satoshis_abs).unwrap_or(0);
                let in_block = tx.get("blockhash").and_then(|v| v.as_str()).is_some();
                let block_hash = tx
                    .get("blockhash")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let block_height = tx.get("blockheight").and_then(|v| v.as_u64()).unwrap_or(0);
                let confirmations = tx
                    .get("confirmations")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);
                let timestamp = tx.get("time").and_then(|v| v.as_i64()).unwrap_or(0);

                // Instant finality: check finalized flag from consensus, then blockhash
                let finalized = tx
                    .get("finalized")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let status = if in_block || finalized {
                    TransactionStatus::Approved
                } else {
                    TransactionStatus::Pending
                };

                let vout = tx.get("vout").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let is_send = category == "send";
                let address = tx
                    .get("address")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                // RPC includes fee in the send amount — subtract it so we
                // display only the actual transferred value.
                let display_amount = if is_send && fee > 0 {
                    amount.saturating_sub(fee)
                } else {
                    amount
                };

                // Skip zero-amount received/generate entries — these are staking
                // inputs with no corresponding payout in that transaction.
                // (Sent transactions with fee but 0 net are valid and kept.)
                if display_amount == 0 && !is_send {
                    return None;
                }

                let memo = tx
                    .get("memo")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());

                Some(TransactionRecord {
                    txid,
                    vout,
                    is_send,
                    address,
                    amount: display_amount,
                    fee,
                    timestamp,
                    status,
                    is_fee: false,
                    is_change: false,
                    block_hash,
                    block_height,
                    confirmations,
                    memo,
                })
            })
            .collect();

        log::info!("✅ Retrieved {} transactions", records.len());
        Ok(records)
    }

    /// Get UTXOs for an address
    pub async fn get_utxos(&self, address: &str) -> Result<Vec<Utxo>, ClientError> {
        // listunspentmulti returns all UTXOs by default (no limit)
        let result = self
            .rpc_call("listunspentmulti", serde_json::json!([[address]]))
            .await?;

        let utxo_values: Vec<serde_json::Value> =
            serde_json::from_value(result).unwrap_or_default();

        let utxos: Vec<Utxo> = utxo_values
            .into_iter()
            .filter_map(|u| {
                let txid = u.get("txid")?.as_str()?.to_string();
                let vout = u.get("vout")?.as_u64()? as u32;
                let amount = u.get("amount").map(json_to_satoshis).unwrap_or(0);
                let addr = u
                    .get("address")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let confirmations =
                    u.get("confirmations").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                let spendable = u.get("spendable").and_then(|v| v.as_bool()).unwrap_or(true);

                Some(Utxo {
                    txid,
                    vout,
                    amount,
                    address: addr,
                    confirmations,
                    spendable,
                })
            })
            .collect();

        let spendable_count = utxos.iter().filter(|u| u.spendable).count();
        log::info!(
            "✅ Retrieved {} UTXOs ({} spendable, {} locked)",
            utxos.len(),
            spendable_count,
            utxos.len() - spendable_count
        );
        Ok(utxos)
    }

    /// Broadcast a signed transaction (hex-encoded bincode)
    pub async fn broadcast_transaction(&self, tx_hex: &str) -> Result<String, ClientError> {
        let result = self
            .rpc_call("sendrawtransaction", serde_json::json!([tx_hex]))
            .await?;

        let txid = result
            .as_str()
            .map(|s| s.to_string())
            .unwrap_or_else(|| result.to_string().trim_matches('"').to_string());

        log::info!("✅ Transaction broadcast: {}", txid);
        Ok(txid)
    }

    /// Validate an address
    pub async fn validate_address(&self, address: &str) -> Result<bool, ClientError> {
        let result = self
            .rpc_call("validateaddress", serde_json::json!([address]))
            .await?;

        let valid = result
            .get("isvalid")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        Ok(valid)
    }

    /// Look up the Ed25519 public key for an address from the masternode's
    /// pubkey cache. Returns `None` if the address has never signed a
    /// transaction on-chain (pubkey unknown).
    pub async fn get_address_pubkey(&self, address: &str) -> Result<Option<[u8; 32]>, ClientError> {
        let result = self
            .rpc_call("getaddresspubkey", serde_json::json!([address]))
            .await?;

        let hex_str = match result.get("pubkey").and_then(|v| v.as_str()) {
            Some(s) if !s.is_empty() => s,
            _ => return Ok(None),
        };

        let bytes = hex::decode(hex_str)
            .map_err(|_| ClientError::InvalidResponse("invalid pubkey hex".into()))?;
        if bytes.len() != 32 {
            return Ok(None);
        }

        let mut pubkey = [0u8; 32];
        pubkey.copy_from_slice(&bytes);
        Ok(Some(pubkey))
    }

    /// Check if masternode is reachable via getblockchaininfo
    pub async fn health_check(&self) -> Result<HealthStatus, ClientError> {
        let result = self
            .rpc_call("getblockchaininfo", serde_json::json!([]))
            .await?;

        // Masternode returns "blocks", fall back to "height" for compat
        let height = result
            .get("blocks")
            .or_else(|| result.get("height"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0);

        // "chain" is network type (mainnet/testnet)
        let network = result
            .get("chain")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        // Fetch connection count and daemon version from getnetworkinfo
        let (peer_count, version) =
            if let Ok(ni) = self.rpc_call("getnetworkinfo", serde_json::json!([])).await {
                let peers = ni.get("connections").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                // subversion is "/timed:0.1.0/" — strip the slashes
                let ver = ni
                    .get("subversion")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .trim_matches('/')
                    .to_string();
                (peers, ver)
            } else {
                (0, String::new())
            };

        let status = HealthStatus {
            status: "healthy".to_string(),
            version: format!("{} ({})", network, version),
            block_height: height,
            peer_count,
        };

        log::info!(
            "✅ Masternode healthy: height={}, peers={}",
            height,
            peer_count
        );
        Ok(status)
    }

    /// Get current blockchain height
    pub async fn get_block_height(&self) -> Result<u64, ClientError> {
        let result = self
            .rpc_call("getblockcount", serde_json::json!([]))
            .await?;

        let height = result.as_u64().unwrap_or(0);
        Ok(height)
    }

    /// Query instant finality status for a transaction
    pub async fn get_transaction_finality(
        &self,
        txid: &str,
    ) -> Result<FinalityStatus, ClientError> {
        let result = self
            .rpc_call("gettransactionfinality", serde_json::json!([txid]))
            .await?;

        let finalized = result
            .get("finalized")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let confirmations = result
            .get("confirmations")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as u32;

        Ok(FinalityStatus {
            txid: txid.to_string(),
            finalized,
            confirmations,
        })
    }

    /// Get peer info from masternode
    pub async fn get_peer_info(&self) -> Result<Vec<PeerInfoResult>, ClientError> {
        let result = self.rpc_call("getpeerinfo", serde_json::json!([])).await?;

        let peers: Vec<serde_json::Value> = serde_json::from_value(result).unwrap_or_default();

        let peer_info: Vec<PeerInfoResult> = peers
            .into_iter()
            .filter_map(|p| {
                let addr = p.get("addr")?.as_str()?.to_string();
                let active = p.get("active").and_then(|v| v.as_bool()).unwrap_or(false);
                Some(PeerInfoResult { addr, active })
            })
            .collect();

        Ok(peer_info)
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
    pub vout: u32,
    pub is_send: bool,
    pub address: String,
    pub amount: u64,
    pub fee: u64,
    pub timestamp: i64,
    pub status: TransactionStatus,
    /// True if this entry represents a network fee (ledger line item).
    #[serde(default)]
    pub is_fee: bool,
    /// True if this is a change output returning to the sender's own wallet.
    #[serde(default)]
    pub is_change: bool,
    /// Hash of the block containing this transaction (empty if unconfirmed).
    #[serde(default)]
    pub block_hash: String,
    /// Height of the block containing this transaction (0 if unconfirmed).
    #[serde(default)]
    pub block_height: u64,
    /// Number of confirmations (0 if unconfirmed).
    #[serde(default)]
    pub confirmations: u64,
    /// Decrypted memo text (populated from masternode response).
    #[serde(default)]
    pub memo: Option<String>,
}

impl Default for TransactionRecord {
    fn default() -> Self {
        Self {
            txid: String::new(),
            vout: 0,
            is_send: false,
            address: String::new(),
            amount: 0,
            fee: 0,
            timestamp: 0,
            status: TransactionStatus::Pending,
            is_fee: false,
            is_change: false,
            block_hash: String::new(),
            block_height: 0,
            confirmations: 0,
            memo: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TransactionStatus {
    /// In the mempool but not yet in a block.
    Pending,
    /// Included in a block — instant finality means this is final.
    Approved,
    /// Transaction was rejected or conflicted.
    Declined,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Utxo {
    pub txid: String,
    pub vout: u32,
    pub amount: u64,
    pub address: String,
    pub confirmations: u32,
    /// False for UTXOs locked as masternode collateral or pending finality.
    #[serde(default = "default_true")]
    pub spendable: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub status: String,
    pub version: String,
    pub block_height: u64,
    pub peer_count: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalityStatus {
    pub txid: String,
    pub finalized: bool,
    pub confirmations: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PeerInfoResult {
    pub addr: String,
    pub active: bool,
}

// ============================================================================
// Error Handling
// ============================================================================

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("HTTP error {0}: {1}")]
    Http(u16, String),

    #[error("RPC error {0}: {1}")]
    RpcError(i64, String),

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

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl ClientError {
    pub fn http(status: u16) -> Self {
        let message = match status {
            401 => "Unauthorized — check rpc_user/rpc_password in config.toml or masternode .cookie file",
            400 => "Bad Request",
            403 => "Forbidden",
            404 => "Not Found",
            500 => "Internal Server Error",
            503 => "Service Unavailable",
            _ => "Unknown Error",
        };
        Self::Http(status, message.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_creation() {
        let client = MasternodeClient::new("http://127.0.0.1:24101".to_string(), None);
        assert_eq!(client.endpoint(), "http://127.0.0.1:24101");
    }

    #[tokio::test]
    async fn test_client_creation_bare_endpoint() {
        let client = MasternodeClient::new("127.0.0.1:24101".to_string(), None);
        assert_eq!(client.endpoint(), "http://127.0.0.1:24101");
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
        let status = TransactionStatus::Approved;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""approved""#);
    }
}
