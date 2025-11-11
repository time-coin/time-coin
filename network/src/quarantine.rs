//! Peer quarantine system for nodes on different chains or forks
//!
//! Tracks and isolates peers that are detected to be on:
//! - Different genesis blocks
//! - Forked chains
//! - Invalid chains with suspicious heights

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub enum QuarantineReason {
    GenesisMismatch {
        our_genesis: String,
        their_genesis: String,
    },
    ForkDetected {
        height: u64,
        our_hash: String,
        their_hash: String,
    },
    SuspiciousHeight {
        their_height: u64,
        max_expected: u64,
    },
    ConsensusViolation {
        reason: String,
    },
}

impl std::fmt::Display for QuarantineReason {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            QuarantineReason::GenesisMismatch {
                our_genesis,
                their_genesis,
            } => {
                let our_display = if our_genesis.len() >= 16 {
                    &our_genesis[..16]
                } else {
                    our_genesis
                };
                let their_display = if their_genesis.len() >= 16 {
                    &their_genesis[..16]
                } else {
                    their_genesis
                };
                write!(
                    f,
                    "Genesis mismatch: ours={}..., theirs={}...",
                    our_display, their_display
                )
            }
            QuarantineReason::ForkDetected {
                height,
                our_hash,
                their_hash,
            } => {
                let our_display = if our_hash.len() >= 16 {
                    &our_hash[..16]
                } else {
                    our_hash
                };
                let their_display = if their_hash.len() >= 16 {
                    &their_hash[..16]
                } else {
                    their_hash
                };
                write!(
                    f,
                    "Fork at height {}: ours={}..., theirs={}...",
                    height, our_display, their_display
                )
            }
            QuarantineReason::SuspiciousHeight {
                their_height,
                max_expected,
            } => write!(
                f,
                "Suspicious height {} (max expected: {})",
                their_height, max_expected
            ),
            QuarantineReason::ConsensusViolation { reason } => {
                write!(f, "Consensus violation: {}", reason)
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct QuarantineEntry {
    pub peer_ip: IpAddr,
    pub reason: QuarantineReason,
    pub quarantined_at: Instant,
    pub attempts: u32,
}

pub struct PeerQuarantine {
    quarantined: Arc<RwLock<HashMap<IpAddr, QuarantineEntry>>>,
    quarantine_duration: Duration,
}

impl PeerQuarantine {
    /// Create a new peer quarantine system
    pub fn new() -> Self {
        Self {
            quarantined: Arc::new(RwLock::new(HashMap::new())),
            quarantine_duration: Duration::from_secs(3600), // 1 hour default
        }
    }

    /// Create with custom quarantine duration
    pub fn with_duration(duration: Duration) -> Self {
        Self {
            quarantined: Arc::new(RwLock::new(HashMap::new())),
            quarantine_duration: duration,
        }
    }

    /// Add a peer to quarantine
    pub async fn quarantine_peer(&self, peer_ip: IpAddr, reason: QuarantineReason) {
        let mut quarantined = self.quarantined.write().await;

        let entry = quarantined.entry(peer_ip).or_insert(QuarantineEntry {
            peer_ip,
            reason: reason.clone(),
            quarantined_at: Instant::now(),
            attempts: 0,
        });

        entry.attempts += 1;
        entry.quarantined_at = Instant::now();
        entry.reason = reason.clone();

        eprintln!(
            "🚫 Peer {} quarantined: {} (attempt #{})",
            peer_ip, reason, entry.attempts
        );
    }

    /// Check if a peer is quarantined
    pub async fn is_quarantined(&self, peer_ip: &IpAddr) -> bool {
        let quarantined = self.quarantined.read().await;
        if let Some(entry) = quarantined.get(peer_ip) {
            // Check if quarantine has expired
            if entry.quarantined_at.elapsed() < self.quarantine_duration {
                return true;
            }
        }
        false
    }

    /// Get quarantine reason for a peer
    pub async fn get_reason(&self, peer_ip: &IpAddr) -> Option<QuarantineReason> {
        let quarantined = self.quarantined.read().await;
        quarantined.get(peer_ip).map(|e| e.reason.clone())
    }

    /// Remove a peer from quarantine (manual override)
    pub async fn release_peer(&self, peer_ip: &IpAddr) {
        let mut quarantined = self.quarantined.write().await;
        if quarantined.remove(peer_ip).is_some() {
            eprintln!("✅ Peer {} released from quarantine", peer_ip);
        }
    }

    /// Get all quarantined peers
    pub async fn get_quarantined_peers(&self) -> Vec<QuarantineEntry> {
        let quarantined = self.quarantined.read().await;
        quarantined.values().cloned().collect()
    }

    /// Clean up expired quarantine entries
    pub async fn cleanup_expired(&self) {
        let mut quarantined = self.quarantined.write().await;
        quarantined.retain(|_, entry| entry.quarantined_at.elapsed() < self.quarantine_duration);
    }

    /// Get count of quarantined peers
    pub async fn count(&self) -> usize {
        let quarantined = self.quarantined.read().await;
        quarantined.len()
    }

    /// Check if peer should be excluded from consensus
    pub async fn should_exclude_from_consensus(&self, peer_ip: &IpAddr) -> bool {
        if let Some(reason) = self.get_reason(peer_ip).await {
            match reason {
                QuarantineReason::GenesisMismatch { .. } => true, // Always exclude
                QuarantineReason::ForkDetected { .. } => true,    // Always exclude
                QuarantineReason::SuspiciousHeight { .. } => true, // Always exclude
                QuarantineReason::ConsensusViolation { .. } => true, // Always exclude
            }
        } else {
            false
        }
    }
}

impl Default for PeerQuarantine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_quarantine_peer() {
        let quarantine = PeerQuarantine::new();
        let peer_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        assert!(!quarantine.is_quarantined(&peer_ip).await);

        quarantine
            .quarantine_peer(
                peer_ip,
                QuarantineReason::GenesisMismatch {
                    our_genesis: "abc123".to_string(),
                    their_genesis: "def456".to_string(),
                },
            )
            .await;

        assert!(quarantine.is_quarantined(&peer_ip).await);
    }

    #[tokio::test]
    async fn test_quarantine_expiry() {
        let quarantine = PeerQuarantine::with_duration(Duration::from_millis(100));
        let peer_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        quarantine
            .quarantine_peer(
                peer_ip,
                QuarantineReason::ForkDetected {
                    height: 100,
                    our_hash: "abc".to_string(),
                    their_hash: "def".to_string(),
                },
            )
            .await;

        assert!(quarantine.is_quarantined(&peer_ip).await);

        // Wait for expiry
        tokio::time::sleep(Duration::from_millis(150)).await;

        assert!(!quarantine.is_quarantined(&peer_ip).await);
    }

    #[tokio::test]
    async fn test_release_peer() {
        let quarantine = PeerQuarantine::new();
        let peer_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        quarantine
            .quarantine_peer(
                peer_ip,
                QuarantineReason::SuspiciousHeight {
                    their_height: 1000,
                    max_expected: 100,
                },
            )
            .await;

        assert!(quarantine.is_quarantined(&peer_ip).await);

        quarantine.release_peer(&peer_ip).await;

        assert!(!quarantine.is_quarantined(&peer_ip).await);
    }

    #[tokio::test]
    async fn test_should_exclude_from_consensus() {
        let quarantine = PeerQuarantine::new();
        let peer_ip = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));

