//! Grant System Handlers

use crate::{
    constants::{GRANT_ACTIVATION_DAYS, GRANT_AMOUNT_SATOSHIS, GRANT_DECOMMISSION_DAYS},
    error::{ApiError, ApiResult},
    grant_models::*,
    grant_security::generate_secure_token,
    state::ApiState,
};
use axum::{extract::Path, extract::State, Json};
use chrono::{Duration, Utc};
use validator::Validate;

// ============================================
// GRANT APPLICATION
// ============================================

pub async fn apply_for_grant(
    State(state): State<ApiState>,
    Json(req): Json<GrantApplication>,
) -> ApiResult<Json<GrantApplicationResponse>> {
    // Validate request using validator crate
    req.validate()
        .map_err(|e| ApiError::BadRequest(format!("Validation failed: {}", e)))?;

    // NEW: Check rate limit
    if let Some(rate_limiter) = &state.grant_rate_limiter {
        rate_limiter.check_rate_limit(&req.email).await?;
    }

    let mut grants = state.grants.write().await;

    // Check if email already applied
    if grants.iter().any(|g| g.email == req.email) {
        return Ok(Json(GrantApplicationResponse {
            success: false,
            message: "Email already applied for a grant".to_string(),
            verification_sent: false,
        }));
    }

    // Create grant application with cryptographic token
    let verification_token = generate_secure_token();
    let expires_at = Utc::now() + Duration::days(GRANT_ACTIVATION_DAYS);

    let grant = crate::state::GrantData {
        email: req.email.clone(),
        verification_token: verification_token.clone(),
        verified: false,
        status: "pending".to_string(),
        grant_amount: GRANT_AMOUNT_SATOSHIS,
        applied_at: Utc::now().timestamp(),
        verified_at: None,
        activated_at: None,
        expires_at: Some(expires_at.timestamp()),
        masternode_address: None,
        public_key: None,
    };

    grants.push(grant);

    // In production, send verification email here
    // For dev/testnet, just log it
    tracing::info!(
        "Grant application: {} - Verification token: {}",
        req.email,
        verification_token
    );

    Ok(Json(GrantApplicationResponse {
        success: true,
        message: format!(
            "Grant application submitted! Check your email to verify. Verification link: /grant/verify/{}",
            verification_token
        ),
        verification_sent: true,
    }))
}

// ============================================
// EMAIL VERIFICATION
// ============================================

pub async fn verify_grant(
    State(state): State<ApiState>,
    Path(token): Path<String>,
) -> ApiResult<Json<GrantVerificationResponse>> {
    let mut grants = state.grants.write().await;

    if let Some(grant) = grants.iter_mut().find(|g| g.verification_token == token) {
        if grant.verified {
            return Ok(Json(GrantVerificationResponse {
                success: false,
                message: "Email already verified".to_string(),
                grant_amount: "1000 TIME".to_string(),
                expires_in_days: 0,
            }));
        }

        grant.verified = true;
        grant.verified_at = Some(Utc::now().timestamp());
        grant.status = "verified".to_string();

        let expires_in = if let Some(expires_at) = grant.expires_at {
            let now = Utc::now().timestamp();
            ((expires_at - now) / 86400) as u32
        } else {
            30
        };

        tracing::info!("Grant verified: {}", grant.email);

        Ok(Json(GrantVerificationResponse {
            success: true,
            message: format!(
                "Email verified! You have {} days to activate your masternode with 1000 TIME",
                expires_in
            ),
            grant_amount: "1000 TIME".to_string(),
            expires_in_days: expires_in,
        }))
    } else {
        Err(ApiError::InvalidAddress(
            "Invalid verification token".to_string(),
        ))
    }
}

// ============================================
// GRANT STATUS
// ============================================

pub async fn get_grant_status(
    State(state): State<ApiState>,
    Path(email): Path<String>,
) -> ApiResult<Json<GrantStatusResponse>> {
    let grants = state.grants.read().await;

    if let Some(grant) = grants.iter().find(|g| g.email == email) {
        let days_remaining = if let Some(expires_at) = grant.expires_at {
            let now = Utc::now().timestamp();
            Some(((expires_at - now) / 86400) as i32)
        } else {
            None
        };

        Ok(Json(GrantStatusResponse {
            email: grant.email.clone(),
            status: grant.status.clone(),
            grant_amount: grant.grant_amount,
            grant_amount_time: format!("{} TIME", grant.grant_amount / 100_000_000),
            verified: grant.verified,
            activated: grant.activated_at.is_some(),
            masternode_address: grant.masternode_address.clone(),
            expires_at: grant.expires_at,
            days_remaining,
        }))
    } else {
        Err(ApiError::InvalidAddress(
            "Email not found in grant applications".to_string(),
        ))
    }
}

// ============================================
// MASTERNODE ACTIVATION
// ============================================

