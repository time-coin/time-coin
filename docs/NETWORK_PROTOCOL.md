# Network Protocol

## Overview

TIME Coin uses a custom peer-to-peer network protocol for node communication. The protocol ensures secure, reliable communication between nodes through magic bytes identification, handshake validation, and genesis block verification.

## Connection Magic Bytes

### Purpose

Magic bytes are 4-byte sequences that appear at the start of every network message. They serve multiple critical functions:

1. **Message Synchronization** - Help nodes identify the beginning of valid messages in the data stream
2. **Network Isolation** - Prevent mainnet nodes from accidentally connecting to testnet nodes
3. **Protocol Validation** - Quickly reject malformed or cross-network messages

### Magic Byte Values

TIME Coin uses memorable magic byte sequences inspired by the "frozen time" concept of 24-hour blocks:

#### Mainnet Magic Bytes
```
0xC01D7E4D
Hex: [0xC0, 0x1D, 0x7E, 0x4D]
Mnemonic: "COLD TIME" (C0 1D 7E 4D)
```

The mainnet magic represents "COLD TIME" - a reference to TIME Coin's frozen time concept where blocks occur every 24 hours.

#### Testnet Magic Bytes
```
0x7E577E4D
Hex: [0x7E, 0x57, 0x7E, 0x4D]
Mnemonic: "TEST TIME" (7E 57 7E 4D)
```

The testnet magic is distinct from mainnet to prevent accidental cross-network communication during development and testing.

## Message Format

All network messages follow a consistent binary format:

```
┌─────────────┬─────────────┬─────────────────────┐
│ Magic Bytes │   Length    │    JSON Payload     │
│   4 bytes   │   4 bytes   │      N bytes        │
└─────────────┴─────────────┴─────────────────────┘
```

### Structure Details

1. **Magic Bytes (4 bytes)** - Network identifier (mainnet or testnet)
2. **Length (4 bytes)** - Payload size in big-endian format (u32)
3. **JSON Payload (N bytes)** - Message content serialized as JSON

### Size Limits

- Maximum message size: 1 MB (1,048,576 bytes)
- Messages exceeding this limit are rejected to prevent memory exhaustion attacks

## Handshake Protocol

When two nodes connect, they exchange handshake messages to establish compatibility and verify network membership.

### Handshake Message Structure

```json
{
  "version": "0.1.0-9569fe2",
  "commit_date": "2025-11-07T15:09:21Z",
  "commit_count": "1234",
  "protocol_version": 1,
  "network": "Testnet",
  "listen_addr": "192.168.1.100:24100",
  "timestamp": 1730995200,
  "capabilities": ["masternode", "sync"],
  "wallet_address": "TIME1abc...",
  "genesis_hash": "00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048"
}
```

### Handshake Fields

- **version** - Software version with git hash (e.g., "0.1.0-9569fe2")
- **commit_date** - ISO 8601 timestamp of the git commit
- **commit_count** - Total number of commits in the git history
- **protocol_version** - Protocol compatibility version (currently 1)
- **network** - Network type: "Mainnet" or "Testnet"
- **listen_addr** - Node's listening address for peer connections
- **timestamp** - Unix timestamp of connection
- **capabilities** - Array of node capabilities (e.g., masternode, sync)
- **wallet_address** - Optional masternode wallet address for rewards
- **genesis_hash** - Optional genesis block hash for chain verification

### Handshake Sequence

#### Outgoing Connection

```
Client                                    Server
  │                                          │
  │──── [Magic][Len][Handshake JSON] ───────>│
  │                                          │
  │<─── [Magic][Len][Handshake JSON] ───────│
  │                                          │
  │── Validate handshake response ───────────│
  │                                          │
  └── Connection established ────────────────┘
```

1. **Send Handshake** - Client sends handshake with magic bytes
2. **Receive Handshake** - Client receives server's handshake
3. **Validation** - Client validates network, protocol, and genesis
4. **Connection Ready** - If validation passes, connection is established

#### Incoming Connection

```
Server                                    Client
  │                                          │
  │<─── [Magic][Len][Handshake JSON] ───────│
  │                                          │
  │── Validate magic bytes ─────────────────>│
  │                                          │
  │──── [Magic][Len][Handshake JSON] ───────>│
  │                                          │
  └── Connection established ────────────────┘
```

1. **Receive Handshake** - Server receives client's handshake
2. **Validate** - Server validates magic bytes and handshake
3. **Send Response** - Server sends its own handshake
4. **Connection Ready** - If validation passes, connection is established

## Validation Rules

### Magic Bytes Validation

When receiving any message, nodes:

1. Read first 4 bytes
2. Compare against expected magic for configured network
3. Reject connection if mismatch

**Example Error:**
```
Invalid magic bytes: expected [0xC0, 0x1D, 0x7E, 0x4D], got [0x7E, 0x57, 0x7E, 0x4D]
```

This error indicates a mainnet node received a message from a testnet node.

### Network Type Validation

Nodes verify the `network` field in handshake matches their configuration:

```rust
if self.network != expected_network {
    return Err(format!(
        "Network mismatch: expected {:?}, got {:?}",
        expected_network, self.network
    ));
}
```

### Protocol Version Validation

Nodes check protocol version compatibility:

```rust
if self.protocol_version != PROTOCOL_VERSION {
    return Err(format!(
        "Protocol version mismatch: expected {}, got {}",
        PROTOCOL_VERSION, self.protocol_version
    ));
}
```

Current protocol version is `1`. Future versions may introduce backward-compatible changes.

### Genesis Block Validation

When both peers provide a genesis hash, they must match:

```rust
if their_genesis != our_genesis {
    return Err(format!(
        "Genesis block mismatch: expected {}..., got {}...",
        &our_genesis[..16],
        &their_genesis[..16]
    ));
}
```

This prevents nodes from connecting to incompatible chains (e.g., after a hard fork).

**Backward Compatibility:** If either peer doesn't provide a genesis hash, validation is skipped. This allows older nodes without genesis validation to connect.

## Implementation Details

### Sending Messages

```rust
async fn send_handshake(
    stream: &mut TcpStream,
    handshake: &HandshakeMessage,
    network: &NetworkType,
) -> Result<(), String> {
    // Serialize handshake to JSON
    let json = serde_json::to_vec(handshake)?;
    let len = json.len() as u32;
    
    // Write magic bytes
    let magic = network.magic_bytes();
    stream.write_all(&magic).await?;
    
    // Write length (big-endian)
    stream.write_all(&len.to_be_bytes()).await?;
    
    // Write JSON payload
    stream.write_all(&json).await?;
    stream.flush().await?;
    
    Ok(())
}
```

### Receiving Messages

```rust
async fn receive_handshake(
    stream: &mut TcpStream,
    network: &NetworkType,
) -> Result<HandshakeMessage, String> {
    // Read magic bytes
    let mut magic_bytes = [0u8; 4];
    stream.read_exact(&mut magic_bytes).await?;
    
    // Validate magic bytes
    let expected_magic = network.magic_bytes();
    if magic_bytes != expected_magic {
        return Err(format!(
            "Invalid magic bytes: expected {:?}, got {:?}",
            expected_magic, magic_bytes
        ));
    }
    
    // Read length
    let mut len_bytes = [0u8; 4];
    stream.read_exact(&mut len_bytes).await?;
    let len = u32::from_be_bytes(len_bytes) as usize;
    
    // Validate size
    if len > 1024 * 1024 {
        return Err("Message too large".into());
    }
    
    // Read payload
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    
    // Deserialize JSON
    serde_json::from_slice(&buf)
}
```

## Example: Complete Handshake

### Wire Format (Hexadecimal)

A complete handshake message on mainnet might look like this:

```
┌─────────────────────────────────────────────────────────────┐
│ C0 1D 7E 4D                          │ Magic bytes (mainnet) │
├─────────────────────────────────────────────────────────────┤
│ 00 00 01 5A                          │ Length: 346 bytes     │
├─────────────────────────────────────────────────────────────┤
│ 7B 22 76 65 72 73 69 6F 6E 22 3A ... │ JSON payload begins   │
│ ...                                  │                       │
└─────────────────────────────────────────────────────────────┘
```

### JSON Payload

```json
{
  "version": "0.1.0-9569fe2",
  "commit_date": "2025-11-07T15:09:21Z",
  "commit_count": "1234",
  "protocol_version": 1,
  "network": "Mainnet",
  "listen_addr": "192.168.1.100:24000",
  "timestamp": 1730995200,
  "capabilities": ["masternode", "sync"],
  "wallet_address": "TIME1qw4f5g6h7j8k9l0m1n2p3q4r5s6t7u8v9w0x1y",
  "genesis_hash": "00000000839a8e6886ab5951d76f411475428afc90947ee320161bbf18eb6048"
}
```

## Version Compatibility

### Version Checking

Nodes compare version information to detect outdated software:

```rust
pub fn is_version_outdated(peer_version: &str) -> bool {
    let current_hash = GIT_HASH;
    let peer_hash = peer_version.split('-').next_back().unwrap_or("");
    
    // Different git commits mean different versions
    current_hash != peer_hash && !peer_hash.is_empty()
}
```

### Update Detection

