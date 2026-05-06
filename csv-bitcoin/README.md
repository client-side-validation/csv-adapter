# CSV Adapter Bitcoin

[![Crates.io](https://img.shields.io/crates/v/csv-adapter-bitcoin.svg)](https://crates.io/crates/csv-adapter-bitcoin)
[![Documentation](https://docs.rs/csv-adapter-bitcoin/badge.svg)](https://docs.rs/csv-adapter-bitcoin)
[![License](https://img.shields.io/crates/l/csv-adapter-bitcoin.svg)](https://github.com/client-side-validation/csv-adapter#license)

Bitcoin adapter for **CSV (Client-Side Validation)** with UTXO seals and SPV proofs.

## Overview

This crate implements the [`AnchorLayer`] trait for Bitcoin, using UTXOs as single-use seals and Tapret/Opret for commitment publication. Bitcoin provides **L1 Structural** single-use enforcement through its UTXO model — when a UTXO is spent, it is consumed forever.

[`AnchorLayer`]: https://docs.rs/csv-adapter-core/latest/csv_adapter_core/trait.AnchorLayer.html

### Key Features

- **UTXO Seals**: Native Bitcoin single-use enforcement (structural guarantee)
- **Tapret Commitments**: Taproot-based commitment anchoring (BIP-341)
- **SPV Proofs**: Simplified Payment Verification with Merkle proofs
- **BIP-86 Wallets**: Taproot key path derivation
- **Signet/Testnet/Regtest**: Full network support
- **RPC Integration**: Optional Bitcoin Core RPC client

## Installation

```bash
cargo add csv-adapter-bitcoin
```

Or in your `Cargo.toml`:

```toml
[dependencies]
csv-adapter-bitcoin = "0.1"
```

### Features

| Feature | Description | Default |
|---------|-------------|---------|
| `rpc` | Enable Bitcoin Core RPC client (`bitcoincore-rpc`) | ❌ |
| `signet-rest` | Enable Signet REST API client (mempool.space) | ❌ |
| `production` | Enable all production-ready features (`rpc`) | ❌ |

## Quick Start

### Creating a Bitcoin Anchor Layer

```rust
use csv_adapter_bitcoin::{BitcoinAnchorLayer, Network};

// Create adapter for Signet (test network)
let adapter = BitcoinAnchorLayer::signet()?;

// Or for mainnet
// let adapter = BitcoinAnchorLayer::mainnet()?;
```

### Working with UTXO Seals

```rust
use csv_adapter_bitcoin::{BitcoinAnchorLayer, WalletUtxo};
use csv_adapter_core::{Hash, AnchorLayer};

let adapter = BitcoinAnchorLayer::signet()?;

// Create a seal from a UTXO
let seal = adapter.create_seal(Some(100_000))?;

// Publish a commitment (anchors a Right to Bitcoin)
let commitment = Hash::new([0xAB; 32]);
let anchor = adapter.publish(commitment, seal)?;
```

### Taproot Commitments

```rust
use csv_adapter_bitcoin::{mine_tapret_nonce, TapretCommitment};

// Create a Tapret commitment
let tapret = TapretCommitment::new(internal_key, merkle_root)?;

// Mine a nonce for the Tapret script
let nonce = mine_tapret_nonce(&tapret)?;
```

### SPV Proof Verification

```rust
use csv_adapter_bitcoin::SpvVerifier;

// Verify a Merkle proof against a block header
let verifier = SpvVerifier::new(block_header, merkle_branch);
let is_valid = verifier.verify(txid)?;
```

## Architecture

```
BitcoinAnchorLayer
├── UTXO Seals         ← Native single-use (L1 Structural)
├── Tapret/Opret       ← Commitment publication
├── SPV Proofs         ← Merkle tree verification
├── BIP-86 Wallets     ← Taproot key derivation
└── RPC/REST Clients   ← Network interaction
```

### Seal Lifecycle

1. **Create**: Derive a seal from an unspent UTXO
2. **Consume**: Spend the UTXO (seal is gone forever)
3. **Anchor**: Publish commitment via Tapret/Opret
4. **Verify**: SPV proof confirms the transaction in a block

## Examples

See the [`examples/`](examples/) directory for usage patterns:

- **`signet_real_tx_demo`** — Real transaction on Signet (requires `signet-rest` feature)
- **`signet_funding_addr`** — Generate funding addresses (requires `signet-rest` feature)
- **`signet_fixed_seed`** — Deterministic wallet generation (requires `signet-rest` feature)

Run examples with:

```bash
cargo run --example signet_fixed_seed --features signet-rest
```

## License

This project is dual-licensed under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

## Contributing

We welcome contributions! Please see our [GitHub repository](https://github.com/client-side-validation/csv-adapter) for more information.
