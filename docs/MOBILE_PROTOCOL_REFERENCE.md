# TIME Coin Mobile Protocol Reference

## Overview

This document describes the TCP protocol messages for mobile wallet integration.

**Port**: 24100 (testnet), 24101 (mainnet)  
**Format**: Length-prefixed JSON over TCP  
**Encoding**: UTF-8 JSON

## Message Format

All messages use this wire format:

```
┌────────────────┬──────────────────────┐
│ Length (4 byte)│ JSON Message         │
│ Big-endian u32 │ UTF-8 encoded        │
└────────────────┴──────────────────────┘
```

### Example: RegisterXpub

**Wire format:**
```
[0x00, 0x00, 0x00, 0x4C]  // Length: 76 bytes
[JSON payload]             // {"RegisterXpub":{"xpub":"xpub..."}}
```

## Protocol Messages

### 1. RegisterXpub (Client → Server)

Register an xpub for real-time notifications.

**Request:**
```json
{
  "RegisterXpub": {
    "xpub": "xpub6CUGRUonZSQ4TWtTMmzXdrXDtyPWKiKbERr4d5qkSmh5h17d1t3S..."
  }
}
```

**Response:**
```json
{
  "XpubRegistered": {
    "success": true,
    "message": "xPub registered successfully. Monitoring 20 addresses."
  }
}
```

**Error Response:**
```json
{
  "XpubRegistered": {
    "success": false,
    "message": "Invalid xpub format"
  }
}
```

**What happens:**
1. Server derives first 20 addresses from xpub (BIP-44 gap limit)
2. Server scans blockchain for historical transactions
3. Server subscribes to future transactions for those addresses
4. Server sends `UtxoUpdate` with current state

---

### 2. UtxoUpdate (Server → Client)

Sent after xpub registration with current wallet state.

**Message:**
```json
{
  "UtxoUpdate": {
    "xpub": "xpub6CUGRUonZSQ4TWtTMmzXdrXDtyPWKiKbERr4d5qkSmh5h17d1t3S...",
    "utxos": [
      {
        "txid": "a1b2c3d4e5f6...",
        "vout": 0,
        "address": "TIME1mkLKMVtAbyBefLRcn8cLqjaxbVuBYheeYB",
        "amount": 100000000,
        "block_height": 1234,
        "confirmations": 5
      },
      {
        "txid": "f6e5d4c3b2a1...",
        "vout": 1,
        "address": "TIME1anotherAddressHere123456789",
        "amount": 50000000,
        "block_height": 1235,
        "confirmations": 4
      }
    ]
  }
}
```

**Field Descriptions:**
- `txid`: Transaction ID (hex string)
- `vout`: Output index in transaction
- `address`: TIME Coin address receiving the UTXO
- `amount`: Amount in satoshis (1 TIME = 100,000,000 satoshis)
- `block_height`: Block number where tx was confirmed (null if unconfirmed)
- `confirmations`: Number of confirmations (0 if in mempool)

---

### 3. NewTransactionNotification (Server → Client)

Sent in real-time when a new transaction is detected for a registered address.

**Message:**
```json
{
  "NewTransactionNotification": {
    "transaction": {
      "tx_hash": "abc123def456...",
      "from_address": "TIME1senderAddressHere123456789",
      "to_address": "TIME1mkLKMVtAbyBefLRcn8cLqjaxbVuBYheeYB",
      "amount": 50000000,
      "timestamp": 1732034400,
      "block_height": 1236,
      "confirmations": 0
    }
  }
}
```

**Field Descriptions:**
- `tx_hash`: Transaction ID
- `from_address`: Sender address
- `to_address`: Recipient address (one of your addresses)
- `amount`: Amount sent to `to_address` in satoshis
- `timestamp`: Unix timestamp (seconds since epoch)
- `block_height`: Block height (0 if unconfirmed)
- `confirmations`: Number of confirmations

**When this is sent:**
- Transaction enters mempool (confirmations: 0)
- Transaction gets confirmed in block (confirmations: 1+)
- Each subsequent block (confirmations: 2, 3, 4...)

---

### 4. Ping / Pong (Keepalive)

Keep connection alive and detect disconnections.

**Server → Client:**
```json
{ "Ping": {} }
```

**Client → Server:**
```json
{ "Pong": {} }
```

**Frequency**: Every 30 seconds from server  
**Timeout**: If no response within 60 seconds, connection is closed

