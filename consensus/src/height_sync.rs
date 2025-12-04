//! Block Height Synchronization
//!
//! Ensures nodes stay synchronized by:
//! - Querying peer heights
//! - Detecting when behind
//! - Requesting missing blocks
//! - Waiting for height consensus before production

use std::collections::HashMap;
use std::time::Duration;

/// Height information from a peer
#[derive(Debug, Clone)]
pub struct PeerHeight {
    pub peer_address: String,
    pub height: u64,
    pub timestamp: i64,
}

/// Result of height consensus check
#[derive(Debug)]
pub struct HeightConsensus {
    pub has_consensus: bool,
    pub consensus_height: u64,
    pub our_height: u64,
    pub peer_heights: Vec<PeerHeight>,
    pub behind_by: i64,
}

/// Manager for block height synchronization
pub struct HeightSyncManager {
    /// Minimum percentage of peers that must agree (e.g., 67 = 67%)
    consensus_threshold_percent: u64,

    /// Maximum time to wait for height consensus (seconds)
    consensus_timeout_secs: u64,

    /// Maximum blocks behind before triggering sync
    max_blocks_behind: u64,
}

impl HeightSyncManager {
    pub fn new(consensus_threshold_percent: u64) -> Self {
        Self {
            consensus_threshold_percent,
            consensus_timeout_secs: 30,
            max_blocks_behind: 5,
        }
    }

    /// Query a peer for their current block height
    pub async fn query_peer_height(&self, peer: &str) -> Option<u64> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .ok()?;

        let url = format!("http://{}:24101/api/blockchain/height", peer);

        match client.get(&url).send().await {
            Ok(response) => response.json::<u64>().await.ok(),
            Err(_) => None,
        }
    }

    /// Query all peers for their heights
    pub async fn query_all_peer_heights(&self, peers: &[String]) -> Vec<PeerHeight> {
        let mut peer_heights = Vec::new();
        let timestamp = chrono::Utc::now().timestamp();

        for peer in peers {
            if let Some(height) = self.query_peer_height(peer).await {
                peer_heights.push(PeerHeight {
                    peer_address: peer.clone(),
                    height,
                    timestamp,
                });
            }
        }

        peer_heights
    }

    /// Check if we have height consensus
    pub fn check_height_consensus(
        &self,
        our_height: u64,
        peer_heights: Vec<PeerHeight>,
    ) -> HeightConsensus {
        if peer_heights.is_empty() {
            return HeightConsensus {
                has_consensus: true, // Solo node
                consensus_height: our_height,
                our_height,
                peer_heights: vec![],
                behind_by: 0,
            };
        }

        // Find most common height among peers
        let mut height_counts: HashMap<u64, usize> = HashMap::new();
        for peer in &peer_heights {
            *height_counts.entry(peer.height).or_insert(0) += 1;
        }

        let (consensus_height, count) = height_counts
            .iter()
            .max_by_key(|(_, count)| *count)
            .map(|(h, c)| (*h, *c))
            .unwrap_or((our_height, 0));

        let total_peers = peer_heights.len();
        let consensus_percent = (count * 100) / total_peers.max(1);

        let has_consensus = consensus_percent >= self.consensus_threshold_percent as usize;
        let behind_by = consensus_height as i64 - our_height as i64;

        HeightConsensus {
            has_consensus,
            consensus_height,
            our_height,
            peer_heights,
            behind_by,
        }
    }

    /// Wait for height consensus with timeout
    pub async fn wait_for_height_consensus(
        &self,
        expected_height: u64,
        peers: &[String],
        get_current_height: impl Fn() -> u64,
    ) -> Result<HeightConsensus, String> {
        let start = std::time::Instant::now();
        let timeout = Duration::from_secs(self.consensus_timeout_secs);

        loop {
            let our_height = get_current_height();
            let peer_heights = self.query_all_peer_heights(peers).await;
            let consensus = self.check_height_consensus(our_height, peer_heights);

            // Check if we're at the expected height with consensus
            if consensus.has_consensus && consensus.consensus_height == expected_height {
                println!("✅ Height consensus reached at {}", expected_height);
                return Ok(consensus);
            }

            // Check timeout
            if start.elapsed() > timeout {
                println!(
                    "⚠️  Height consensus timeout: our={}, consensus={}, expected={}",
                    consensus.our_height, consensus.consensus_height, expected_height
                );
                return Ok(consensus); // Return current state
            }

            // Brief wait before retry
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    }

    /// Determine if we need to sync blocks
    pub fn needs_sync(&self, consensus: &HeightConsensus) -> bool {
        consensus.behind_by > 0 && consensus.behind_by as u64 <= self.max_blocks_behind
    }

    /// Determine if we're too far behind (need full resync)
    pub fn needs_full_resync(&self, consensus: &HeightConsensus) -> bool {
        consensus.behind_by as u64 > self.max_blocks_behind
    }
}

