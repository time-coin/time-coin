# TIME Coin Mobile App Development Summary

## Decision: Post-WebSocket Architecture

Since WebSocket support was just removed from the codebase, the mobile app strategy is:

## ✅ Recommended Architecture

```
┌─────────────────────────────────────────────────────────┐
│           Mobile Notification Strategy                  │
├─────────────────────────────────────────────────────────┤
│                                                           │
│  📱 App Foreground:   TCP Direct Connection (Port 24100) │
│     - Real-time push notifications                       │
│     - < 1 second latency                                 │
│     - Already implemented in backend ✅                  │
│     - NO CHANGES NEEDED to start                         │
│                                                           │
│  🔕 App Background:   Firebase Cloud Messaging (FCM)     │
│     - Push notifications when app closed                 │
│     - Battery efficient                                  │
│     - Requires backend changes (Phase 2)                 │
│                                                           │
│  🔄 Fallback:         HTTP Polling (Optional)            │
│     - Emergency backup if TCP fails                      │
│     - Already available via REST API ✅                  │
│                                                           │
└─────────────────────────────────────────────────────────┘
```

## Current Backend Capabilities (Ready to Use)

Your TIME Coin infrastructure **already supports mobile apps**:

### ✅ TCP Protocol (Port 24100)
- **RegisterXpub** - Subscribe wallet to notifications
- **NewTransactionNotification** - Real-time transaction alerts
- **UtxoUpdate** - Balance updates
- **Ping/Pong** - Connection keepalive

### ✅ HTTP REST API
- `POST /wallet/sync-xpub` - Sync wallet state
- `POST /wallet/send` - Send transaction
- `GET /transactions/{txid}` - Get transaction details
- `GET /balance/{address}` - Check balance
- `GET /utxos/{address}` - List unspent outputs

### ⚠️ NOT YET IMPLEMENTED (Phase 2)
- FCM device registration endpoint
- FCM push notification sender
- Background notification infrastructure

## Development Phases

### Phase 1: MVP with TCP (6-8 weeks)

**What you get:**
- ✅ Android app that works **immediately** with existing backend
- ✅ Real-time notifications when app is open
- ✅ Send/receive TIME coins
- ✅ Transaction history
- ✅ Balance display
- ✅ QR code scanning
- ✅ Secure wallet storage

**Backend changes needed:** **ZERO** ✅

### Phase 2: Add FCM for Background (2-3 weeks)

**What you add:**
- ✅ Push notifications when app is closed
- ✅ Battery-efficient background updates
- ✅ Wake app on incoming transaction

**Backend changes needed:**
1. Add endpoint: `POST /wallet/register_fcm_device`
2. Store `xpub → fcm_token` mapping
3. Send FCM push when transaction detected
4. Integrate FCM SDK in masternode

## Repository Structure

### ✅ Create Separate Repository

**Recommended:** `time-coin-mobile` (new GitHub repo)

```
time-coin-mobile/
├── android/                    Native Android (Kotlin)
│   ├── app/
│   │   ├── src/main/kotlin/
│   │   │   ├── network/
│   │   │   │   ├── TcpProtocolClient.kt
│   │   │   │   └── HttpApiClient.kt
│   │   │   ├── wallet/
│   │   │   │   ├── Wallet.kt
│   │   │   │   └── Bip39Generator.kt
│   │   │   ├── ui/
│   │   │   └── fcm/
│   │   └── build.gradle.kts
│   └── AndroidManifest.xml
├── ios/                        Future iOS app
└── README.md
```

**Why separate:**
- Different tech stack (Kotlin/Swift vs Rust)
- Different release cycle
- Easier for mobile developers
- Cleaner CI/CD for app stores

## Technology Stack

**Recommended for Android:**
- **Language:** Kotlin
- **UI:** Jetpack Compose
- **Crypto:** BitcoinJ (BIP-39, BIP-44)
- **Network:** OkHttp + Retrofit
- **Database:** Room
- **Security:** Android Keystore + Biometric

**Alternative (Cross-platform):**
- Flutter (Dart) - Single codebase for Android + iOS
- React Native (JavaScript) - Good ecosystem

## Timeline Estimate

