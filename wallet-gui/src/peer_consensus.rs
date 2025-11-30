// Peer Consensus Validation System
// Implements multi-peer consensus for reliable blockchain data

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};

const QUERY_TIMEOUT: Duration = Duration::from_secs(30);
const MIN_PEERS_FOR_CONSENSUS: usize = 3;
const CONSENSUS_THRESHOLD: f64 = 0.67; // 2/3 agreement required

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockchainInfo {
    pub height: u64,
    pub best_block_hash: String,
    pub total_transactions: u64,
}

#[derive(Debug, Clone)]
pub struct PeerResponse<T> {
    pub peer_addr: String,
    pub data: T,
    pub latency_ms: u64,
}

#[derive(Debug, Clone)]
pub struct ConsensusResult<T> {
    pub consensus_value: T,
    pub agreement_count: usize,
    pub total_responses: usize,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct PeerScore {
    pub success_rate: f64,
    pub avg_latency_ms: u64,
    pub total_queries: u64,
    pub failed_queries: u64,
}

pub struct PeerConsensus {
    peer_scores: Arc<RwLock<HashMap<String, PeerScore>>>,
}

impl PeerConsensus {
    pub fn new() -> Self {
        Self {
            peer_scores: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Query blockchain height from multiple peers and return consensus
    pub async fn query_blockchain_height(
        &self,
        peer_addrs: Vec<String>,
    ) -> Result<ConsensusResult<u64>, String> {
        if peer_addrs.len() < MIN_PEERS_FOR_CONSENSUS {
            return Err(format!(
                "Need at least {} peers for consensus, got {}",
                MIN_PEERS_FOR_CONSENSUS,
                peer_addrs.len()
            ));
        }

        let mut responses = Vec::new();

        // Query all peers in parallel
        let mut tasks = Vec::new();
        for peer_addr in peer_addrs {
            let peer_addr_clone = peer_addr.clone();
            let task =
                tokio::spawn(async move { Self::query_single_peer_height(peer_addr_clone).await });
            tasks.push(task);
        }

        // Collect responses
        for task in tasks {
            if let Ok(Ok(response)) = task.await {
                responses.push(response);
            }
        }

        if responses.is_empty() {
            return Err("No peers responded successfully".to_string());
        }

        // Update peer scores
        self.update_peer_scores(&responses).await;

        // Find consensus
        self.find_consensus_height(responses)
    }

    async fn query_single_peer_height(peer_addr: String) -> Result<PeerResponse<u64>, String> {
        let start = std::time::Instant::now();

        // TODO: Replace with actual TCP protocol call
        // For now, simulate query
        let result = timeout(QUERY_TIMEOUT, async {
            // Simulated network call
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok::<u64, String>(0) // Would return actual height
        })
        .await;

        let latency_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(height)) => Ok(PeerResponse {
                peer_addr,
                data: height,
                latency_ms,
            }),
            Ok(Err(e)) => Err(format!("Peer {} error: {}", peer_addr, e)),
            Err(_) => Err(format!(
                "Peer {} timeout after {:?}",
                peer_addr, QUERY_TIMEOUT
            )),
        }
    }

    fn find_consensus_height(
        &self,
        responses: Vec<PeerResponse<u64>>,
    ) -> Result<ConsensusResult<u64>, String> {
        let mut height_counts: HashMap<u64, usize> = HashMap::new();

        for response in &responses {
            *height_counts.entry(response.data).or_insert(0) += 1;
        }

        // Find the height with most agreement
        let (consensus_height, agreement_count) = height_counts
            .into_iter()
            .max_by_key(|(_, count)| *count)
            .ok_or("No consensus found")?;

        let total_responses = responses.len();
        let confidence = agreement_count as f64 / total_responses as f64;

        if confidence < CONSENSUS_THRESHOLD {
            return Err(format!(
                "Consensus threshold not met: {:.2}% < {:.2}%",
                confidence * 100.0,
                CONSENSUS_THRESHOLD * 100.0
            ));
        }

        Ok(ConsensusResult {
            consensus_value: consensus_height,
            agreement_count,
            total_responses,
            confidence,
        })
    }

