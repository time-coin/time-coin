#!/bin/bash

# Backup files
cp api/src/routes.rs api/src/routes.rs.backup
cp api/src/handlers.rs api/src/handlers.rs.backup

# Add route after blockchain/info line
sed -i '/\.route("\/blockchain\/info", get(handlers::get_blockchain_info))/a\        .route("/peers", get(handlers::get_peers))' api/src/routes.rs

# Add handler to handlers.rs (append before the last closing brace)
cat >> api/src/handlers.rs << 'HANDLER'

/// Get connected peers info (similar to bitcoin-cli getpeerinfo)
pub async fn get_peers(State(state): State<ApiState>) -> impl IntoResponse {
    let peers = state.peer_discovery.all_peers();
    
    let peer_info: Vec<serde_json::Value> = peers
        .iter()
        .map(|peer| {
            json!({
                "addr": peer.address.to_string(),
                "ip": peer.address.ip().to_string(),
                "port": peer.address.port(),
                "version": peer.version,
                "network": format!("{:?}", peer.network),
                "lastseen": peer.last_seen,
                "connected": true
            })
        })
        .collect();
    
    Json(json!({
        "peers": peer_info,
        "count": peer_info.len()
    }))
}
HANDLER

echo "✓ Added /peers endpoint"
echo "Review changes:"
echo "  git diff api/src/routes.rs"
echo "  git diff api/src/handlers.rs"
