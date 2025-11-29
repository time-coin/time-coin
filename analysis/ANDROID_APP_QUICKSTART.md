# TIME Coin Android App - Quick Start Guide

## TL;DR

✅ **You can build an Android app NOW using existing TCP protocol**  
✅ **No backend changes needed to start**  
✅ **FCM push notifications can be added later as enhancement**

## Architecture Decision

Since WebSockets were just removed, use this hybrid approach:

```
┌────────────────────────────────────────┐
│        Android App Strategy            │
├────────────────────────────────────────┤
│ Foreground: TCP Direct (port 24100)   │  ← START HERE (Phase 1)
│ Background: FCM Push Notifications     │  ← Add later (Phase 2)
│ Polling:    HTTP fallback (optional)   │  ← Emergency backup
└────────────────────────────────────────┘
```

## Phase 1: MVP with TCP (6-8 weeks)

### What Works Out-of-the-Box

The TIME Coin masternodes already support:

1. **TCP Protocol** on port 24100 (testnet) / 24101 (mainnet)
2. **Message Types**:
   - `RegisterXpub` - Subscribe to wallet notifications
   - `NewTransactionNotification` - Real-time transaction alerts
   - `UtxoUpdate` - Balance updates
3. **HTTP REST API** for wallet sync and transaction submission

### TCP Protocol Example

```kotlin
// Connect to masternode
class TimeCoinProtocol(private val xpub: String) {
    private lateinit var socket: Socket
    
    suspend fun connect(host: String, port: Int = 24100) = withContext(Dispatchers.IO) {
        socket = Socket(host, port)
        
        // Send RegisterXpub message
        sendMessage(mapOf(
            "RegisterXpub" to mapOf("xpub" to xpub)
        ))
        
        // Wait for confirmation
        val response = receiveMessage()
        check(response.has("XpubRegistered")) { "Registration failed" }
        
        // Start listening for notifications
        startListening()
    }
    
    private fun sendMessage(message: Map<String, Any>) {
        val json = JSONObject(message).toString().toByteArray()
        val output = socket.getOutputStream()
        
        // Send 4-byte length prefix (big-endian)
        output.write(ByteBuffer.allocate(4).putInt(json.size).array())
        
        // Send JSON message
        output.write(json)
        output.flush()
    }
    
    private fun receiveMessage(): JSONObject {
        val input = socket.getInputStream()
        
        // Read 4-byte length
        val lengthBytes = ByteArray(4)
        input.read(lengthBytes)
        val length = ByteBuffer.wrap(lengthBytes).int
        
        // Read message
        val messageBytes = ByteArray(length)
        input.read(messageBytes)
        
        return JSONObject(String(messageBytes))
    }
    
    private suspend fun startListening() = withContext(Dispatchers.IO) {
        while (socket.isConnected) {
            try {
                val message = receiveMessage()
                
                when {
                    message.has("NewTransactionNotification") -> {
                        val tx = message.getJSONObject("NewTransactionNotification")
                            .getJSONObject("transaction")
                        handleNewTransaction(tx)
                    }
                    message.has("UtxoUpdate") -> {
                        val update = message.getJSONObject("UtxoUpdate")
                        handleUtxoUpdate(update)
                    }
                    message.has("Ping") -> {
                        sendMessage(mapOf("Pong" to emptyMap<String, Any>()))
                    }
                }
            } catch (e: Exception) {
                Log.e("TCP", "Error receiving message: $e")
                break
            }
        }
    }
    
    private fun handleNewTransaction(tx: JSONObject) {
        val amount = tx.getLong("amount")
        val from = tx.getString("from_address")
        
        // Show notification
        withContext(Dispatchers.Main) {
            showNotification("Received $amount TIME from ${from.take(8)}...")
        }
    }
}
```

### Usage in Android Activity

```kotlin
class WalletActivity : AppCompatActivity() {
    private lateinit var protocol: TimeCoinProtocol
    
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        // Connect when app is in foreground
        lifecycleScope.launch {
            protocol = TimeCoinProtocol(wallet.xpub)
            protocol.connect("time-coin.io", 24100)
        }
    }
    
    override fun onPause() {
        super.onPause()
        // Disconnect when going to background
        protocol.disconnect()
    }
}
```

## Phase 2: Add FCM for Background (2-3 weeks)

Once you have the MVP working, add FCM for background notifications:

### Backend Changes Needed

1. Add endpoint: `POST /wallet/register_fcm_device`
2. Store mapping: `xpub → fcm_token`
3. When transaction detected, send FCM push
4. FCM wakes app, app fetches details via HTTP API

### Android FCM Integration

```kotlin
// MyFirebaseMessagingService.kt
class MyFirebaseMessagingService : FirebaseMessagingService() {
    
    override fun onMessageReceived(message: RemoteMessage) {
        val txid = message.data["txid"] ?: return
        val amount = message.data["amount"] ?: return
        
        // Show notification
        showNotification("💰 Received $amount TIME")
        
        // Update wallet in background
        CoroutineScope(Dispatchers.IO).launch {
            val api = TimeCoinApi()
            val transaction = api.getTransaction(txid)
            saveTransaction(transaction)
        }
    }
    
    override fun onNewToken(token: String) {
        // Register with masternode
        CoroutineScope(Dispatchers.IO).launch {
            TimeCoinApi().registerFcmDevice(
                xpub = wallet.xpub,
                fcmToken = token
            )
        }
    }
}
```

## HTTP API Endpoints (Already Available)

Use these for wallet operations:

### Sync Wallet
```kotlin
POST /wallet/sync-xpub
Body: { "xpub": "xpub..." }

Response: {
    "utxos": [...],
    "balance": 100000,
    "recent_transactions": [...]
}
```

