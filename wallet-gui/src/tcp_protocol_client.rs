//! TCP-based TIME Coin Protocol Client
//!
//! Communicates with masternodes using raw TCP and NetworkMessage protocol

use bincode;
use std::sync::Arc;
use time_network::protocol::{NetworkMessage, UtxoInfo, WalletTransaction};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::{mpsc, RwLock};
use wallet::NetworkType;

pub struct TcpProtocolClient {
    network: NetworkType,
    xpub: Option<String>,
    connected_peers: Arc<RwLock<Vec<String>>>,
    active: Arc<RwLock<bool>>,
}

impl TcpProtocolClient {
    pub fn new(network: NetworkType) -> Self {
        Self {
            network,
            xpub: None,
            connected_peers: Arc::new(RwLock::new(Vec::new())),
            active: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn set_xpub(&mut self, xpub: String) {
        self.xpub = Some(xpub);
    }

    /// Connect to a masternode and register xpub
    pub async fn connect_to_masternode(
        &self,
        masternode_url: &str,
        tx_sender: mpsc::UnboundedSender<WalletTransaction>,
    ) -> Result<(), String> {
        // Extract IP from URL
        let ip = masternode_url
            .replace("http://", "")
            .replace("https://", "")
            .split(':')
            .next()
            .ok_or("Invalid masternode URL")?
            .to_string();

        // Connect to TCP port (24100 for testnet, 24101 for mainnet)
        let port = if self.network == NetworkType::Testnet {
            24100
        } else {
            24101
        };
        let tcp_addr = format!("{}:{}", ip, port);

        log::info!("Connecting to masternode via TCP: {}", tcp_addr);

        let mut stream = TcpStream::connect(&tcp_addr)
            .await
            .map_err(|e| format!("TCP connection failed: {}", e))?;

        *self.active.write().await = true;

        // Register xpub if available
        if let Some(xpub) = &self.xpub {
            log::info!("Registering xpub with masternode...");
            let msg = NetworkMessage::RegisterXpub {
                xpub: xpub.clone(),
            };
            self.send_message(&mut stream, &msg).await?;

            // Wait for response
            match self.receive_message(&mut stream).await? {
                NetworkMessage::XpubRegistered { success, message } => {
                    if success {
                        log::info!("✓ xPub registered successfully: {}", message);
                    } else {
                        return Err(format!("xPub registration failed: {}", message));
                    }
                }
                _ => {
                    return Err("Unexpected response to xPub registration".to_string());
                }
            }
        }

        // Spawn reader task
        let active = self.active.clone();
        tokio::spawn(async move {
            loop {
                if !*active.read().await {
                    break;
                }

                match Self::receive_message_static(&mut stream).await {
                    Ok(msg) => match msg {
                        NetworkMessage::NewTransactionNotification { transaction } => {
                            log::info!("Received transaction notification: {}", transaction.tx_hash);
                            if let Err(e) = tx_sender.send(transaction) {
                                log::error!("Failed to forward transaction: {}", e);
                            }
                        }
                        NetworkMessage::UtxoUpdate { xpub, utxos } => {
                            log::info!("Received UTXO update for {}: {} UTXOs", xpub, utxos.len());
                            // TODO: Process UTXO updates
                        }
                        NetworkMessage::Ping => {
                            // Respond to ping
                            let pong = NetworkMessage::Pong;
                            if let Err(e) =
                                Self::send_message_static(&mut stream, &pong).await
                            {
                                log::error!("Failed to send pong: {}", e);
                            }
                        }
                        _ => {
                            log::debug!("Received message: {:?}", msg);
                        }
                    },
                    Err(e) => {
                        log::error!("Connection error: {}", e);
                        break;
                    }
                }
            }
            log::info!("TCP protocol client disconnected");
        });

        Ok(())
    }

    /// Send a message over TCP
    async fn send_message(&self, stream: &mut TcpStream, msg: &NetworkMessage) -> Result<(), String> {
        Self::send_message_static(stream, msg).await
    }

    async fn send_message_static(stream: &mut TcpStream, msg: &NetworkMessage) -> Result<(), String> {
        let encoded = bincode::serialize(msg)
            .map_err(|e| format!("Failed to serialize message: {}", e))?;
        
        // Send length prefix (4 bytes)
        let len = encoded.len() as u32;
        stream
            .write_all(&len.to_be_bytes())
            .await
            .map_err(|e| format!("Failed to send length: {}", e))?;
        
        // Send message
        stream
            .write_all(&encoded)
            .await
            .map_err(|e| format!("Failed to send message: {}", e))?;
        
        stream.flush().await.map_err(|e| format!("Failed to flush: {}", e))?;
        
        Ok(())
    }

    /// Receive a message from TCP
    async fn receive_message(&self, stream: &mut TcpStream) -> Result<NetworkMessage, String> {
        Self::receive_message_static(stream).await
    }

    async fn receive_message_static(stream: &mut TcpStream) -> Result<NetworkMessage, String> {
        // Read length prefix
        let mut len_buf = [0u8; 4];
        stream
            .read_exact(&mut len_buf)
            .await
            .map_err(|e| format!("Failed to read length: {}", e))?;
        let len = u32::from_be_bytes(len_buf) as usize;

        // Read message
        let mut buf = vec![0u8; len];
        stream
            .read_exact(&mut buf)
            .await
            .map_err(|e| format!("Failed to read message: {}", e))?;

        // Deserialize
        bincode::deserialize(&buf)
            .map_err(|e| format!("Failed to deserialize message: {}", e))
    }

    pub async fn disconnect(&self) {
        *self.active.write().await = false;
    }
}
