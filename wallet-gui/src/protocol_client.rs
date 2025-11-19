//! TIME Coin Protocol Client
//!
//! TCP-only communication with masternode for transaction submission and peer discovery

use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;
use time_network::protocol::NetworkMessage;
use wallet::NetworkType;

#[derive(Debug, thiserror::Error)]
pub enum ProtocolError {
    #[error("Connection error: {0}")]
    ConnectionError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    SerializationError(#[from] bincode::Error),

    #[error("JSON error: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

pub type ProtocolResult<T> = Result<T, ProtocolError>;

/// Simple protocol client using TCP only
pub struct ProtocolClient {
    peer_address: String,
    network: NetworkType,
    timeout: Duration,
}

impl ProtocolClient {
    pub fn new(peer_address: String, network: NetworkType) -> Self {
        Self {
            peer_address,
            network,
            timeout: Duration::from_secs(10),
        }
    }

    /// Connect to peer and send a message
    fn send_message(&self, message: &NetworkMessage) -> ProtocolResult<NetworkMessage> {
        let mut stream = TcpStream::connect(&self.peer_address)
            .map_err(|e| ProtocolError::ConnectionError(e.to_string()))?;

        stream.set_read_timeout(Some(self.timeout))?;
        stream.set_write_timeout(Some(self.timeout))?;

        // Serialize message
        let data = bincode::serialize(message)?;

        // Send length prefix (4 bytes)
        let len = data.len() as u32;
        stream.write_all(&len.to_be_bytes())?;

        // Send data
        stream.write_all(&data)?;
        stream.flush()?;

        // Read response length
        let mut len_buf = [0u8; 4];
        stream.read_exact(&mut len_buf)?;
        let response_len = u32::from_be_bytes(len_buf) as usize;

        // Read response data
        let mut response_buf = vec![0u8; response_len];
        stream.read_exact(&mut response_buf)?;

        // Deserialize response
        let response: NetworkMessage = bincode::deserialize(&response_buf)?;
        Ok(response)
    }

    /// Get peer list from masternode
    pub fn get_peer_list(&self) -> ProtocolResult<Vec<String>> {
        let message = NetworkMessage::GetPeerList;
        let response = self.send_message(&message)?;

        match response {
            NetworkMessage::PeerList(peers) => Ok(peers
                .into_iter()
                .map(|p| format!("{}:{}", p.ip, p.port))
                .collect()),
            _ => Err(ProtocolError::InvalidResponse(
                "Expected PeerList response".to_string(),
            )),
        }
    }

    /// Submit transaction to masternode
    pub fn submit_transaction(&self, transaction: wallet::Transaction) -> ProtocolResult<String> {
        // Simply send the transaction via the protocol
        // The masternode will broadcast it
        use bincode;
        let tx_data =
            bincode::serialize(&transaction).map_err(ProtocolError::SerializationError)?;
        let txid = hex::encode(sha2::Sha256::digest(&tx_data));

        // For now just return the txid
        // TODO: Actually send transaction to masternode
        Ok(txid)
    }

    /// Register xpub with masternode for address monitoring
    pub fn register_xpub(&self, xpub: String) -> ProtocolResult<()> {
        let message = NetworkMessage::RegisterXpub { xpub };
        let response = self.send_message(&message)?;

        match response {
            NetworkMessage::XpubRegistered {
                success,
                message: msg,
            } => {
                if success {
                    Ok(())
                } else {
                    Err(ProtocolError::InvalidResponse(msg))
                }
            }
            _ => Err(ProtocolError::InvalidResponse(
                "Expected XpubRegistered response".to_string(),
            )),
        }
    }

    /// Request blockchain data for syncing
    pub fn request_blockchain_data(
        &self,
        start_height: u64,
    ) -> ProtocolResult<Vec<wallet::Transaction>> {
        let message = NetworkMessage::GetBlocks {
            start_height,
            end_height: start_height + 100,
        };
        let response = self.send_message(&message)?;

        match response {
            NetworkMessage::BlocksData { .. } => {
                // TODO: Extract transactions from blocks
                Ok(Vec::new())
            }
            _ => Err(ProtocolError::InvalidResponse(
                "Expected BlocksData response".to_string(),
            )),
        }
    }

    /// Ping peer to check connectivity
    pub fn ping(&self) -> ProtocolResult<()> {
        let message = NetworkMessage::Ping;
        let response = self.send_message(&message)?;

        match response {
            NetworkMessage::Pong => Ok(()),
            _ => Err(ProtocolError::InvalidResponse(
                "Expected Pong response".to_string(),
            )),
        }
    }
}

/// Wallet sync data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletSyncData {
    pub transactions: Vec<wallet::Transaction>,
    pub current_height: u64,
    pub total_balance: u64,
}
