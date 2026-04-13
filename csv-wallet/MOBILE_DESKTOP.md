# Mobile & Desktop Configuration

## Current Status

CSV Wallet is built with Dioxus 0.6, which supports multiple platforms from a single codebase.

**Current target**: Web (wasm32)
**Future targets**: Desktop (macOS, Windows, Linux), Mobile (iOS, Android)

## Adding Desktop Support

### 1. Update Cargo.toml

```toml
[dependencies]
dioxus = { version = "0.6", features = ["web", "router", "desktop"] }
```

### 2. Create src-tauri/ (for native desktop)

```
csv-wallet/
├── src-tauri/
│   ├── Cargo.toml
│   ├── tauri.conf.json
│   ├── build.rs
│   └── src/
│       └── main.rs
```

**src-tauri/tauri.conf.json:**
```json
{
  "build": {
    "beforeBuildCommand": "dx build --release",
    "devPath": "http://localhost:8080",
    "distDir": "../target/dx/csv-wallet/release/web/public"
  },
  "package": {
    "productName": "CSV Wallet",
    "version": "0.1.0"
  },
  "tauri": {
    "bundle": {
      "active": true,
      "targets": "all",
      "identifier": "com.csv-adapter.wallet",
      "icon": ["icons/32x32.png", "icons/128x128.png", "icons/icon.icns", "icons/icon.ico"]
    },
    "security": {
      "csp": "default-src 'self'"
    }
  }
}
```

### 3. Build Desktop

```bash
cargo install tauri-cli
cargo tauri dev    # Development
cargo tauri build  # Production
```

## Adding Mobile Support

### iOS Configuration

**ios/CSVWallet/Info.plist additions:**
```xml
<key>NSFaceIDUsageDescription</key>
<string>Use FaceID to unlock wallet</string>
<key>NSCameraUsageDescription</key>
<string>Scan QR codes for addresses</string>
```

### Android Configuration

**android/app/build.gradle:**
```gradle
android {
    defaultConfig {
        applicationId "com.csvadapter.wallet"
        minSdkVersion 24
        targetSdkVersion 34
        versionCode 1
        versionName "0.1.0"
    }
}
```

### Mobile-Specific Features

1. **Biometric Authentication**: Use FaceID/TouchID/Fingerprint for wallet unlock
2. **Secure Enclave**: Store encrypted keys in device secure storage (Keychain/Keystore)
3. **QR Code Scanner**: Scan addresses and wallet connection QR codes
4. **Push Notifications**: Notify on seal consumption, transfers
5. **Deep Links**: Handle `csv://` URLs for payment requests

## Platform-Specific Storage

| Platform | Storage Method |
|----------|---------------|
| Web | localStorage / IndexedDB |
| Desktop | File system (encrypted SQLite) |
| iOS | Keychain + encrypted file |
| Android | Keystore + encrypted SharedPreferences |

## Build Matrix

| Platform | Command | Status |
|----------|---------|--------|
| Web (dev) | `dx serve` | ✅ Working |
| Web (prod) | `dx build --release` | ⏳ Ready |
| Desktop | `cargo tauri build` | 📋 Planned |
| iOS | `dx build --platform ios` | 📋 Planned |
| Android | `dx build --platform android` | 📋 Planned |

## Next Steps

1. Test web production build
2. Add Tauri desktop configuration
3. Add Capacitor/Cordova for mobile
4. Implement platform-specific storage
5. Add biometric authentication
6. Test on real devices
