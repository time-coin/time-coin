use crate::monitoring::NetworkMonitor;
use std::sync::Arc;

/// Debug information provider
pub struct DebugEndpoints {
    monitor: Arc<NetworkMonitor>,
}

impl DebugEndpoints {
    pub fn new(monitor: Arc<NetworkMonitor>) -> Self {
        Self { monitor }
    }

    /// Get network status summary
    pub async fn get_network_status(&self) -> String {
        let metrics = self.monitor.get_network_metrics().await;

        format!(
            "Network Status:\n\
             - Peers: {} connected ({} active)\n\
             - Sync: {:.2}% complete\n\
             - Height: {} / {}\n\
             - Avg Latency: {} ms\n\
             - Uptime: {} seconds",
            metrics.peer_count,
            metrics.active_connections,
            metrics.sync_progress,
            metrics.current_height,
            metrics.network_height,
            metrics.avg_latency_ms,
            metrics.uptime_secs
        )
    }

    /// Get detailed peer information
    pub async fn get_peer_info(&self) -> String {
        let peers = self.monitor.get_all_peer_metrics().await;

        if peers.is_empty() {
            return "No peers connected".to_string();
        }

        let mut output = String::from("Connected Peers:\n\n");

        for peer in peers {
            let status = if peer.success_rate > 90.0 {
                "✓ Healthy"
            } else if peer.success_rate > 70.0 {
                "⚠ Warning"
            } else {
                "✗ Poor"
            };

            output.push_str(&format!(
                "{} [{}]\n\
                 - Latency: {} ms\n\
                 - Success Rate: {:.1}%\n\
                 - Sent: {} bytes\n\
                 - Received: {} bytes\n\
                 - Failures: {}\n\n",
                peer.peer_id,
                status,
                peer.latency_ms,
                peer.success_rate,
                peer.bytes_sent,
                peer.bytes_received,
                peer.failures
            ));
        }

        output
    }

    /// Get sync progress details
    pub async fn get_sync_status(&self) -> String {
        let metrics = self.monitor.get_network_metrics().await;

        let status = if metrics.sync_progress >= 99.9 {
            "✓ Synced"
        } else if metrics.sync_progress > 0.0 {
            "⏳ Syncing"
        } else {
            "✗ Not Started"
        };

        let blocks_behind = metrics
            .network_height
            .saturating_sub(metrics.current_height);

        format!(
            "Blockchain Sync Status: {}\n\
             - Current Height: {}\n\
             - Network Height: {}\n\
             - Blocks Behind: {}\n\
             - Progress: {:.2}%",
            status,
            metrics.current_height,
            metrics.network_height,
            blocks_behind,
            metrics.sync_progress
        )
    }

    /// Get full debug report
    pub async fn get_full_report(&self) -> String {
        self.monitor.generate_debug_report().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitoring::NetworkMonitor;

    #[tokio::test]
    async fn test_network_status() {
        let monitor = Arc::new(NetworkMonitor::new());
        let endpoints = DebugEndpoints::new(monitor.clone());

        monitor.update_network_metrics(3, 2, 50, 100).await;

        let status = endpoints.get_network_status().await;
        assert!(status.contains("3 connected"));
        assert!(status.contains("50.00%"));
    }

    #[tokio::test]
    async fn test_sync_status() {
        let monitor = Arc::new(NetworkMonitor::new());
        let endpoints = DebugEndpoints::new(monitor.clone());

        monitor.update_network_metrics(3, 2, 100, 100).await;

        let status = endpoints.get_sync_status().await;
        assert!(status.contains("Synced"));
    }
}
