mod balance;
mod error;
mod handlers;
pub mod instant_finality_handlers;
pub mod masternode_handlers;
pub mod proposal_handlers;
pub mod quarantine_handlers;
mod response;
mod routes;
mod rpc_handlers;
mod services;
mod state;
pub mod treasury_handlers;
pub mod tx_sync_handlers;
pub mod wallet_send_handler;
pub mod wallet_sync_handlers;

pub use error::{ApiError, ApiResult};
pub use services::{BlockchainService, TreasuryService, WalletService};
pub use state::ApiState;

// Re-export create_routes for testing
pub use routes::create_routes;

use axum::http::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Method,
};
use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

pub async fn start_server(
    addr: SocketAddr,
    state: ApiState,
) -> Result<(), Box<dyn std::error::Error>> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]);

    let app = routes::create_routes().with_state(state).layer(cors);

    let listener = tokio::net::TcpListener::bind(addr).await?;

    axum::serve(listener, app).await?;

    Ok(())
}