    /// Query mempool from multiple peers
    pub async fn query_mempool(
        &self,
        peer_addrs: Vec<String>,
    ) -> Result<ConsensusResult<Vec<String>>, String> {
        if peer_addrs.len() < MIN_PEERS_FOR_CONSENSUS {
            return Err(format!(
                "Need at least {} peers for consensus",
                MIN_PEERS_FOR_CONSENSUS
            ));
        }

        let mut responses = Vec::new();

        // Query all peers in parallel
        let mut tasks = Vec::new();
        for peer_addr in peer_addrs {
            let peer_addr_clone = peer_addr.clone();
            let task =
                tokio::spawn(async move { Self::query_single_peer_mempool(peer_addr_clone).await });
            tasks.push(task);
        }

        // Collect responses
        for task in tasks {
            if let Ok(Ok(response)) = task.await {
                responses.push(response);
            }
        }

        if responses.is_empty() {
            return Err("No peers responded successfully".to_string());
        }

        // Update peer scores
        self.update_peer_scores(&responses).await;

        // Find consensus on mempool contents
        self.find_consensus_mempool(responses)
    }

    async fn query_single_peer_mempool(
        peer_addr: String,
    ) -> Result<PeerResponse<Vec<String>>, String> {
        let start = std::time::Instant::now();

        let result = timeout(QUERY_TIMEOUT, async {
            // TODO: Replace with actual TCP protocol call
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok::<Vec<String>, String>(vec![])
        })
        .await;

        let latency_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(txids)) => Ok(PeerResponse {
                peer_addr,
                data: txids,
                latency_ms,
            }),
            Ok(Err(e)) => Err(format!("Peer {} error: {}", peer_addr, e)),
            Err(_) => Err(format!("Peer {} timeout", peer_addr)),
        }
    }

    fn find_consensus_mempool(
        &self,
        responses: Vec<PeerResponse<Vec<String>>>,
    ) -> Result<ConsensusResult<Vec<String>>, String> {
        // Count how many peers report each transaction
        let mut tx_counts: HashMap<String, usize> = HashMap::new();

        for response in &responses {
            for txid in &response.data {
                *tx_counts.entry(txid.clone()).or_insert(0) += 1;
            }
        }

        let total_responses = responses.len();
        let threshold_count = (total_responses as f64 * CONSENSUS_THRESHOLD).ceil() as usize;

        // Only include transactions that appear in threshold% of peers
        let consensus_txs: Vec<String> = tx_counts
            .into_iter()
            .filter(|(_, count)| *count >= threshold_count)
            .map(|(txid, _)| txid)
            .collect();

        let agreement_count = threshold_count;
        let confidence = agreement_count as f64 / total_responses as f64;

        Ok(ConsensusResult {
            consensus_value: consensus_txs,
            agreement_count,
            total_responses,
            confidence,
        })
    }

    /// Validate UTXO set consistency across peers
    pub async fn validate_utxo_set(
        &self,
        peer_addrs: Vec<String>,
        address: &str,
    ) -> Result<ConsensusResult<Vec<String>>, String> {
        if peer_addrs.len() < MIN_PEERS_FOR_CONSENSUS {
            return Err(format!(
                "Need at least {} peers for consensus",
                MIN_PEERS_FOR_CONSENSUS
            ));
        }

        let mut responses = Vec::new();

        // Query all peers in parallel
        let mut tasks = Vec::new();
        for peer_addr in peer_addrs {
            let peer_addr_clone = peer_addr.clone();
            let address_clone = address.to_string();
            let task = tokio::spawn(async move {
                Self::query_single_peer_utxos(peer_addr_clone, address_clone).await
            });
            tasks.push(task);
        }

        // Collect responses
        for task in tasks {
            if let Ok(Ok(response)) = task.await {
                responses.push(response);
            }
        }

        if responses.is_empty() {
            return Err("No peers responded successfully".to_string());
        }

        // Update peer scores
        self.update_peer_scores(&responses).await;

        // Find consensus on UTXOs
        self.find_consensus_utxos(responses)
    }

    async fn query_single_peer_utxos(
        peer_addr: String,
        address: String,
    ) -> Result<PeerResponse<Vec<String>>, String> {
        let start = std::time::Instant::now();

        let result = timeout(QUERY_TIMEOUT, async move {
            // TODO: Replace with actual TCP protocol call
            tokio::time::sleep(Duration::from_millis(100)).await;
            Ok::<Vec<String>, String>(vec![])
        })
        .await;

        let latency_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(Ok(utxos)) => Ok(PeerResponse {
                peer_addr,
                data: utxos,
                latency_ms,
            }),
            Ok(Err(e)) => Err(format!("Peer {} error: {}", peer_addr, e)),
            Err(_) => Err(format!("Peer {} timeout", peer_addr)),
        }
    }

    fn find_consensus_utxos(
        &self,
        responses: Vec<PeerResponse<Vec<String>>>,
    ) -> Result<ConsensusResult<Vec<String>>, String> {
        // Similar to mempool consensus
        let mut utxo_counts: HashMap<String, usize> = HashMap::new();

        for response in &responses {
            for utxo in &response.data {
                *utxo_counts.entry(utxo.clone()).or_insert(0) += 1;
            }
        }

        let total_responses = responses.len();
        let threshold_count = (total_responses as f64 * CONSENSUS_THRESHOLD).ceil() as usize;

        let consensus_utxos: Vec<String> = utxo_counts
            .into_iter()
            .filter(|(_, count)| *count >= threshold_count)
            .map(|(utxo, _)| utxo)
            .collect();

        let agreement_count = threshold_count;
        let confidence = agreement_count as f64 / total_responses as f64;

        Ok(ConsensusResult {
            consensus_value: consensus_utxos,
            agreement_count,
            total_responses,
            confidence,
        })
    }

    /// Detect if any peers are on a different fork
    pub async fn detect_peer_divergence(
        &self,
        peer_addrs: Vec<String>,
    ) -> Result<Vec<String>, String> {
        let responses = self.query_blockchain_height(peer_addrs.clone()).await?;

        let divergent_peers = Vec::new();

        // Any peer not on consensus height is divergent
        for peer_addr in peer_addrs {
            // TODO: Check if this peer's height matches consensus
            // For now, return empty list
        }

        Ok(divergent_peers)
    }

    /// Update peer reliability scores based on responses
    async fn update_peer_scores<T>(&self, responses: &[PeerResponse<T>]) {
        let mut scores = self.peer_scores.write().await;

        for response in responses {
            let score = scores
                .entry(response.peer_addr.clone())
                .or_insert(PeerScore {
                    success_rate: 1.0,
                    avg_latency_ms: response.latency_ms,
                    total_queries: 0,
                    failed_queries: 0,
                });

            score.total_queries += 1;

            // Update rolling average latency
            score.avg_latency_ms = ((score.avg_latency_ms * (score.total_queries - 1))
                + response.latency_ms)
                / score.total_queries;

            // Update success rate
            score.success_rate =
                (score.total_queries - score.failed_queries) as f64 / score.total_queries as f64;
        }
    }

    /// Get peer reliability scores
    pub async fn get_peer_scores(&self) -> HashMap<String, PeerScore> {
        self.peer_scores.read().await.clone()
    }

    /// Get best peers sorted by reliability and latency
    pub async fn get_best_peers(&self, count: usize) -> Vec<String> {
        let scores = self.peer_scores.read().await;

        let mut peer_list: Vec<_> = scores.iter().collect();

        // Sort by success rate (descending) then latency (ascending)
        peer_list.sort_by(|a, b| {
            b.1.success_rate
                .partial_cmp(&a.1.success_rate)
                .unwrap()
                .then(a.1.avg_latency_ms.cmp(&b.1.avg_latency_ms))
        });

        peer_list
            .into_iter()
            .take(count)
            .map(|(addr, _)| addr.clone())
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_consensus_threshold() {
        let consensus = PeerConsensus::new();

        let responses = vec![
            PeerResponse {
                peer_addr: "peer1".to_string(),
                data: 100u64,
                latency_ms: 50,
            },
            PeerResponse {
                peer_addr: "peer2".to_string(),
                data: 100u64,
                latency_ms: 60,
            },
            PeerResponse {
                peer_addr: "peer3".to_string(),
                data: 99u64,
                latency_ms: 70,
            },
        ];

        let result = consensus.find_consensus_height(responses);
        assert!(result.is_ok());

        let consensus_result = result.unwrap();
        assert_eq!(consensus_result.consensus_value, 100);
        assert_eq!(consensus_result.agreement_count, 2);
    }

    #[tokio::test]
    async fn test_peer_scoring() {
        let consensus = PeerConsensus::new();

        let responses = vec![PeerResponse {
            peer_addr: "peer1".to_string(),
            data: 100u64,
            latency_ms: 50,
        }];

        consensus.update_peer_scores(&responses).await;

        let scores = consensus.get_peer_scores().await;
        assert!(scores.contains_key("peer1"));
        assert_eq!(scores.get("peer1").unwrap().avg_latency_ms, 50);
    }
}
