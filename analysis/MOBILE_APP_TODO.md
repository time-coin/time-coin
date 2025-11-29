# Mobile App Development TODO

## Overview
Plan for creating iOS and Android mobile wallet apps for TIME Coin.

## ⚠️ UPDATE: WebSocket Removed - Using TCP Protocol

WebSocket support has been removed from the codebase. The mobile app strategy is now:

## Push Notification Strategy (Post-WebSocket)

### ✅ Option 1: TCP Protocol (RECOMMENDED for Phase 1)
- **Status**: Already implemented and working
- **Port**: 24100 (testnet), 24101 (mainnet)
- **Protocol**: Length-prefixed JSON over TCP
- **Pros**: Real-time, proven, works in wallet-gui, NO BACKEND CHANGES NEEDED
- **Cons**: Only works when app is in foreground, battery usage
- **Use Case**: App foreground state
- **Timeline**: Can start immediately

### 🔜 Option 2: Firebase Cloud Messaging (Phase 2 Enhancement)
- **Status**: Not yet implemented - requires backend changes
- **Platform**: Android (FCM), iOS (APNs)
- **Pros**: Works when app closed/backgrounded, battery efficient
- **Cons**: Requires backend integration, slight latency (1-3s)
- **Use Case**: App background state
- **Timeline**: Add after MVP

### ✅ BEST: Hybrid Approach (Recommended)
1. **Foreground**: Use TCP direct connection (instant, < 1s latency)
2. **Background**: Use FCM push notifications (battery efficient)
3. **Automatic**: Switch between modes based on app state
4. **Fallback**: HTTP polling as emergency backup

## Implementation Tasks

### Phase 1: MVP with TCP (6-8 weeks) - NO BACKEND CHANGES NEEDED ✅

**Backend Status**: Ready to use!
- ✅ TCP protocol implemented (port 24100)
- ✅ RegisterXpub message working
- ✅ NewTransactionNotification working
- ✅ UtxoUpdate working
- ✅ HTTP REST API for wallet sync

### Phase 2: FCM Integration (2-3 weeks) - BACKEND CHANGES REQUIRED

#### 2.1 Backend (Masternode) Changes
- [ ] Add device token registration endpoint
  - `POST /wallet/register_fcm_device`
  - Store mapping: xpub → device_token(s)
  - Support multiple devices per xpub
- [ ] Integrate FCM SDK for Android notifications
- [ ] Integrate APNs SDK for iOS notifications (future)
- [ ] Create notification payload format
- [ ] Add unsubscribe/remove device endpoint
  - `DELETE /wallet/unregister_fcm_device`
- [ ] Handle token expiration/refresh
- [ ] Integrate FCM sender into transaction monitoring

### 2. Mobile App Architecture (Phase 1)

#### Technology Stack Options:
- **React Native**: Single codebase, JavaScript
- **Flutter**: Single codebase, Dart, better performance
- **Native**: Separate Swift (iOS) and Kotlin (Android) apps, best performance

#### Core Features (Phase 1 - TCP Only):
- [ ] BIP-39 mnemonic phrase generation/import
- [ ] Secure keystore (iOS Keychain, Android Keystore)
- [ ] HD wallet address derivation (BIP-44)
- [ ] TCP protocol client for real-time notifications
  - Connect to masternode port 24100
  - Send RegisterXpub message
  - Receive NewTransactionNotification
  - Handle UtxoUpdate messages
- [ ] HTTP API client for wallet operations
  - Sync wallet state (`POST /wallet/sync-xpub`)
  - Send transactions (`POST /wallet/send`)
  - Get transaction details
- [ ] Send/receive transactions
- [ ] Transaction history
- [ ] Balance display
- [ ] QR code scanning for addresses
- [ ] Address book/contacts
- [ ] Biometric authentication (Face ID, Touch ID, fingerprint)

#### Phase 2 Features (FCM):
- [ ] Firebase Cloud Messaging integration
- [ ] Background push notification handling
- [ ] Foreground/background mode switching
- [ ] Device token registration with backend

### 3. Security Considerations
- [ ] Never send private keys to server
- [ ] Encrypt local wallet data
- [ ] Use secure enclave/keystore for key storage
- [ ] Implement app PIN/biometric lock
- [ ] Certificate pinning for API calls
- [ ] Handle jailbreak/root detection

### 4. Network Efficiency
- [ ] Connection pooling for API calls
- [ ] Efficient sync algorithm (only fetch new transactions)
- [ ] Background fetch for periodic updates (iOS)
- [ ] WorkManager for background sync (Android)
- [ ] Battery optimization

### 5. User Experience
- [ ] Onboarding flow
- [ ] Backup reminder for mnemonic phrase
- [ ] Push notification preferences
- [ ] Network status indicator (mainnet/testnet)
- [ ] Transaction confirmation UI
- [ ] Error handling and retry logic

