# TIME Coin Mobile Repository Setup Guide

## Step-by-Step Guide to Creating time-coin-mobile Repository

### Option 1: Using GitHub CLI (Recommended)

```bash
# 1. Create repository on GitHub
gh repo create time-coin-mobile \
  --public \
  --description "TIME Coin mobile wallet for Android and iOS" \
  --clone

# 2. Navigate into the repository
cd time-coin-mobile

# 3. Create initial structure
mkdir -p android ios docs

# 4. Create README
cat > README.md << 'EOF'
# TIME Coin Mobile Wallet

Cross-platform mobile wallet for TIME Coin cryptocurrency.

## Features

- 📱 Native Android app (Phase 1)
- 🍎 Native iOS app (Phase 2 - planned)
- 🔐 Secure BIP-39/BIP-44 HD wallet
- ⚡ Real-time transaction notifications via TCP
- 🔔 Push notifications via FCM (background)
- 💰 Send/receive TIME coins
- 📊 Transaction history
- 🔒 Biometric authentication

## Project Status

- ✅ Android MVP: In Development
- ⏳ iOS: Planned

## Documentation

- [Android Quick Start](docs/ANDROID_QUICKSTART.md)
- [TCP Protocol](docs/TCP_PROTOCOL.md)
- [Architecture](docs/ARCHITECTURE.md)

## Requirements

### Android
- Android Studio Hedgehog (2023.1.1) or newer
- Minimum SDK: 26 (Android 8.0)
- Target SDK: 34 (Android 14)
- Kotlin 1.9+

### iOS (Future)
- Xcode 15+
- iOS 15+
- Swift 5.9+

## Quick Start

### Android Development

```bash
# Open Android project
cd android
# Open in Android Studio: File → Open → select android/
```

### Build from Command Line

```bash
cd android
./gradlew assembleDebug
```

## Architecture

```
┌─────────────────────────────────────────┐
│          Mobile App Architecture         │
├─────────────────────────────────────────┤
│                                           │
│  App Foreground → TCP (port 24100)      │
│    - Real-time notifications              │
│    - < 1 second latency                   │
│                                           │
│  App Background → FCM Push               │
│    - Wake app when closed                 │
│    - Battery efficient                    │
│                                           │
│  HTTP API Fallback                       │
│    - Wallet sync                          │
│    - Transaction submission               │
│                                           │
└─────────────────────────────────────────┘
```

## Tech Stack

### Android
- **Language**: Kotlin
- **UI**: Jetpack Compose
- **Crypto**: BitcoinJ (BIP-39, BIP-44)
- **Network**: OkHttp, Raw Sockets
- **Database**: Room
- **Security**: Android Keystore, Biometric

### iOS (Planned)
- **Language**: Swift
- **UI**: SwiftUI
- **Crypto**: BitcoinKit
- **Network**: URLSession, Raw Sockets

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md)

## Security

Report security issues to: security@time-coin.io

## License

Licensed under the same terms as TIME Coin core:
- MIT License: [LICENSE-MIT](LICENSE-MIT)
- Apache License 2.0: [LICENSE-APACHE](LICENSE-APACHE)

## Links

- [TIME Coin Main Repository](https://github.com/yourusername/time-coin)
- [Documentation](https://docs.time-coin.io)
- [Website](https://time-coin.io)
EOF

# 5. Create .gitignore
cat > .gitignore << 'EOF'
# Android
android/.gradle/
android/build/
android/local.properties
android/.idea/
android/*.iml
android/app/build/
android/app/release/
android/.DS_Store

# iOS
ios/Pods/
ios/*.xcworkspace
!ios/*.xcworkspace/contents.xcworkspacedata
ios/*.xcuserstate
ios/DerivedData/
ios/.DS_Store

# Secrets
*.keystore
*.jks
google-services.json
GoogleService-Info.plist
.env
secrets/

# IDE
.vscode/
.idea/

# OS
.DS_Store
Thumbs.db
EOF

# 6. Create initial docs
cat > docs/ARCHITECTURE.md << 'EOF'
# TIME Coin Mobile Architecture

## Overview

The TIME Coin mobile app uses a hybrid notification strategy:

1. **Foreground**: TCP direct connection to masternode
2. **Background**: FCM push notifications
3. **Fallback**: HTTP API polling

## Components

### Wallet Layer
- BIP-39 mnemonic generation
- BIP-44 address derivation
- Private key management (secure keystore)
- Transaction signing

### Network Layer
- TCP protocol client (port 24100)
- HTTP REST API client
- FCM push notification handler

### Storage Layer
- Encrypted wallet data
- Transaction history
- Address book
- User preferences

### UI Layer
- Balance display
- Send/receive screens
- Transaction history
- QR scanner
- Settings

## Security Model

1. **Private keys** stored in OS secure keystore
2. **Local data** encrypted with AES-256
3. **Biometric** authentication for transactions
4. **Certificate pinning** for API calls
5. **Root detection** on startup

## Data Flow

### Send Transaction
```
User Input → Validate → Build TX → Sign (keystore) 
  → Submit (HTTP) → Monitor (TCP) → Update UI
