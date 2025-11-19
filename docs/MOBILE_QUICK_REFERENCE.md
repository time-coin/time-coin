# Mobile App Quick Reference Card

## 🎯 Executive Summary

**Can I start building an Android app now?** ✅ **YES!**

**Do I need backend changes?** ❌ **NO!** (for MVP)

**What notification method?** 
- Phase 1: TCP (foreground only) - **Ready to use**
- Phase 2: FCM (background) - **Add later**

## 📱 Phase 1 MVP (Start Now - 6-8 weeks)

### Backend Status
✅ **READY** - No changes needed
- TCP Protocol: Port 24100 (testnet), 24101 (mainnet)
- HTTP REST API: All wallet endpoints working
- Real-time notifications: Working in desktop wallet

### Android App Stack
```
- Language: Kotlin
- UI: Jetpack Compose
- Crypto: BitcoinJ (BIP-39, BIP-44)
- Network: Raw TCP Sockets + OkHttp
- Storage: Room + EncryptedSharedPreferences
- Security: Android Keystore + Biometric
```

### Connection Flow
```kotlin
// 1. Connect to masternode
val socket = Socket("time-coin.io", 24100)

// 2. Register xpub
send(JSONObject().apply {
    put("RegisterXpub", JSONObject().apply {
        put("xpub", wallet.xpub)
    })
})

// 3. Receive confirmation
val response = receive()
// {"XpubRegistered": {"success": true, "message": "..."}}

// 4. Listen for notifications
while (connected) {
    val msg = receive()
    if (msg.has("NewTransactionNotification")) {
        showNotification("Received TIME!")
    }
}
```

### Message Format
```
Wire format: [4-byte length][JSON message]
Length: Big-endian uint32
Encoding: UTF-8 JSON
```

### Key Messages
- `RegisterXpub` - Subscribe to notifications
- `XpubRegistered` - Confirmation response
- `NewTransactionNotification` - Transaction received
- `UtxoUpdate` - Balance update
- `Ping/Pong` - Keepalive

## 🔔 Phase 2 FCM (Add Later - 2-3 weeks)

### When App is Backgrounded
1. TCP connection closes (OS limitation)
2. FCM receives push from backend
3. OS wakes app
4. User sees notification
5. App fetches details via HTTP

### Backend Changes Needed
```rust
// Add to api/src/routes.rs
.route("/wallet/register_fcm_device", post(register_fcm_device))
.route("/wallet/unregister_fcm_device", delete(unregister_fcm_device))

// Store: xpub → fcm_token
// When tx detected → send FCM push
```

### Android Changes
```kotlin
// Add Firebase
implementation("com.google.firebase:firebase-messaging:23.4.0")

// Handle push
override fun onMessageReceived(message: RemoteMessage) {
    val txid = message.data["txid"]
    showNotification("Received ${message.data["amount"]} TIME")
    fetchTransactionDetails(txid)
}
```

## 📂 Project Structure

### Separate Repository (Recommended)
```
time-coin-mobile/           (NEW GitHub repo)
├── android/
│   ├── app/
│   │   ├── src/main/kotlin/
│   │   │   ├── network/
│   │   │   │   ├── TcpProtocolClient.kt
│   │   │   │   └── HttpApiClient.kt
│   │   │   ├── wallet/
│   │   │   │   └── Wallet.kt
│   │   │   ├── ui/
│   │   │   └── fcm/ (Phase 2)
│   │   └── build.gradle.kts
│   └── AndroidManifest.xml
├── ios/ (Future)
└── README.md
```

## 🚀 Quick Start Commands

### 1. Create Repository
```bash
gh repo create time-coin-mobile --public
cd time-coin-mobile
```

### 2. Create Android Project
```
Android Studio → New Project
  → Empty Activity
  → Name: TIME Coin Wallet
  → Package: com.timecoin.wallet
  → Language: Kotlin
  → Minimum SDK: 26 (Android 8.0)
```

### 3. Add Dependencies
```kotlin
// app/build.gradle.kts
dependencies {
    implementation("org.bitcoinj:bitcoinj-core:0.16.2")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("androidx.security:security-crypto:1.1.0-alpha06")
    implementation("androidx.biometric:biometric:1.2.0-alpha05")
}
```

### 4. Test Connection
```kotlin
class MainActivity : AppCompatActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        
        lifecycleScope.launch {
            val client = TcpProtocolClient(xpub = "xpub...")
            client.connect("time-coin.io", 24100)
        }
    }
}
```

## ⏱️ Timeline

| Week | Task |
|------|------|
| 1-2 | Project setup, wallet generation |
| 3-4 | TCP client, API integration |
| 5-6 | UI (send/receive/history) |
| 7-8 | Security, testing, polish |
| **Total** | **6-8 weeks for MVP** |

Add 2-3 weeks later for FCM (Phase 2)

## 💰 Costs

- Google Play Account: **$25** (one-time)
- Firebase (testing): **$0** (free tier)
- Firebase (production): **~$50-200/month**
- Development: **2-3 months** (1 developer)

## 🔒 Security Checklist

- ✅ Android Keystore for private keys
- ✅ EncryptedSharedPreferences for data
- ✅ Biometric authentication
- ✅ Certificate pinning
- ✅ Root detection
- ✅ Never send keys to server

## 📚 Documentation

1. **`MOBILE_APP_SUMMARY.md`** - Start here (overview)
2. **`ANDROID_APP_QUICKSTART.md`** - Developer guide
3. **`MOBILE_PROTOCOL_REFERENCE.md`** - TCP protocol spec
4. **`MOBILE_NOTIFICATION_STRATEGY.md`** - Architecture details
5. **`MOBILE_APP_TODO.md`** - Full implementation plan

## 🔍 Testing

### Test TCP Connection
```bash
# Install netcat
# Connect to testnet
nc time-coin.io 24100

# Send RegisterXpub (manual hex encoding)
```

### Test from Android
```kotlin
// In emulator or device
val client = TcpProtocolClient("xpub...")
client.connect("10.0.2.2", 24100) // Emulator → host machine
// OR
client.connect("time-coin.io", 24100) // Real device → internet
```

## ❓ FAQ

**Q: Can I use WebSocket?**  
A: No, WebSocket was removed. Use TCP or HTTP.

**Q: Does TCP work when app is closed?**  
A: No, that's why Phase 2 adds FCM.

**Q: Can I connect to multiple masternodes?**  
A: Yes, for redundancy. Use first successful connection.

**Q: Do I need a separate repo?**  
A: Recommended. Different tech stack, release cycle.

**Q: Flutter or Native Android?**  
A: Native Kotlin for best performance. Flutter if you want iOS too.

**Q: When do I need FCM?**  
A: Not for MVP. Add when you have users who need background notifications.

## ✅ You're Ready!

**Everything you need is already implemented in the backend.**

**Start building the Android app today using the TCP protocol.**

**Add FCM later as an enhancement.**

---

**Next step:** Read `ANDROID_APP_QUICKSTART.md` and start coding!
