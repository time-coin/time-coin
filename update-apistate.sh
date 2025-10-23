#!/bin/bash

# Backup first
cp api/src/state.rs api/src/state.rs.backup

# Add the import at the top
sed -i '1a use network::discovery::PeerDiscovery;\nuse std::sync::Arc;\nuse tokio::sync::RwLock;' api/src/state.rs

# Add peer_discovery field to ApiState struct (after network field)
sed -i '/pub network: String,/a\    pub peer_discovery: Arc<RwLock<PeerDiscovery>>,' api/src/state.rs

# Update the new() function signature and initialization
# This is trickier - let me show you what needs to change

echo "✓ Step 1 complete - now you need to manually update:"
echo "1. ApiState::new() to accept peer_discovery parameter"
echo "2. The place where ApiState is created to pass peer_discovery"
echo ""
echo "Check api/src/state.rs"
