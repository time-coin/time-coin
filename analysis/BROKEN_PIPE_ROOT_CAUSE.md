# Broken Pipe Root Cause Analysis & Fix

## Problem Discovered
```
✗ Failed to send proposal to 178.128.199.144: Broken pipe (os error 32)
✗ Failed to send vote to 134.199.175.106: Broken pipe (os error 32)
```

Consensus voting was failing because 3-4 out of 5 peers consistently returned "Broken pipe" errors when broadcasting proposals and votes, even though the connection map showed `connection=true`.

## Root Cause: Incomplete Ping Implementation

### The Flawed Ping Function

**Before (network/src/connection.rs):**
```rust
pub async fn ping(&mut self) -> Result<(), String> {
    let msg = crate::protocol::NetworkMessage::Ping;
    let json = serde_json::to_vec(&msg)?;
    let len = json.len() as u32;
    self.stream.write_all(&len.to_be_bytes()).await?;
    self.stream.write_all(&json).await?;
    self.stream.flush().await?;
    Ok(())  // ⚠️ Returns success without waiting for pong!
}
```

**The Critical Flaw:**
- Ping only **sends** a message
- **Never waits for pong response**
- Only fails if TCP socket write fails
- Cannot detect "half-dead" connections

### What's a "Half-Dead" Connection?

A connection where:
1. ✅ TCP socket accepts writes (so `ping()` returns Ok)
2. ❌ Remote end isn't actually processing messages
3. ❌ Messages pile up in TCP send buffer
4. ❌ Eventually buffer fills and pipe breaks

This happens when:
- Remote process is overloaded/hung
- Network congestion prevents message delivery
- Remote end closed connection but TCP hasn't detected it yet
- TCP keep-alive hasn't triggered yet (can take minutes)

### Why Heartbeats Weren't Working

The keep-alive loop (every 30 seconds) does:
```rust
let ping_result = conn_guard.ping().await;
match ping_result {
    Ok(_) => {
        consecutive_failures = 0;  // Reset!
        manager.peer_seen(peer_ip).await;
    }
    Err(e) => {
        consecutive_failures += 1;
        if consecutive_failures >= 3 {
            break;  // Remove connection
        }
    }
}
```

**Problem:** `ping()` returns `Ok` even when remote isn't responding, so:
- `consecutive_failures` stays at 0
- Connection appears healthy
- **But messages are silently piling up in buffers**
- When broadcast tries to send, buffer is full → **Broken pipe!**

## The Fix

### 1. Proper Ping-Pong Validation

**After (network/src/connection.rs):**
```rust
pub async fn ping(&mut self) -> Result<(), String> {
    // Send ping
    let msg = crate::protocol::NetworkMessage::Ping;
    let json = serde_json::to_vec(&msg)?;
    let len = json.len() as u32;
    self.stream.write_all(&len.to_be_bytes()).await?;
    self.stream.write_all(&json).await?;
    self.stream.flush().await?;

    // ✅ Wait for pong response with timeout
    let pong_result = tokio::time::timeout(
        std::time::Duration::from_secs(5),
        self.receive_message(),
    ).await;

    match pong_result {
        Ok(Ok(NetworkMessage::Pong)) => Ok(()),
        Ok(Ok(_)) => Err("Unexpected response (expected Pong)".to_string()),
        Ok(Err(e)) => Err(format!("Pong receive error: {}", e)),
        Err(_) => Err("Pong timeout (5s)".to_string()),
    }
}
```

**Key Changes:**
- ✅ Sends ping AND waits for pong
- ✅ 5-second timeout for pong response
- ✅ Validates bidirectional communication
- ✅ Detects half-dead connections immediately

### 2. Automatic Broken Connection Cleanup

**Enhanced send_to_peer_tcp() (network/src/manager.rs):**
```rust
pub async fn send_to_peer_tcp(&self, peer_ip: IpAddr, message: NetworkMessage) 
    -> Result<(), String> 
{
    let conn_arc = {
        let connections = self.connections.read().await;
        connections.get(&peer_ip).cloned()
    };

    if let Some(conn_arc) = conn_arc {
        let mut conn = conn_arc.lock().await;
        match conn.send_message(message).await {
            Ok(_) => Ok(()),
            Err(e) => {
                // ✅ Detect broken pipe and clean up
                if e.contains("Broken pipe") || e.contains("Connection reset") {
                    println!("   🔄 Broken connection detected to {}, removing", peer_ip);
                    drop(conn);
                    
                    // Remove stale connection
                    let mut connections = self.connections.write().await;
                    connections.remove(&peer_ip);
                    drop(connections);
                    
                    // Mark for reconnection
                    self.remove_connected_peer(&peer_ip).await;
                }
                Err(e)
            }
        }
    } else {
        Err("No TCP connection available".to_string())
    }
}
```

**Benefits:**
- ✅ Detects broken pipe errors
- ✅ Automatically removes stale connections
- ✅ Triggers reconnection via existing reconnection task
- ✅ Prevents repeated failures to same dead connection

## Expected Behavior After Fix

### Healthy Connection Detection
```
Keep-alive loop (every 30s):
1. Send ping
2. Wait up to 5s for pong
3. If pong received → connection healthy, reset failure counter
4. If timeout/error → increment failure counter
5. After 3 failures (90s) → remove connection and reconnect
```

### Immediate Broken Pipe Handling
```
When broadcast detects broken pipe:
1. Log: "🔄 Broken connection detected to X, removing"
2. Remove from connections map
3. Remove from peers map  
4. Reconnection task will detect missing peer
5. Automatic reconnection within 2 minutes
```

### Vote Broadcast Success
```
📤 Broadcasting vote to 5 peers
   Available TCP connections: 5
   Peer 69.167.168.176: connection=true
   ✓ Vote sent to 69.167.168.176
   Peer 50.28.104.50: connection=true  
   ✓ Vote sent to 50.28.104.50
   ... (all succeed) ...
   📊 Vote broadcast: 5 successful, 0 failed

🗳️  Received block vote from 69.167.168.176 for block #1
   ✅ Vote recorded successfully
🗳️  Received block vote from 50.28.104.50 for block #1
   ✅ Vote recorded successfully
```

## Why This Was Hard to Diagnose

1. **Silent Failures**: Original ping "succeeded" while connections were dying
2. **Race Condition**: Connection appeared healthy in map but was already dead
3. **Timing**: Only manifested during high-frequency broadcasts (catch-up)
4. **Intermittent**: Some messages got through before buffer filled

## Testing the Fix

After deploying:
1. **Check ping failures in logs** - should see actual ping failures now
2. **Monitor connection cleanup** - broken connections removed within 90s
3. **Watch vote broadcasts** - should see 5/5 or 4/5 success (not 1/5 or 2/5)
4. **Verify consensus** - catch-up should succeed with 4+ votes

## Long-term Benefits

- **Faster failure detection**: 5s instead of indefinite
- **Prevents cascading failures**: Broken connections removed immediately
- **Bidirectional validation**: Both send and receive verified
- **Self-healing**: Automatic reconnection to recovered peers
- **Accurate connection state**: Map reflects reality

---
Date: 2025-11-28
Commits: c434166, aaae601, f924783
