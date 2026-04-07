# CSV Adapter

The **CSV (Client-Side Validation) Adapter** is a multi-chain framework for anchoring state transitions to various blockchain layers. It provides a chain-agnostic core for managing seals and anchors, with specific implementations for Bitcoin, Ethereum, Sui, Aptos, and Celestia.

## Features

- **Multi-Chain Support**: Native adapters for Bitcoin (UTXO), Ethereum (MPT), Sui (Objects), Aptos (Resources), and Celestia (DA).
- **Client-Side Validation**: High-performance state transition verification without global consensus on every detail.
- **Persistence Layer**: Integrated SQLite-based store for local seal and anchor management.
- **Modular Architecture**: Easy to extend with new blockchain adapters by implementing the `AnchorLayer` trait.

## Repository Structure

- `csv-adapter-core`: The core traits, types, and state machine logic.
- `csv-adapter-bitcoin`: Bitcoin adapter using UTXO-based seals and OP_RETURN/Tapret anchoring.
- `csv-adapter-ethereum`: Ethereum adapter with support for storage slot seals and MPT proofs.
- `csv-adapter-sui`: Sui adapter using object-based seals and checkpoint verification.
- `csv-adapter-aptos`: Aptos adapter with resource-based seals and event verification.
- `csv-adapter-celestia`: Celestia adapter for data availability anchoring (In Progress).
- `csv-adapter-store`: SQLite-based persistence for client-side state.

## Getting Started

### Prerequisites

- Rust (latest stable)
- SQLite3
- `cargo-nextest` for running tests

### Installation

```bash
git clone https://github.com/your-org/csv-adapter.git
cd csv-adapter
cargo build
```

## Testing

This repository uses `cargo-nextest` for faster and more reliable test execution.

To run all tests in the workspace:
```bash
cargo nextest run
```

To run tests for a specific crate:
```bash
cargo nextest run -p csv-adapter-core
```

## Production Readiness

For a detailed evaluation of the project's production readiness, see [docs/PRODUCTION_READINESS_EVALUATION.md](docs/PRODUCTION_READINESS_EVALUATION.md).

## License

This project is licensed under the MIT License or Apache License 2.0.
