mod error;
mod routes;
mod state;

pub use error::{ApiError, ApiResult};
pub use state::ApiState;

use axum::http::{
    header::{AUTHORIZATION, CONTENT_TYPE},
    Method,
};
use tower_http::cors::{Any, CorsLayer};
use std::net::SocketAddr;

pub async fn start_server(addr: SocketAddr, state: ApiState) -> Result<(), Box<dyn std::error::Error>> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE]);

    let app = routes::create_routes()
        .with_state(state)
        .layer(cors);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    
    axum::serve(listener, app).await?;
    
    Ok(())
}
