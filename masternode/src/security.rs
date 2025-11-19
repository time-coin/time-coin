//! Secure message handler for masternode with integrated security features

use std::net::IpAddr;
use std::sync::Arc;
use time_network::{
    AuthError, AuthenticatedMessage, NonceTracker, PeerQuarantine, QuarantineReason, RateLimiter,
    RateLimiterConfig,
};
use tracing::{error, info, warn};

/// Secure message handler with rate limiting, authentication, and quarantine
pub struct SecureMasternodeHandler {
    rate_limiter: Arc<RateLimiter>,
    nonce_tracker: Arc<NonceTracker>,
    quarantine: Arc<PeerQuarantine>,
}

impl SecureMasternodeHandler {
    /// Create a new secure message handler
    pub fn new(quarantine: Arc<PeerQuarantine>) -> Self {
        // Configure rate limiter for masternode operations
        let rate_config = RateLimiterConfig {
            max_requests: 100, // 100 requests per minute
            window: std::time::Duration::from_secs(60),
            burst_size: 20, // Max 20 requests per second
        };

        Self {
            rate_limiter: Arc::new(RateLimiter::with_config(rate_config)),
            nonce_tracker: Arc::new(NonceTracker::new()),
            quarantine,
        }
    }

    /// Check if peer should be allowed to send messages
    pub async fn check_peer_allowed(&self, peer_ip: IpAddr) -> Result<(), SecurityError> {
        // Check if peer is quarantined
        if self.quarantine.is_quarantined(&peer_ip).await {
            warn!(peer = %peer_ip, "Rejected message from quarantined peer");
            return Err(SecurityError::PeerQuarantined(peer_ip));
        }

        // Check rate limit
        if let Err(e) = self.rate_limiter.check_rate_limit(peer_ip).await {
            warn!(peer = %peer_ip, error = %e, "Rate limit exceeded");

            // Quarantine peer for rate limit violation
            let requests = self.rate_limiter.get_request_count(peer_ip).await;
            self.quarantine
                .quarantine_peer(
                    peer_ip,
                    QuarantineReason::RateLimitExceeded {
                        requests_per_second: requests / 60,
                    },
                )
                .await;

            return Err(SecurityError::RateLimitExceeded(peer_ip));
        }

        Ok(())
    }

    /// Verify authenticated message
    pub async fn verify_message(
        &self,
        peer_ip: IpAddr,
        msg: &AuthenticatedMessage,
        expected_pubkey: &[u8],
    ) -> Result<(), SecurityError> {
        // Verify signature
        if let Err(e) = msg.verify(expected_pubkey) {
            warn!(
                peer = %peer_ip,
                sender = %msg.sender,
                error = %e,
                "Message verification failed"
            );

            // Quarantine peer for invalid signature
            self.quarantine
                .quarantine_peer(
                    peer_ip,
                    QuarantineReason::ConsensusViolation {
                        reason: format!("Invalid message signature: {}", e),
                    },
                )
                .await;

            return Err(SecurityError::InvalidSignature(peer_ip));
        }

        // Check replay attack
        if let Err(e) = self.nonce_tracker.check_and_mark(&msg.nonce).await {
            warn!(
                peer = %peer_ip,
                sender = %msg.sender,
                nonce = %msg.nonce,
                error = %e,
                "Replay attack detected"
            );

            // Quarantine peer for replay attack (severe offense)
            self.quarantine
                .quarantine_peer(
                    peer_ip,
                    QuarantineReason::ConsensusViolation {
                        reason: format!("Replay attack: {}", e),
                    },
                )
                .await;

            return Err(SecurityError::ReplayAttack(peer_ip));
        }

        info!(
            peer = %peer_ip,
            sender = %msg.sender,
            age_sec = msg.age_seconds(),
            "Message verified successfully"
        );

        Ok(())
    }

    /// Handle incoming message with full security checks
    pub async fn handle_secure_message<F, T>(
        &self,
        peer_ip: IpAddr,
        msg: AuthenticatedMessage,
        expected_pubkey: &[u8],
        handler: F,
    ) -> Result<T, SecurityError>
    where
        F: FnOnce(Vec<u8>) -> Result<T, String>,
    {
        // Check peer is allowed
        self.check_peer_allowed(peer_ip).await?;

        // Verify message
        self.verify_message(peer_ip, &msg, expected_pubkey).await?;

        // Process message
        handler(msg.payload).map_err(|e| {
            error!(
                peer = %peer_ip,
                error = %e,
                "Message processing failed"
            );
            SecurityError::ProcessingError(e)
        })
    }

