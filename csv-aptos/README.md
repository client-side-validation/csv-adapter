# CSV Adapter Aptos

[![Crates.io](https://img.shields.io/crates/v/csv-adapter-aptos.svg)](https://crates.io/crates/csv-adapter-aptos)
[![Documentation](https://docs.rs/csv-adapter-aptos/badge.svg)](https://docs.rs/csv-adapter-aptos)
[![License](https://img.shields.io/crates/l/csv-adapter-aptos.svg)](https://github.com/client-side-validation/csv-adapter#license)

Aptos adapter for **CSV (Client-Side Validation)** with resource-based seals.

## Overview

This crate implements the [`AnchorLayer`] trait for Aptos, using Move resources with key + delete as seals. Aptos provides **L2 Type-Enforced** single-use through the Move language's resource model — resources cannot be duplicated or dropped, only moved or destroyed.

[`AnchorLayer`]: https://docs.rs/csv-adapter-core/latest/csv_adapter_core/trait.AnchorLayer.html

### Key Features

- **Resource Seals**: Move resource lifecycle management (L2 Type-Enforced)
- **Event Proofs**: Commitment publication via Aptos events
- **HotStuff Finality**: Deterministic finality via 2f+1 certification
- **Ed25519 Signatures**: Native Aptos signing scheme
- **Testnet Support**: Full testnet RPC compatibility
- **Mock RPC**: Built-in mock for offline testing

## Installation

```bash
cargo add csv-adapter-aptos
```

Or in your `Cargo.toml`:

```toml
[dependencies]
csv-adapter-aptos = "0.1"
```

### Features

| Feature | Description | Default |
|---------|-------------|---------|
| `rpc` | Enable real Aptos RPC client with `aptos-sdk` | ❌ |
| `aptos-sdk` | Enable full Aptos SDK integration | ❌ |

## Quick Start

### Creating an Aptos Anchor Layer

```rust
use csv_adapter_aptos::{AptosAnchorLayer, AptosConfig, AptosNetwork};

// Create adapter with mock RPC for testing
let adapter = AptosAnchorLayer::with_mock().unwrap();

// Or with configuration (requires `rpc` feature)
// let config = AptosConfig::new(AptosNetwork::Devnet);
// let rpc = ...;
// let adapter = AptosAnchorLayer::from_config(config, rpc).unwrap();
```

### Working with Resource Seals

```rust
use csv_adapter_aptos::AptosAnchorLayer;
use csv_adapter_core::{Hash, AnchorLayer};

let adapter = AptosAnchorLayer::with_mock()?;

// Create a seal from a Move resource
let seal = adapter.create_seal(Some(resource_address))?;

// Publish a commitment
let commitment = Hash::new([0xAB; 32]);
let anchor = adapter.publish(commitment, seal)?;
```

### Checkpoint Proof Verification

```rust
use csv_adapter_aptos::CheckpointVerifier;

// Verify HotStuff consensus certification
let verifier = CheckpointVerifier::new(ledger_info);
let is_final = verifier.verify_finality(&transaction)?;
```

## Architecture

```
AptosAnchorLayer
├── Resource Seals     ← L2 Type-Enforced single-use (Move)
├── Event Proofs       ← Commitment publication via events
├── Ledger Info        ← HotStuff consensus finality
├── Seal Registry      ← Double-spend prevention
└── RPC Client         ← Aptos fullnode interaction (optional)
```

### Seal Lifecycle

1. **Create**: Derive a seal from a Move resource address
2. **Consume**: Destroy the resource via `move_from` (seal is gone)
3. **Anchor**: Emit event with commitment hash
4. **Verify**: Ledger info proof confirms consensus finality

## License

This project is dual-licensed under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

## Contributing

We welcome contributions! Please see our [GitHub repository](https://github.com/client-side-validation/csv-adapter) for more information.