```

### Receive Transaction
```
Masternode → TCP Notification → Update Balance 
  → Show Notification → Update History
```

### Background Notification
```
Masternode → FCM → OS Wakes App → Fetch Details 
  → Update State → Show Notification
```

## Protocol

See [TCP_PROTOCOL.md](TCP_PROTOCOL.md) for details.
EOF

# 7. Copy protocol docs from main repo
cat > docs/TCP_PROTOCOL.md << 'EOF'
# TIME Coin TCP Protocol

## Connection

```
Host: masternode IP/domain
Port: 24100 (testnet), 24101 (mainnet)
Protocol: Length-prefixed JSON over TCP
```

## Message Format

```
[4-byte length (big-endian)][UTF-8 JSON message]
```

## Messages

### RegisterXpub (Client → Server)
```json
{
  "RegisterXpub": {
    "xpub": "xpub6CUGRUonZSQ4..."
  }
}
```

### XpubRegistered (Server → Client)
```json
{
  "XpubRegistered": {
    "success": true,
    "message": "Monitoring 20 addresses"
  }
}
```

### NewTransactionNotification (Server → Client)
```json
{
  "NewTransactionNotification": {
    "transaction": {
      "tx_hash": "abc123...",
      "from_address": "TIME1...",
      "to_address": "TIME1...",
      "amount": 50000000,
      "timestamp": 1732034400,
      "block_height": 0,
      "confirmations": 0
    }
  }
}
```

### UtxoUpdate (Server → Client)
```json
{
  "UtxoUpdate": {
    "xpub": "xpub6CUGRUonZSQ4...",
    "utxos": [
      {
        "txid": "abc123...",
        "vout": 0,
        "address": "TIME1...",
        "amount": 100000000,
        "block_height": 1234,
        "confirmations": 5
      }
    ]
  }
}
```

See main TIME Coin repo for full protocol specification.
EOF

# 8. Create Android project placeholder
cat > android/README.md << 'EOF'
# TIME Coin Android App

## Setup

1. Open Android Studio
2. File → Open → Select this `android/` directory
3. Wait for Gradle sync
4. Run on emulator or device

## Build

```bash
# Debug build
./gradlew assembleDebug

# Release build (requires signing key)
./gradlew assembleRelease
```

## Testing

```bash
# Unit tests
./gradlew test