pub async fn activate_masternode(
    State(state): State<ApiState>,
    Json(req): Json<MasternodeActivationRequest>,
) -> ApiResult<Json<MasternodeActivationResponse>> {
    // Validate request
    req.validate()
        .map_err(|e| ApiError::BadRequest(format!("Validation failed: {}", e)))?;

    let mut grants = state.grants.write().await;

    // Find grant
    let grant = grants
        .iter_mut()
        .find(|g| g.email == req.grant_email)
        .ok_or_else(|| ApiError::NotFound("Grant not found for this email".to_string()))?;

    // CRITICAL: Enforce email verification
    if !grant.verified {
        tracing::warn!(
            email = %req.grant_email,
            "masternode_activation_attempted_without_verification"
        );
        return Err(ApiError::Unauthorized(
            "Email must be verified before activating masternode. Check your email for verification link.".to_string()
        ));
    }

    if grant.status == "active" {
        return Err(ApiError::BadRequest(
            "Masternode already activated for this grant".to_string(),
        ));
    }

    // Check expiration
    if let Some(expires_at) = grant.expires_at {
        if Utc::now().timestamp() > expires_at {
            grant.status = "forfeited".to_string();
            tracing::warn!(
                email = %req.grant_email,
                expired_at = expires_at,
                "grant_expired"
            );
            return Err(ApiError::BadRequest(format!(
                "Grant has expired. Must activate within {} days of email verification.",
                GRANT_ACTIVATION_DAYS
            )));
        }
    }

    // Generate masternode address from public key
    let mn_address = time_crypto::public_key_to_address(&req.public_key);

    // Check treasury balance from blockchain UTXO set
    let blockchain = state.blockchain.read().await;
    let treasury_address = "TIME1treasury00000000000000000000000000";
    let treasury_balance = blockchain.get_balance(treasury_address);

    if treasury_balance < GRANT_AMOUNT_SATOSHIS {
        return Err(ApiError::InsufficientBalance {
            have: treasury_balance,
            need: GRANT_AMOUNT_SATOSHIS,
        });
    }
    drop(blockchain);

    // Note: Actual fund transfer happens via blockchain transactions
    // The balance HashMap was only a temporary tracking mechanism
    // In production, this would create a transaction from treasury to masternode address

    // Update grant
    grant.status = "active".to_string();
    grant.activated_at = Some(Utc::now().timestamp());
    grant.masternode_address = Some(mn_address.clone());
    grant.public_key = Some(req.public_key.clone());

    tracing::info!(
        "Masternode activated: {} for grant {}",
        mn_address,
        req.grant_email
    );

    Ok(Json(MasternodeActivationResponse {
        success: true,
        message: "Masternode activated successfully! 1000 TIME locked.".to_string(),
        masternode_address: mn_address,
        locked_amount: "1000 TIME".to_string(),
        status: "active".to_string(),
    }))
}

// ============================================
// DECOMMISSION MASTERNODE
// ============================================

pub async fn decommission_masternode(
    State(state): State<ApiState>,
    Json(req): Json<DecommissionRequest>,
) -> ApiResult<Json<DecommissionResponse>> {
    let mut grants = state.grants.write().await;

    let grant = grants
        .iter_mut()
        .find(|g| g.masternode_address.as_ref() == Some(&req.masternode_address))
        .ok_or_else(|| ApiError::InvalidAddress("Masternode not found".to_string()))?;

    if grant.status != "active" {
        return Err(ApiError::InvalidAddress(
            "Masternode is not active".to_string(),
        ));
    }

    // Start decommission process
    grant.status = "decommissioning".to_string();
    let unlock_date = Utc::now() + Duration::days(GRANT_DECOMMISSION_DAYS);

    tracing::info!(
        "Masternode decommissioning started: {} - Unlock date: {}",
        req.masternode_address,
        unlock_date
    );

    Ok(Json(DecommissionResponse {
        success: true,
        message: format!(
            "Decommission started. Funds will unlock in {} days",
            GRANT_DECOMMISSION_DAYS
        ),
        unlock_date: unlock_date.to_rfc3339(),
        days_until_unlock: GRANT_DECOMMISSION_DAYS as u32,
    }))
}

// ============================================
// EMAIL LIST EXPORT (ADMIN ONLY)
// ============================================

pub async fn export_email_list(
    State(state): State<ApiState>,
) -> ApiResult<Json<EmailListExportResponse>> {
    let grants = state.grants.read().await;

    let emails: Vec<EmailEntry> = grants
        .iter()
        .map(|g| EmailEntry {
            email: g.email.clone(),
            verified: g.verified,
            status: g.status.clone(),
            applied_at: chrono::DateTime::from_timestamp(g.applied_at, 0)
                .unwrap_or_default()
                .to_rfc3339(),
        })
        .collect();

    let verified_count = grants.iter().filter(|g| g.verified).count();
    let active_count = grants.iter().filter(|g| g.status == "active").count();

    Ok(Json(EmailListExportResponse {
        total_emails: emails.len(),
        verified_emails: verified_count,
        active_masternodes: active_count,
        emails,
    }))
}
