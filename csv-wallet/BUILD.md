# Build Guide

## Quick Start

```bash
# Development
cd csv-wallet
cargo check          # Verify compilation
dx serve             # Run development server with hot reload
```

## Build for Different Platforms

### Web (Default)

```bash
# Install Dioxus CLI
cargo install dioxus-cli

# Development server
dx serve

# Production build
dx build --release
```

Output will be in `target/dx/csv-wallet/release/web/`

### Desktop (Tauri/Electron)

```bash
# Build for desktop
dx build --platform desktop --release

# Or use Tauri directly
cargo install tauri-cli
cargo tauri build
```

### Mobile - iOS

```bash
# Prerequisites:
# - Xcode (macOS only)
# - iOS development certificate

# Build for iOS
dx build --platform ios --release

# Open in Xcode
open target/dx/csv-wallet/release/ios/CSV_Wallet.xcodeproj
```

### Mobile - Android

```bash
# Prerequisites:
# - Android SDK & NDK
# - JAVA_HOME set
# - ANDROID_HOME set

# Build for Android
dx build --platform android --release

# Or use cargo-ndk
cargo install cargo-ndk
cargo ndk -t arm64-v8a build --release
```

## Production Deployment

### Static Hosting (GitHub Pages, Netlify, Vercel)

```bash
# Build for production
dx build --release

# Deploy the contents of target/dx/csv-wallet/release/web/public
```

### Docker

```dockerfile
FROM rust:latest as builder
WORKDIR /app
COPY . .
RUN cargo install dioxus-cli
RUN dx build --release

FROM nginx:alpine
COPY --from=builder /app/target/dx/csv-wallet/release/web/public /usr/share/nginx/html
```

## Troubleshooting

### Dependency Conflicts

csv-wallet is intentionally excluded from the main workspace due to web-sys version conflicts with aptos-sdk. Build it independently:

```bash
cd csv-wallet
cargo check  # NOT cargo check -p csv-wallet from workspace root
```

### Wasm Target

```bash
rustup target add wasm32-unknown-unknown
cargo build --target wasm32-unknown-unknown
```

### Common Issues

1. **"localStorage not available"**: Ensure you're running in a browser context
2. **CORS errors**: Use a local dev server, don't open HTML file directly
3. **Build fails**: Run `cargo clean` and rebuild
