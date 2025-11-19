//! Rate limiting for network connections to prevent DoS attacks

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::warn;

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Time window for rate limiting
    pub window: Duration,
    /// Maximum burst size
    pub burst_size: u32,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            max_requests: 100,
            window: Duration::from_secs(60),
            burst_size: 20,
        }
    }
}

/// Track request history for an IP
#[derive(Debug)]
struct RequestHistory {
    /// Request timestamps within the current window
    requests: Vec<Instant>,
    /// Last cleanup time
    last_cleanup: Instant,
}

impl RequestHistory {
    fn new() -> Self {
        Self {
            requests: Vec::new(),
            last_cleanup: Instant::now(),
        }
    }

    /// Clean up old requests outside the window
    fn cleanup(&mut self, window: Duration) {
        let cutoff = Instant::now() - window;
        self.requests.retain(|&time| time > cutoff);
        self.last_cleanup = Instant::now();
    }

    /// Check if rate limit is exceeded
    fn is_rate_limited(&self, config: &RateLimiterConfig) -> bool {
        self.requests.len() >= config.max_requests as usize
    }

    /// Check if burst limit is exceeded
    fn is_burst_limited(&self, config: &RateLimiterConfig) -> bool {
        let recent = Instant::now() - Duration::from_secs(1);
        let recent_count = self.requests.iter().filter(|&&t| t > recent).count();
        recent_count >= config.burst_size as usize
    }

    /// Add a request timestamp
    fn add_request(&mut self) {
        self.requests.push(Instant::now());
    }
}

/// Rate limiter for network requests
pub struct RateLimiter {
    config: RateLimiterConfig,
    history: Arc<RwLock<HashMap<IpAddr, RequestHistory>>>,
}

impl RateLimiter {
    /// Create a new rate limiter with default config
    pub fn new() -> Self {
        Self::with_config(RateLimiterConfig::default())
    }

    /// Create a new rate limiter with custom config
    pub fn with_config(config: RateLimiterConfig) -> Self {
        Self {
            config,
            history: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Check if a request from an IP should be allowed
    pub async fn check_rate_limit(&self, ip: IpAddr) -> Result<(), RateLimitError> {
        let mut history = self.history.write().await;
        let entry = history.entry(ip).or_insert_with(RequestHistory::new);

        // Periodic cleanup
        if entry.last_cleanup.elapsed() > Duration::from_secs(60) {
            entry.cleanup(self.config.window);
        }

        // Check burst limit
        if entry.is_burst_limited(&self.config) {
            warn!("Burst limit exceeded for IP: {}", ip);
            return Err(RateLimitError::BurstLimitExceeded {
                ip,
                limit: self.config.burst_size,
            });
        }

        // Check rate limit
        if entry.is_rate_limited(&self.config) {
            warn!("Rate limit exceeded for IP: {}", ip);
            return Err(RateLimitError::RateLimitExceeded {
                ip,
                limit: self.config.max_requests,
                window: self.config.window,
            });
        }

        // Allow request and record it
        entry.add_request();
        Ok(())
    }

    /// Reset rate limit for an IP (used when clearing bans)
    pub async fn reset(&self, ip: IpAddr) {
        let mut history = self.history.write().await;
        history.remove(&ip);
    }

    /// Get current request count for an IP
    pub async fn get_request_count(&self, ip: IpAddr) -> u32 {
        let history = self.history.read().await;
        history
            .get(&ip)
            .map(|h| h.requests.len() as u32)
            .unwrap_or(0)
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("Rate limit exceeded for {ip}: {limit} requests per {window:?}")]
    RateLimitExceeded {
        ip: IpAddr,
        limit: u32,
        window: Duration,
    },

    #[error("Burst limit exceeded for {ip}: {limit} requests per second")]
    BurstLimitExceeded { ip: IpAddr, limit: u32 },
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_rate_limit_basic() {
        let config = RateLimiterConfig {
            max_requests: 5,
            window: Duration::from_secs(60),
            burst_size: 3,
        };
        let limiter = RateLimiter::with_config(config);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // First 5 requests should succeed
        for _ in 0..5 {
            assert!(limiter.check_rate_limit(ip).await.is_ok());
        }

        // 6th request should fail
        assert!(limiter.check_rate_limit(ip).await.is_err());
    }

    #[tokio::test]
    async fn test_burst_limit() {
        let config = RateLimiterConfig {
            max_requests: 100,
            window: Duration::from_secs(60),
            burst_size: 2,
        };
        let limiter = RateLimiter::with_config(config);
        let ip = IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1));

        // First 2 requests should succeed
        assert!(limiter.check_rate_limit(ip).await.is_ok());
        assert!(limiter.check_rate_limit(ip).await.is_ok());

        // 3rd rapid request should fail burst limit
        assert!(limiter.check_rate_limit(ip).await.is_err());
    }
}
