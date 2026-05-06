# CSV Adapter Ethereum

[![Crates.io](https://img.shields.io/crates/v/csv-adapter-ethereum.svg)](https://crates.io/crates/csv-adapter-ethereum)
[![Documentation](https://docs.rs/csv-adapter-ethereum/badge.svg)](https://docs.rs/csv-adapter-ethereum)
[![License](https://img.shields.io/crates/l/csv-adapter-ethereum.svg)](https://github.com/client-side-validation/csv-adapter#license)

Ethereum adapter for **CSV (Client-Side Validation)** with nullifier-based seals and MPT proofs.

## Overview

This crate implements the [`AnchorLayer`] trait for Ethereum, using nullifier registration as **L3 Cryptographic** single-use enforcement. Unlike Bitcoin's structural single-use, Ethereum requires explicit nullifier tracking via smart contracts.

[`AnchorLayer`]: https://docs.rs/csv-adapter-core/latest/csv_adapter_core/trait.AnchorLayer.html

### Key Features

- **Nullifier Seals**: Cryptographic single-use enforcement via smart contract registry
- **MPT Proofs**: Merkle Patricia Trie inclusion proofs
- **LOG Events**: Commitment publication via Ethereum events
- **Alloy Integration**: Full compatibility with Alloy 0.9 ecosystem
- **Light Client**: State root verification without full node
- **Sepolia/Testnet**: Full testnet support

## Installation

```bash
cargo add csv-adapter-ethereum
```

Or in your `Cargo.toml`:

```toml
[dependencies]
csv-adapter-ethereum = "0.1"
```

### Features

| Feature | Description | Default |
|---------|-------------|---------|
| `rpc` | Enable full RPC client with Alloy integration | ❌ |
| `production` | Enable all production-ready features (`rpc`) | ❌ |

## Quick Start

### Creating an Ethereum Anchor Layer

```rust
use csv_adapter_ethereum::EthereumAnchorLayer;

// Create adapter with mock RPC for testing
let adapter = EthereumAnchorLayer::mock();

// Or with real RPC (requires `rpc` feature)
// let adapter = EthereumAnchorLayer::from_rpc(rpc_url, chain_id)?;
```

### Working with Nullifier Seals

```rust
use csv_adapter_ethereum::EthereumAnchorLayer;
use csv_adapter_core::{Hash, AnchorLayer};

let adapter = EthereumAnchorLayer::mock();

// Create a nullifier-based seal
let nullifier = Hash::new([0x01; 32]);
let seal = adapter.create_seal_from_nullifier(nullifier)?;

// Publish a commitment
let commitment = Hash::new([0xAB; 32]);
let anchor = adapter.publish(commitment, seal)?;
```

### MPT Proof Verification

```rust
use csv_adapter_ethereum::mpt::verify_mpt_proof;

// Verify a storage proof against state root
let is_valid = verify_mpt_proof(
    state_root,
    account_address,
    storage_slot,
    proof_nodes,
)?;
```

## Architecture

```
EthereumAnchorLayer
├── Nullifier Registry  ← L3 Cryptographic single-use
├── MPT Proofs         ← Merkle Patricia Trie verification
├── LOG Events         ← Commitment publication
├── Finality Checker   ← Block finality verification
└── Alloy RPC          ← Ethereum node interaction (optional)
```

### Seal Lifecycle

1. **Create**: Generate a nullifier hash (H(right_id || secret))
2. **Register**: Submit nullifier to smart contract (on-chain)
3. **Anchor**: Emit LOG event with commitment hash
4. **Verify**: MPT proof confirms inclusion in state root

## License

This project is dual-licensed under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

## Contributing

We welcome contributions! Please see our [GitHub repository](https://github.com/client-side-validation/csv-adapter) for more information.
