//! Network State Management and Resilience
//!
//! Implements automatic reconnection, transaction queuing, and graceful degradation
//! for network failures.

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// Maximum pending transactions to queue
const MAX_PENDING_TRANSACTIONS: usize = 100;

/// Initial backoff duration
const INITIAL_BACKOFF_SECS: u64 = 1;

/// Maximum backoff duration
const MAX_BACKOFF_SECS: u64 = 60;

/// Offline mode threshold (5 minutes)
const OFFLINE_THRESHOLD_SECS: u64 = 300;

/// Network connection state
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkState {
    /// No connection
    Offline,
    /// Attempting to connect
    Connecting { attempt: u32 },
    /// Connected to peers
    Connected { peer_count: usize },
    /// Syncing blockchain
    Syncing { progress: u8 },
    /// Fully synced and ready
    Ready,
    /// Error state
    Error(String),
}

impl NetworkState {
    /// Check if network is ready for transactions
    pub fn is_ready(&self) -> bool {
        matches!(self, NetworkState::Ready)
    }

    /// Check if network is online (any connection)
    pub fn is_online(&self) -> bool {
        !matches!(self, NetworkState::Offline | NetworkState::Error(_))
    }

    /// Get display string for UI
    pub fn display_string(&self) -> String {
        match self {
            NetworkState::Offline => "⚫ Offline".to_string(),
            NetworkState::Connecting { attempt } => format!("🔄 Connecting (attempt {})", attempt),
            NetworkState::Connected { peer_count } => {
                format!("🟡 Connected ({} peers)", peer_count)
            }
            NetworkState::Syncing { progress } => format!("🔄 Syncing ({}%)", progress),
            NetworkState::Ready => "🟢 Ready".to_string(),
            NetworkState::Error(msg) => format!("🔴 Error: {}", msg),
        }
    }

    /// Get color for UI display
    pub fn display_color(&self) -> (u8, u8, u8) {
        match self {
            NetworkState::Offline => (128, 128, 128),         // Gray
            NetworkState::Connecting { .. } => (255, 165, 0), // Orange
            NetworkState::Connected { peer_count } => {
                if *peer_count < 2 {
                    (255, 255, 0) // Yellow
                } else if *peer_count < 5 {
                    (173, 255, 47) // Yellow-green
                } else {
                    (0, 255, 0) // Green
                }
            }
            NetworkState::Syncing { .. } => (255, 165, 0), // Orange
            NetworkState::Ready => (0, 255, 0),            // Green
            NetworkState::Error(_) => (255, 0, 0),         // Red
        }
    }
}

/// Pending transaction for later broadcast
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PendingTransaction {
    /// Transaction ID
    pub txid: String,
    /// Transaction data (serialized)
    pub data: Vec<u8>,
    /// When it was queued
    pub queued_at: std::time::SystemTime,
    /// Number of broadcast attempts
    pub attempts: u32,
}

/// Network state manager with resilience features
pub struct NetworkStateManager {
    /// Current network state
    state: Arc<Mutex<NetworkState>>,
    /// Last successful connection time
    last_connected: Arc<Mutex<Option<Instant>>>,
    /// Reconnection backoff
    backoff_seconds: Arc<Mutex<u64>>,
    /// Pending transactions queue
    pending_txs: Arc<Mutex<VecDeque<PendingTransaction>>>,
    /// State change listeners
    state_listeners: Arc<Mutex<Vec<tokio::sync::mpsc::UnboundedSender<NetworkState>>>>,
}

