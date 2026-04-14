# CSV Adapter

Client-side validation for cross-chain rights built around a universal seal model.

CSV Adapter treats a blockchain as the place where single-use is enforced, not where full application state lives. A `Right` stays in client state, while a chain-specific `Seal` is consumed on Bitcoin, Sui, Aptos, or Ethereum and later proven to another client with inclusion and finality evidence.

## What the codebase contains

The repository is a multi-package project with a Rust workspace at its core and several adjacent tools:

| Area | Purpose |
|------|---------|
| `csv-adapter-core` | Protocol types, proofs, validation logic, state machine, and the `AnchorLayer` trait |
| `csv-adapter-*` | Chain adapters for Bitcoin, Ethereum, Sui, and Aptos |
| `csv-adapter` | Unified Rust meta-crate and client API |
| `csv-cli` | Command-line entry point for wallets, rights, proofs, and cross-chain flows |
| `csv-wallet` | Wallet UI and supporting services |
| `typescript-sdk` | TypeScript SDK package |
| `csv-mcp-server` | MCP server for agent-oriented workflows |
| `csv-local-dev` | Local chain simulator and developer environment |
| `csv-explorer` | Explorer, API, indexer, and storage stack |

## Core idea

CSV is not a bridge. It is a verification model.

1. A right is anchored to a chain-specific seal.
2. The seal is consumed on the source chain.
3. The sender produces a proof bundle from source-chain data.
4. The receiver verifies the proof locally or through a destination-chain verifier.
5. The right is accepted because the proof is valid, not because a bridge attested to it.

This lets the system preserve each chain's native single-use guarantee:

| Chain | Seal mechanism | Enforcement strength |
|-------|----------------|----------------------|
| Bitcoin | UTXO spend | Structural |
| Sui | Object deletion | Structural |
| Aptos | Resource destruction | Type-enforced |
| Ethereum | Nullifier registration | Contract-enforced |

## Quick start

```bash
git clone https://github.com/zorvan/csv-adapter.git
cd csv-adapter
cargo build --workspace
cargo test --workspace
```

Build the CLI:

```bash
cargo build -p csv-cli --release
./target/release/csv --help
```

Example Rust entry point:

```rust
use csv_adapter::prelude::*;

let client = CsvClient::builder()
    .with_chain(Chain::Bitcoin)
    .with_store_backend(StoreBackend::InMemory)
    .build()?;

let rights = client.rights();
let transfers = client.transfers();
let proofs = client.proofs();
```

## Documentation

Start with [Documentation Hub](docs/INDEX.md).

| Document | Purpose |
|----------|---------|
| [Architecture](docs/ARCHITECTURE.md) | Current system model, invariants, and package boundaries |
| [Cross-Chain Spec](docs/CROSS_CHAIN_SPEC.md) | Protocol semantics and proof model |
| [Developer Guide](docs/DEVELOPER_GUIDE.md) | Build, test, extend, and operate the repo |
| [Implementation Status](docs/CROSS_CHAIN_IMPLEMENTATION.md) | What is implemented now and where gaps remain |
| [Blueprint](docs/BLUEPRINT.md) | Product and engineering roadmap |
| [Explorer and Wallet Indexing](docs/EXPLORER_WALLET_INDEXING.md) | Explorer indexing and wallet integration design |
| [AluVM Note](docs/ALUVM.md) | Experimental design note for future VM integration |
| [E2E Manual](docs/E2E_TESTNET_MANUAL.md) | Testnet walkthrough |
| [E2E Report](docs/TESTNET_E2E_REPORT.md) | Recorded test outcomes |

## Codebase analysis

From `repomix-output.xml` and the live source tree, the repo is strongest where it has a clear center:

- `csv-adapter-core` is the architectural anchor. Its exported modules and the `AnchorLayer` trait provide a coherent protocol boundary for every chain adapter.
- The Rust packages are relatively well factored: protocol in `core`, per-chain enforcement in adapters, and user operations in `csv-cli` and `csv-adapter`.
- The broader repo has grown into a product ecosystem, not just a Rust library. The explorer, wallet, TypeScript SDK, MCP server, and local-dev tooling matter and should be reflected in top-level docs.

The main weakness was documentation drift:

- README and docs mixed shipped behavior with aspirational roadmap material.
- Several files duplicated the same DX and agent-planning content with slightly different claims.
- Some links pointed to files that are no longer present in this checkout.

This cleanup turns the docs into a smaller canonical set so future updates have one obvious place to land.

## License

MIT or Apache-2.0.
