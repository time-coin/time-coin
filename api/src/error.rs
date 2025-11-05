//! API Error Handling

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde_json::json;
use thiserror::Error;

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Error, Debug)]
pub enum ApiError {
    #[error("Invalid address: {0}")]
    InvalidAddress(String),

    #[error("Insufficient balance: have {have}, need {need}")]
    InsufficientBalance { have: u64, need: u64 },

    #[error("Transaction not found: {0}")]
    TransactionNotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Invalid signature")]
    InvalidSignature,

    #[error("Invalid private key")]
    InvalidPrivateKey,

    #[error("Internal error: {0}")]
    Internal(String),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match self {
            ApiError::InvalidAddress(msg) => (StatusCode::BAD_REQUEST, "invalid_address", msg),
            ApiError::InsufficientBalance { have, need } => (
                StatusCode::BAD_REQUEST,
                "insufficient_balance",
                format!("Balance {} is less than required {}", have, need),
            ),
            ApiError::TransactionNotFound(txid) => (
                StatusCode::NOT_FOUND,
                "transaction_not_found",
                format!("Transaction {} not found", txid),
            ),
            ApiError::InvalidSignature => (
                StatusCode::BAD_REQUEST,
                "invalid_signature",
                "Transaction signature is invalid".to_string(),
            ),
            ApiError::InvalidPrivateKey => (
                StatusCode::BAD_REQUEST,
                "invalid_private_key",
                "Private key format is invalid".to_string(),
            ),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "unauthorized", msg),
            ApiError::Internal(msg) => (StatusCode::INTERNAL_SERVER_ERROR, "internal_error", msg),
        };

        let body = Json(json!({
            "error": error_type,
            "message": message,
        }));

        (status, body).into_response()
    }
}
