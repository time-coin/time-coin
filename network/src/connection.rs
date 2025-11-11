//! Peer connection with handshake
use crate::discovery::{NetworkType, PeerInfo};
use crate::protocol::HandshakeMessage;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Arc as StdArc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType {
    Ping,
    Pong,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkMessage {
    pub msg_type: MessageType,
    pub payload: Vec<u8>,
}

pub struct PeerConnection {
    stream: TcpStream,
    peer_info: Arc<Mutex<PeerInfo>>,
}

impl PeerConnection {
    pub async fn connect(
        peer: Arc<Mutex<PeerInfo>>,
        network: NetworkType,
        our_listen_addr: SocketAddr,
        // Add optional blockchain state for registration
        blockchain: Option<StdArc<tokio::sync::RwLock<time_core::state::BlockchainState>>>,
        consensus: Option<StdArc<time_consensus::ConsensusEngine>>,
    ) -> Result<Self, String> {
        let peer_addr = peer.lock().await.address;
        let mut stream = TcpStream::connect(peer_addr)
            .await
            .map_err(|e| format!("Connect failed: {}", e))?;

        // Get our genesis hash if blockchain is available
        let our_genesis_hash = if let Some(bc) = &blockchain {
            let chain = bc.read().await;
            Some(chain.genesis_hash().to_string())
        } else {
            None
        };

        let our_handshake = HandshakeMessage::new_with_genesis(
            network.clone(),
            our_listen_addr,
            our_genesis_hash.clone(),
        );
        Self::send_handshake(&mut stream, &our_handshake, &network).await?;
        let their_handshake = Self::receive_handshake(&mut stream, &network).await?;

        // Validate with genesis check
        their_handshake.validate_with_genesis(&network, our_genesis_hash.as_deref())?;

        // Update peer info with version AND commit info
        peer.lock().await.update_version_with_build_info(
            their_handshake.version.clone(),
            their_handshake.commit_date.clone(),
            their_handshake.commit_count.clone(),
        );

        // Log peer connection with full version info
        let peer_date = their_handshake.commit_date.as_deref().unwrap_or("unknown");
        println!(
            "🔗 Connected to peer: {} | Version: {} | Committed: {} | Commits: {}",
            peer_addr.ip(),
            their_handshake.version,
            peer_date,
            their_handshake.commit_count.as_deref().unwrap_or("unknown")
        );

        // Auto-register masternode if wallet address provided
        if let Some(wallet_addr) = &their_handshake.wallet_address {
            if let (Some(blockchain), Some(consensus)) = (&blockchain, &consensus) {
                Self::auto_register_masternode(
                    peer_addr.ip().to_string(),
                    wallet_addr.clone(),
                    blockchain.clone(),
                    consensus.clone(),
                )
                .await;
            }
        }

        // Check version and warn ONLY if peer is running a NEWER version
        let peer_date = their_handshake.commit_date.as_deref();
        if crate::protocol::should_warn_version_update(
            peer_date,
            their_handshake.commit_count.as_deref(),
        ) {
            let warning = crate::protocol::version_update_warning(
                &format!("{}", peer_addr),
                &their_handshake.version,
                peer_date.unwrap_or("unknown"),
                their_handshake.commit_count.as_deref().unwrap_or("0"),
            );
            eprintln!("{}", warning);
        }

        Ok(PeerConnection {
            stream,
            peer_info: peer,
        })
    }

    /// Auto-register a masternode when peer connects
    async fn auto_register_masternode(
        node_ip: String,
        wallet_address: String,
        blockchain: StdArc<tokio::sync::RwLock<time_core::state::BlockchainState>>,
        consensus: StdArc<time_consensus::ConsensusEngine>,
    ) {
        use time_core::MasternodeTier;

        println!(
            "🔐 Auto-registering masternode: {} -> {}",
            node_ip, wallet_address
        );

        // Register in blockchain state
        let mut chain = blockchain.write().await;
        match chain.register_masternode(
            node_ip.clone(),
            MasternodeTier::Free, // Default to Free tier
            "peer_connection".to_string(),
            wallet_address.clone(),
        ) {
            Ok(_) => {
                drop(chain);

                // Also register in consensus
                consensus.add_masternode(node_ip.clone()).await;
                consensus
                    .register_wallet(node_ip.clone(), wallet_address.clone())
                    .await;

                println!(
                    "✅ Masternode auto-registered: {} -> {}",
                    node_ip, wallet_address
                );
            }
            Err(e) => {
                println!("⚠️  Auto-registration skipped for {}: {:?}", node_ip, e);
            }
        }
    }

    async fn send_handshake(
        stream: &mut TcpStream,
        h: &HandshakeMessage,
        network: &NetworkType,
    ) -> Result<(), String> {
        let json = serde_json::to_vec(h).map_err(|e| e.to_string())?;
        let len = json.len() as u32;
        
        // Write magic bytes first
        let magic = network.magic_bytes();
        stream
            .write_all(&magic)
            .await
            .map_err(|e| format!("Failed to write magic bytes: {}", e))?;
        
        // Then write length and payload
        stream
            .write_all(&len.to_be_bytes())
            .await
            .map_err(|e| e.to_string())?;
        stream.write_all(&json).await.map_err(|e| e.to_string())?;
        stream.flush().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn receive_handshake(
        stream: &mut TcpStream,
        network: &NetworkType,
    ) -> Result<HandshakeMessage, String> {
        // Read and validate magic bytes
        let mut magic_bytes = [0u8; 4];
        stream
            .read_exact(&mut magic_bytes)
            .await
            .map_err(|e| format!("Failed to read magic bytes: {}", e))?;
        
        let expected_magic = network.magic_bytes();
        if magic_bytes != expected_magic {
            return Err(format!(
                "Invalid magic bytes: expected {:?}, got {:?}",
                expected_magic, magic_bytes
            ));
        }
        
        // Read length
        let mut len_bytes = [0u8; 4];
        stream
            .read_exact(&mut len_bytes)
            .await
            .map_err(|e| e.to_string())?;
        let len = u32::from_be_bytes(len_bytes) as usize;
        if len > 1024 * 1024 {
            return Err("Too large".into());
        }
        
        // Read payload
        let mut buf = vec![0u8; len];
        stream
            .read_exact(&mut buf)
            .await
            .map_err(|e| e.to_string())?;
        serde_json::from_slice(&buf).map_err(|e| e.to_string())
    }

    pub async fn peer_info(&self) -> PeerInfo {
        self.peer_info.lock().await.clone()
    }

    pub async fn ping(&mut self) -> Result<(), String> {
        let msg = NetworkMessage {
            msg_type: MessageType::Ping,
            payload: vec![],
        };
        let json = serde_json::to_vec(&msg).map_err(|e| e.to_string())?;
        let len = json.len() as u32;
        self.stream
            .write_all(&len.to_be_bytes())
            .await
            .map_err(|e| e.to_string())?;
        self.stream
            .write_all(&json)
            .await
            .map_err(|e| e.to_string())?;
        self.stream.flush().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn keep_alive(mut self) {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            if self.ping().await.is_err() {
                break;
            }
            self.peer_info.lock().await.touch();
        }
    }
}

use tokio::net::TcpListener;

pub struct PeerListener {
    listener: TcpListener,
    network: NetworkType,
    our_listen_addr: SocketAddr,
}

impl PeerListener {
    pub async fn bind(listen_addr: SocketAddr, network: NetworkType) -> Result<Self, String> {
        let listener = TcpListener::bind(listen_addr)
            .await
            .map_err(|e| format!("Failed to bind: {}", e))?;
        println!("👂 Listening for peers on {}", listen_addr);
        Ok(PeerListener {
            listener,
            network,
            our_listen_addr: listen_addr,
        })
    }

    pub async fn accept(&self) -> Result<PeerConnection, String> {
        let (mut stream, addr) = self
            .listener
            .accept()
            .await
            .map_err(|e| format!("Accept failed: {}", e))?;
        println!("📥 Incoming connection from {}", addr);

        let their_handshake = PeerConnection::receive_handshake(&mut stream, &self.network).await?;
        their_handshake.validate(&self.network)?;

        let our_handshake = HandshakeMessage::new(self.network.clone(), self.our_listen_addr);
        PeerConnection::send_handshake(&mut stream, &our_handshake, &self.network).await?;

        let peer_info =
            PeerInfo::with_version(addr, self.network.clone(), their_handshake.version.clone());

        println!("✓ Accepted {} (v{})", addr, their_handshake.version);

        Ok(PeerConnection {
            stream,
            peer_info: Arc::new(Mutex::new(peer_info)),
        })
    }
}