### Send Transaction
```kotlin
POST /wallet/send
Body: {
    "from_address": "TIME1...",
    "to_address": "TIME1...",
    "amount": 50000,
    "signature": "..."
}

Response: {
    "success": true,
    "txid": "abc123..."
}
```

### Get Transaction
```kotlin
GET /transactions/{txid}

Response: {
    "txid": "abc123...",
    "inputs": [...],
    "outputs": [...],
    "confirmations": 5
}
```

## Project Structure

```
time-coin-mobile/
├── app/
│   ├── src/main/
│   │   ├── kotlin/
│   │   │   ├── com/timecoin/wallet/
│   │   │   │   ├── network/
│   │   │   │   │   ├── TcpProtocolClient.kt      // TCP connection
│   │   │   │   │   ├── HttpApiClient.kt          // REST API
│   │   │   │   │   └── FcmService.kt             // Phase 2
│   │   │   │   ├── wallet/
│   │   │   │   │   ├── Wallet.kt                 // BIP-39 wallet
│   │   │   │   │   ├── Bip39Generator.kt         // Mnemonic
│   │   │   │   │   └── AddressDerivation.kt      // BIP-44
│   │   │   │   ├── ui/
│   │   │   │   │   ├── MainActivity.kt
│   │   │   │   │   ├── SendActivity.kt
│   │   │   │   │   └── ReceiveActivity.kt
│   │   │   │   └── storage/
│   │   │   │       ├── WalletDatabase.kt         // Room DB
│   │   │   │       └── SecurePreferences.kt      // Encrypted prefs
│   ├── build.gradle.kts
│   └── AndroidManifest.xml
└── README.md
```

## Dependencies

```kotlin
// app/build.gradle.kts
dependencies {
    // Bitcoin crypto
    implementation("org.bitcoinj:bitcoinj-core:0.16.2")
    
    // Network
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("com.squareup.retrofit2:retrofit:2.9.0")
    implementation("com.squareup.retrofit2:converter-gson:2.9.0")
    
    // Android
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.7.0")
    implementation("androidx.room:room-runtime:2.6.1")
    kapt("androidx.room:room-compiler:2.6.1")
    
    // Security
    implementation("androidx.security:security-crypto:1.1.0-alpha06")
    implementation("androidx.biometric:biometric:1.2.0-alpha05")
    
    // FCM (Phase 2)
    implementation("com.google.firebase:firebase-messaging:23.4.0")
}
```

## Testing Checklist

### Phase 1 (TCP)
- [ ] Connect to testnet masternode (port 24100)
- [ ] Register xpub successfully
- [ ] Receive NewTransactionNotification
- [ ] Display incoming transaction in UI
- [ ] Update balance in real-time
- [ ] Handle connection loss gracefully
- [ ] Reconnect when app returns to foreground

### Phase 2 (FCM)
- [ ] Register FCM token with backend
- [ ] Receive push when app is closed
- [ ] Notification opens app to transaction details
- [ ] Background fetch updates wallet state
- [ ] Token refresh handled properly

## Security Best Practices

```kotlin
// Store private key in Android Keystore
class SecureKeyStorage(context: Context) {
    private val keyStore = KeyStore.getInstance("AndroidKeyStore").apply {
        load(null)
    }
    
    fun savePrivateKey(key: ByteArray) {
        val encryptedPrefs = EncryptedSharedPreferences.create(
            context,
            "wallet_prefs",
            MasterKey.Builder(context)
                .setKeyScheme(MasterKey.KeyScheme.AES256_GCM)
                .build(),
            EncryptedSharedPreferences.PrefKeyEncryptionScheme.AES256_SIV,
            EncryptedSharedPreferences.PrefValueEncryptionScheme.AES256_GCM
        )
        
        encryptedPrefs.edit()
            .putString("private_key", Base64.encodeToString(key, Base64.DEFAULT))
            .apply()
    }
}

// Biometric authentication
class BiometricAuth(private val activity: FragmentActivity) {
    fun authenticate(onSuccess: () -> Unit) {
        val biometricPrompt = BiometricPrompt(
            activity,
            ContextCompat.getMainExecutor(activity),
            object : BiometricPrompt.AuthenticationCallback() {
                override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) {
                    onSuccess()
                }
            }
        )
        
        val promptInfo = BiometricPrompt.PromptInfo.Builder()
            .setTitle("Authenticate to access wallet")
            .setNegativeButtonText("Cancel")
            .build()
        
        biometricPrompt.authenticate(promptInfo)
    }
}
```

## Development Timeline

| Week | Task | Status |
|------|------|--------|
| 1-2 | Project setup, wallet generation (BIP-39) | 🔜 |
| 3-4 | TCP protocol client, masternode connection | 🔜 |
| 5-6 | UI: Balance, send, receive, history | 🔜 |
| 7-8 | Real-time notifications, testing | 🔜 |
| 9-10 | Security hardening, Polish | 🔜 |
| 11-12 | FCM integration (optional) | 🔜 |

## Next Steps

1. **Create new repository**: `time-coin-mobile`
2. **Set up Android Studio project**
3. **Implement BIP-39 wallet generation**
4. **Test TCP connection to testnet masternode**
5. **Build send/receive UI**
6. **Add real-time notifications**

## Resources

- **BIP-39**: https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki
- **BIP-44**: https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki
- **Android Keystore**: https://developer.android.com/training/articles/keystore
- **Firebase Setup**: https://firebase.google.com/docs/android/setup

## Questions?

Check the main documentation:
- `docs/MOBILE_NOTIFICATION_STRATEGY.md` - Full architecture details
- `docs/wallet-push-notifications.md` - TCP protocol details
- `docs/TIME_COIN_PROTOCOL.md` - Protocol specification
