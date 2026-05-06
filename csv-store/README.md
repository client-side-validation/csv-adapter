# CSV Adapter Store

[![Crates.io](https://img.shields.io/crates/v/csv-adapter-store.svg)](https://crates.io/crates/csv-adapter-store)
[![Documentation](https://docs.rs/csv-adapter-store/badge.svg)](https://docs.rs/csv-adapter-store)
[![License](https://img.shields.io/crates/l/csv-adapter-store.svg)](https://github.com/client-side-validation/csv-adapter#license)

SQLite persistence layer for **CSV Adapter** seal and anchor storage.

## Overview

This crate provides SQLite-backed storage for CSV Adapter seals and anchors. It implements the [`SealStore`] trait from `csv-adapter-core`, enabling persistent tracking of consumed seals and published anchors across all supported chains.

[`SealStore`]: https://docs.rs/csv-adapter-core/latest/csv_adapter_core/trait.SealStore.html

### Key Features

- **Seal Tracking**: Persist consumed seals with chain-specific isolation
- **Anchor Storage**: Store published anchors with finality status
- **Multi-Chain Support**: Separate namespaces per blockchain
- **Reorg Recovery**: Handle chain reorganizations gracefully
- **In-Memory Mode**: Built-in support for testing without disk I/O
- **Indexed Queries**: Optimized lookups by chain, height, and status

## Installation

```bash
cargo add csv-adapter-store
```

Or in your `Cargo.toml`:

```toml
[dependencies]
csv-adapter-store = "0.1"
```

## Quick Start

### Creating a Store

```rust
use csv_adapter_store::SqliteSealStore;

// Open a persistent store at a file path
let store = SqliteSealStore::open("csv_data.db")?;

// Or create an in-memory store (for testing)
let store = SqliteSealStore::in_memory()?;
```

### Recording Seal Consumption

```rust
use csv_adapter_store::SqliteSealStore;
use csv_adapter_core::{SealRecord, Hash};

let mut store = SqliteSealStore::in_memory()?;

// Record a consumed seal
let record = SealRecord {
    chain: "bitcoin".to_string(),
    seal_id: vec![0x01; 32],
    consumed_at_height: 100_000,
    commitment_hash: Hash::new([0xAB; 32]),
    recorded_at: 1700000000,
};

store.save_seal(&record)?;

// Check if a seal has been consumed
let is_consumed = store.is_seal_consumed("bitcoin", &[0x01; 32])?;
assert!(is_consumed);
```

### Managing Anchors

```rust
use csv_adapter_store::SqliteSealStore;
use csv_adapter_core::{AnchorRecord, Hash};

let mut store = SqliteSealStore::in_memory()?;

// Record a published anchor
let record = AnchorRecord {
    chain: "ethereum".to_string(),
    anchor_id: vec![0x02; 32],
    block_height: 5_000_000,
    commitment_hash: Hash::new([0xCD; 32]),
    is_finalized: false,
    confirmations: 0,
    recorded_at: 1700000000,
};

store.save_anchor(&record)?;

// Check for pending anchors
let pending = store.pending_anchors("ethereum")?;
assert_eq!(pending.len(), 1);

// Finalize after confirmations
store.finalize_anchor("ethereum", &[0x02; 32], 15)?;
```

### Handling Reorganizations

```rust
use csv_adapter_store::SqliteSealStore;

let mut store = SqliteSealStore::in_memory()?;

// Remove seals/anchors after a reorg height
let removed_seals = store.remove_seals_after("bitcoin", 99_500)?;
let removed_anchors = store.remove_anchors_after("bitcoin", 99_500)?;

println!("Removed {} seals and {} anchors due to reorg", 
         removed_seals, removed_anchors);
```

## Schema

The store creates two tables:

### `seals`

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Auto-incrementing primary key |
| `chain` | TEXT | Blockchain identifier |
| `seal_id` | BLOB | Unique seal identifier |
| `consumed_at_height` | INTEGER | Block height of consumption |
| `commitment_hash` | BLOB | Associated commitment hash |
| `recorded_at` | INTEGER | Unix timestamp of recording |

### `anchors`

| Column | Type | Description |
|--------|------|-------------|
| `id` | INTEGER | Auto-incrementing primary key |
| `chain` | TEXT | Blockchain identifier |
| `anchor_id` | BLOB | Unique anchor identifier |
| `block_height` | INTEGER | Block height of publication |
| `commitment_hash` | BLOB | Associated commitment hash |
| `is_finalized` | INTEGER | Finality status (0/1) |
| `confirmations` | INTEGER | Number of confirmations |
| `recorded_at` | INTEGER | Unix timestamp of recording |

## License

This project is dual-licensed under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <https://www.apache.org/licenses/LICENSE-2.0>)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or <https://opensource.org/licenses/MIT>)

at your option.

## Contributing

We welcome contributions! Please see our [GitHub repository](https://github.com/client-side-validation/csv-adapter) for more information.
