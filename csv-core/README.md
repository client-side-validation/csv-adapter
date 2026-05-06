# CSV Adapter Core

[![Crates.io](https://img.shields.io/crates/v/csv-adapter-core.svg)](https://crates.io/crates/csv-adapter-core)
[![Documentation](https://docs.rs/csv-adapter-core/badge.svg)](https://docs.rs/csv-adapter-core)
[![License](https://img.shields.io/crates/l/csv-adapter-core.svg)](https://github.com/client-side-validation/csv-adapter#license)

Chain-agnostic core traits and types for **CSV (Client-Side Validation)** adapters.

## Overview

This crate provides the foundational types and traits for the CSV protocol, a client-side validation system built on the **Universal Seal Primitive (USP)**. Sanads are anchored to single-use seals on any chain. To transfer a Sanad, the seal is consumed on-chain and the new owner verifies the consumption proof locally — no bridges, no minting, no cross-chain messaging.

### Key Types

- **[`Sanad`]** — A verifiable, single-use digital sanad that can be transferred cross-chain
- **[`Hash`]** — A 32-byte cryptographic hash (SHA-256 based)
- **[`Commitment`]** — A binding between a sanad's state and its anchor on a blockchain
- **[`SealPoint`]** / **[`CommitAnchor`]** — References to consumed seals and published anchors
- **[`InclusionProof`]** / **[`FinalityProof`]** / **[`ProofBundle`]** — Cryptographic proofs
- **[`SealProtocol`]** — The core trait each blockchain adapter implements
- **[`ChainDriver`]** — Scalable chain adapter trait for dynamic registration
- **[`AdapterFactory`]** — Factory for creating chain adapters dynamically
- **[`SignatureScheme`]** — Supported signing algorithms (secp256k1, Ed25519)

[`Sanad`]: https://docs.rs/csv-adapter-core/latest/csv_adapter_core/struct.Sanad.html
[`Hash`]: https://docs.rs/csv-adapter-core/latest/csv_adapter_core/struct.Hash.html
[`Commitment`]: https://docs.rs/csv-adapter-core/latest/csv_adapter_core/struct.Commitment.html
[`SealPoint`]: https://docs.rs/csv-adapter-core/latest/csv_adapter_core/struct.SealPoint.html
[`CommitAnchor`]: https://docs.rs/csv-adapter-core/latest/csv_adapter_core/struct.CommitAnchor.html
[`InclusionProof`]: https://docs.rs/csv-adapter-core/latest/csv_adapter_core/struct.InclusionProof.html
[`FinalityProof`]: https://docs.rs/csv-adapter-core/latest/csv_adapter_core/struct.FinalityProof.html
[`ProofBundle`]: https://docs.rs/csv-adapter-core/latest/csv_adapter_core/struct.ProofBundle.html
[`SealProtocol`]: https://docs.rs/csv-adapter-core/latest/csv_adapter_core/trait.SealProtocol.html
[`SignatureScheme`]: https://docs.rs/csv-adapter-core/latest/csv_adapter_core/enum.SignatureScheme.html

## Installation

```bash
cargo add csv-adapter-core
```

Or in your `Cargo.toml`:

```toml
[dependencies]
csv-adapter-core = "0.1"
```

### Features

| Feature | Description | Default |
|---------|-------------|---------|
| `std` | Enable standard library support | ✅ |
| `tapret` | Enable Taproot commitment support (requires `bitcoin` crate) | ❌ |

## Quick Start

### Creating a Sanad

```rust
use csv_adapter_core::{Sanad, Hash, OwnershipProof, SignatureScheme};

// Create a commitment hash
let commitment = Hash::new([0xAB; 32]);

// Create an ownership proof
let owner = OwnershipProof {
    proof: vec![/* signature bytes */],
    owner: vec![/* owner address bytes */],
    scheme: Some(SignatureScheme::Secp256k1),
};

// Create a Sanad
let sanad = Sanad::new(commitment, owner, b"unique-salt");

// Transfer to a new owner
let new_owner = OwnershipProof {
    proof: vec![/* new signature bytes */],
    owner: vec![/* new owner address bytes */],
    scheme: Some(SignatureScheme::Secp256k1),
};
let transferred = sanad.transfer(new_owner, b"transfer-salt");
```

### Working with Commitments

```rust
use csv_adapter_core::{Commitment, SealPoint};

// Create a commitment with all fields
let commitment = Commitment::simple(
    contract_id,       // What Sanad this is for
    previous_commitment, // Hash of previous commitment (or zero for genesis)
    payload_hash,       // What changed
    &seal_ref,          // What seal was consumed
    domain_separator,   // Chain-specific isolation
);
```

### Verifying Signatures

```rust
use csv_adapter_core::{verify_signatures, SignatureScheme};

let signatures = vec![/* signature data */];
let public_keys = vec![/* public key data */];

let is_valid = verify_signatures(&signatures, &public_keys, SignatureScheme::Secp256k1);
```

## Architecture

The core crate defines the **Universal Seal Primitive** — a chain-agnostic abstraction for single-use enforcement:

### Scalable Chain Adapter System

The new scalable architecture uses a **plugin-based registry pattern** for dynamic chain support:

```
┌──────────────────────────────────────────────────────────────┐
│                    csv-adapter-core                           │
│                                                               │
│   ChainDriver ─────── Trait for all chain adapters          │
│   ├─ chain_id() ─────── Returns chain identifier             │
│   ├─ capabilities() ─── Returns ChainCapabilities            │
│   ├─ create_client() ── Creates RPC client                   │
│   └─ create_wallet() ── Creates wallet interface            │
│                                                               │
│   AdapterFactory ────── Dynamic adapter registry              │
│   ├─ register() ─────── Register new chain adapter           │
│   ├─ create_adapter() ─ Create adapter by chain ID         │
│   └─ supported_chains() ─ List all registered chains         │
│                                                               │
│   ChainRegistry ─────── Store adapter instances               │
│   ChainCapabilities ─── Chain metadata (account model, etc)   │
└──────────────┬───────────────────────────────┬──────────────┘
               │                               │
    ┌──────────┴──────────┐          ┌──────────┴──────────┐
    │  Built-in Adapters  │          │  Custom Adapters    │
    │  • BitcoinAdapter   │          │  • YourChainDriver │
    │  • EthereumAdapter  │          │  (dynamically       │
    │  • SolanaAdapter    │          │   registered)       │
    │  • SuiAdapter       │          │                     │
    │  • AptosAdapter     │          │                     │
    └─────────────────────┘          └─────────────────────┘
```

### Legacy SealProtocol System

```
┌──────────────────────────────────────────────────────────────┐
│                    csv-adapter-core                           │
│                                                               │
│   Sanad ────────────── Portable digital sanad                 │
│   Commitment ──────── Hash chain of state transitions         │
│   SealPoint ─────────── Reference to consumed seal              │
│   SealProtocol ─────── Trait: per-chain implementation         │
│   ProofBundle ─────── Inclusion + finality proofs             │
│   SealRegistry ────── Cross-chain double-spend prevention     │
└──────────────┬──────────┬──────────┬──────────────────────────┘
               │          │          │
      ┌────────┴┐  ┌─────┴┐  ┌─────┴────┐
      │Bitcoin  │  │Ethereum│  │Sui/Aptos │
      │Adapter  │  │Adapter │  │Adapters  │
      └─────────┘  └────────┘  └──────────┘
```

Each chain adapter implements `SealProtocol` using its native single-use mechanism:

| Chain | Level | Seal Type | Guarantee |
|-------|-------|-----------|-----------|
| Bitcoin | L1 Structural | UTXO spend | Native single-use |
| Sui | L1 Structural | Object deletion | Native single-use |
| Aptos | L2 Type-Enforced | Resource destruction | Language-level scarcity |
| Ethereum | L3 Cryptographic | Nullifier registration | Cryptographic single-use |
| Solana | L1 Structural | Account-based with program-derived addresses | Native single-use |

## Scalable Architecture Usage

### Using the Adapter Factory

```rust
use csv_adapter_core::{AdapterFactory, ChainDriver, ChainCapabilities};

// Create factory with all built-in adapters
let factory = AdapterFactory::new();

// Get supported chains
let chains = factory.supported_chains();
// Returns: ["bitcoin", "ethereum", "solana", "sui", "aptos"]

// Create adapter by chain ID
if let Some(adapter) = factory.create_adapter("solana") {
    println!("Chain: {}", adapter.chain_name());
    println!("Capabilities: {:?}", adapter.capabilities());
}
```

### Registering Custom Adapters

```rust
use csv_adapter_core::{AdapterFactory, ChainDriver};

// Create your custom adapter
pub struct MyChainDriver;
impl ChainDriver for MyChainDriver {
    fn chain_id(&self) -> &'static str { "mychain" }
    fn chain_name(&self) -> &'static str { "My Chain" }
    fn capabilities(&self) -> ChainCapabilities { /* ... */ }
    // ... implement other methods
}

// Register with factory
let mut factory = AdapterFactory::new();
factory.register("mychain", Arc::new(|| Box::new(MyChainDriver)));

// Now available everywhere
let adapter = factory.create_adapter("mychain");
```

### Using the Chain Registry

```rust
use csv_adapter_core::ChainRegistry;

// Create registry and register adapters
let mut registry = ChainRegistry::new();
registry.register(Box::new(SolanaAdapter::new()));

// Get adapter and query capabilities
if let Some(adapter) = registry.get("solana") {
    let caps = adapter.capabilities();
    assert!(caps.supports_smart_contracts);
    assert!(caps.account_model == AccountModel::Account);
}
```

## Examples

See the [`examples/`](examples/) directory for usage patterns:

- **`basic_sanad`** — Creating, transferring, and verifying Sanads
- **`commitment_chain`** — Building and validating commitment chains
- **`complete_scalable_demo`** — Using the new scalable architecture

Run examples with:

```bash
cargo run --example basic_sanad
cargo run --example commitment_chain
cargo run --example complete_scalable_demo
```

## License

This project is dual-licensed under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

## Contributing

We welcome contributions! Please see our [GitHub repository](https://github.com/client-side-validation/csv-adapter) for more information.
