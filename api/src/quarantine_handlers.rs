//! Quarantine management API handlers

use crate::{ApiResult, ApiState};
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Serialize)]
pub struct QuarantineEntry {
    pub peer_ip: String,
    pub reason: String,
    pub quarantined_at: String,
    pub attempts: u32,
    pub expires_in_seconds: u64,
}

#[derive(Serialize)]
pub struct QuarantineStatusResponse {
    pub total_quarantined: usize,
    pub entries: Vec<QuarantineEntry>,
}

#[derive(Deserialize)]
pub struct ReleaseRequest {
    pub peer_ip: String,
}

#[derive(Serialize)]
pub struct ReleaseResponse {
    pub success: bool,
    pub message: String,
}

/// Get list of quarantined peers
pub async fn get_quarantined_peers(
    State(state): State<ApiState>,
) -> ApiResult<Json<QuarantineStatusResponse>> {
    if let Some(quarantine) = &state.quarantine {
        let entries = quarantine.get_quarantined_peers().await;
        let now = Instant::now();
        
        let formatted_entries: Vec<QuarantineEntry> = entries
            .iter()
            .map(|entry| {
                let expires_in = if now < entry.expires_at {
                    entry.expires_at.duration_since(now).as_secs()
                } else {
                    0
                };
                
                QuarantineEntry {
                    peer_ip: entry.peer_ip.to_string(),
                    reason: entry.reason.to_string(),
                    quarantined_at: format!("{:?}", entry.quarantined_at),
                    attempts: entry.attempts,
                    expires_in_seconds: expires_in,
                }
            })
            .collect();

        Ok(Json(QuarantineStatusResponse {
            total_quarantined: formatted_entries.len(),
            entries: formatted_entries,
        }))
    } else {
        // Quarantine not configured
        Ok(Json(QuarantineStatusResponse {
            total_quarantined: 0,
            entries: vec![],
        }))
    }
}

/// Release a peer from quarantine
pub async fn release_peer(
    State(state): State<ApiState>,
    Json(request): Json<ReleaseRequest>,
) -> ApiResult<Json<ReleaseResponse>> {
    if let Some(quarantine) = &state.quarantine {
        match request.peer_ip.parse() {
            Ok(peer_addr) => {
                quarantine.release_peer(&peer_addr).await;
                Ok(Json(ReleaseResponse {
                    success: true,
                    message: format!("Peer {} released from quarantine", request.peer_ip),
                }))
            }
            Err(_) => Ok(Json(ReleaseResponse {
                success: false,
                message: format!("Invalid peer IP address: {}", request.peer_ip),
            })),
        }
    } else {
        Ok(Json(ReleaseResponse {
            success: false,
            message: "Quarantine system not configured".to_string(),
        }))
    }
}

/// Get quarantine statistics
#[derive(Serialize)]
pub struct QuarantineStatsResponse {
    pub total_quarantined: usize,
    pub genesis_mismatch: usize,
    pub fork_detected: usize,
    pub suspicious_height: usize,
    pub consensus_violation: usize,
    pub invalid_block: usize,
    pub invalid_transaction: usize,
    pub protocol_mismatch: usize,
    pub connection_failures: usize,
    pub rate_limit_exceeded: usize,
    pub excessive_timeouts: usize,
}

pub async fn get_quarantine_stats(
    State(state): State<ApiState>,
) -> ApiResult<Json<QuarantineStatsResponse>> {
    if let Some(quarantine) = &state.quarantine {
        let stats = quarantine.get_stats().await;
        Ok(Json(QuarantineStatsResponse {
            total_quarantined: stats.total_quarantined,
            genesis_mismatch: stats.genesis_mismatch,
            fork_detected: stats.fork_detected,
            suspicious_height: stats.suspicious_height,
            consensus_violation: stats.consensus_violation,
            invalid_block: stats.invalid_block,
            invalid_transaction: stats.invalid_transaction,
            protocol_mismatch: stats.protocol_mismatch,
            connection_failures: stats.connection_failures,
            rate_limit_exceeded: stats.rate_limit_exceeded,
            excessive_timeouts: stats.excessive_timeouts,
        }))
    } else {
        // Return empty stats if quarantine not configured
        Ok(Json(QuarantineStatsResponse {
            total_quarantined: 0,
            genesis_mismatch: 0,
            fork_detected: 0,
            suspicious_height: 0,
            consensus_violation: 0,
            invalid_block: 0,
            invalid_transaction: 0,
            protocol_mismatch: 0,
            connection_failures: 0,
            rate_limit_exceeded: 0,
            excessive_timeouts: 0,
        }))
    }
}