# Instrumentation tests
./gradlew connectedAndroidTest
```

## Project Structure

```
app/
├── src/
│   ├── main/
│   │   ├── kotlin/com/timecoin/wallet/
│   │   │   ├── network/
│   │   │   │   ├── TcpProtocolClient.kt
│   │   │   │   └── HttpApiClient.kt
│   │   │   ├── wallet/
│   │   │   │   ├── Wallet.kt
│   │   │   │   ├── Bip39.kt
│   │   │   │   └── AddressDerivation.kt
│   │   │   ├── storage/
│   │   │   │   ├── WalletDatabase.kt
│   │   │   │   └── SecurePreferences.kt
│   │   │   ├── ui/
│   │   │   │   ├── MainActivity.kt
│   │   │   │   ├── SendScreen.kt
│   │   │   │   ├── ReceiveScreen.kt
│   │   │   │   └── HistoryScreen.kt
│   │   │   └── fcm/
│   │   │       └── TimeCoinMessagingService.kt
│   │   ├── res/
│   │   └── AndroidManifest.xml
│   ├── test/
│   └── androidTest/
└── build.gradle.kts
```
EOF

# 9. Create iOS placeholder
cat > ios/README.md << 'EOF'
# TIME Coin iOS App

Coming soon!

## Planned Features

- SwiftUI interface
- TCP protocol client
- APNs push notifications
- Keychain integration
- Face ID / Touch ID

## Requirements

- Xcode 15+
- iOS 15+
- Swift 5.9+
EOF

# 10. Create CONTRIBUTING.md
cat > CONTRIBUTING.md << 'EOF'
# Contributing to TIME Coin Mobile

## Development Setup

### Android

1. Install Android Studio
2. Clone repository
3. Open `android/` directory in Android Studio
4. Install dependencies (Gradle sync)
5. Run on emulator

### Code Style

- Follow Kotlin coding conventions
- Use ktlint for formatting
- Write unit tests for business logic
- Write UI tests for critical flows

## Testing

### Before Submitting PR

- [ ] All tests pass
- [ ] No lint errors
- [ ] Code is formatted
- [ ] Documentation updated
- [ ] Security review (if touching crypto/keys)

## Pull Request Process

1. Fork the repository
2. Create feature branch (`git checkout -b feature/amazing-feature`)
3. Commit changes (`git commit -m 'Add amazing feature'`)
4. Push to branch (`git push origin feature/amazing-feature`)
5. Open Pull Request

## Security

Report security vulnerabilities privately to: security@time-coin.io

Do NOT open public issues for security bugs.

## License

By contributing, you agree your contributions will be licensed under MIT and Apache 2.0.
EOF

# 11. Copy licenses from main repo
# (You'll need to do this manually or script it)
echo "TODO: Copy LICENSE-MIT and LICENSE-APACHE from main repo"

# 12. Create initial commit
git add .
git commit -m "Initial commit: TIME Coin mobile wallet repository structure"

# 13. Push to GitHub
git push -u origin main

echo "✅ Repository created successfully!"
echo ""
echo "Next steps:"
echo "1. Copy LICENSE files from main TIME Coin repo"
echo "2. Create Android project in Android Studio"
echo "3. Set up GitHub Actions for CI/CD"
echo "4. Configure branch protection rules"
```

### Option 2: Using GitHub Web Interface

1. **Go to GitHub**: https://github.com/new

2. **Fill in repository details**:
   - Repository name: `time-coin-mobile`
   - Description: `TIME Coin mobile wallet for Android and iOS`
   - Public repository
   - ✅ Add README file
   - ✅ Add .gitignore (select "Android")
   - ✅ Choose license (MIT or Apache 2.0)

3. **Clone locally**:
   ```bash
   git clone https://github.com/yourusername/time-coin-mobile.git
   cd time-coin-mobile
   ```

4. **Follow steps 3-11 from Option 1** to set up structure

## Android Project Setup

### Create New Android Project in Android Studio

1. **Open Android Studio**

2. **New Project**:
   - Template: "Empty Activity"
   - Name: "TIME Coin Wallet"
   - Package name: `com.timecoin.wallet`
   - Save location: `time-coin-mobile/android/`
   - Language: Kotlin
   - Minimum SDK: 26 (Android 8.0)
   - Build configuration language: Kotlin DSL (build.gradle.kts)

3. **Configure build.gradle.kts (Project level)**:
   ```kotlin
   // time-coin-mobile/android/build.gradle.kts
   plugins {
       id("com.android.application") version "8.2.0" apply false
       id("org.jetbrains.kotlin.android") version "1.9.20" apply false
       id("com.google.devtools.ksp") version "1.9.20-1.0.14" apply false
   }
   ```

