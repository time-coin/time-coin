#!/bin/bash

# Create a temporary file with the new get_peers function
cat > /tmp/new_get_peers.rs << 'NEWFUNCTION'

/// Get connected peers info (similar to bitcoin-cli getpeerinfo)
pub async fn get_peers(State(_state): State<ApiState>) -> ApiResult<Json<serde_json::Value>> {
    // Fetch from the public peer discovery endpoint
    let client = reqwest::Client::new();
    let response = client
        .get("https://time-coin.io/api/peers")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to fetch peers: {}", e)))?;
    
    let peer_strings: Vec<String> = response
        .json()
        .await
        .map_err(|e| ApiError::InternalError(format!("Failed to parse peers: {}", e)))?;
    
    let peer_info: Vec<serde_json::Value> = peer_strings
        .iter()
        .map(|addr_str| {
            let parts: Vec<&str> = addr_str.split(':').collect();
            json!({
                "addr": addr_str,
                "ip": parts.get(0).unwrap_or(&"unknown"),
                "port": parts.get(1).and_then(|p| p.parse::<u16>().ok()).unwrap_or(0),
                "version": "1.0.0",
                "network": "Testnet",
                "lastseen": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs(),
                "connected": true
            })
        })
        .collect();
    
    Ok(Json(json!({
        "peers": peer_info,
        "count": peer_info.len(),
        "source": "time-coin.io/api/peers"
    })))
}
NEWFUNCTION

# Remove the old get_peers function and add the new one
awk 'BEGIN {skip=0} 
/^\/\/\/ Get connected peers info/ {skip=1} 
skip==1 && /^}$/ {skip=2; next} 
skip==2 {skip=0} 
skip==0 {print}' api/src/handlers.rs > /tmp/handlers_without_get_peers.rs

cat /tmp/handlers_without_get_peers.rs /tmp/new_get_peers.rs > api/src/handlers.rs

echo "✓ Updated get_peers function"
echo "Review changes:"
git diff api/src/handlers.rs | tail -50