impl NetworkStateManager {
    /// Create new network state manager
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(NetworkState::Offline)),
            last_connected: Arc::new(Mutex::new(None)),
            backoff_seconds: Arc::new(Mutex::new(INITIAL_BACKOFF_SECS)),
            pending_txs: Arc::new(Mutex::new(VecDeque::new())),
            state_listeners: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Get current network state
    pub async fn get_state(&self) -> NetworkState {
        self.state.lock().await.clone()
    }

    /// Update network state
    pub async fn set_state(&self, new_state: NetworkState) {
        let mut state = self.state.lock().await;
        *state = new_state.clone();

        // Notify listeners
        let listeners = self.state_listeners.lock().await;
        for listener in listeners.iter() {
            let _ = listener.send(new_state.clone());
        }
    }

    /// Subscribe to state changes
    pub async fn subscribe(&self) -> tokio::sync::mpsc::UnboundedReceiver<NetworkState> {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        self.state_listeners.lock().await.push(tx);
        rx
    }

    /// Mark as connected
    pub async fn mark_connected(&self, peer_count: usize) {
        *self.last_connected.lock().await = Some(Instant::now());
        *self.backoff_seconds.lock().await = INITIAL_BACKOFF_SECS;
        self.set_state(NetworkState::Connected { peer_count }).await;
    }

    /// Mark as ready
    pub async fn mark_ready(&self) {
        *self.last_connected.lock().await = Some(Instant::now());
        self.set_state(NetworkState::Ready).await;
    }

    /// Mark as syncing
    pub async fn mark_syncing(&self, progress: u8) {
        self.set_state(NetworkState::Syncing { progress }).await;
    }

    /// Mark as offline
    pub async fn mark_offline(&self) {
        self.set_state(NetworkState::Offline).await;
    }

    /// Mark as error
    pub async fn mark_error(&self, error: String) {
        self.set_state(NetworkState::Error(error)).await;
    }

    /// Check if in offline mode (no connection for >5 min)
    pub async fn is_offline_mode(&self) -> bool {
        if let Some(last) = *self.last_connected.lock().await {
            last.elapsed().as_secs() > OFFLINE_THRESHOLD_SECS
        } else {
            true
        }
    }

    /// Get next reconnection backoff duration
    pub async fn get_backoff_duration(&self) -> Duration {
        let seconds = *self.backoff_seconds.lock().await;
        Duration::from_secs(seconds)
    }

    /// Increase backoff (exponential)
    pub async fn increase_backoff(&self) {
        let mut backoff = self.backoff_seconds.lock().await;
        *backoff = (*backoff * 2).min(MAX_BACKOFF_SECS);
    }

    /// Reset backoff to initial value
    pub async fn reset_backoff(&self) {
        *self.backoff_seconds.lock().await = INITIAL_BACKOFF_SECS;
    }

    /// Queue transaction for later broadcast
    pub async fn queue_transaction(&self, txid: String, data: Vec<u8>) -> Result<(), String> {
        let mut queue = self.pending_txs.lock().await;

        if queue.len() >= MAX_PENDING_TRANSACTIONS {
            return Err("Transaction queue is full".to_string());
        }

        let pending = PendingTransaction {
            txid,
            data,
            queued_at: std::time::SystemTime::now(),
            attempts: 0,
        };

        queue.push_back(pending);
        log::info!(
            "📮 Queued transaction for later broadcast ({} pending)",
            queue.len()
        );

        Ok(())
    }

    /// Get next pending transaction
    pub async fn get_next_pending_tx(&self) -> Option<PendingTransaction> {
        let mut queue = self.pending_txs.lock().await;
        queue.pop_front()
    }

    /// Get pending transaction count
    pub async fn pending_tx_count(&self) -> usize {
        self.pending_txs.lock().await.len()
    }

    /// Retry broadcasting pending transactions
    pub async fn retry_pending_txs<F>(&self, broadcast_fn: F) -> usize
    where
        F: Fn(
            PendingTransaction,
        ) -> std::pin::Pin<Box<dyn std::future::Future<Output = bool> + Send>>,
    {
        let mut broadcasted = 0;
        let mut failed = VecDeque::new();

        while let Some(mut pending) = self.get_next_pending_tx().await {
            pending.attempts += 1;

            if broadcast_fn(pending.clone()).await {
                log::info!(
                    "✅ Successfully broadcast pending transaction: {}",
                    pending.txid
                );
                broadcasted += 1;
            } else if pending.attempts < 3 {
                // Retry up to 3 times
                failed.push_back(pending);
            } else {
                log::warn!(
                    "❌ Gave up broadcasting transaction after 3 attempts: {}",
                    pending.txid
                );
            }
        }

        // Re-queue failed transactions
        if !failed.is_empty() {
            let mut queue = self.pending_txs.lock().await;
            queue.extend(failed);
        }

        broadcasted
    }

    /// Clear all pending transactions
    pub async fn clear_pending_txs(&self) {
        self.pending_txs.lock().await.clear();
    }

    /// Start automatic reconnection loop
    pub async fn start_reconnection_loop<F>(&self, connect_fn: F)
    where
        F: Fn() -> std::pin::Pin<Box<dyn std::future::Future<Output = Result<(), String>> + Send>>
            + Send
            + 'static,
    {
        let state_mgr = self.clone();

        tokio::spawn(async move {
            let mut attempt = 0u32;

            loop {
                let current_state = state_mgr.get_state().await;

                // Only try to reconnect if offline
                if matches!(current_state, NetworkState::Offline) {
                    attempt += 1;
                    state_mgr
                        .set_state(NetworkState::Connecting { attempt })
                        .await;

                    log::info!("🔄 Reconnection attempt #{}", attempt);

                    match connect_fn().await {
                        Ok(_) => {
                            log::info!("✅ Reconnection successful");
                            attempt = 0;
                            state_mgr.reset_backoff().await;
                        }
                        Err(e) => {
                            log::warn!("❌ Reconnection failed: {}", e);
                            state_mgr.mark_offline().await;
                            state_mgr.increase_backoff().await;
                        }
                    }
                }

                // Wait with exponential backoff
                let backoff = state_mgr.get_backoff_duration().await;
                tokio::time::sleep(backoff).await;
            }
        });
    }
}