4. **Configure build.gradle.kts (App level)**:
   ```kotlin
   // time-coin-mobile/android/app/build.gradle.kts
   plugins {
       id("com.android.application")
       id("org.jetbrains.kotlin.android")
       id("com.google.devtools.ksp")
   }
   
   android {
       namespace = "com.timecoin.wallet"
       compileSdk = 34
       
       defaultConfig {
           applicationId = "com.timecoin.wallet"
           minSdk = 26
           targetSdk = 34
           versionCode = 1
           versionName = "1.0.0"
           
           testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
           vectorDrawables {
               useSupportLibrary = true
           }
       }
       
       buildTypes {
           release {
               isMinifyEnabled = true
               proguardFiles(
                   getDefaultProguardFile("proguard-android-optimize.txt"),
                   "proguard-rules.pro"
               )
               signingConfig = signingConfigs.getByName("debug") // TODO: Create release signing config
           }
       }
       
       compileOptions {
           sourceCompatibility = JavaVersion.VERSION_17
           targetCompatibility = JavaVersion.VERSION_17
       }
       
       kotlinOptions {
           jvmTarget = "17"
       }
       
       buildFeatures {
           compose = true
       }
       
       composeOptions {
           kotlinCompilerExtensionVersion = "1.5.4"
       }
       
       packaging {
           resources {
               excludes += "/META-INF/{AL2.0,LGPL2.1}"
               excludes += "META-INF/versions/9/previous-compilation-data.bin"
           }
       }
   }
   
   dependencies {
       // Android Core
       implementation("androidx.core:core-ktx:1.12.0")
       implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.7.0")
       implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.7.0")
       
       // Compose
       implementation(platform("androidx.compose:compose-bom:2024.01.00"))
       implementation("androidx.compose.ui:ui")
       implementation("androidx.compose.ui:ui-graphics")
       implementation("androidx.compose.ui:ui-tooling-preview")
       implementation("androidx.compose.material3:material3")
       implementation("androidx.activity:activity-compose:1.8.2")
       implementation("androidx.navigation:navigation-compose:2.7.6")
       
       // Bitcoin/Crypto
       implementation("org.bitcoinj:bitcoinj-core:0.16.2")
       
       // Network
       implementation("com.squareup.okhttp3:okhttp:4.12.0")
       implementation("com.squareup.retrofit2:retrofit:2.9.0")
       implementation("com.squareup.retrofit2:converter-gson:2.9.0")
       
       // Database
       val roomVersion = "2.6.1"
       implementation("androidx.room:room-runtime:$roomVersion")
       implementation("androidx.room:room-ktx:$roomVersion")
       ksp("androidx.room:room-compiler:$roomVersion")
       
       // Security
       implementation("androidx.security:security-crypto:1.1.0-alpha06")
       implementation("androidx.biometric:biometric:1.2.0-alpha05")
       
       // QR Code
       implementation("com.google.zxing:core:3.5.2")
       
       // Coroutines
       implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.7.3")
       
       // Firebase (Phase 2)
       // implementation("com.google.firebase:firebase-messaging:23.4.0")
       
       // Testing
       testImplementation("junit:junit:4.13.2")
       testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.7.3")
       androidTestImplementation("androidx.test.ext:junit:1.1.5")
       androidTestImplementation("androidx.test.espresso:espresso-core:3.5.1")
       androidTestImplementation(platform("androidx.compose:compose-bom:2024.01.00"))
       androidTestImplementation("androidx.compose.ui:ui-test-junit4")
       debugImplementation("androidx.compose.ui:ui-tooling")
       debugImplementation("androidx.compose.ui:ui-test-manifest")
   }
   ```

5. **Commit Android project**:
   ```bash
   cd time-coin-mobile
   git add android/
   git commit -m "Add Android project structure"
   git push
   ```

## Repository Settings

### Configure Branch Protection

1. Go to: `Settings → Branches → Add rule`

2. **Branch name pattern**: `main`

3. **Enable**:
   - ✅ Require pull request reviews before merging
   - ✅ Require status checks to pass before merging
   - ✅ Require branches to be up to date before merging
   - ✅ Require linear history
   - ✅ Include administrators

### Set Up GitHub Actions

Create `.github/workflows/android-ci.yml`:

```yaml
name: Android CI

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main, develop ]

jobs:
  build:
    runs-on: ubuntu-latest
    
    steps:
    - uses: actions/checkout@v4
    
    - name: Set up JDK 17
      uses: actions/setup-java@v3
      with:
        java-version: '17'
        distribution: 'temurin'
        cache: gradle
    
    - name: Grant execute permission for gradlew
      working-directory: ./android
      run: chmod +x gradlew
    
    - name: Build with Gradle
      working-directory: ./android
      run: ./gradlew build
    
    - name: Run tests
      working-directory: ./android
      run: ./gradlew test
    
    - name: Upload build artifacts
      uses: actions/upload-artifact@v3
      with:
        name: app-debug
        path: android/app/build/outputs/apk/debug/app-debug.apk
```

### Add Repository Topics

Settings → General → Topics:
- `cryptocurrency`
- `bitcoin`
- `mobile-wallet`
- `android`
- `kotlin`
- `blockchain`
- `time-coin`

### Create Project Board (Optional)

Projects → New project → Board template:
- 📋 Backlog
- 🏃 In Progress
- 👀 In Review
- ✅ Done

## Initial Issues to Create

Create these issues to track work:

### Issue #1: Project Setup
```markdown
**Title**: Android project initial setup

**Description**:
- [ ] Configure Gradle build files
- [ ] Add dependencies
- [ ] Set up package structure
- [ ] Create README
- [ ] Configure ProGuard rules

**Labels**: setup, android
```

### Issue #2: TCP Protocol Client
```markdown
**Title**: Implement TCP protocol client

**Description**:
Implement client for TIME Coin TCP protocol (port 24100).

**Tasks**:
- [ ] Create TcpProtocolClient class
- [ ] Implement message serialization/deserialization
- [ ] Handle RegisterXpub message
- [ ] Handle NewTransactionNotification
- [ ] Handle UtxoUpdate
- [ ] Add reconnection logic
- [ ] Write unit tests

**Labels**: enhancement, network, high-priority
```

### Issue #3: Wallet Core
```markdown
**Title**: Implement BIP-39/BIP-44 wallet

**Description**:
Create wallet functionality using BitcoinJ.

**Tasks**:
- [ ] Mnemonic generation (BIP-39)
- [ ] Seed derivation
- [ ] Address derivation (BIP-44)
- [ ] Private key management
- [ ] Transaction signing
- [ ] Integration with Android Keystore
- [ ] Write unit tests

**Labels**: enhancement, wallet, high-priority
```

## Documentation Structure

```
time-coin-mobile/
├── README.md                    # Overview, quick start
├── CONTRIBUTING.md              # How to contribute
├── LICENSE-MIT                  # MIT license
├── LICENSE-APACHE              # Apache 2.0 license
├── docs/
│   ├── ARCHITECTURE.md         # System architecture
│   ├── TCP_PROTOCOL.md         # TCP protocol spec
│   ├── ANDROID_SETUP.md        # Android dev setup
│   ├── IOS_SETUP.md            # iOS dev setup (future)
│   ├── SECURITY.md             # Security considerations
│   └── API.md                  # HTTP API reference
├── android/                     # Android app
└── ios/                        # iOS app (future)
```

## Recommended Git Workflow

### Branching Strategy

```
main (production-ready)
  ↓
develop (integration branch)
  ↓
feature/tcp-client
feature/wallet-core
feature/ui-send-receive
bugfix/connection-timeout
```

### Commit Message Convention

```
feat: Add TCP protocol client
fix: Resolve connection timeout issue
docs: Update setup instructions
test: Add wallet derivation tests
refactor: Simplify message parsing
chore: Update dependencies
```

### Tags for Releases

```bash
git tag -a v0.1.0 -m "Initial alpha release"
git push origin v0.1.0
```

## Security Considerations

### Don't Commit These Files

Already in `.gitignore`, but double-check:
- `*.keystore` - Signing keys
- `*.jks` - Signing keys
- `google-services.json` - Firebase config
- `local.properties` - Local paths
- `.env` - Environment variables

### Use GitHub Secrets

For CI/CD, add these secrets:
- `ANDROID_KEYSTORE` - Base64 encoded keystore
- `KEYSTORE_PASSWORD` - Keystore password
- `KEY_ALIAS` - Key alias
- `KEY_PASSWORD` - Key password

## Next Steps After Repository Creation

1. ✅ Create repository structure
2. ✅ Set up Android project
3. ✅ Configure CI/CD
4. ✅ Set up branch protection
5. ⏳ Implement TCP client (Issue #2)
6. ⏳ Implement wallet core (Issue #3)
7. ⏳ Build UI screens
8. ⏳ Add security features
9. ⏳ Testing and QA
10. ⏳ Release MVP

## Useful Commands

```bash
# Clone and set up
git clone https://github.com/yourusername/time-coin-mobile.git
cd time-coin-mobile

# Create feature branch
git checkout -b feature/tcp-client

# Build Android app
cd android
./gradlew assembleDebug

# Run tests
./gradlew test

# Install on device
./gradlew installDebug

# Create release build
./gradlew assembleRelease
```

## Resources

- [Android Developer Guide](https://developer.android.com/)
- [Kotlin Documentation](https://kotlinlang.org/docs/)
- [BitcoinJ Documentation](https://bitcoinj.org/)
- [Jetpack Compose](https://developer.android.com/jetpack/compose)
- [Material Design 3](https://m3.material.io/)

## Support

- GitHub Issues: https://github.com/yourusername/time-coin-mobile/issues
- Main Repo: https://github.com/yourusername/time-coin
- Documentation: Refer to `docs/` directory
