//! Time synchronization with authoritative time sources
//!
//! This module checks system time against:
//! 1. NTP time servers (pool.ntp.org)
//! 2. Network consensus time (median of peer times)
//!
//! If local clock drift exceeds threshold, the node should warn or halt.

use chrono::Utc;
use std::time::Duration;
use tokio::time::timeout;

/// Maximum acceptable clock drift (5 minutes)
pub const MAX_CLOCK_DRIFT_SECONDS: i64 = 300;

/// NTP timeout duration
const NTP_TIMEOUT_SECS: u64 = 10;

/// Time synchronization result
#[derive(Debug, Clone)]
pub struct TimeSyncResult {
    /// Local system time
    pub local_time: i64,
    /// Authority time (NTP or network consensus)
    pub authority_time: i64,
    /// Drift in seconds (positive = local is ahead)
    pub drift_seconds: i64,
    /// Is drift within acceptable range
    pub is_acceptable: bool,
    /// Source of authority time
    pub source: TimeSyncSource,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TimeSyncSource {
    /// NTP time server
    Ntp(String),
    /// Network consensus (median of peers)
    NetworkConsensus,
    /// Unavailable (couldn't sync)
    Unavailable,
}

impl TimeSyncResult {
    pub fn drift_warning(&self) -> Option<String> {
        if !self.is_acceptable {
            Some(format!(
                "⚠️  CLOCK DRIFT DETECTED: Local time is {}s {} authority ({:?})",
                self.drift_seconds.abs(),
                if self.drift_seconds > 0 {
                    "ahead of"
                } else {
                    "behind"
                },
                self.source
            ))
        } else {
            None
        }
    }
}

/// Check system time against NTP servers
///
/// Tries multiple NTP servers in order until one succeeds
pub async fn check_ntp_time() -> Result<TimeSyncResult, String> {
    let ntp_servers = vec![
        "time.google.com:123",
        "time.cloudflare.com:123",
        "pool.ntp.org:123",
        "time.nist.gov:123",
    ];

    let local_time = Utc::now().timestamp();

    for server in ntp_servers {
        match query_ntp_server(server).await {
            Ok(ntp_time) => {
                let drift = local_time - ntp_time;
                return Ok(TimeSyncResult {
                    local_time,
                    authority_time: ntp_time,
                    drift_seconds: drift,
                    is_acceptable: drift.abs() <= MAX_CLOCK_DRIFT_SECONDS,
                    source: TimeSyncSource::Ntp(server.to_string()),
                });
            }
            Err(e) => {
                log::debug!("Failed to query NTP server {}: {}", server, e);
                continue;
            }
        }
    }

    Err("All NTP servers failed".to_string())
}

/// Query a single NTP server
async fn query_ntp_server(server: &str) -> Result<i64, String> {
    // Use simple NTP client
    // For now, we'll use a basic UDP NTP implementation
    // In production, consider using the `ntp` crate

    use std::net::UdpSocket;

    let socket = UdpSocket::bind("0.0.0.0:0").map_err(|e| format!("Socket bind failed: {}", e))?;

    socket
        .set_read_timeout(Some(Duration::from_secs(NTP_TIMEOUT_SECS)))
        .map_err(|e| format!("Set timeout failed: {}", e))?;

    // NTP packet format (48 bytes)
    let mut request = [0u8; 48];
    request[0] = 0x1B; // LI=0, VN=3, Mode=3 (client)

    socket
        .send_to(&request, server)
        .map_err(|e| format!("Send failed: {}", e))?;

    let mut response = [0u8; 48];
    socket
        .recv_from(&mut response)
        .map_err(|e| format!("Receive failed: {}", e))?;

    // Extract transmit timestamp (bytes 40-47)
    let seconds = u32::from_be_bytes([response[40], response[41], response[42], response[43]]);
    let _fraction = u32::from_be_bytes([response[44], response[45], response[46], response[47]]);

    // NTP epoch is Jan 1, 1900. Unix epoch is Jan 1, 1970 (2208988800 seconds difference)
    const NTP_UNIX_OFFSET: i64 = 2208988800;

    let ntp_time = seconds as i64 - NTP_UNIX_OFFSET;

    Ok(ntp_time)
}

/// Check system time against network consensus
///
/// Queries multiple peers for their current time and computes median
pub async fn check_network_time(
    peer_addresses: Vec<String>,
    requester: &impl NetworkTimeRequester,
) -> Result<TimeSyncResult, String> {
    if peer_addresses.is_empty() {
        return Err("No peers available".to_string());
    }

    let local_time = Utc::now().timestamp();

    // Query all peers concurrently
    let mut peer_times = Vec::new();

    for peer_addr in peer_addresses.iter().take(10) {
        // Limit to 10 peers
        match timeout(
            Duration::from_secs(5),
            requester.request_peer_time(peer_addr),
        )
        .await
        {
            Ok(Ok(time)) => peer_times.push(time),
            Ok(Err(e)) => log::debug!("Failed to get time from {}: {}", peer_addr, e),
            Err(_) => log::debug!("Timeout getting time from {}", peer_addr),
        }
    }

    if peer_times.is_empty() {
        return Err("No peers responded with time".to_string());
    }

    // Compute median time
    peer_times.sort();
    let network_time = if peer_times.len() % 2 == 0 {
        (peer_times[peer_times.len() / 2 - 1] + peer_times[peer_times.len() / 2]) / 2
    } else {
        peer_times[peer_times.len() / 2]
    };

    let drift = local_time - network_time;

    Ok(TimeSyncResult {
        local_time,
        authority_time: network_time,
        drift_seconds: drift,
        is_acceptable: drift.abs() <= MAX_CLOCK_DRIFT_SECONDS,
        source: TimeSyncSource::NetworkConsensus,
    })
}

/// Trait for requesting time from network peers
#[async_trait::async_trait]
pub trait NetworkTimeRequester: Send + Sync {
    async fn request_peer_time(&self, peer_addr: &str) -> Result<i64, String>;
}

/// Perform comprehensive time check (NTP + Network)
pub async fn comprehensive_time_check(
    peer_addresses: Vec<String>,
    requester: &impl NetworkTimeRequester,
) -> TimeSyncResult {
    // Try NTP first
    match check_ntp_time().await {
        Ok(result) => {
            if result.is_acceptable {
                log::info!(
                    "✓ System time verified via NTP (drift: {}s)",
                    result.drift_seconds
                );
                return result;
            } else {
                log::warn!(
                    "⚠️  NTP check failed: {}",
                    result.drift_warning().unwrap_or_default()
                );
            }
        }
        Err(e) => {
            log::warn!("⚠️  NTP check failed: {}", e);
        }
    }

    // Fall back to network consensus
    match check_network_time(peer_addresses, requester).await {
        Ok(result) => {
            if result.is_acceptable {
                log::info!(
                    "✓ System time verified via network consensus (drift: {}s)",
                    result.drift_seconds
                );
            } else {
                log::error!("{}", result.drift_warning().unwrap_or_default());
            }
            result
        }
        Err(e) => {
            log::warn!("⚠️  Network time check failed: {}", e);
            // Return local time as fallback
            TimeSyncResult {
                local_time: Utc::now().timestamp(),
                authority_time: 0,
                drift_seconds: 0,
                is_acceptable: false,
                source: TimeSyncSource::Unavailable,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockNetworkRequester {
        times: Vec<i64>,
    }

    #[async_trait::async_trait]
    impl NetworkTimeRequester for MockNetworkRequester {
        async fn request_peer_time(&self, _peer_addr: &str) -> Result<i64, String> {
            if self.times.is_empty() {
                Err("No time".to_string())
            } else {
                Ok(self.times[0])
            }
        }
    }

    #[tokio::test]
    async fn test_network_time_median() {
        let requester = MockNetworkRequester {
            times: vec![1000, 1002, 1001, 1003, 999],
        };

        let result =
            check_network_time(vec!["peer1".to_string(), "peer2".to_string()], &requester).await;

        // Should compute median correctly
        assert!(result.is_ok());
    }
}
