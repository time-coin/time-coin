mod error;
pub mod masternode_handlers;
mod routes;
mod state; // Add this line

pub use error::{ApiError, ApiResult};
pub use state::ApiState;

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
