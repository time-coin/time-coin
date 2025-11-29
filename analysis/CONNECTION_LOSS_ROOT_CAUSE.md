# Connection Loss Root Cause Analysis
**Date:** 2025-11-29  
**Issue:** TCP connections breaking during vote broadcast

## 🔍 The Problem

Nodes successfully broadcast votes but connections break immediately:

```
📤 Broadcasting vote to 2 peers
   Available TCP connections: 2
   Peer 161.35.129.70: connection=true
   ✓ Vote sent to 161.35.129.70
   🔄 Broken connection detected to 161.35.129.70, removing from pool
```

## 💥 Root Cause Analysis

### Initial Hypothesis (WRONG)
- Thought: ACKs not being sent back
- Reality: CLI does send ACKs properly (lines 2016-2026 in main.rs)

### Actual Root Cause

The connection breaks **DURING the send**, not while waiting for ACK!

From `network/src/manager.rs` line 1375-1430:

```rust
// Send the vote message
match conn.send_message(message.clone()).await {
    Ok(_) => {
        // Wait for acknowledgment
        match tokio::time::timeout(...).await {
            // ACK handling
        }
    }
    Err(e) => {
        // This is where it fails!
        if e.contains("Broken pipe") || e.contains("Connection reset") {
            println!("🔄 Broken connection detected");
            // Remove connection
        }
    }
}
```

### Why Connections Break

**The TCP connection is dead BEFORE we try to send!**

Possible reasons:
1. **No TCP keep-alive** - Idle connections timeout silently
2. **Firewall NAT timeout** - Stateful firewalls drop idle connections (typically 60-300 seconds)
3. **OS socket timeout** - Default TCP socket timeouts
4. **Peer restarts** - When a peer restarts, our end doesn't know immediately

### The Evidence

Looking at timing:
```
02:28:26 - Node starts
02:28:36 - Connects to peers
02:29:17 - Tries to send vote
    Result: "Broken pipe"
```

**51 seconds of idle time** before first send attempt!

Most NAT/firewall timeouts are 60-120 seconds, but some aggressive ones are 30 seconds.

## 💡 The Solution

### Short Term: TCP Keep-Alive
Enable TCP keep-alive on all sockets:
```rust
socket.set_keepalive(Some(Duration::from_secs(30)))?;
```

This sends periodic probe packets to keep the connection alive.

### Medium Term: Connection Health Checks
Periodically send lightweight ping messages:
```rust
// Every 30 seconds
send_ping_to_all_peers().await;
```

### Long Term: Automatic Reconnection
When a connection breaks, automatically reconnect:
```rust
if conn.send_message(msg).is_err() {
    // Remove dead connection
    self.remove_connection(peer_ip);
    
    // Trigger reconnect
    self.connect_to_peer(peer_ip).await?;
    
    // Retry send
    conn.send_message(msg).await?;
}
```

## 🎯 Implementation Priority

1. **Enable TCP keep-alive** (5 min fix) - HIGHEST PRIORITY
2. **Add connection health monitor** (30 min) - HIGH  
3. **Implement auto-reconnect** (1 hour) - MEDIUM
4. **Add connection state tracking** (2 hours) - LOW

## 📊 Expected Results

With TCP keep-alive enabled:
- Connections will stay alive during idle periods
- Firewall/NAT timeouts won't kill connections
- Vote broadcasts will succeed immediately
- No more "Broken pipe" errors during send

## 🔧 Files to Modify

1. `network/src/connection.rs` - Add keep-alive to socket creation
2. `network/src/manager.rs` - Add connection health monitoring
3. `cli/src/main.rs` - Handle connection recovery in listener loop
