//! Simple async TCP client for wallet operations

#![allow(dead_code)]

use std::time::Duration;
use time_network::protocol::{HandshakeMessage, NetworkMessage, WalletTransaction};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use wallet::NetworkType;

#[derive(Debug, Clone)]
pub struct SimpleClient {
    masternode_addr: String,
    network: NetworkType,
}

#[derive(Debug, thiserror::Error)]
pub enum ClientError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Timeout")]
    Timeout,

    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

pub type Result<T> = std::result::Result<T, ClientError>;

impl SimpleClient {
    pub fn new(masternode_addr: String, network: NetworkType) -> Self {
        Self {
            masternode_addr,
            network,
        }
    }

    async fn connect(&self) -> Result<TcpStream> {
        let stream = tokio::time::timeout(
            Duration::from_secs(3),
            TcpStream::connect(&self.masternode_addr),
        )
        .await
        .map_err(|_| ClientError::Timeout)??;

        Ok(stream)
    }

    async fn send_message(&self, message: NetworkMessage) -> Result<NetworkMessage> {
        log::info!("🔌 Connecting to {}...", self.masternode_addr);
        let mut stream = self.connect().await?;
        log::info!("✅ Connected to masternode");

        let network_type = match self.network {
            NetworkType::Mainnet => time_network::discovery::NetworkType::Mainnet,
            NetworkType::Testnet => time_network::discovery::NetworkType::Testnet,
        };

        let handshake = HandshakeMessage::new(network_type, "0.0.0.0:0".parse().unwrap());
        let magic = network_type.magic_bytes();

        // Send handshake
        log::info!("🤝 Sending handshake...");
        let handshake_json = serde_json::to_vec(&handshake)?;
        let handshake_len = (handshake_json.len() as u32).to_be_bytes();

        stream.write_all(&magic).await?;
        stream.write_all(&handshake_len).await?;
        stream.write_all(&handshake_json).await?;
        stream.flush().await?;

        // Read their handshake
        log::info!("👂 Reading masternode handshake...");
        let mut their_magic = [0u8; 4];
        let mut their_len = [0u8; 4];
        stream.read_exact(&mut their_magic).await?;
        stream.read_exact(&mut their_len).await?;

        let len = u32::from_be_bytes(their_len) as usize;
        if len > 10 * 1024 {
            return Err(ClientError::InvalidResponse("Handshake too large".into()));
        }

        let mut their_handshake = vec![0u8; len];
        stream.read_exact(&mut their_handshake).await?;
        log::info!("✅ Handshake complete");

        // Send our message
        log::info!("📨 Sending request message...");
        let msg_json = serde_json::to_vec(&message)?;
        let msg_len = (msg_json.len() as u32).to_be_bytes();
        log::info!("📤 Message size: {} bytes", msg_json.len());

        stream.write_all(&magic).await?;
        stream.write_all(&msg_len).await?;
        stream.write_all(&msg_json).await?;
        stream.flush().await?;

        log::info!("📤 Sent message, waiting for response...");

        // Read response with timeout
        let response = tokio::time::timeout(Duration::from_secs(30), async {
            log::info!("⏳ Waiting for response (30s timeout)...");
            let mut resp_magic = [0u8; 4];
            let mut resp_len = [0u8; 4];

            log::info!("📖 Reading response magic and length...");
            stream.read_exact(&mut resp_magic).await?;
            stream.read_exact(&mut resp_len).await?;

            let len = u32::from_be_bytes(resp_len) as usize;
            log::info!(
                "📦 Response size: {} bytes ({:.2} MB)",
                len,
                len as f64 / 1024.0 / 1024.0
            );

            if len > 100 * 1024 * 1024 {
                // 100MB limit (increased from 10MB)
                log::error!("❌ Response too large: {} MB", len as f64 / 1024.0 / 1024.0);
                return Err(ClientError::InvalidResponse("Response too large".into()));
            }

            if len == 0 {
                log::warn!("⚠️ Empty response from masternode");
                return Err(ClientError::InvalidResponse("Empty response".into()));
            }

            log::info!("📥 Reading {} bytes of data...", len);
            let mut resp_data = vec![0u8; len];
            stream.read_exact(&mut resp_data).await?;

            log::info!("🔍 Parsing JSON response...");
            let response: NetworkMessage = serde_json::from_slice(&resp_data)?;
            log::info!("✅ Response parsed successfully");
            Ok(response)
        })
        .await
        .map_err(|_| {
            log::error!("⏱️ Response timed out after 30 seconds");
            ClientError::Timeout
        })??;

        Ok(response)
    }

    pub async fn get_transactions(&self, xpub: &str) -> Result<Vec<WalletTransaction>> {
        let message = NetworkMessage::RequestWalletTransactions {
            xpub: xpub.to_string(),
        };

        let response = self.send_message(message).await?;

        match response {
            NetworkMessage::WalletTransactionsResponse { transactions, .. } => Ok(transactions),
            _ => Err(ClientError::InvalidResponse(
                "Expected WalletTransactionsResponse".into(),
            )),
        }
    }

    pub async fn submit_transaction(&self, tx: time_core::Transaction) -> Result<String> {
        let txid = tx.txid.clone();
        let message = NetworkMessage::TransactionBroadcast(tx);

        self.send_message(message).await?;
        Ok(txid)
    }

    pub async fn register_xpub(&self, xpub: &str) -> Result<()> {
        let message = NetworkMessage::RegisterXpub {
            xpub: xpub.to_string(),
        };

        let response = self.send_message(message).await?;

        match response {
            NetworkMessage::XpubRegistered { success, .. } => {
                if success {
                    Ok(())
                } else {
                    Err(ClientError::InvalidResponse("Registration failed".into()))
                }
            }
            _ => Err(ClientError::InvalidResponse(
                "Expected XpubRegistered".into(),
            )),
        }
    }
}
