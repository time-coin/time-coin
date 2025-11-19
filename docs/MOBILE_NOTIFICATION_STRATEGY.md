# Mobile Notification Strategy for TIME Coin Android App

## Current Architecture (Post-WebSocket Removal)

TIME Coin now uses **TCP Protocol** for real-time wallet notifications:

- **Port**: 24100 (testnet), 24101 (mainnet)
- **Protocol**: Length-prefixed JSON messages over TCP
- **Messages**: `RegisterXpub`, `NewTransactionNotification`, `UtxoUpdate`
- **Working**: Already implemented and used by wallet-gui

## Problem: TCP on Mobile

❌ **TCP doesn't work well for mobile apps because:**
1. Android kills background connections to save battery
2. Mobile networks disconnect frequently (WiFi ↔ cellular)
3. Apps can't maintain persistent TCP connections when backgrounded
4. Battery drain from keeping sockets alive

## ✅ Recommended Solution: Hybrid Architecture

### Architecture Overview

```
┌─────────────────────────────────────────────────────────────┐
│                    Android App States                        │
├─────────────────────────────────────────────────────────────┤
│                                                               │
│  FOREGROUND                    BACKGROUND                    │
│  ┌──────────────┐              ┌──────────────┐            │
│  │  TCP Direct  │              │     FCM      │            │
│  │  Connection  │──┐           │Push Notif.   │            │
│  │ (instant <1s)│  │           │(wake app)    │            │
│  └──────────────┘  │           └──────────────┘            │
│                     │                   │                     │
│                     ▼                   ▼                     │
│              ┌────────────────────────────┐                  │
│              │   Masternode (24100)      │                  │
│              │  - TCP Protocol           │                  │
│              │  - FCM Relay              │                  │
│              │  - Transaction Monitor    │                  │
│              └────────────────────────────┘                  │
└─────────────────────────────────────────────────────────────┘
```

### When App is in FOREGROUND:
1. ✅ Use **TCP direct connection** (existing protocol)
2. ✅ Connect to masternode port 24100
3. ✅ Send `RegisterXpub` message
4. ✅ Receive `NewTransactionNotification` in real-time
5. ✅ Instant updates (< 1 second latency)
6. ✅ No backend changes needed - already works!

### When App is in BACKGROUND:
1. ✅ Use **Firebase Cloud Messaging (FCM)**
2. ✅ Masternode sends push notification via FCM
3. ✅ Android OS wakes app even when closed
4. ✅ App fetches transaction details via HTTP API
5. ✅ Battery efficient (no persistent connection)

## Implementation Plan

### Phase 1: Android App (No Backend Changes Needed)

The Android app can start working **immediately** using existing infrastructure:

```kotlin
// Foreground: Direct TCP connection
class TcpProtocolClient(private val xpub: String) {
    private var socket: Socket? = null
    
    suspend fun connect(masternodeIp: String) {
        // Connect to port 24100 (testnet)
        socket = Socket(masternodeIp, 24100)
        
        // Send RegisterXpub
        val message = JSONObject().apply {
            put("RegisterXpub", JSONObject().apply {
                put("xpub", xpub)
            })
        }
        sendMessage(message)
        
        // Listen for notifications
        while (isActive) {
            val notification = receiveMessage()
            if (notification.has("NewTransactionNotification")) {
                handleNewTransaction(notification)
            }
        }
    }
    
    private fun sendMessage(msg: JSONObject) {
        val json = msg.toString().toByteArray()
        val length = ByteBuffer.allocate(4).putInt(json.size).array()
        socket?.getOutputStream()?.apply {
            write(length)
            write(json)
            flush()
        }
    }
    
    private fun receiveMessage(): JSONObject {
        val input = socket?.getInputStream() ?: throw IOException("Socket closed")
        
        // Read 4-byte length prefix
        val lengthBytes = ByteArray(4)
        input.read(lengthBytes)
        val length = ByteBuffer.wrap(lengthBytes).int
        
        // Read message
        val messageBytes = ByteArray(length)
        input.read(messageBytes)
        
        return JSONObject(String(messageBytes))
    }
}
```

**Usage in Android:**
```kotlin
// In Activity/Fragment when app is visible
lifecycleScope.launch {
    val client = TcpProtocolClient(wallet.xpub)
    client.connect("masternode.time-coin.io")
}

// Automatically disconnect when app goes to background
```

### Phase 2: Add FCM Backend Support (Backend Changes)

Add FCM push notifications for when app is backgrounded:

#### 2.1 Add FCM Device Registration Endpoint

**File**: `api/src/routes.rs`
```rust
.route("/wallet/register_fcm_device", post(register_fcm_device))
.route("/wallet/unregister_fcm_device", delete(unregister_fcm_device))
```

**File**: `api/src/fcm_handlers.rs` (new file)
```rust
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct RegisterFcmDeviceRequest {
    pub xpub: String,
    pub fcm_token: String,
    pub platform: String, // "android"
}

#[derive(Serialize)]
pub struct RegisterFcmDeviceResponse {
    pub success: bool,
    pub message: String,
}

pub async fn register_fcm_device(
    State(state): State<ApiState>,
    Json(request): Json<RegisterFcmDeviceRequest>,
) -> Result<Json<RegisterFcmDeviceResponse>, ApiError> {
    // Store: xpub -> fcm_token mapping in database
    // When transaction detected for xpub addresses, send FCM push
    
    println!("📱 Registered FCM device for xpub: {}...", &request.xpub[..10]);
    
    Ok(Json(RegisterFcmDeviceResponse {
        success: true,
        message: "Device registered for push notifications".to_string(),
    }))
}
```

#### 2.2 Add FCM Notification Sender

**File**: `api/Cargo.toml`
```toml
[dependencies]
fcm = "0.9"  # Firebase Cloud Messaging client
```

**File**: `api/src/fcm_sender.rs` (new file)
```rust
use fcm::{Client, MessageBuilder, NotificationBuilder};

pub struct FcmNotificationSender {
    client: Client,
}

impl FcmNotificationSender {
    pub fn new(server_key: String) -> Self {
        Self {
            client: Client::new(),
        }
    }
    
    pub async fn send_transaction_notification(
        &self,
        fcm_token: &str,
        tx: &WalletTransaction,
    ) -> Result<(), String> {
        let notification = NotificationBuilder::new()
            .title("💰 TIME Coin Received")
            .body(&format!("Received {} TIME", tx.amount))
            .build();
        
        let message = MessageBuilder::new(fcm_token)
            .notification(notification)
            .data("txid", &tx.tx_hash)
            .data("amount", &tx.amount.to_string())
            .data("from_address", &tx.from_address)
            .build();
        
        self.client.send(message).await
            .map_err(|e| format!("FCM error: {}", e))?;
        
        Ok(())
    }
}
```

#### 2.3 Integrate FCM into Transaction Monitoring

**File**: `masternode/src/utxo_integration.rs` (modify existing)
```rust
// When new transaction detected for registered xpub
if let Some(fcm_sender) = &state.fcm_sender {
    if let Some(fcm_token) = get_fcm_token_for_xpub(&xpub).await {
        // Send FCM push notification
        fcm_sender.send_transaction_notification(&fcm_token, &tx).await;
        println!("📲 Sent FCM push to device");
    }
}
```

### Phase 3: Android App FCM Integration

```kotlin
// In AndroidManifest.xml
<service
    android:name=".MyFirebaseMessagingService"
    android:exported="false">
    <intent-filter>
        <action android:name="com.google.firebase.MESSAGING_EVENT" />
    </intent-filter>
</service>

// MyFirebaseMessagingService.kt
class MyFirebaseMessagingService : FirebaseMessagingService() {
    override fun onMessageReceived(message: RemoteMessage) {
        val txid = message.data["txid"]
        val amount = message.data["amount"]?.toLong() ?: 0
        
        // Show notification
        showNotification("Received ${amount} TIME", txid)
        
        // Fetch full transaction details via HTTP API
        lifecycleScope.launch {
            val tx = api.getTransaction(txid)
            updateWalletState(tx)
        }
    }
    
    override fun onNewToken(token: String) {
        // Register token with masternode
        lifecycleScope.launch {
            api.registerFcmDevice(wallet.xpub, token)
        }
    }
}
```

## Comparison: TCP vs FCM

| Feature | TCP (Foreground) | FCM (Background) |
|---------|-----------------|------------------|
| **Latency** | < 1 second | 1-3 seconds |
| **Battery** | Moderate | Excellent |
| **Reliability** | High when connected | Very High |
| **When App Closed** | ❌ Doesn't work | ✅ Works |
| **Backend Changes** | ✅ Already done | ⚠️ Need to add |
| **Android Support** | ✅ Easy | ✅ Easy |

## Repository Structure Recommendation

### ✅ RECOMMENDED: Separate Repository

