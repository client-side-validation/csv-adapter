# CSV Adapter — Unified Meta-Crate

[![Build](https://img.shields.io/badge/build-passing-brightgreen)]()
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)]()

A single entry point for all CSV (Client-Side Validation) operations, unifying the individual chain adapter crates behind a coherent, ergonomic API.

> **We are not building a bridge. We are building a validation system where each chain enforces single-use at its strongest available guarantee, and clients verify everything else.**

---

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
csv-adapter = { path = "../csv-adapter", features = ["bitcoin", "wallet"] }
```

```rust
use csv_adapter::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    // Build a client with Bitcoin support
    let client = CsvClient::builder()
        .with_chain(Chain::Bitcoin)
        .with_store_backend(StoreBackend::InMemory)
        .build()?;

    // Access managers
    let rights = client.rights();
    let transfers = client.transfers();
    let proofs = client.proofs();

    // Watch events
    let mut events = client.watch();

    Ok(())
}
```

---

## Builder Pattern

The `CsvClient` uses a fluent builder pattern for construction:

### Minimal Client

```rust
let client = CsvClient::builder()
    .with_chain(Chain::Bitcoin)
    .with_store_backend(StoreBackend::InMemory)
    .build()?;
```

### Full Configuration

```rust
let wallet = Wallet::generate();
println!("Save this mnemonic: {}", wallet.mnemonic_phrase());

let client = CsvClient::builder()
    .with_all_chains()                      // Enable Bitcoin, Ethereum, Sui, Aptos
    .with_wallet(wallet)                     // Attach wallet for signing
    .with_store_backend(StoreBackend::InMemory)
    .build()?;

// Access all managers
let rights = client.rights();
let transfers = client.transfers();
let proofs = client.proofs();
let wallet_mgr = client.wallet()?;
```

### With SQLite Persistence

```toml
# Cargo.toml
csv-adapter = { path = "../csv-adapter", features = ["bitcoin", "sqlite"] }
```

```rust
let client = CsvClient::builder()
    .with_chain(Chain::Bitcoin)
    .with_store_backend(StoreBackend::Sqlite {
        path: "~/.csv/data.db".to_string(),
    })
    .build()?;
```

### From Configuration File

```rust
let config = Config::load();  // Loads from ~/.csv/config.toml
let client = CsvClient::builder()
    .with_config(config)
    .build()?;
```

---

## Feature Flags

| Feature | Description | Dependencies |
|---------|-------------|-------------|
| `bitcoin` | Enable Bitcoin adapter | csv-adapter-bitcoin, bip32 |
| `ethereum` | Enable Ethereum adapter | csv-adapter-ethereum |
| `sui` | Enable Sui adapter | csv-adapter-sui |
| `aptos` | Enable Aptos adapter | csv-adapter-aptos |
| `all-chains` | Enable all chain adapters | All chain adapters |
| `tokio` | Enable tokio async runtime (default) | tokio |
| `async-std` | Enable async-std runtime | async-std |
| `sqlite` | Enable SQLite persistence | csv-adapter-store |
| `in-memory` | Enable in-memory store backend | — |
| `wallet` | Enable unified wallet management | bip32, ed25519-dalek |

---

## Architecture

```
csv-adapter (this crate)
├── csv-adapter-core       (always included — Right, Hash, Commitment, traits)
├── csv-adapter-bitcoin    (optional — UTXO seals, Tapret anchoring)
├── csv-adapter-ethereum   (optional — nullifier seals, MPT proofs)
├── csv-adapter-sui        (optional — object seals, checkpoint finality)
├── csv-adapter-aptos      (optional — resource seals, HotStuff finality)
└── csv-adapter-store      (optional — SQLite persistence)
```

### Module Structure

| Module | Purpose |
|--------|---------|
| `client` | `CsvClient` — main entry point |
| `builder` | `ClientBuilder` — fluent construction |
| `config` | `Config` — serializable configuration |
| `wallet` | `Wallet` / `WalletManager` — multi-chain HD wallet |
| `rights` | `RightsManager` — create, query, manage Rights |
| `transfers` | `TransferManager` — cross-chain transfers |
| `proofs` | `ProofManager` — generate and verify proofs |
| `events` | `EventStream` — real-time event streaming |
| `errors` | `CsvError` — unified error type |
| `prelude` | One-stop import: `use csv_adapter::prelude::*` |

---

## Error Handling

All operations return `Result<T, CsvError>`. Errors integrate with [`ErrorSuggestion`](https://docs.rs/csv-adapter-core) for machine-actionable fix hints:

```rust
use csv_adapter::prelude::*;

fn handle_error(err: CsvError) {
    // Get agent-friendly suggestion
    let suggestion = err.to_suggestion();
    eprintln!("Error [{}]: {}", suggestion.error_code, suggestion.message);

    if let Some(fix) = &suggestion.fix {
        println!("Suggested fix: {:?}", fix);
    }

    // Check if retryable
    if err.is_retryable() {
        println!("This error is transient — retry may succeed");
    }

    // Check specific error types
    if err.is_insufficient_funds() {
        println!("Need to fund wallet");
    }
}
```

---

## Key Concepts

### Right

A **Right** is a verifiable, single-use digital claim. It exists in client state (not on any chain) and is anchored to a single-use seal on a specific chain.

### Seal

A **Seal** is the on-chain mechanism that enforces a Right's single-use. Chain-specific and exists on one chain only:

- **Bitcoin**: UTXO spend (L1 Structural)
- **Sui**: Object deletion (L1 Structural)
- **Aptos**: Resource destruction (L2 Type-Enforced)
- **Ethereum**: Nullifier registration (L3 Cryptographic)

### Cross-Chain Transfer

A Right doesn't "move" between chains. The source chain's seal is consumed, a proof is generated, and the destination chain verifies the proof locally. No bridges, no wrapped tokens, no cross-chain messaging.

---

## Configuration

### Environment Variables

| Variable | Description |
|----------|-------------|
| `CSV_NETWORK` | Override network (mainnet/testnet/devnet/regtest) |
| `CSV_BITCOIN_RPC_URL` | Bitcoin RPC endpoint |
| `CSV_ETHEREUM_RPC_URL` | Ethereum RPC endpoint |
| `CSV_SUI_RPC_URL` | Sui RPC endpoint |
| `CSV_APTOS_RPC_URL` | Aptos RPC endpoint |
| `CSV_STORE_BACKEND` | Store backend (`sqlite` or `in-memory`) |
| `CSV_STORE_PATH` | SQLite database path |

### Config File (`~/.csv/config.toml`)

```toml
network = "testnet"

[chains.bitcoin]
enabled = true
finality_depth = 6
[chains.bitcoin.rpc]
url = "https://mempool.space/api"
timeout_ms = 30000

[chains.ethereum]
enabled = true
finality_depth = 12
[chains.ethereum.rpc]
url = "https://eth.llamarpc.com"

[store]
backend = "sqlite"
path = "~/.csv/data.db"
```

---

## Documentation

| Document | Description |
|----------|-------------|
| [Main README](../README.md) | Project overview |
| [Blueprint](../docs/BLUEPRINT.md) | Full specification |
| [Developer Guide](../docs/DEVELOPER_GUIDE.md) | How to build on CSV |
| [Cross-Chain Spec](../docs/CROSS_CHAIN_SPEC.md) | Protocol specification |

---

## License

MIT or Apache-2.0 — choose the license that best fits your use case.
