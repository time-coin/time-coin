# TIME Coin TODO List

## Mobile Wallet App

### Push Notification System
For mobile apps (iOS/Android), implement native push notifications:

**iOS (APNs - Apple Push Notification service)**
- Register device token with relay server
- Masternodes notify relay server of transactions
- Relay server sends push to iOS device
- Works even when app is closed/backgrounded

**Android (FCM - Firebase Cloud Messaging)**
- Register device token with FCM
- Masternodes notify FCM of transactions
- FCM delivers push to Android device
- More flexible than iOS, can include data payload

**Architecture:**
```
Wallet App → Register with Relay Server (device token + addresses to watch)
Masternode → Detects transaction → Notifies Relay Server
Relay Server → Sends push notification via APNs/FCM
Mobile Device → Receives notification → Wakes app → Updates UI
```

**Privacy Considerations:**
- Relay server learns which addresses belong to which device
- Solution: Use encrypted payloads, relay only notifies "you have a transaction"
- Wallet fetches details directly from masternode

**Fallback:**
- When app is active: Use WebSocket connection (like desktop)
- When app is backgrounded: Use push notifications
- Periodic background refresh as backup

### Implementation Steps
1. Create relay server infrastructure
2. Implement device token registration API
3. Add push notification handlers in masternodes
4. Build mobile apps with native push support
5. Add encryption for privacy

## Wallet Security

### Password/Authentication Protection
- [ ] Add password encryption for wallet.dat file
- [ ] Implement fingerprint authentication (mobile)
- [ ] Implement PIN authentication (mobile & desktop)
- [ ] Add biometric authentication support (Face ID, Touch ID)
- [ ] Add optional 2FA for transaction signing
- [ ] Secure key storage (Keychain on iOS, Keystore on Android)

### Desktop Wallet Security
- [ ] Password protect wallet.dat
- [ ] Auto-lock after inactivity
- [ ] Secure clipboard clearing after copy
- [ ] Memory protection for sensitive data
