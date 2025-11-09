//! API Request Handlers

use crate::ApiState;
use axum::{extract::State, http::StatusCode, Json};
use serde_json::json;

// Get the node's wallet address
pub async fn get_node_wallet(
    State(state): State<ApiState>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    Ok(Json(json!({
        "wallet_address": state.wallet_address,
        "status": "success"
    })))
}
