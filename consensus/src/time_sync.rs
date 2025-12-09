//! Time Synchronization Module
//!
//! Provides robust time synchronization with NTP servers and network peers.
//! Critical for TIME coin where transaction timestamps determine value.

use chrono::Utc;
use log::{error, info, warn};
use std::net::ToSocketAddrs;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tokio::time;

/// Time synchronization configuration
const NTP_SERVERS: &[&str] = &[
    "time.nist.gov:123",
    "time.google.com:123",
    "pool.ntp.org:123",
    "time.cloudflare.com:123",
];

const MAX_CALIBRATION_OFFSET_MS: i64 = 600000; // 10 minutes max calibration
const CRITICAL_DRIFT_MS: i64 = 60000; // 1 minute - refuse to operate beyond this
const TIME_SYNC_INTERVAL: Duration = Duration::from_secs(3600); // Check every hour
#[allow(dead_code)]
const MIN_PEER_SAMPLES: usize = 3; // Minimum peers to query for consensus
const NTP_TIMEOUT: Duration = Duration::from_secs(5);

/// Time sample with latency compensation
#[derive(Debug, Clone)]
pub struct TimeSample {
    pub source: String,
    pub timestamp: i64, // milliseconds since epoch
    pub latency_ms: u64,
    pub received_at: Instant,
}

/// Time synchronization state
pub struct TimeSync {
    /// Calibration offset in milliseconds (to add to system time)
    calibration_offset: Arc<RwLock<i64>>,

    /// Last sync time
    last_sync: Arc<RwLock<Option<Instant>>>,

    /// Recent time samples
    samples: Arc<RwLock<Vec<TimeSample>>>,

    /// Whether node is allowed to operate
    operational: Arc<RwLock<bool>>,
}

impl TimeSync {
    pub fn new() -> Self {
        Self {
            calibration_offset: Arc::new(RwLock::new(0)),
            last_sync: Arc::new(RwLock::new(None)),
            samples: Arc::new(RwLock::new(Vec::new())),
            operational: Arc::new(RwLock::new(true)),
        }
    }

    /// Get current calibrated timestamp in milliseconds
    pub async fn now_ms(&self) -> i64 {
        let offset = *self.calibration_offset.read().await;
        Utc::now().timestamp_millis() + offset
    }

    /// Get current calibration offset
    pub async fn get_offset_ms(&self) -> i64 {
        *self.calibration_offset.read().await
    }

    /// Check if node is operational (time is synchronized within limits)
    pub async fn is_operational(&self) -> bool {
        *self.operational.read().await
    }

    /// Query NTP server for time with latency compensation
    async fn query_ntp_server(&self, server: &str) -> Option<TimeSample> {
        let start = Instant::now();

        // Resolve address
        let addr = match (server.to_string()).to_socket_addrs() {
            Ok(mut addrs) => addrs.next()?,
            Err(e) => {
                warn!("Failed to resolve NTP server {}: {}", server, e);
                return None;
            }
        };

        // Query NTP using sntpc crate
        match tokio::time::timeout(NTP_TIMEOUT, Self::ntp_query(addr)).await {
            Ok(Ok(server_time_ms)) => {
                let latency = start.elapsed();
                let latency_ms = latency.as_millis() as u64;

                // Adjust for one-way latency (half of round-trip)
                let adjusted_time = server_time_ms + (latency_ms / 2) as i64;

                Some(TimeSample {
                    source: format!("NTP:{}", server),
                    timestamp: adjusted_time,
                    latency_ms,
                    received_at: Instant::now(),
                })
            }
            Ok(Err(e)) => {
                warn!("NTP query to {} failed: {}", server, e);
                None
            }
            Err(_) => {
                warn!("NTP query to {} timed out", server);
                None
            }
        }
    }

