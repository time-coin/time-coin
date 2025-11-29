# Why Are Connections Closing?

## Root Cause Analysis

Looking at the logs, connections are closing with "Broken pipe" errors. This happens when:

### **1. TCP Connection Timeout**
**Most Likely Cause**

The OS closes idle TCP connections after a certain period. On Linux:
- Default TCP keepalive: **7200 seconds (2 hours)**
- But routers/firewalls often close earlier: **60-120 seconds**

**Our current behavior:**
- We send **ping every 30 seconds**
- But if the connection is **busy handling other requests**, no data flows
- NAT/firewall sees "no activity" and closes the connection
- Next operation gets "Broken pipe"

### **2. Simultaneous Read/Write Conflicts**
**Likely Contributing Factor**

Looking at the logs:
```
Broadcasting proposal to 3 peers
Broadcasting vote to 3 peers
🔄 Broken connection detected to 165.232.154.150
```

Multiple operations happening at once:
- Heartbeat trying to ping
- Consensus trying to send proposal
- Vote being broadcast
- Sync requesting blockchain info

**TCP stream is NOT thread-safe** - simultaneous writes can corrupt the stream!

### **3. Long Operations Without Activity**

```rust
// In connection.rs:
pub async fn receive_message(&mut self) -> Result<NetworkMessage, String> {
    // Read 4 bytes for length
    let mut len_buf = [0u8; 4];
    self.stream.read_exact(&mut len_buf).await  // ← Blocks here
        .map_err(|e| format!("Failed to read length: {}", e))?;
```

If a peer is **slow to respond**, the connection appears idle to the OS/firewall, which then closes it.

### **4. OS/Firewall Connection Tracking**

Most firewalls use **connection tracking** with timeouts:
- **AWS/Digital Ocean**: 350-600 seconds for established connections
- **Home routers**: 60-300 seconds
- **Corporate firewalls**: 30-120 seconds

If no packets flow for this duration → connection dropped by firewall.

### **5. Peer Process Crashes/Restarts**

When a peer restarts:
- Old TCP connection becomes invalid
- Next write attempt returns "Broken pipe"

## Evidence from Logs

```
Nov 29 04:30:13: Failed to broadcast UpdateTip to 165.232.154.150: Connection broken
```

This happens **during active use** (broadcasting), not during idle time → suggests:
1. Connection was already dead (firewall timeout)
2. Or simultaneous write conflict corrupted the stream

## The Real Problem

**We're using a SINGLE TCP connection for ALL communication:**
- Heartbeats (ping/pong every 30s)
- Block requests/responses
- Transaction broadcasts
- Consensus proposals/votes
- Blockchain info queries

**When the connection is busy:**
- Heartbeat ping might be queued
- Firewall sees "no activity" on the socket
- Connection tracking timeout expires
- Next operation: "Broken pipe"

## Industry Standard Solutions

### **Bitcoin Core Approach:**
- **Application-level keepalive**: Send version/verack messages
- **Timeouts on all operations**: 20-60 second max per request
- **Connection pooling**: Multiple connections per peer
- **Circuit breaker**: Drop connection after N failures

### **Ethereum (geth) Approach:**
- **Separate connections** for different protocols
- **Heartbeat at protocol level** (not just TCP)
- **Aggressive timeout**: 30 seconds per request
- **Connection health scoring**

### **HTTP/2 Approach:**
- **Ping frames** every 30 seconds
- **GOAWAY frames** for graceful shutdown
- **Stream multiplexing** (multiple requests on one connection)

## Recommendations

### **Immediate (Phase 1):**
1. ✅ **Reduce TCP keepalive interval**: 30s → 15s OS level
2. ✅ **Add connection locking**: Prevent simultaneous writes
3. ✅ **Add write timeouts**: Max 5s per write operation

### **Short-term (Phase 2):**
4. **Application-level ping**: Send ping as first priority, bypass queue
5. **Connection health tracking**: Score based on success rate
6. **Proactive replacement**: Replace poor connections before they break

### **Long-term (Phase 3):**
7. **Multiple connections per peer**: One for sync, one for consensus
8. **Connection pooling**: Maintain 2-3 connections to important peers
9. **Protocol multiplexing**: Use message IDs to multiplex on single connection

## Quick Win: TCP Keepalive Tuning

Current OS defaults (Linux):
```
net.ipv4.tcp_keepalive_time = 7200   # 2 hours - WAY too long
net.ipv4.tcp_keepalive_intvl = 75    # 75 seconds between probes
net.ipv4.tcp_keepalive_probes = 9    # 9 probes before declaring dead
```

For blockchain P2P, we want:
```
net.ipv4.tcp_keepalive_time = 60     # 60 seconds - first probe
net.ipv4.tcp_keepalive_intvl = 10    # 10 seconds between probes
net.ipv4.tcp_keepalive_probes = 3    # 3 probes = 30s to detect dead
```

Or set per-socket in Rust:
```rust
use socket2::{Socket, TcpKeepalive};

let keepalive = TcpKeepalive::new()
    .with_time(Duration::from_secs(60))
    .with_interval(Duration::from_secs(10));
    
socket.set_tcp_keepalive(&keepalive)?;
```

## The Real Fix: Connection Write Locking

The most likely issue is **simultaneous writes** corrupting the stream. We need:

```rust
struct PeerConnection {
    stream: Arc<Mutex<TcpStream>>,  // Prevent simultaneous writes
    read_half: Arc<Mutex<ReadHalf>>,
    write_half: Arc<Mutex<WriteHalf>>,
}
```

This ensures only ONE write at a time, preventing corruption.