impl Default for NetworkStateManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for NetworkStateManager {
    fn clone(&self) -> Self {
        Self {
            state: self.state.clone(),
            last_connected: self.last_connected.clone(),
            backoff_seconds: self.backoff_seconds.clone(),
            pending_txs: self.pending_txs.clone(),
            state_listeners: self.state_listeners.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_network_state_transitions() {
        let mgr = NetworkStateManager::new();

        assert_eq!(mgr.get_state().await, NetworkState::Offline);

        mgr.mark_connected(3).await;
        assert!(matches!(
            mgr.get_state().await,
            NetworkState::Connected { peer_count: 3 }
        ));

        mgr.mark_syncing(50).await;
        assert!(matches!(
            mgr.get_state().await,
            NetworkState::Syncing { progress: 50 }
        ));

        mgr.mark_ready().await;
        assert_eq!(mgr.get_state().await, NetworkState::Ready);
    }

    #[tokio::test]
    async fn test_backoff_increase() {
        let mgr = NetworkStateManager::new();

        assert_eq!(mgr.get_backoff_duration().await, Duration::from_secs(1));

        mgr.increase_backoff().await;
        assert_eq!(mgr.get_backoff_duration().await, Duration::from_secs(2));

        mgr.increase_backoff().await;
        assert_eq!(mgr.get_backoff_duration().await, Duration::from_secs(4));

        mgr.reset_backoff().await;
        assert_eq!(mgr.get_backoff_duration().await, Duration::from_secs(1));
    }

    #[tokio::test]
    async fn test_transaction_queue() {
        let mgr = NetworkStateManager::new();

        mgr.queue_transaction("tx1".to_string(), vec![1, 2, 3])
            .await
            .unwrap();
        mgr.queue_transaction("tx2".to_string(), vec![4, 5, 6])
            .await
            .unwrap();

        assert_eq!(mgr.pending_tx_count().await, 2);

        let tx1 = mgr.get_next_pending_tx().await.unwrap();
        assert_eq!(tx1.txid, "tx1");

        assert_eq!(mgr.pending_tx_count().await, 1);
    }
}
