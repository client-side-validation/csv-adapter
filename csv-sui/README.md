# CSV Adapter Sui

[![Crates.io](https://img.shields.io/crates/v/csv-adapter-sui.svg)](https://crates.io/crates/csv-adapter-sui)
[![Documentation](https://docs.rs/csv-adapter-sui/badge.svg)](https://docs.rs/csv-adapter-sui)
[![License](https://img.shields.io/crates/l/csv-adapter-sui.svg)](https://github.com/client-side-validation/csv-adapter#license)

Sui adapter for **CSV (Client-Side Validation)** with object-based seals.

## Overview

This crate implements the [`AnchorLayer`] trait for Sui, using owned objects with one-time attributes as seals. Sui provides **L1 Structural** single-use enforcement through its object model — when an object is consumed (transferred/deleted), it cannot be reused.

[`AnchorLayer`]: https://docs.rs/csv-adapter-core/latest/csv_adapter_core/trait.AnchorLayer.html

### Key Features

- **Object Seals**: Native Sui single-use enforcement (structural guarantee)
- **Checkpoint Proofs**: Deterministic finality via checkpoint certification
- **Dynamic Fields**: Commitment anchoring via object metadata
- **Ed25519 Signatures**: Native Sui signing scheme
- **Testnet Support**: Full testnet RPC compatibility
- **Mock RPC**: Built-in mock for offline testing

## Installation

```bash
cargo add csv-adapter-sui
```

Or in your `Cargo.toml`:

```toml
[dependencies]
csv-adapter-sui = "0.1"
```

### Features

| Feature | Description | Default |
|---------|-------------|---------|
| `rpc` | Enable real Sui RPC client | ❌ |

## Quick Start

### Creating a Sui Anchor Layer

```rust
use csv_adapter_sui::{SuiAnchorLayer, SuiConfig, SuiNetwork};

// Create adapter with mock RPC for testing
let adapter = SuiAnchorLayer::with_mock().unwrap();

// Or with configuration (requires `rpc` feature)
// let config = SuiConfig::new(SuiNetwork::Testnet);
// let rpc = ...;
// let adapter = SuiAnchorLayer::from_config(config, rpc).unwrap();
```

### Working with Object Seals

```rust
use csv_adapter_sui::SuiAnchorLayer;
use csv_adapter_core::{Hash, AnchorLayer};

let adapter = SuiAnchorLayer::with_mock()?;

// Create a seal from a Sui object
let seal = adapter.create_seal(Some(object_id))?;

// Publish a commitment
let commitment = Hash::new([0xAB; 32]);
let anchor = adapter.publish(commitment, seal)?;
```

### Checkpoint Proof Verification

```rust
use csv_adapter_sui::CheckpointVerifier;

// Verify a checkpoint certification
let verifier = CheckpointVerifier::new(checkpoint);
let is_final = verifier.verify_finality(&transaction)?;
```

## Architecture

```
SuiAnchorLayer
├── Object Seals       ← L1 Structural single-use
├── Dynamic Fields     ← Commitment publication
├── Checkpoint Proofs  ← Narwhal consensus finality
├── Seal Registry      ← Double-spend prevention
└── RPC Client         ← Sui fullnode interaction (optional)
```

### Seal Lifecycle

1. **Create**: Derive a seal from a Sui object ID
2. **Consume**: Transfer/delete the object (seal is gone)
3. **Anchor**: Create dynamic field with commitment
4. **Verify**: Checkpoint proof confirms consensus finality

## License

This project is dual-licensed under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

## Contributing

We welcome contributions! Please see our [GitHub repository](https://github.com/client-side-validation/csv-adapter) for more information.
