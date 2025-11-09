//! Monitoring and Logging for Phased Consensus
//!
//! Provides comprehensive logging, metrics, and monitoring hooks
//! for all phases of the consensus protocol

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::phased_protocol::Phase;

/// Event type for monitoring
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusEvent {
    /// Protocol started
    ProtocolStarted { block_height: u64 },

    /// Phase transition
    PhaseTransition { from: Phase, to: Phase },

    /// Heartbeat received
    HeartbeatReceived { node_id: String },

    /// Leader elected
    LeaderElected { leader: String, weight: u64 },

    /// Vote received
    VoteReceived {
        voter: String,
        approve: bool,
        weight: u64,
    },

    /// Consensus reached
    ConsensusReached {
        approval_weight: u64,
        total_weight: u64,
    },

    /// Consensus failed
    ConsensusFailed {
        approval_weight: u64,
        total_weight: u64,
    },

    /// Fallback initiated
    FallbackInitiated { reason: String, attempt: u32 },

    /// Emergency block created
    EmergencyBlock { reason: String },

    /// Block finalized
    BlockFinalized { block_height: u64, hash: String },

    /// Protocol completed
    ProtocolCompleted { duration_ms: u64, success: bool },
}

/// Metrics for consensus performance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConsensusMetrics {
    /// Block height
    pub block_height: u64,

    /// Total protocol duration (ms)
    pub total_duration_ms: u64,

    /// Phase durations (ms)
    pub phase_durations: HashMap<String, u64>,

    /// Number of heartbeats received
    pub heartbeat_count: usize,

    /// Number of votes received
    pub vote_count: usize,

    /// Consensus approval percentage
    pub approval_percentage: f32,

    /// Number of fallback attempts
    pub fallback_attempts: usize,

    /// Whether emergency mode was used
    pub emergency_mode: bool,

    /// Final result
    pub success: bool,
}

/// Event record with timestamp
#[derive(Debug, Clone)]
struct EventRecord {
    event: ConsensusEvent,
    timestamp: DateTime<Utc>,
}

/// Public representation of an event with its timestamp
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventWithTimestamp {
    /// The consensus event
    pub event: ConsensusEvent,
    /// When the event occurred
    pub timestamp: DateTime<Utc>,
}

/// Monitoring manager
pub struct ConsensusMonitor {
    /// Event history
    events: Arc<RwLock<Vec<EventRecord>>>,

    /// Current metrics
    metrics: Arc<RwLock<Option<ConsensusMetrics>>>,