Nodes display warnings when connecting to peers running newer versions:

```
╔══════════════════════════════════════════════════════════════╗
║  ⚠️  UPDATE AVAILABLE - NEWER VERSION DETECTED              ║
╚══════════════════════════════════════════════════════════════╝

Peer 192.168.1.100:24000 is running a NEWER version:

Peer Version:   0.1.0-abc1234 (commit #1250)
Peer Committed: 2025-11-08T10:30:00Z

Your Version:   0.1.0-9569fe2 (commit #1234)
Your Committed: 2025-11-07T15:09:21Z

⚠️  RECOMMENDED ACTION:
1. Update your node to the latest version
2. Run: git pull && cargo build --release
3. Restart your node service
```

This ensures operators stay informed about available updates and maintain network compatibility.

## Error Handling

### Connection Rejection Scenarios

Connections are rejected for the following reasons:

1. **Invalid Magic Bytes**
   ```
   Error: "Invalid magic bytes: expected [0xC0, 0x1D, 0x7E, 0x4D], got [0x7E, 0x57, 0x7E, 0x4D]"
   Reason: Testnet node tried to connect to mainnet
   ```

2. **Network Mismatch**
   ```
   Error: "Network mismatch: expected Mainnet, got Testnet"
   Reason: Handshake network field doesn't match
   ```

3. **Protocol Version Mismatch**
   ```
   Error: "Protocol version mismatch: expected 1, got 2"
   Reason: Incompatible protocol versions
   ```

4. **Genesis Block Mismatch**
   ```
   Error: "Genesis block mismatch: expected 00000000839a8e68..., got 0000000000000000..."
   Reason: Nodes are on different chains
   ```

5. **Message Too Large**
   ```
   Error: "Message too large"
   Reason: Payload exceeds 1 MB limit
   ```

### Connection Recovery

When a connection fails validation:

1. Connection is immediately closed
2. Error is logged with details
3. Peer may be temporarily banned (future feature)
4. Node continues listening for valid connections

## Security Considerations

### Magic Bytes Security

Magic bytes provide:

- **Fast rejection** of invalid connections before processing
- **Network isolation** between mainnet and testnet
- **DOS protection** by quickly filtering garbage data

However, magic bytes are **not cryptographic**. They're easily observable and should not be considered secret.

### Handshake Security

The handshake includes:

- **Genesis validation** to prevent cross-chain connections
- **Protocol version** checking for compatibility
- **Timestamp** for freshness validation (future feature)

Future enhancements may include:

- Challenge-response authentication
- Encrypted handshakes
- Peer identity verification

## Network Ports

### Default Ports

- **Mainnet P2P:** 24000 (24-hour theme)
- **Testnet P2P:** 24100
- **Mainnet RPC:** 24001
- **Testnet RPC:** 24101

### Configuration

Ports can be configured via:

1. Command-line arguments: `--port 24000`
2. Configuration file: `config.toml`
3. Environment variables: `TIME_NETWORK_PORT`

## Future Enhancements

### Planned Features

1. **Encrypted Communication** - TLS/SSL support for message privacy
2. **Peer Authentication** - Challenge-response or certificate-based auth
3. **Message Compression** - Reduce bandwidth for large messages
4. **Protocol Versioning** - Backward-compatible protocol evolution
5. **NAT Traversal** - UPnP/STUN support for firewalled nodes

### Protocol Evolution

Future protocol versions will maintain backward compatibility where possible. Major breaking changes will increment the protocol version number.

## Reference Implementation

### Source Files

- `network/src/protocol.rs` - Protocol constants and structures
- `network/src/connection.rs` - Connection and handshake implementation

### Magic Bytes Definition

```rust
/// Magic bytes for network message identification
pub mod magic_bytes {
    /// Mainnet magic bytes: 0xC01D7E4D ("COLD TIME")
    pub const MAINNET: [u8; 4] = [0xC0, 0x1D, 0x7E, 0x4D];
    
    /// Testnet magic bytes: 0x7E577E4D ("TEST TIME")
    pub const TESTNET: [u8; 4] = [0x7E, 0x57, 0x7E, 0x4D];
}
```

### Protocol Version

```rust
/// Protocol version for compatibility checking
pub const PROTOCOL_VERSION: u32 = 1;
```

## Conclusion

TIME Coin's network protocol provides a robust foundation for peer-to-peer communication with magic bytes identification, comprehensive handshake validation, and genesis block verification. The protocol ensures network isolation, version compatibility, and chain integrity while maintaining simplicity and efficiency.

For questions or contributions, see the [main documentation](README.md) or visit the [TIME Coin forum](https://forum.time-coin.io).
