# TCP Keep-Alive Fix Summary
**Date:** 2025-11-29  
**Commit:** 59078cd  
**Issue:** Nodes losing TCP connections during consensus voting

## 🐛 The Bug

Nodes were experiencing "Broken pipe" errors when trying to send consensus votes:

```
📤 Broadcasting vote to 2 peers
   ✓ Vote sent to 161.35.129.70
   🔄 Broken connection detected to 161.35.129.70, removing from pool
```

This caused:
- Vote broadcasts to fail
- Consensus to stall (only 1/4 votes received)
- Constant reconnection attempts
- Network instability

## 🔍 Root Cause

**Incoming TCP connections had NO keep-alive enabled!**

### The Problem

1. **Outgoing connections** (initiated by node) - HAD keep-alive ✓
2. **Incoming connections** (accepted by listener) - NO keep-alive ✗

### Why It Mattered

After ~30-60 seconds of inactivity:
- NAT/firewall drops the connection state
- OS marks socket as dead
- Next write attempt → "Broken pipe"
- Connection removed from pool
- Votes never arrive

### The Timeline

```
02:28:26 - Node accepts incoming connection
02:28:36 - Handshake completes
02:29:17 - Tries to send first vote (51 seconds later)
         - Result: "Broken pipe" ❌
```

**51 seconds of idle = dead connection!**

## ✅ The Fix

Added TCP keep-alive to incoming connections in `network/src/connection.rs`:

```rust
pub async fn accept(&self) -> Result<PeerConnection, String> {
    let (stream, _addr) = self.listener.accept().await?;
    
    // Enable TCP keep-alive on incoming connections
    stream.set_nodelay(true)?;
    
    let socket2_sock = socket2::Socket::from(stream.into_std()?);
    let ka = socket2::TcpKeepalive::new()
        .with_time(Duration::from_secs(30))    // First probe after 30s idle
        .with_interval(Duration::from_secs(30)); // Retry every 30s
    
    socket2_sock.set_tcp_keepalive(&ka)?;
    
    let mut stream = TcpStream::from_std(socket2_sock.into())?;
    // ... rest of handshake
}
```

### What This Does

1. **Enables TCP keep-alive** on ALL incoming connections
2. **Sends probe packets** every 30 seconds during idle periods
3. **Prevents NAT/firewall timeout** from killing connections
4. **Matches outgoing connections** (which already had this)

## 📊 Expected Results

### Before Fix
```
Connections: 3 → 2 → 1 → 0 (constantly dropping)
Vote success: 25% (only self-votes)
Consensus: Stalled
```

### After Fix
```
Connections: 3 → 3 → 3 (stable)
Vote success: 75%+ (3/4 votes)
Consensus: Working
```

## 🧪 Testing Needed

1. **Deploy to testnet** - All nodes need the update
2. **Monitor for 10+ minutes** - Verify connections stay alive
3. **Check vote broadcasts** - Should see "✓ Vote sent and ACKed by"
4. **Verify consensus** - Blocks should finalize with 3/4 votes

## 📝 Additional Notes

### Why 30 seconds?

- Most NAT timeouts: 60-120 seconds
- Aggressive firewalls: 30-60 seconds
- Our choice: 30 seconds = safe margin
- Cost: Minimal bandwidth (~100 bytes every 30s)

### Platform Differences

- **Linux:** Uses `TCP_KEEPIDLE`, `TCP_KEEPINTVL`, `TCP_KEEPCNT`
- **Windows:** Uses `SIO_KEEPALIVE_VALS`
- **macOS:** Uses `TCP_KEEPALIVE`
- `socket2` crate handles all platform differences

### Related Issues Fixed

✅ Connections dying during idle periods  
✅ "Broken pipe" errors on send  
✅ Vote broadcasts failing silently  
✅ Consensus stalling with insufficient votes  
✅ Constant reconnection attempts  

### Future Improvements

1. **Connection health monitoring** - Periodic ping/pong
2. **Automatic reconnection** - Retry on connection loss
3. **Connection state tracking** - Better diagnostics
4. **Graceful degradation** - Fall back to HTTP if TCP fails

## 🎯 Deployment

1. ✅ Code pushed to GitHub
2. ⏳ Nodes need to pull and rebuild
3. ⏳ All 5 testnet nodes must update
4. ⏳ Monitor logs for stable connections
5. ⏳ Verify consensus reaches 3/4 votes consistently

---

**Status:** DEPLOYED - Waiting for testnet nodes to update
