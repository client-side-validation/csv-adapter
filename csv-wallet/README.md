# CSV Wallet — Standalone Multi-Chain Wallet

[![Build](https://img.shields.io/badge/build-passing-bsanadgreen)]()
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)]()

**CSV Wallet** is a standalone multi-chain wallet for the **CSV (Client-Side Validation)** ecosystem. Built with [Dioxus](https://dioxuslabs.com/), it provides a modern web-based interface for managing wallets, seals, assets, and transfers across multiple blockchain networks.

## Features

### 🔐 Wallet Management

- **Create Wallet**: Generate new HD wallets with BIP-39 mnemonic phrases
- **Import Wallet**: Restore wallets from existing mnemonic phrases
- **Export Wallet**: Backup and export wallet recovery phrases
- **Multi-Chain Addresses**: Automatic address derivation for all supported chains

### 🔒 Seal Management

- **Create Seals**: Generate seals on any supported chain (Bitcoin, Ethereum, Sui, Aptos)
- **Monitor Seals**: Track seal status (unconsumed, consumed, double-spent)
- **Seal History**: View complete seal transaction history
- **Transfer Seals**: Consume seals and generate ownership proofs

### 💎 Asset Tracking

- **View Assets**: See all owned Sanads across all chains
- **Valuation**: Real-time USD value tracking for all assets
- **Asset Details**: Detailed information about each asset including commitment, ownership, and seal data
- **Portfolio Overview**: Total portfolio value at a glance

### 🌐 Multi-Chain Support

- **Bitcoin**: Signet (testnet) and Mainnet
- **Ethereum**: Sepolia (testnet) and Mainnet
- **Sui**: Testnet and Mainnet
- **Aptos**: Testnet and Mainnet

### ⚙️ Network Configuration

- **Network Switching**: Easily switch between testnet and mainnet per chain
- **Explorer Integration**: View transactions and seals in blockchain explorers
- **Faucet Access**: Request testnet funds for development

## Quick Start

### Prerequisites

- Rust 1.75 or higher
- Dioxus CLI (for web development)

```bash
cargo install dioxus-cli
```

### Build

```bash
# Check compilation
cargo check -p csv-wallet

# Build for web (wasm32)
cd csv-wallet
dx build --release

# Serve locally with hot reload
dx serve
```

Then open **http://localhost:8181** in your browser.

### Available Wallet Commands (matching csv-cli)

The wallet UI supports all `csv-cli` wallet commands:

| Tab | Description | csv-cli Equivalent |
|-----|-------------|-------------------|
| **Overview** | Dashboard with quick actions | - |
| **Generate** | Create new wallet with chain/network selection | `csv wallet generate <chain> [network]` |
| **Import** | Import from private key or mnemonic | `csv wallet import <chain> <secret>` |
| **Balance** | Check wallet balance | `csv wallet balance <chain>` |
| **Fund** | Request test tokens from faucet | `csv wallet fund <chain>` |
| **Export** | Export wallet address/data | `csv wallet export <chain>` |
| **List** | View all wallets table | `csv wallet list` |

## Architecture

```
csv-wallet/
├── src/
│   ├── main.rs                    # Dioxus app entry point
│   ├── wallet_core.rs             # Core wallet functionality
│   │
│   ├── core/                      # Core modules
│   │   ├── wallet.rs              # Extended wallet types
│   │   ├── key_manager.rs         # Key derivation & signing
│   │   ├── storage.rs             # Wallet storage (in-memory/IndexedDB)
│   │   └── encryption.rs          # AES-256-GCM encryption
│   │
│   ├── chains/                    # Chain integrations
│   │   ├── bitcoin.rs             # Bitcoin address derivation
│   │   ├── ethereum.rs            # Ethereum address derivation
│   │   ├── sui.rs                 # Sui address derivation
│   │   └── aptos.rs               # Aptos address derivation
│   │
│   ├── seals/                     # Seal management
│   │   ├── manager.rs             # Seal CRUD operations
│   │   ├── store.rs               # Seal persistence
│   │   └── monitor.rs             # On-chain status monitoring
│   │
│   ├── assets/                    # Asset tracking
│   │   ├── tracker.rs             # Asset management
│   │   ├── valuation.rs           # Price feeds & valuation
│   │   └── details.rs             # Asset detail views
│   │
│   ├── services/                  # External services
│   │   └── network.rs             # Network configuration
│   │
│   ├── hooks/                     # Dioxus hooks
│   │   ├── use_wallet.rs          # Wallet state management
│   │   ├── use_network.rs         # Network state management
│   │   ├── use_seals.rs           # Seal state management
│   │   └── use_assets.rs          # Asset state management
│   │
│   ├── components/                # Reusable UI components
│   │   ├── header.rs              # App header
│   │   ├── sidebar.rs             # Navigation sidebar
│   │   ├── wallet_card.rs         # Wallet display
│   │   ├── asset_list.rs          # Asset list
│   │   ├── seal_list.rs           # Seal list
│   │   └── modal.rs               # Modal dialogs
│   │
│   └── pages/                     # Page components
│       ├── dashboard.rs           # Main dashboard
│       ├── wallet_create.rs       # Create wallet
│       ├── wallet_import.rs       # Import wallet
│       ├── wallet_export.rs       # Export wallet
│       ├── seals.rs               # Seal management
│       ├── assets.rs              # Asset overview
│       ├── transfer.rs            # Transfer interface
│       └── settings.rs            # Settings
│
├── Cargo.toml
└── README.md
```

## Technology Stack

| Component | Technology | Rationale |
|-----------|-----------|-----------|
| **UI Framework** | Dioxus 0.6 | Cross-platform (web, desktop, mobile) from single codebase |
| **State Management** | Dioxus Signals | Built-in reactive state management |
| **Cryptography** | bip32, secp256k1, ed25519-dalek | Industry-standard cryptographic libraries |
| **Encryption** | AES-256-GCM | Secure wallet encryption |
| **Serialization** | serde, serde_json | Rust serialization standard |
| **Chain Types** | csv-adapter-core | Reuses existing CSV ecosystem types |

## Cross-Platform Roadmap

### Current: Web (wasm32)

The wallet currently runs as a web application in the browser using WebAssembly.

### Future: Desktop

```bash
# Build for desktop (using Dioxus desktop renderer)
dx build --platform desktop
```

### Future: Mobile (iOS & Android)

```bash
# Build for iOS
dx build --platform ios

# Build for Android
dx build --platform android
```

Dioxus uses native platform renderers, providing true native performance on each platform from the same codebase.

## Security Considerations

### Current Implementation

- Mnemonic phrases are stored in memory only
- No persistent storage implemented yet
- Encryption infrastructure ready (AES-256-GCM)

### Production Requirements

- [ ] Encrypt wallet at rest with user password
- [ ] Implement IndexedDB or secure native storage
- [ ] Add auto-lock timeout
- [ ] Implement hardware wallet support (Ledger, Trezor)
- [ ] Add biometric authentication (mobile)
- [ ] Implement secure memory clearing (zeroize)
- [ ] Add multi-signature support

## Usage Examples

### Creating a Wallet

```rust
use csv_wallet::wallet_core::ExtendedWallet;

// Generate new wallet
let wallet = ExtendedWallet::generate();

// Get all addresses
let addresses = wallet.all_addresses();
for (chain, addr) in addresses {
    println!("{}: {}", chain, addr);
}

// Export mnemonic for backup
let recovery_phrase = wallet.mnemonic.clone();
```

### Importing a Wallet

```rust
use csv_wallet::wallet_core::ExtendedWallet;

// Restore from mnemonic
let recovery_phrase = "word1 word2 word3 ... word24";
let wallet = ExtendedWallet::from_mnemonic(recovery_phrase)
    .expect("Invalid mnemonic");
```

### Managing Seals

```rust
use csv_wallet::seals::{SealManager, SealStore};
use csv_adapter_core::Chain;

let store = SealStore::new();
let manager = SealManager::new(store);

// Create a seal on Bitcoin testnet
// You MUST obtain the seal_ref from a chain adapter's create_seal() method
// to get a real chain-native seal. Fake/timestamp-based seal IDs are rejected.
//
// Example with a real seal reference from Bitcoin adapter:
// let seal_ref = bitcoin_adapter.create_seal(satoshi_amount).await?;
// let seal = manager.create_seal(Chain::Bitcoin, Some(100_000), Some(seal_ref))?;

// Check seal status
let is_consumed = manager.is_seal_consumed(&seal.id)?;
```

## Integration with CSV Ecosystem

CSV Wallet integrates seamlessly with the broader CSV Adapter ecosystem:

- **csv-adapter-core**: Core types (Sanad, Chain, etc.)
- **csv-adapter-bitcoin**: Bitcoin seal operations
- **csv-adapter-ethereum**: Ethereum seal operations
- **csv-adapter-sui**: Sui seal operations
- **csv-adapter-aptos**: Aptos seal operations
- **csv-cli**: Command-line interface (complementary tool)
- **csv-explorer**: Blockchain explorer (future integration)

## Development

### Project Structure

The wallet is organized as a workspace member of the main csv-adapter project:

```toml
[workspace]
members = [
    "csv-adapter-core",
    "csv-adapter-bitcoin",
    "csv-adapter-ethereum",
    "csv-adapter-sui",
    "csv-adapter-aptos",
    "csv-adapter-store",
    "csv-adapter",
    "csv-cli",
    "csv-wallet",  # <-- This crate
]
```

### Adding Features

To add new functionality:

1. Create module in appropriate directory (`core/`, `chains/`, etc.)
2. Add hook in `hooks/` for state management
3. Create components in `components/` for reusable UI
4. Create pages in `pages/` for routes
5. Update `routes.rs` if adding new routes

### Testing

```bash
# Run tests (when tests are added)
cargo test -p csv-wallet

# Run with wasm target
cargo test -p csv-wallet --target wasm32-unknown-unknown
```

## Contributing

1. Fork the repository
2. Create your feature branch (`git checkout -b feature/amazing-feature`)
3. Commit your changes (`git commit -m 'Add amazing feature'`)
4. Push to the branch (`git push origin feature/amazing-feature`)
5. Open a Pull Request

## License

MIT or Apache-2.0 — choose the license that best fits your use case.

## Acknowledgments

- **Dioxus Team**: For the excellent cross-platform UI framework
- **CSV Adapter Contributors**: For the client-side validation infrastructure
- **Rust Community**: For the cryptographic libraries