```
time-coin-mobile/                    (NEW GitHub repo)
├── android/                         Native Android app
│   ├── app/
│   │   ├── src/main/
│   │   │   ├── kotlin/
│   │   │   │   ├── network/
│   │   │   │   │   ├── TcpProtocolClient.kt
│   │   │   │   │   ├── HttpApiClient.kt
│   │   │   │   ├── wallet/
│   │   │   │   │   ├── Wallet.kt
│   │   │   │   │   ├── Bip39.kt
│   │   │   │   ├── ui/
│   │   │   │   └── fcm/
│   │   │   │       └── MyFirebaseMessagingService.kt
│   │   └── AndroidManifest.xml
│   └── build.gradle.kts
├── ios/                             (Future iOS app)
├── shared/                          Shared crypto logic
│   └── rust/                        Rust crypto via JNI
└── README.md
```

**Why separate repo:**
- ✅ Different tech stack (Kotlin/Swift vs Rust)
- ✅ Different release cycle than node software
- ✅ Easier for mobile developers to contribute
- ✅ Can use mobile-specific CI/CD (Play Store, App Store)
- ✅ Cleaner dependency management

## Development Timeline

### MVP (8-10 weeks)

**Week 1-2: Android Project Setup**
- ✅ Create repository
- ✅ Set up Android Studio project
- ✅ Add Bitcoin crypto libraries (BIP-39, BIP-44)
- ✅ Implement wallet generation/import

**Week 3-4: Network Layer**
- ✅ Implement TCP protocol client (port 24100)
- ✅ Test with existing masternode
- ✅ Implement HTTP API client for wallet sync
- ✅ Add connection state management

**Week 5-6: Core Wallet Features**
- ✅ Balance display
- ✅ Transaction history
- ✅ Send transaction UI
- ✅ Receive/QR code display
- ✅ Address book

**Week 7-8: Real-Time Notifications**
- ✅ TCP notifications when app foreground
- ✅ Local notifications for incoming payments
- ✅ Transaction status updates

**Week 9-10: Security & Polish**
- ✅ Android Keystore integration
- ✅ Biometric authentication
- ✅ PIN lock
- ✅ Testing on multiple devices

### Phase 2: FCM Integration (2-3 weeks)

**Backend (1 week):**
- Add FCM device registration endpoints
- Integrate FCM SDK in masternode
- Test push notifications

**Android (1-2 weeks):**
- Integrate Firebase SDK
- Implement background notification handling
- Test background/foreground switching

## Next Steps to Start Now

1. **Create GitHub Repository**: `time-coin-mobile`
2. **Set up Android Project**: Use Android Studio wizard
3. **Add Dependencies**:
   ```kotlin
   // build.gradle.kts
   dependencies {
       implementation("org.bitcoinj:bitcoinj-core:0.16.2")  // BIP-39, BIP-44
       implementation("com.squareup.okhttp3:okhttp:4.12.0")  // HTTP client
       implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.7.3")
       // Firebase (Phase 2)
       implementation("com.google.firebase:firebase-messaging:23.4.0")
   }
   ```

4. **Start with MVP Features**:
   - Wallet generation (BIP-39 mnemonic)
   - TCP connection to masternode
   - Display balance from HTTP API
   - Simple send transaction UI

5. **Test with Existing Infrastructure**:
   - Connect to your testnet masternode
   - Use existing TCP protocol (no changes needed!)
   - Register xpub and receive notifications

## Cost Estimate

- **Google Play Developer Account**: $25 (one-time)
- **Firebase Free Tier**: Sufficient for MVP testing
- **Firebase Paid (Production)**: ~$50-200/month (scales with users)
- **Development Time**: 2-3 months (1 developer)

## Security Checklist

- ✅ Private keys never leave device
- ✅ Keys stored in Android Keystore (hardware-backed)
- ✅ Local wallet data encrypted
- ✅ SSL certificate pinning for API calls
- ✅ Biometric authentication support
- ✅ Root/jailbreak detection
- ✅ Secure clipboard handling
- ✅ Transaction confirmation prompts

## Conclusion

**Start with TCP for foreground notifications** - it already works with zero backend changes needed! This gets you a working Android app in 6-8 weeks.

**Add FCM later** for background push notifications once you have users and need that feature. This is an incremental improvement, not a blocker for launch.

The existing TIME Coin infrastructure is **already mobile-ready** via the TCP protocol. You can build and ship an Android app using the current backend with no modifications required!