/// Block verification across peers
pub struct BlockVerificationManager {
    /// Minimum percentage of peers that must agree on block hash
    verification_threshold_percent: u64,
}

impl BlockVerificationManager {
    pub fn new(verification_threshold_percent: u64) -> Self {
        Self {
            verification_threshold_percent,
        }
    }

    /// Query a peer for their block hash at a specific height
    pub async fn query_peer_block_hash(&self, peer: &str, height: u64) -> Option<String> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()
            .ok()?;

        let url = format!("http://{}:24101/api/blockchain/block/{}/hash", peer, height);

        match client.get(&url).send().await {
            Ok(response) => response.text().await.ok(),
            Err(_) => None,
        }
    }

    /// Verify block at height matches across peers
    pub async fn verify_block_consensus(
        &self,
        height: u64,
        our_hash: &str,
        peers: &[String],
    ) -> Result<bool, String> {
        if peers.is_empty() {
            return Ok(true); // Solo node - trust our hash
        }

        let mut matching = 0;
        let mut total = 0;

        println!(
            "🔍 Verifying block {} hash across {} peers",
            height,
            peers.len()
        );

        for peer in peers {
            if let Some(peer_hash) = self.query_peer_block_hash(peer, height).await {
                total += 1;
                if peer_hash.trim() == our_hash.trim() {
                    matching += 1;
                    println!("  ✓ {} agrees", peer);
                } else {
                    println!("  ✗ {} has different hash", peer);
                    println!("    Ours:   {}", &our_hash[..our_hash.len().min(16)]);
                    println!("    Theirs: {}", &peer_hash[..peer_hash.len().min(16)]);
                }
            }
        }

        if total == 0 {
            println!("⚠️  No peers responded for block verification");
            return Ok(true); // Can't verify, allow production
        }

        let match_percent = (matching * 100) / total;
        let has_consensus = match_percent >= self.verification_threshold_percent as usize;

        if has_consensus {
            println!(
                "✅ Block verification passed: {}/{} peers agree ({}%)",
                matching, total, match_percent
            );
        } else {
            println!(
                "❌ Block verification failed: only {}/{} peers agree ({}%)",
                matching, total, match_percent
            );
        }

        Ok(has_consensus)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_height_consensus_solo_node() {
        let manager = HeightSyncManager::new(67);
        let consensus = manager.check_height_consensus(100, vec![]);

        assert!(consensus.has_consensus);
        assert_eq!(consensus.consensus_height, 100);
        assert_eq!(consensus.behind_by, 0);
    }

    #[test]
    fn test_height_consensus_behind() {
        let manager = HeightSyncManager::new(67);
        let peer_heights = vec![
            PeerHeight {
                peer_address: "peer1".to_string(),
                height: 105,
                timestamp: 0,
            },
            PeerHeight {
                peer_address: "peer2".to_string(),
                height: 105,
                timestamp: 0,
            },
            PeerHeight {
                peer_address: "peer3".to_string(),
                height: 105,
                timestamp: 0,
            },
        ];

        let consensus = manager.check_height_consensus(100, peer_heights);

        assert!(consensus.has_consensus);
        assert_eq!(consensus.consensus_height, 105);
        assert_eq!(consensus.behind_by, 5);
    }

    #[test]
    fn test_needs_sync() {
        let manager = HeightSyncManager::new(67);

        let consensus = HeightConsensus {
            has_consensus: true,
            consensus_height: 105,
            our_height: 100,
            peer_heights: vec![],
            behind_by: 5,
        };

        assert!(manager.needs_sync(&consensus));
        assert!(!manager.needs_full_resync(&consensus));
    }
}
