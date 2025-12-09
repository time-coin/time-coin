//! Grant Security Module
//!
//! This module provides security features for the grant system:
//! - Cryptographic token generation (not predictable UUIDs)
//! - Rate limiting to prevent spam
//! - Email verification enforcement

use crate::{ApiError, ApiResult};
use chrono::Utc;
use rand::Rng;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Maximum grant applications per email per hour
const MAX_APPLICATIONS_PER_HOUR: usize = 5;

/// Maximum grant applications per IP address (future enhancement)
#[allow(dead_code)]
const MAX_APPLICATIONS_PER_IP: usize = 10;

/// Generate a cryptographically secure verification token
///
/// Returns a 64-character hexadecimal string (32 random bytes)
/// This is much more secure than UUIDs which can be predictable
pub fn generate_secure_token() -> String {
    let mut rng = rand::rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.random()).collect();
    hex::encode(bytes)
}

/// Rate limiter for grant applications
///
/// Tracks application timestamps per email address and enforces limits
#[derive(Clone)]
pub struct GrantRateLimiter {
    /// Map of email -> list of application timestamps
    applications: Arc<RwLock<HashMap<String, Vec<i64>>>>,
}

impl GrantRateLimiter {
    /// Create a new rate limiter
    pub fn new() -> Self {
        Self {
            applications: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if an email has exceeded the rate limit
    ///
    /// # Arguments
    /// * `email` - Email address to check
    ///
    /// # Returns
    /// * `Ok(())` if within limits
    /// * `Err(ApiError)` if rate limit exceeded
    pub async fn check_rate_limit(&self, email: &str) -> ApiResult<()> {
        let mut apps = self.applications.write().await;
        let now = Utc::now().timestamp();
        let hour_ago = now - 3600;

        let recent = apps.entry(email.to_string()).or_insert_with(Vec::new);

        // Clean old entries (older than 1 hour)
        recent.retain(|&ts| ts > hour_ago);

        if recent.len() >= MAX_APPLICATIONS_PER_HOUR {
            tracing::warn!(
                email = %email,
                attempts = recent.len(),
                "grant_application_rate_limit_exceeded"
            );
            return Err(ApiError::BadRequest(format!(
                "Rate limit exceeded. Maximum {} applications per hour. Try again later.",
                MAX_APPLICATIONS_PER_HOUR
            )));
        }

        recent.push(now);

        tracing::info!(
            email = %email,
            recent_applications = recent.len(),
            "grant_application_rate_limit_checked"
        );

        Ok(())
    }

    /// Clean up old rate limit entries (for maintenance)
    ///
    /// Removes all entries older than 1 hour
    pub async fn cleanup(&self) {
        let mut apps = self.applications.write().await;
        let now = Utc::now().timestamp();
        let hour_ago = now - 3600;

        let original_count = apps.len();

        // Remove entries with no recent applications
        apps.retain(|_, timestamps| {
            timestamps.retain(|&ts| ts > hour_ago);
            !timestamps.is_empty()
        });

        let cleaned_count = original_count - apps.len();
        if cleaned_count > 0 {
            tracing::info!(
                cleaned_entries = cleaned_count,
                remaining_entries = apps.len(),
                "grant_rate_limiter_cleanup"
            );
        }
    }

    /// Get current application count for an email (for testing/debugging)
    #[cfg(test)]
    pub async fn get_count(&self, email: &str) -> usize {
        let apps = self.applications.read().await;
        apps.get(email).map(|v| v.len()).unwrap_or(0)
    }
}

impl Default for GrantRateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_secure_token() {
        let token1 = generate_secure_token();
        let token2 = generate_secure_token();

        // Should be 64 characters (32 bytes hex encoded)
        assert_eq!(token1.len(), 64);
        assert_eq!(token2.len(), 64);

        // Should be different (not predictable)
        assert_ne!(token1, token2);

        // Should be valid hex
        assert!(token1.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[tokio::test]
    async fn test_rate_limiter_allows_within_limit() {
        let limiter = GrantRateLimiter::new();
        let email = "test@example.com";

        // Should allow up to MAX_APPLICATIONS_PER_HOUR
        for i in 0..MAX_APPLICATIONS_PER_HOUR {
            let result = limiter.check_rate_limit(email).await;
            assert!(
                result.is_ok(),
                "Should allow application {} of {}",
                i + 1,
                MAX_APPLICATIONS_PER_HOUR
            );
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_over_limit() {
        let limiter = GrantRateLimiter::new();
        let email = "test@example.com";

        // Fill up to limit
        for _ in 0..MAX_APPLICATIONS_PER_HOUR {
            limiter.check_rate_limit(email).await.unwrap();
        }

        // Next one should fail
        let result = limiter.check_rate_limit(email).await;
        assert!(result.is_err(), "Should block over-limit application");

        if let Err(ApiError::BadRequest(msg)) = result {
            assert!(msg.contains("Rate limit exceeded"));
        } else {
            panic!("Expected BadRequest error");
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_different_emails() {
        let limiter = GrantRateLimiter::new();

        // Different emails should have independent limits
        for _ in 0..MAX_APPLICATIONS_PER_HOUR {
            limiter.check_rate_limit("user1@example.com").await.unwrap();
            limiter.check_rate_limit("user2@example.com").await.unwrap();
        }

        // Both should now be at limit
        assert!(limiter.check_rate_limit("user1@example.com").await.is_err());
        assert!(limiter.check_rate_limit("user2@example.com").await.is_err());
    }

    #[tokio::test]
    async fn test_rate_limiter_cleanup() {
        let limiter = GrantRateLimiter::new();
        let email = "test@example.com";

        // Add some applications
        for _ in 0..3 {
            limiter.check_rate_limit(email).await.unwrap();
        }

        assert_eq!(limiter.get_count(email).await, 3);

        // Cleanup should not remove recent entries
        limiter.cleanup().await;
        assert_eq!(limiter.get_count(email).await, 3);
    }
}