        assert!(!quarantine.should_exclude_from_consensus(&peer_ip).await);

        quarantine
            .quarantine_peer(
                peer_ip,
                QuarantineReason::GenesisMismatch {
                    our_genesis: "abc123".to_string(),
                    their_genesis: "def456".to_string(),
                },
            )
            .await;

        assert!(quarantine.should_exclude_from_consensus(&peer_ip).await);
    }

    #[tokio::test]
    async fn test_cleanup_expired() {
        let quarantine = PeerQuarantine::with_duration(Duration::from_millis(100));
        let peer1 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 1));
        let peer2 = IpAddr::V4(Ipv4Addr::new(192, 168, 1, 2));

        quarantine
            .quarantine_peer(
                peer1,
                QuarantineReason::ForkDetected {
                    height: 100,
                    our_hash: "abc".to_string(),
                    their_hash: "def".to_string(),
                },
            )
            .await;

        tokio::time::sleep(Duration::from_millis(50)).await;

        quarantine
            .quarantine_peer(
                peer2,
                QuarantineReason::ForkDetected {
                    height: 100,
                    our_hash: "abc".to_string(),
                    their_hash: "ghi".to_string(),
                },
            )
            .await;

        assert_eq!(quarantine.count().await, 2);

        tokio::time::sleep(Duration::from_millis(60)).await;
        quarantine.cleanup_expired().await;

        // peer1 should be expired, peer2 should remain
        assert_eq!(quarantine.count().await, 1);
        assert!(!quarantine.is_quarantined(&peer1).await);
        assert!(quarantine.is_quarantined(&peer2).await);
    }
}