---

### 5. RequestWalletTransactions (Client → Server)

Request historical transactions for an xpub (alternative to RegisterXpub).

**Request:**
```json
{
  "RequestWalletTransactions": {
    "xpub": "xpub6CUGRUonZSQ4TWtTMmzXdrXDtyPWKiKbERr4d5qkSmh5h17d1t3S..."
  }
}
```

**Response:**
```json
{
  "WalletTransactionsResponse": {
    "transactions": [
      {
        "tx_hash": "abc123...",
        "from_address": "TIME1sender...",
        "to_address": "TIME1recipient...",
        "amount": 100000000,
        "timestamp": 1732034400,
        "block_height": 1234,
        "confirmations": 5
      }
    ],
    "last_synced_height": 1239
  }
}
```

**Note**: This is a one-time query. Use `RegisterXpub` for ongoing notifications.

---

## Connection Flow

### Initial Connection & Sync

```
1. Client connects to masternode:24100
2. Client → Server: RegisterXpub
3. Server → Client: XpubRegistered (success: true)
4. Server → Client: UtxoUpdate (current wallet state)
5. Connection stays open for real-time updates
```

### Real-Time Notification Flow

```
1. Transaction broadcast to network
2. Masternode detects transaction to registered address
3. Server → Client: NewTransactionNotification (confirmations: 0)
4. Client displays: "Incoming: 0.5 TIME (pending)"
5. Transaction included in block
6. Server → Client: NewTransactionNotification (confirmations: 1)
7. Client displays: "Received: 0.5 TIME (1 confirmation)"
```

### Keepalive Flow

```
Every 30 seconds:
1. Server → Client: Ping
2. Client → Server: Pong
```

---

## Implementation Examples

### Kotlin (Android)

```kotlin
class TimeCoinProtocol(private val xpub: String) {
    private lateinit var socket: Socket
    private lateinit var input: InputStream
    private lateinit var output: OutputStream
    
    suspend fun connect(host: String, port: Int = 24100) = withContext(Dispatchers.IO) {
        socket = Socket(host, port)
        input = socket.getInputStream()
        output = socket.getOutputStream()
        
        // Register xpub
        send(JSONObject().apply {
            put("RegisterXpub", JSONObject().apply {
                put("xpub", xpub)
            })
        })
        
        // Wait for confirmation
        val response = receive()
        val registered = response.getJSONObject("XpubRegistered")
        if (!registered.getBoolean("success")) {
            throw IOException("Registration failed: ${registered.getString("message")}")
        }
        
        // Start listening
        startReceiving()
    }
    
    private fun send(message: JSONObject) {
        val bytes = message.toString().toByteArray(Charsets.UTF_8)
        val length = ByteBuffer.allocate(4).putInt(bytes.size).array()
        output.write(length)
        output.write(bytes)
        output.flush()
    }
    
    private fun receive(): JSONObject {
        // Read 4-byte length prefix
        val lengthBytes = ByteArray(4)
        var bytesRead = 0
        while (bytesRead < 4) {
            val n = input.read(lengthBytes, bytesRead, 4 - bytesRead)
            if (n == -1) throw IOException("Connection closed")
            bytesRead += n
        }
        val length = ByteBuffer.wrap(lengthBytes).int
        
        // Read message
        val messageBytes = ByteArray(length)
        bytesRead = 0
        while (bytesRead < length) {
            val n = input.read(messageBytes, bytesRead, length - bytesRead)
            if (n == -1) throw IOException("Connection closed")
            bytesRead += n
        }
        
        return JSONObject(String(messageBytes, Charsets.UTF_8))
    }
    
    private suspend fun startReceiving() = withContext(Dispatchers.IO) {
        while (socket.isConnected) {
            try {
                val message = receive()
                handleMessage(message)
            } catch (e: Exception) {
                Log.e("Protocol", "Error: $e")
                break
            }
        }
    }
    
    private fun handleMessage(message: JSONObject) {
        when {
            message.has("NewTransactionNotification") -> {
                val tx = message.getJSONObject("NewTransactionNotification")
                    .getJSONObject("transaction")
                onNewTransaction(tx)
            }
            message.has("UtxoUpdate") -> {
                val update = message.getJSONObject("UtxoUpdate")
                onUtxoUpdate(update)
            }
            message.has("Ping") -> {
                send(JSONObject().apply { put("Pong", JSONObject()) })
            }
        }
    }
    
    private fun onNewTransaction(tx: JSONObject) {
        val amount = tx.getLong("amount")
        val toAddress = tx.getString("to_address")
        val confirmations = tx.getInt("confirmations")
        
        Log.i("Wallet", "New TX: $amount satoshis to $toAddress ($confirmations conf)")
        // Update UI, show notification, etc.
    }
    
    private fun onUtxoUpdate(update: JSONObject) {
        val utxos = update.getJSONArray("utxos")
        Log.i("Wallet", "Received ${utxos.length()} UTXOs")
        // Update wallet balance, transaction history, etc.
    }
}
```