    /// Protocol start time
    start_time: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl Default for ConsensusMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsensusMonitor {
    pub fn new() -> Self {
        Self {
            events: Arc::new(RwLock::new(Vec::new())),
            metrics: Arc::new(RwLock::new(None)),
            start_time: Arc::new(RwLock::new(None)),
        }
    }

    /// Start monitoring a new consensus round
    pub async fn start_round(&self, block_height: u64) {
        let mut start = self.start_time.write().await;
        *start = Some(Utc::now());

        self.record_event(ConsensusEvent::ProtocolStarted { block_height })
            .await;

        // Initialize metrics
        let mut metrics = self.metrics.write().await;
        *metrics = Some(ConsensusMetrics {
            block_height,
            total_duration_ms: 0,
            phase_durations: HashMap::new(),
            heartbeat_count: 0,
            vote_count: 0,
            approval_percentage: 0.0,
            fallback_attempts: 0,
            emergency_mode: false,
            success: false,
        });
    }

    /// Record an event
    pub async fn record_event(&self, event: ConsensusEvent) {
        let mut events = self.events.write().await;
        events.push(EventRecord {
            event: event.clone(),
            timestamp: Utc::now(),
        });

        // Update metrics based on event
        self.update_metrics_for_event(&event).await;

        // Log event
        self.log_event(&event);
    }

    /// Update metrics based on event
    async fn update_metrics_for_event(&self, event: &ConsensusEvent) {
        let mut metrics = self.metrics.write().await;
        if let Some(ref mut m) = *metrics {
            match event {
                ConsensusEvent::HeartbeatReceived { .. } => {
                    m.heartbeat_count += 1;
                }
                ConsensusEvent::VoteReceived { .. } => {
                    m.vote_count += 1;
                }
                ConsensusEvent::ConsensusReached {
                    approval_weight,
                    total_weight,
                } => {
                    m.approval_percentage =
                        (*approval_weight as f32 / *total_weight as f32) * 100.0;
                }
                ConsensusEvent::FallbackInitiated { attempt, .. } => {
                    m.fallback_attempts = *attempt as usize;
                }
                ConsensusEvent::EmergencyBlock { .. } => {
                    m.emergency_mode = true;
                }
                ConsensusEvent::BlockFinalized { .. } => {
                    m.success = true;
                }
                _ => {}
            }
        }
    }

    /// Log an event with formatted output
    fn log_event(&self, event: &ConsensusEvent) {
        match event {
            ConsensusEvent::ProtocolStarted { block_height } => {
                println!("🚀 Consensus protocol started for block #{}", block_height);
            }
            ConsensusEvent::PhaseTransition { from, to } => {
                println!("➡️  Phase transition: {:?} → {:?}", from, to);
            }
            ConsensusEvent::HeartbeatReceived { node_id } => {
                println!("💓 Heartbeat received from {}", node_id);
            }
            ConsensusEvent::LeaderElected { leader, weight } => {
                println!("👑 Leader elected: {} (weight: {})", leader, weight);
            }
            ConsensusEvent::VoteReceived {
                voter,
                approve,
                weight,
            } => {
                let status = if *approve {
                    "✅ APPROVE"
                } else {
                    "❌ REJECT"
                };
                println!("🗳️  Vote from {} - {} (weight: {})", voter, status, weight);
            }
            ConsensusEvent::ConsensusReached {
                approval_weight,
                total_weight,
            } => {
                let percentage = (*approval_weight as f32 / *total_weight as f32) * 100.0;
                println!(
                    "✅ Consensus reached! {}/{} ({:.1}%)",
                    approval_weight, total_weight, percentage
                );
            }
            ConsensusEvent::ConsensusFailed {
                approval_weight,
                total_weight,
            } => {
                let percentage = (*approval_weight as f32 / *total_weight as f32) * 100.0;
                println!(
                    "❌ Consensus failed: {}/{} ({:.1}%)",
                    approval_weight, total_weight, percentage
                );
            }
            ConsensusEvent::FallbackInitiated { reason, attempt } => {
                println!("🔄 Fallback #{} initiated: {}", attempt, reason);
            }
            ConsensusEvent::EmergencyBlock { reason } => {
                println!("🚨 EMERGENCY BLOCK: {}", reason);
            }
            ConsensusEvent::BlockFinalized { block_height, hash } => {
                println!(
                    "✨ Block #{} finalized: {}...",
                    block_height,
                    &hash[..16.min(hash.len())]
                );
            }
            ConsensusEvent::ProtocolCompleted {
                duration_ms,
                success,
            } => {
                let status = if *success {
                    "✅ SUCCESS"
                } else {
                    "❌ FAILED"
                };
                println!(
                    "🏁 Protocol completed: {} ({:.2}s)",
                    status,
                    *duration_ms as f64 / 1000.0
                );
            }
        }
    }

    /// Record phase duration
    pub async fn record_phase_duration(&self, phase: Phase, duration_ms: u64) {
        let mut metrics = self.metrics.write().await;
        if let Some(ref mut m) = *metrics {
            m.phase_durations
                .insert(format!("{:?}", phase), duration_ms);
        }
    }

    /// Complete the monitoring round and generate final metrics
    pub async fn complete_round(&self, success: bool) -> Option<ConsensusMetrics> {
        let start = self.start_time.read().await;
        let duration_ms = start.map(|s| (Utc::now() - s).num_milliseconds() as u64);

        if let Some(duration) = duration_ms {
            self.record_event(ConsensusEvent::ProtocolCompleted {
                duration_ms: duration,
                success,
            })
            .await;

            let mut metrics = self.metrics.write().await;
            if let Some(ref mut m) = *metrics {
                m.total_duration_ms = duration;
                m.success = success;
            }

            metrics.clone()
        } else {
            None
        }
    }

    /// Get current metrics
    pub async fn get_metrics(&self) -> Option<ConsensusMetrics> {
        self.metrics.read().await.clone()
    }

    /// Get event history
    pub async fn get_events(&self) -> Vec<ConsensusEvent> {
        self.events
            .read()
            .await
            .iter()
            .map(|r| r.event.clone())
            .collect()
    }

    /// Get event history with timestamps
    ///
    /// Returns all events along with their timestamps for detailed timing analysis
    pub async fn get_events_with_timestamps(&self) -> Vec<EventWithTimestamp> {
        self.events
            .read()
            .await
            .iter()
            .map(|r| EventWithTimestamp {
                event: r.event.clone(),
                timestamp: r.timestamp,
            })
            .collect()
    }

    /// Get events within a specific time range
    ///
    /// Useful for debugging timing issues and analyzing event sequences
    ///
    /// # Arguments
    /// * `start` - Start of the time range (inclusive)
    /// * `end` - End of the time range (inclusive)
    pub async fn get_events_in_time_range(
        &self,
        start: DateTime<Utc>,
        end: DateTime<Utc>,
    ) -> Vec<EventWithTimestamp> {
        self.events
            .read()
            .await
            .iter()
            .filter(|r| r.timestamp >= start && r.timestamp <= end)
            .map(|r| EventWithTimestamp {
                event: r.event.clone(),
                timestamp: r.timestamp,
            })
            .collect()
    }

    /// Print summary report
    ///
    /// Optionally displays event timing information for detailed analysis
    ///
    /// # Arguments
    /// * `show_event_timeline` - If true, shows all events with their timestamps
    pub async fn print_summary(&self, show_event_timeline: bool) {
        let metrics = self.metrics.read().await;
        if let Some(ref m) = *metrics {
            println!();
            println!("╔════════════════════════════════════════════════════════════╗");
            println!("║            CONSENSUS METRICS SUMMARY                       ║");
            println!("╚════════════════════════════════════════════════════════════╝");
            println!();
            println!("Block Height:        #{}", m.block_height);
            println!(
                "Total Duration:      {:.2}s",
                m.total_duration_ms as f64 / 1000.0
            );
            println!(
                "Result:              {}",
                if m.success {
                    "✅ SUCCESS"
                } else {
                    "❌ FAILED"
                }
            );
            println!();
            println!("─────────────────────────────────────────────────────────────");
            println!("Phase Durations:");
            for (phase, duration) in &m.phase_durations {
                println!("  {:25} {:6}ms", phase, duration);
            }
            println!("─────────────────────────────────────────────────────────────");
            println!("Participation:");
            println!("  Heartbeats:        {}", m.heartbeat_count);
            println!("  Votes:             {}", m.vote_count);
            println!("  Approval:          {:.1}%", m.approval_percentage);
            println!();
            if m.fallback_attempts > 0 {
                println!("Fallback Attempts:   {}", m.fallback_attempts);
            }
            if m.emergency_mode {
                println!("Emergency Mode:      🚨 YES");
            }

            // Show event timeline if requested
            if show_event_timeline {
                println!();
                println!("─────────────────────────────────────────────────────────────");
                println!("Event Timeline:");
                let events_with_timestamps = self.get_events_with_timestamps().await;

                if let Some(first_event) = events_with_timestamps.first() {
                    let start_time = first_event.timestamp;

                    for event_record in events_with_timestamps {
                        let elapsed = (event_record.timestamp - start_time).num_milliseconds();
                        println!("  [{:>6}ms] {:?}", elapsed, event_record.event);
                    }
                }
                println!("─────────────────────────────────────────────────────────────");
            }

            println!("╚════════════════════════════════════════════════════════════╝");
            println!();
        }
    }

    /// Reset for next round
    pub async fn reset(&self) {
        self.events.write().await.clear();
        self.metrics.write().await.take();
        self.start_time.write().await.take();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_monitor_event_recording() {
        let monitor = ConsensusMonitor::new();
        monitor.start_round(100).await;

        monitor
            .record_event(ConsensusEvent::HeartbeatReceived {
                node_id: "node1".to_string(),
            })
            .await;

        monitor
            .record_event(ConsensusEvent::HeartbeatReceived {
                node_id: "node2".to_string(),
            })
            .await;

        let events = monitor.get_events().await;
        assert_eq!(events.len(), 3); // ProtocolStarted + 2 heartbeats

        let metrics = monitor.get_metrics().await.unwrap();
        assert_eq!(metrics.heartbeat_count, 2);
    }

    #[tokio::test]
    async fn test_metrics_calculation() {
        let monitor = ConsensusMonitor::new();
        monitor.start_round(100).await;

        monitor
            .record_event(ConsensusEvent::VoteReceived {
                voter: "node1".to_string(),
                approve: true,
                weight: 10,
            })
            .await;

        monitor
            .record_event(ConsensusEvent::VoteReceived {
                voter: "node2".to_string(),
                approve: true,
                weight: 8,
            })
            .await;

        monitor
            .record_event(ConsensusEvent::ConsensusReached {
                approval_weight: 18,
                total_weight: 20,
            })
            .await;

        let metrics = monitor.get_metrics().await.unwrap();
        assert_eq!(metrics.vote_count, 2);
        assert_eq!(metrics.approval_percentage, 90.0);
    }

    #[tokio::test]
    async fn test_round_completion() {
        let monitor = ConsensusMonitor::new();
        monitor.start_round(100).await;

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let metrics = monitor.complete_round(true).await.unwrap();
        assert!(metrics.total_duration_ms >= 100);
        assert!(metrics.success);
    }

    #[tokio::test]
    async fn test_get_events_with_timestamps() {
        let monitor = ConsensusMonitor::new();
        monitor.start_round(100).await;

        // Add a small delay between events to ensure different timestamps
        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        monitor
            .record_event(ConsensusEvent::HeartbeatReceived {
                node_id: "node1".to_string(),
            })
            .await;

        tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

        monitor
            .record_event(ConsensusEvent::HeartbeatReceived {
                node_id: "node2".to_string(),
            })
            .await;

        let events_with_timestamps = monitor.get_events_with_timestamps().await;

        // Should have 3 events: ProtocolStarted + 2 heartbeats
        assert_eq!(events_with_timestamps.len(), 3);

        // Verify timestamps are in chronological order
        for i in 0..events_with_timestamps.len() - 1 {
            assert!(
                events_with_timestamps[i].timestamp <= events_with_timestamps[i + 1].timestamp,
                "Timestamps should be in chronological order"
            );
        }

        // Verify first event is ProtocolStarted
        match &events_with_timestamps[0].event {
            ConsensusEvent::ProtocolStarted { block_height } => {
                assert_eq!(*block_height, 100);
            }
            _ => panic!("First event should be ProtocolStarted"),
        }
    }

    #[tokio::test]
    async fn test_get_events_in_time_range() {
        let monitor = ConsensusMonitor::new();

        let start_time = Utc::now();
        monitor.start_round(100).await;

        // Record some events with delays
        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        monitor
            .record_event(ConsensusEvent::HeartbeatReceived {
                node_id: "node1".to_string(),
            })
            .await;

        let mid_time = Utc::now();

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;
        monitor
            .record_event(ConsensusEvent::HeartbeatReceived {
                node_id: "node2".to_string(),
            })
            .await;

        let end_time = Utc::now();

        // Get events in the full time range
        let all_events = monitor.get_events_in_time_range(start_time, end_time).await;
        assert_eq!(all_events.len(), 3); // ProtocolStarted + 2 heartbeats

        // Get events after mid_time (should only include the last heartbeat)
        let later_events = monitor.get_events_in_time_range(mid_time, end_time).await;
        assert_eq!(later_events.len(), 1);
        match &later_events[0].event {
            ConsensusEvent::HeartbeatReceived { node_id } => {
                assert_eq!(node_id, "node2");
            }
            _ => panic!("Expected HeartbeatReceived event"),
        }
    }

    #[tokio::test]
    async fn test_timestamp_field_is_used() {
        // This test verifies that the timestamp field is actually being read
        let monitor = ConsensusMonitor::new();
        monitor.start_round(100).await;

        let before = Utc::now();

        monitor
            .record_event(ConsensusEvent::LeaderElected {
                leader: "leader1".to_string(),
                weight: 100,
            })
            .await;

        let after = Utc::now();

        let events = monitor.get_events_with_timestamps().await;

        // Find the LeaderElected event and verify its timestamp
        let leader_event = events
            .iter()
            .find(|e| matches!(e.event, ConsensusEvent::LeaderElected { .. }))
            .expect("Should find LeaderElected event");

        // Timestamp should be between before and after
        assert!(
            leader_event.timestamp >= before && leader_event.timestamp <= after,
            "Timestamp should be within the expected range"
        );
    }

    #[tokio::test]
    async fn test_print_summary_with_timeline() {
        let monitor = ConsensusMonitor::new();
        monitor.start_round(100).await;

        monitor
            .record_event(ConsensusEvent::HeartbeatReceived {
                node_id: "node1".to_string(),
            })
            .await;

        monitor
            .record_event(ConsensusEvent::VoteReceived {
                voter: "node1".to_string(),
                approve: true,
                weight: 10,
            })
            .await;

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        monitor.complete_round(true).await;

        // This should not panic and should print the timeline
        monitor.print_summary(true).await;

        // Also test without timeline
        monitor.print_summary(false).await;
    }

    #[tokio::test]
    async fn test_empty_time_range() {
        let monitor = ConsensusMonitor::new();

        // Create a time range before any events
        let start = Utc::now();
        let end = start + chrono::Duration::milliseconds(100);

        let events = monitor.get_events_in_time_range(start, end).await;
        assert_eq!(events.len(), 0, "Should have no events in empty time range");
    }
}
