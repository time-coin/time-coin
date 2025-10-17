//! Grant System Models

use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct GrantApplication {
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GrantApplicationResponse {
    pub success: bool,
    pub message: String,
    pub verification_sent: bool,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GrantVerificationResponse {
    pub success: bool,
    pub message: String,
    pub grant_amount: String, // "1000 TIME"
    pub expires_in_days: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GrantStatusResponse {
    pub email: String,
    pub status: String,
    pub grant_amount: u64,
    pub grant_amount_time: String,
    pub verified: bool,
    pub activated: bool,
    pub masternode_address: Option<String>,
    pub expires_at: Option<i64>,
    pub days_remaining: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MasternodeActivationRequest {
    pub grant_email: String,
    pub public_key: String,
    pub ip_address: String,
    pub port: u16,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MasternodeActivationResponse {
    pub success: bool,
    pub message: String,
    pub masternode_address: String,
    pub locked_amount: String,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DecommissionRequest {
    pub masternode_address: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct DecommissionResponse {
    pub success: bool,
    pub message: String,
    pub unlock_date: String,
    pub days_until_unlock: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UnlockStatusResponse {
    pub masternode_address: String,
    pub status: String,
    pub locked_amount: String,
    pub decommissioned_at: Option<String>,
    pub unlock_at: Option<String>,
    pub can_withdraw: bool,
    pub days_remaining: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmailListExportResponse {
    pub total_emails: usize,
    pub verified_emails: usize,
    pub active_masternodes: usize,
    pub emails: Vec<EmailEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct EmailEntry {
    pub email: String,
    pub verified: bool,
    pub status: String,
    pub applied_at: String,
}