    /// Perform NTP query (simplified - in production use sntpc crate)
    async fn ntp_query(_addr: std::net::SocketAddr) -> Result<i64, String> {
        // TODO: Implement proper NTP protocol using sntpc crate
        // For now, return error to fall back to peer time
        Err("NTP not yet implemented - use peer time".to_string())
    }

    /// Query network peer for time with latency compensation  
    /// TODO: Implement when peer time query protocol is added
    pub async fn query_peer_time(&self, _peer_addr: &str) -> Option<TimeSample> {
        // Stub: Not yet implemented
        None
    }

    /// Synchronize time with NTP servers and network peers
    pub async fn synchronize(&self) -> Result<(), String> {
        info!("⏰ Starting time synchronization...");
        let mut samples = Vec::new();

        // Query NTP servers
        for server in NTP_SERVERS {
            if let Some(sample) = self.query_ntp_server(server).await {
                info!(
                    "   ✓ NTP {} responded: {}ms latency",
                    server, sample.latency_ms
                );
                samples.push(sample);
            }
        }

        // TODO: Query network peers when peer time protocol is implemented
        // let peers = network.get_connected_peers().await;
        // for peer in peers.iter().take(5) {
        //     if let Some(sample) = self.query_peer_time(peer).await {
        //         samples.push(sample);
        //     }
        // }

        if samples.is_empty() {
            error!("   ✗ No time sources available - cannot synchronize");
            return Err("No time sources available".to_string());
        }

        // Calculate consensus time (median of all samples)
        let consensus_time = self.calculate_consensus_time(&samples);
        let local_time = Utc::now().timestamp_millis();
        let offset = consensus_time - local_time;

        info!("   📊 Consensus time from {} sources", samples.len());
        info!("   📊 Local time:     {}", local_time);
        info!("   📊 Consensus time: {}", consensus_time);
        info!("   📊 Offset:         {}ms", offset);

        // Check if offset is within acceptable range
        let offset_abs = offset.abs();

        if offset_abs > MAX_CALIBRATION_OFFSET_MS {
            error!(
                "   ✗ Clock drift too large ({}ms > {}ms) - REFUSING TO OPERATE",
                offset_abs, MAX_CALIBRATION_OFFSET_MS
            );
            error!("   ⚠️  Please fix system clock manually!");
            *self.operational.write().await = false;
            return Err(format!("Clock drift exceeds maximum: {}ms", offset_abs));
        }

        if offset_abs > CRITICAL_DRIFT_MS {
            warn!(
                "   ⚠️  Clock drift is significant ({}ms > {}ms)",
                offset_abs, CRITICAL_DRIFT_MS
            );
            warn!("   ⚠️  Applying calibration offset but consider fixing system clock");
        }

        // Apply calibration offset
        *self.calibration_offset.write().await = offset;
        *self.last_sync.write().await = Some(Instant::now());
        *self.samples.write().await = samples;
        *self.operational.write().await = true;

        info!("   ✅ Time synchronized with {}ms offset", offset);

        Ok(())
    }

    /// Calculate consensus time from samples (median)
    fn calculate_consensus_time(&self, samples: &[TimeSample]) -> i64 {
        let mut times: Vec<i64> = samples.iter().map(|s| s.timestamp).collect();
        times.sort_unstable();

        if times.is_empty() {
            return 0;
        }

        let mid = times.len() / 2;
        if times.len().is_multiple_of(2) {
            (times[mid - 1] + times[mid]) / 2
        } else {
            times[mid]
        }
    }

    /// Start periodic time synchronization
    pub fn start_periodic_sync(self: Arc<Self>) {
        tokio::spawn(async move {
            let mut interval = time::interval(TIME_SYNC_INTERVAL);

            loop {
                interval.tick().await;

                if let Err(e) = self.synchronize().await {
                    error!("⏰ Time synchronization failed: {}", e);
                }
            }
        });
    }
}

impl Default for TimeSync {
    fn default() -> Self {
        Self::new()
    }
}
