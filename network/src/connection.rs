//! Peer connection with handshake
use crate::discovery::{NetworkType, PeerInfo};
use crate::protocol::HandshakeMessage;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::sync::Mutex;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageType { Ping, Pong }

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
    pub async fn receive_handshake_static(stream: &mut TcpStream) -> Result<HandshakeMessage, String> {
        Self::receive_handshake(stream).await
    }

    pub async fn send_handshake_static(stream: &mut TcpStream, h: &HandshakeMessage) -> Result<(), String> {
        Self::send_handshake(stream, h).await
    }

    pub async fn connect(
        peer: Arc<Mutex<PeerInfo>>,
        network: NetworkType,
        our_listen_addr: SocketAddr,
    ) -> Result<Self, String> {
        let peer_addr = peer.lock().await.address;
        let mut stream = TcpStream::connect(peer_addr).await
            .map_err(|e| format!("Connect failed: {}", e))?;
        
        let our_handshake = HandshakeMessage::new(network.clone(), our_listen_addr);
        Self::send_handshake(&mut stream, &our_handshake).await?;
        let their_handshake = Self::receive_handshake(&mut stream).await?;
        their_handshake.validate(&network)?;
        
        peer.lock().await.update_version(their_handshake.version.clone());
        
        Ok(PeerConnection { stream, peer_info: peer })
    }

    async fn send_handshake(stream: &mut TcpStream, h: &HandshakeMessage) -> Result<(), String> {
        let json = serde_json::to_vec(h).map_err(|e| e.to_string())?;
        let len = json.len() as u32;
        stream.write_all(&len.to_be_bytes()).await.map_err(|e| e.to_string())?;
        stream.write_all(&json).await.map_err(|e| e.to_string())?;
        stream.flush().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    async fn receive_handshake(stream: &mut TcpStream) -> Result<HandshakeMessage, String> {
        let mut len_bytes = [0u8; 4];
        stream.read_exact(&mut len_bytes).await.map_err(|e| e.to_string())?;
        let len = u32::from_be_bytes(len_bytes) as usize;
        if len > 1024 * 1024 { return Err("Too large".into()); }
        let mut buf = vec![0u8; len];
        stream.read_exact(&mut buf).await.map_err(|e| e.to_string())?;
        serde_json::from_slice(&buf).map_err(|e| e.to_string())
    }

    pub async fn peer_info(&self) -> PeerInfo {
        self.peer_info.lock().await.clone()
    }

    pub async fn ping(&mut self) -> Result<(), String> {
        let msg = NetworkMessage { msg_type: MessageType::Ping, payload: vec![] };
        let json = serde_json::to_vec(&msg).map_err(|e| e.to_string())?;
        let len = json.len() as u32;
        self.stream.write_all(&len.to_be_bytes()).await.map_err(|e| e.to_string())?;
        self.stream.write_all(&json).await.map_err(|e| e.to_string())?;
        self.stream.flush().await.map_err(|e| e.to_string())?;
        Ok(())
    }

    pub async fn keep_alive(mut self) {
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(30)).await;
            if self.ping().await.is_err() { break; }
            self.peer_info.lock().await.touch();
        }
    }
}

pub struct PeerListener {
    listener: tokio::net::TcpListener,
    network: NetworkType,
    our_listen_addr: SocketAddr,
}

impl PeerListener {
    pub async fn bind(listen_addr: SocketAddr, network: NetworkType) -> Result<Self, String> {
        let listener = tokio::net::TcpListener::bind(listen_addr).await
            .map_err(|e| format!("Failed to bind listener: {}", e))?;
        println!("👂 Listening for peers on {}", listen_addr);
        Ok(PeerListener { listener, network, our_listen_addr: listen_addr })
    }

    pub async fn accept(&self) -> Result<PeerConnection, String> {
        let (stream, addr) = self.listener.accept().await
            .map_err(|e| format!("Accept failed: {}", e))?;
        println!("📥 Accepting connection from {}...", addr);
        
        let mut stream = stream;
        let their_handshake = PeerConnection::receive_handshake(&mut stream).await?;
        their_handshake.validate(&self.network)?;
        
        let our_handshake = HandshakeMessage::new(self.network.clone(), self.our_listen_addr);
        PeerConnection::send_handshake(&mut stream, &our_handshake).await?;
        
        let peer_info = PeerInfo::with_version(
            their_handshake.listen_addr,
            self.network.clone(),
            their_handshake.version.clone(),
        );
        
        println!("✓ Accepted from {} (v{})", addr, their_handshake.version);
        
        Ok(PeerConnection {
            stream,
            peer_info: Arc::new(Mutex::new(peer_info)),
        })
    }
}
