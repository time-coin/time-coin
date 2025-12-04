//! Network Health Check
//!
//! Validates network connectivity before block production to prevent
//! nodes from creating blocks in isolation

use std::time::Duration;

pub struct NetworkHealthCheck {
    min_peers: usize,
    peer_response_timeout_secs: u64,
}

impl NetworkHealthCheck {
    pub fn new(min_peers: usize) -> Self {
        Self {
            min_peers,
            peer_response_timeout_secs: 5,
        }
    }

    /// Check if network is healthy before producing block
    pub async fn is_network_healthy(&self, peers: &[String]) -> bool {
        if peers.len() < self.min_peers {
            println!("❌ Not enough peers: {} < {}", peers.len(), self.min_peers);
            return false;
        }

        // Ping peers to verify they're responsive
        let mut responding = 0;

        // Sample up to 5 peers to check connectivity
        let sample_size = peers.len().min(5);

        for peer in peers.iter().take(sample_size) {
            if self.ping_peer(peer).await {
                responding += 1;
            }
        }

        let healthy = responding >= self.min_peers.max(1) / 2;

        if healthy {
            println!(
                "✅ Network healthy: {}/{} peers responding",
                responding, sample_size
            );
        } else {
            println!(
                "❌ Network unhealthy: only {}/{} peers responding",
                responding, sample_size
            );
        }

        healthy
    }

    async fn ping_peer(&self, peer: &str) -> bool {
        matches!(
            tokio::time::timeout(
                Duration::from_secs(self.peer_response_timeout_secs),
                self.ping_peer_impl(peer),
            )
            .await,
            Ok(true)
        )
    }

    async fn ping_peer_impl(&self, peer: &str) -> bool {
        // Try to connect to the peer's API port
        let client = match reqwest::Client::builder()
            .timeout(Duration::from_secs(self.peer_response_timeout_secs))
            .build()
        {
            Ok(c) => c,
            Err(_) => return false,
        };

        let url = format!("http://{}:24101/health", peer);

        match client.get(&url).send().await {
            Ok(response) => response.status().is_success(),
            Err(_) => false,
        }
    }
}