| Phase | Duration | Deliverable |
|-------|----------|-------------|
| Phase 1: Android MVP | 6-8 weeks | Working app with TCP notifications |
| Phase 2: FCM Backend | 1 week | Backend support for push |
| Phase 2: FCM Android | 1-2 weeks | Background push notifications |
| Testing & Polish | 2-3 weeks | Production-ready app |
| **Total** | **10-14 weeks** | **2.5-3.5 months** |

## Cost Breakdown

- **Google Play Developer Account:** $25 (one-time)
- **Firebase Free Tier:** Sufficient for testing
- **Firebase Paid (Production):** ~$50-200/month (scales with users)
- **Developer Time:** 2.5-3.5 months (1 developer)

## Quick Start (3 Steps)

### 1. Create Repository
```bash
gh repo create time-coin-mobile --public
cd time-coin-mobile
```

### 2. Set Up Android Project
```bash
# In Android Studio: New Project → Empty Activity
# Package: com.timecoin.wallet
# Language: Kotlin
# Minimum SDK: 26 (Android 8.0)
```

### 3. Test TCP Connection
```kotlin
// Connect to testnet masternode
val client = TcpProtocolClient(xpub)
client.connect("time-coin.io", 24100)

// Listen for notifications
client.onNewTransaction { tx ->
    println("Received ${tx.amount} TIME!")
}
```

## Documentation Created

This summary references the following new documentation:

1. **`MOBILE_NOTIFICATION_STRATEGY.md`** - Complete architecture overview
2. **`ANDROID_APP_QUICKSTART.md`** - Quick start guide for developers
3. **`MOBILE_PROTOCOL_REFERENCE.md`** - TCP protocol specification

## Security Checklist

- ✅ Private keys stored in Android Keystore (hardware-backed)
- ✅ Local data encrypted with EncryptedSharedPreferences
- ✅ Biometric authentication (fingerprint/face)
- ✅ Certificate pinning for API calls
- ✅ Root/jailbreak detection
- ✅ Secure clipboard handling
- ✅ Transaction confirmation prompts
- ✅ Never send private keys to server

## Key Advantages of This Approach

1. **Start Immediately** - No waiting for backend changes
2. **Progressive Enhancement** - Add FCM later as needed
3. **Battery Efficient** - TCP only when app is active
4. **Real-Time** - < 1 second notification latency
5. **Proven** - TCP protocol already works in wallet-gui
6. **Scalable** - FCM handles millions of devices

## Next Actions

### For Mobile Developer:
1. ✅ Review `ANDROID_APP_QUICKSTART.md`
2. ✅ Set up Android project
3. ✅ Implement TCP protocol client
4. ✅ Test with testnet masternode
5. ✅ Build send/receive UI
6. ✅ Add transaction history

### For Backend Team:
1. ⏳ Wait for MVP completion (Phase 1)
2. ⏳ Implement FCM endpoints (Phase 2)
3. ⏳ Test push notifications
4. ⏳ Deploy to production

## Questions?

- **Technical:** See `MOBILE_PROTOCOL_REFERENCE.md`
- **Architecture:** See `MOBILE_NOTIFICATION_STRATEGY.md`
- **Getting Started:** See `ANDROID_APP_QUICKSTART.md`

## Comparison: Before vs After WebSocket Removal

| Feature | With WebSocket | Without WebSocket (Current) |
|---------|----------------|----------------------------|
| **Desktop Wallet** | WebSocket (removed) | ✅ TCP Protocol |
| **Mobile Foreground** | WebSocket (won't work) | ✅ TCP Protocol |
| **Mobile Background** | WebSocket (doesn't work) | 🔜 FCM (to be added) |
| **Battery Impact** | High | Medium (TCP), Low (FCM) |
| **Reliability** | Medium | High |
| **Backend Complexity** | Medium | Low (Phase 1), Medium (Phase 2) |

## Conclusion

✅ **You can start building the Android app TODAY** using the existing TCP protocol.  
✅ **No backend changes needed** for Phase 1 MVP.  
✅ **FCM is optional** and can be added later as an enhancement.

The removal of WebSocket actually **simplifies** mobile development because:
- TCP is more reliable on mobile than WebSocket
- Clear separation of concerns (foreground vs background)
- Progressive enhancement path

**Recommendation:** Start with Phase 1 (TCP-only) to validate the app concept, then add FCM in Phase 2 once you have users who need background notifications.