## Notification Flow

### Phase 1: TCP Direct Connection (Foreground Only)

```
1. App Launch:
   - Generate/import mnemonic phrase
   - Derive xpub from mnemonic
   - Connect to masternode TCP (port 24100)
   - Send RegisterXpub message: { "RegisterXpub": { "xpub": "xpub..." } }
   - Receive XpubRegistered confirmation
   - Receive UtxoUpdate with current wallet state

2. Transaction Arrives (App Open):
   - Masternode detects transaction to registered address
   - Masternode → App: NewTransactionNotification via TCP
   - App displays: "💰 Received X TIME" (< 1 second latency)
   - Transaction updates in real-time
   - Balance updates immediately

3. App Background:
   - TCP connection closes (OS kills background sockets)
   - No notifications received
   - On return to foreground: reconnect and sync

4. App Resume/Foreground:
   - Reconnect to masternode TCP
   - Re-register xpub
   - Receive UtxoUpdate with current state
   - Resume real-time notifications
```

### Phase 2: Hybrid (TCP + FCM)

```
1. App Install/First Launch:
   - Generate/import mnemonic phrase
   - Derive xpub
   - Request FCM notification permission
   - Get FCM device token from Firebase
   - Register: POST /wallet/register_fcm_device
     Body: { "xpub": "xpub...", "fcm_token": "...", "platform": "android" }

2. Transaction Arrives (App Backgrounded):
   - Masternode detects transaction to registered xpub
   - Masternode sends FCM push notification
   - OS wakes app and shows notification
   - User taps → app opens
   - App fetches full details via HTTP API

3. Transaction Arrives (App Foreground):
   - Use TCP direct connection (instant)
   - NewTransactionNotification via TCP
   - No FCM needed

4. App Uninstall/Logout:
   - App sends: DELETE /wallet/unregister_fcm_device
   - Masternode removes device token
```

## Estimated Timeline

### Phase 1: MVP with TCP
- Mobile app MVP (Android): **6-8 weeks**
  - Week 1-2: Project setup, wallet generation
  - Week 3-4: TCP client, HTTP API integration
  - Week 5-6: UI (send/receive/history)
  - Week 7-8: Security, testing, polish
- Backend changes: **ZERO** (already done ✅)
- **Total Phase 1**: 6-8 weeks

### Phase 2: Add FCM (Optional Enhancement)
- Backend FCM integration: **1 week**
- Android FCM client: **1-2 weeks**
- Testing: **1 week**
- **Total Phase 2**: 2-3 weeks

### Phase 3: iOS App (Future)
- iOS app (single developer): **6-8 weeks**
- Code reuse from Android: ~40%
- **Total Phase 3**: 6-8 weeks

**Overall**: 2-3 months for Android MVP, 3-4 months for production with FCM

## Next Steps

### Immediate (Start Now - Phase 1)
1. ✅ Choose mobile framework
   - **Recommended**: Native Android (Kotlin) for best performance
   - **Alternative**: Flutter for cross-platform (Android + iOS)
2. ✅ Create new repository: `time-coin-mobile`
3. ✅ Set up Google Play Developer account ($25 one-time)
4. ✅ Set up Android Studio project
5. ✅ Implement TCP protocol client (port 24100)
6. ✅ Test connection with testnet masternode
7. ✅ Build wallet generation (BIP-39)
8. ✅ Implement send/receive UI
9. ✅ Add transaction history
10. ✅ Internal testing

### Later (Phase 2 - FCM Enhancement)
11. ⏳ Implement backend FCM endpoints
12. ⏳ Integrate Firebase SDK in Android app
13. ⏳ Test background push notifications
14. ⏳ Beta testing with internal users

### Future (Phase 3 - iOS)
15. ⏳ Set up Apple Developer account ($99/year)
16. ⏳ Build iOS app with TCP + APNs
17. ⏳ TestFlight beta testing

## References

### Internal Documentation (START HERE!)
- `MOBILE_APP_SUMMARY.md` - Executive summary and decision rationale
- `ANDROID_APP_QUICKSTART.md` - Quick start guide for Android developers
- `MOBILE_PROTOCOL_REFERENCE.md` - TCP protocol specification
- `MOBILE_NOTIFICATION_STRATEGY.md` - Complete architecture details
- `wallet-gui/src/tcp_protocol_client.rs` - Reference implementation

### External Resources
- [Firebase Cloud Messaging](https://firebase.google.com/docs/cloud-messaging)
- [Apple Push Notification Service](https://developer.apple.com/documentation/usernotifications)
- [BIP-39 Specification](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki)
- [BIP-44 Specification](https://github.com/bitcoin/bips/blob/master/bip-0044.mediawiki)
- [Android Keystore](https://developer.android.com/training/articles/keystore)
- [BitcoinJ Library](https://bitcoinj.org/) - For BIP-39/BIP-44 on Android
