# Mobile App Development TODO

## Overview
Plan for creating iOS and Android mobile wallet apps for TIME Coin.

## Push Notification Strategy

### Option 1: WebSocket (Current Implementation)
- **Pros**: Already implemented, real-time, works on WiFi
- **Cons**: Drains battery when app is backgrounded, may be killed by OS

### Option 2: Platform Push Notifications (RECOMMENDED)
- **iOS**: Apple Push Notification Service (APNs)
- **Android**: Firebase Cloud Messaging (FCM)
- **Architecture**:
  1. Mobile app registers device token with masternode
  2. App subscribes to specific addresses
  3. When transaction arrives, masternode sends push notification via APNs/FCM
  4. Push wakes app even when closed/backgrounded
  5. App fetches full transaction details via API

### Option 3: Hybrid Approach (BEST)
- Use WebSocket when app is in foreground (instant updates)
- Use push notifications when app is backgrounded (battery efficient)
- Automatic fallback between modes

## Implementation Tasks

### 1. Backend (Masternode) Changes
- [ ] Add device token registration endpoint
  - Store mapping: address → device_token(s)
  - Support multiple devices per address
- [ ] Integrate APNs SDK for iOS notifications
- [ ] Integrate FCM SDK for Android notifications
- [ ] Create notification payload format
- [ ] Add unsubscribe/remove device endpoint
- [ ] Handle token expiration/refresh

### 2. Mobile App Architecture

#### Technology Stack Options:
- **React Native**: Single codebase, JavaScript
- **Flutter**: Single codebase, Dart, better performance
- **Native**: Separate Swift (iOS) and Kotlin (Android) apps, best performance

#### Core Features:
- [ ] BIP-39 mnemonic phrase generation/import
- [ ] Secure keystore (iOS Keychain, Android Keystore)
- [ ] HD wallet address derivation
- [ ] Send/receive transactions
- [ ] Transaction history
- [ ] QR code scanning for addresses
- [ ] Address book/contacts
- [ ] Biometric authentication (Face ID, Touch ID, fingerprint)
- [ ] Push notification handling

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

## Push Notification Flow

```
1. App Install/First Launch:
   - Generate/import mnemonic phrase
   - Derive addresses
   - Request push notification permission
   - Get device token from OS
   - Register token with masternode: POST /api/v1/wallet/register_device
     Body: { addresses: [...], device_token: "...", platform: "ios|android" }

2. Transaction Arrives:
   - Masternode detects transaction to registered address
   - Masternode sends push notification:
     - iOS: APNs with payload { alert: "Received X TIME", tx_id: "..." }
     - Android: FCM with same payload
   - OS delivers push to device even if app closed
   - User taps notification → app opens
   - App fetches full transaction details via API

3. App Foreground:
   - Use WebSocket for instant updates (already implemented)
   - No push notifications needed

4. App Uninstall/Logout:
   - App sends unregister request: DELETE /api/v1/wallet/unregister_device
   - Masternode removes device token
```

## Estimated Timeline
- Backend push notification integration: 1-2 weeks
- Mobile app MVP (single platform): 4-6 weeks
- Second platform: 2-3 weeks
- Testing and polish: 2-3 weeks
- **Total**: 3-4 months for production-ready app

## Next Steps
1. Choose mobile framework (React Native vs Flutter vs Native)
2. Set up Apple Developer account ($99/year)
3. Set up Google Play Developer account ($25 one-time)
4. Implement backend push notification support
5. Build prototype app
6. Beta testing with TestFlight (iOS) and internal testing (Android)

## References
- [Apple Push Notification Service](https://developer.apple.com/documentation/usernotifications)
- [Firebase Cloud Messaging](https://firebase.google.com/docs/cloud-messaging)
- [BIP-39 Implementation in Mobile](https://github.com/bitcoin/bips/blob/master/bip-0039.mediawiki)
- [React Native Crypto Libraries](https://www.npmjs.com/package/react-native-bip39)