    /// Reset peer restrictions (for testing or manual override)
    pub async fn reset_peer(&self, peer_ip: IpAddr) {
        self.rate_limiter.reset(peer_ip).await;
        // Note: Quarantine removal should be done through quarantine API
    }

    /// Get security statistics
    pub async fn get_stats(&self) -> SecurityStats {
        let q_stats = self.quarantine.get_stats().await;

        SecurityStats {
            total_quarantined: q_stats.total_quarantined,
            minor_bans: q_stats.connection_failures + q_stats.excessive_timeouts,
            moderate_bans: q_stats.consensus_violation + q_stats.invalid_transaction,
            severe_bans: q_stats.fork_detected + q_stats.suspicious_height + q_stats.invalid_block,
            permanent_bans: q_stats.genesis_mismatch,
        }
    }
}

/// Security statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct SecurityStats {
    pub total_quarantined: usize,
    pub minor_bans: usize,
    pub moderate_bans: usize,
    pub severe_bans: usize,
    pub permanent_bans: usize,
}

#[derive(Debug, thiserror::Error)]
pub enum SecurityError {
    #[error("Peer {0} is quarantined")]
    PeerQuarantined(IpAddr),

    #[error("Rate limit exceeded for peer {0}")]
    RateLimitExceeded(IpAddr),

    #[error("Invalid signature from peer {0}")]
    InvalidSignature(IpAddr),

    #[error("Replay attack detected from peer {0}")]
    ReplayAttack(IpAddr),

    #[error("Message processing error: {0}")]
    ProcessingError(String),

    #[error("Authentication error: {0}")]
    AuthError(#[from] AuthError),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiting() {
        let quarantine = Arc::new(PeerQuarantine::new());
        let handler = SecureMasternodeHandler::new(quarantine);
        let peer_ip: IpAddr = "127.0.0.1".parse().unwrap();

        // First 20 requests should succeed (within burst limit)
        for _ in 0..20 {
            assert!(handler.check_peer_allowed(peer_ip).await.is_ok());
        }

        // 21st request in quick succession should fail (exceeds burst)
        assert!(handler.check_peer_allowed(peer_ip).await.is_err());

        // Peer should be quarantined
        assert!(handler.quarantine.is_quarantined(&peer_ip).await);
    }

    #[tokio::test]
    async fn test_message_authentication() {
        let quarantine = Arc::new(PeerQuarantine::new());
        let handler = SecureMasternodeHandler::new(quarantine);
        let peer_ip: IpAddr = "127.0.0.1".parse().unwrap();

        let payload = b"test message".to_vec();
        let sender = "test_sender".to_string();
        let private_key = b"test_private_key";

        let msg = AuthenticatedMessage::new(payload.clone(), sender, private_key).unwrap();

        // Verify with correct key should succeed
        assert!(handler
            .verify_message(peer_ip, &msg, private_key)
            .await
            .is_ok());

        // Verify with wrong key should fail and quarantine
        let wrong_key = b"wrong_key";
        assert!(handler
            .verify_message(peer_ip, &msg, wrong_key)
            .await
            .is_err());
        assert!(handler.quarantine.is_quarantined(&peer_ip).await);
    }

    #[tokio::test]
    async fn test_replay_attack_prevention() {
        let quarantine = Arc::new(PeerQuarantine::new());
        let handler = SecureMasternodeHandler::new(quarantine);
        let peer_ip: IpAddr = "127.0.0.1".parse().unwrap();

        let payload = b"test".to_vec();
        let sender = "sender".to_string();
        let key = b"key";

        let msg = AuthenticatedMessage::new(payload, sender, key).unwrap();

        // First verification should succeed
        assert!(handler.verify_message(peer_ip, &msg, key).await.is_ok());

        // Second verification with same nonce should fail
        assert!(handler.verify_message(peer_ip, &msg, key).await.is_err());
        assert!(handler.quarantine.is_quarantined(&peer_ip).await);
    }
}
