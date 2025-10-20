//! TIME Coin REST API Server

mod error;
mod grant_handlers;
mod grant_models;
mod handlers;
mod models;
mod routes;
mod state;

pub use error::{ApiError, ApiResult};
pub use routes::create_router;
pub use state::ApiState;

use std::net::SocketAddr;
use tower_http::cors::{Any, CorsLayer};

pub async fn start_server(
    bind_addr: SocketAddr,
    state: ApiState,
) -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter("time_api=debug,tower_http=debug")
        .init();

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = create_router(state).layer(cors);

    tracing::info!("🌐 API server starting on {}", bind_addr);

    let listener = tokio::net::TcpListener::bind(bind_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