### Swift (iOS - Future)

```swift
class TimeCoinProtocol {
    private var inputStream: InputStream?
    private var outputStream: OutputStream?
    
    func connect(host: String, port: Int = 24100) {
        Stream.getStreamsToHost(
            withName: host,
            port: port,
            inputStream: &inputStream,
            outputStream: &outputStream
        )
        
        inputStream?.open()
        outputStream?.open()
        
        // Register xpub
        let message: [String: Any] = [
            "RegisterXpub": ["xpub": xpub]
        ]
        send(message: message)
        
        // Start receiving
        startReceiving()
    }
    
    private func send(message: [String: Any]) {
        guard let data = try? JSONSerialization.data(withJSONObject: message),
              let output = outputStream else { return }
        
        // Send length (4 bytes, big-endian)
        var length = UInt32(data.count).bigEndian
        withUnsafeBytes(of: &length) { bytes in
            output.write(bytes.bindMemory(to: UInt8.self).baseAddress!, maxLength: 4)
        }
        
        // Send message
        data.withUnsafeBytes { bytes in
            output.write(bytes.bindMemory(to: UInt8.self).baseAddress!, maxLength: data.count)
        }
    }
    
    // ... receive implementation
}
```

---

## Error Handling

### Connection Errors

- **Connection refused**: Masternode offline or wrong port
- **Timeout**: Network issues or firewall
- **Invalid xpub**: Server returns `XpubRegistered.success: false`

**Recommended**: Implement exponential backoff for reconnection:
```
Attempt 1: 1 second delay
Attempt 2: 2 seconds delay
Attempt 3: 4 seconds delay
...
Max delay: 60 seconds
```

### Message Errors

- **Invalid JSON**: Malformed message from server
- **Unexpected message type**: Protocol version mismatch
- **Message too large**: Size limit is 10 MB

---

## Security Considerations

1. **TLS/SSL**: Currently not implemented. Consider using SSH tunnel or VPN.
2. **xpub Privacy**: xpub reveals all addresses and balances. Never share publicly.
3. **Connection Hijacking**: Validate server identity (future: certificate pinning)
4. **Message Injection**: Validate all received data before processing

---

## Testing

### Using `netcat` (Linux/Mac)

```bash
# Connect to testnet
nc time-coin.io 24100

# Send RegisterXpub (manual)
echo -ne '\x00\x00\x00\x4C{"RegisterXpub":{"xpub":"xpub..."}}' | nc time-coin.io 24100
```

### Using `telnet`

```bash
telnet time-coin.io 24100
# Then paste JSON message
```

---

## Protocol Versioning

**Current Version**: 1.0  
**Compatibility**: All TIME Coin masternodes v0.1.0+

Future versions will include protocol version negotiation.

---

## FAQ

**Q: Can I connect to multiple masternodes simultaneously?**  
A: Yes, for redundancy. Use first successful connection.

**Q: What happens if connection drops?**  
A: Client should reconnect and re-register xpub. Server sends current state.

**Q: How long do connections stay open?**  
A: Indefinitely, with 30-second ping/pong keepalive.

**Q: Can I use WebSocket instead of raw TCP?**  
A: No, WebSocket support was removed. Use TCP or HTTP API.

**Q: Do I need to re-register xpub after reconnecting?**  
A: Yes, subscriptions are per-connection, not persistent.

---

## See Also

- `ANDROID_APP_QUICKSTART.md` - Quick start guide for Android developers
- `MOBILE_NOTIFICATION_STRATEGY.md` - Overall mobile app architecture
- `wallet-push-notifications.md` - Legacy WebSocket documentation (outdated)
- `TIME_COIN_PROTOCOL.md` - Full protocol specification
