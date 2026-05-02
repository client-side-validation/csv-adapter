# Developer Guide

**Status:** Production-Ready  
**Related docs:** [Motivation](MOTIVATION.md), [Architecture](ARCHITECTURE.md), [Specification](SPECIFICATION.md), [Blueprint](BLUEPRINT.md), [Production Guarantee Plan](PRODUCTION_GUARANTEE_PLAN.md)

## Purpose

This guide is the practical entry point for contributors. It focuses on how to build, test, and extend the repository as it exists today.

## Prerequisites

The root workspace currently declares `rust-version = "1.75"`. Use a recent stable Rust toolchain unless you are reproducing an older environment.

Common local tooling:

```bash
rustup install stable
cargo build --workspace
cargo test --workspace
```

For chain-specific development, install the relevant tools only when you need them:

```bash
# Ethereum / Foundry
curl -L https://foundry.paradigm.xyz | bash
foundryup

# Sui CLI
cargo install --locked --git https://github.com/MystenLabs/sui.git --bin sui

# Aptos CLI
cargo install --git https://github.com/aptos-labs/aptos-core.git aptos
```

## Repository map

### Rust workspace

| Path | Role |
|------|------|
| `csv-adapter-core` | Core protocol types, proofs, validators, and shared traits |
| `csv-adapter-bitcoin` | Bitcoin adapter |
| `csv-adapter-ethereum` | Ethereum adapter and contracts |
| `csv-adapter-sui` | Sui adapter and Move package |
| `csv-adapter-aptos` | Aptos adapter and Move package |
| `csv-adapter-store` | Persistence layer |
| `csv-adapter` | Unified Rust client surface |
| `csv-cli` | Command-line entry point |
| `csv-wallet` | Wallet application |

### Adjacent packages in the monorepo

| Path | Role |
|------|------|
| `typescript-sdk` | TypeScript SDK |
| `csv-mcp-server` | MCP server for agents |
| `csv-local-dev` | Local simulation and dev environment |
| `csv-explorer` | Explorer, indexer, API, UI, and storage |
| `csv-vscode` | VS Code extension |
| `csv-tutorial` | Tutorial content |
| `create-csv-app` | Scaffolding tool |

## Common commands

### Workspace

```bash
cargo build --workspace
cargo test --workspace
```

### CLI

```bash
cargo build -p csv-cli --release
./target/release/csv --help
```

### Package-level exploration

```bash
cargo test -p csv-adapter-core
cargo test -p csv-adapter-bitcoin
cargo test -p csv-adapter-ethereum
```

## Core concepts to keep in mind

| Concept | Practical meaning |
|---------|-------------------|
| `Right` | The portable client-side unit of state |
| `Seal` | The chain-native single-use primitive |
| `Commitment` | Hash-linked transition record |
| `ProofBundle` | The transportable verification artifact |
| `AnchorLayer` | The contract every adapter must satisfy |

If you need the conceptual model first, read [Architecture](ARCHITECTURE.md) before changing code.

## Working with the CLI

The CLI entry point in `csv-cli/src/main.rs` currently exposes these domains:

- `chain`
- `wallet`
- `right`
- `proof`
- `cross-chain`
- `contract`
- `seal`
- `test`
- `validate`

Use the CLI when you want to exercise integration behavior without writing new application code first.

## Extending the system

### Adding or modifying a chain adapter

The implementation pattern is:

1. Define chain-specific types in the adapter crate.
2. Implement `AnchorLayer`.
3. Wire chain-specific RPC and proof extraction.
4. Add tests for inclusion, finality, and replay behavior.
5. Expose the flow through `csv-cli` or the unified client if needed.

The most important file to understand first is `csv-adapter-core/src/traits.rs`.

### Adding higher-level product features

Decide which layer the feature belongs in:

| Feature type | Likely home |
|--------------|-------------|
| New proof or validation rule | `csv-adapter-core` |
| Chain-specific behavior | `csv-adapter-<chain>` |
| User workflow | `csv-cli` or `csv-adapter` |
| Wallet or explorer functionality | `csv-wallet` or `csv-explorer` |
| JS or agent-facing tooling | `typescript-sdk` or `csv-mcp-server` |

## Testing guidance

### Default loop

For most changes:

```bash
cargo test --workspace
```

### Targeted loop

Use a smaller test target while iterating:

```bash
cargo test -p csv-adapter-core
cargo test -p csv-cli
```

### Testnet and manual flows

For testnet integration testing:

```bash
cargo test -p csv-adapter --test testnet_integration_tests
```

## Documentation Update Rule

When a change affects behavior, update the matching canonical doc:

| If you changed... | Update... |
|-------------------|-----------|
| Protocol meaning or invariants | [Specification](SPECIFICATION.md) |
| System boundaries or package responsibilities | [Architecture](ARCHITECTURE.md) |
| Build and workflow instructions | [Developer Guide](DEVELOPER_GUIDE.md) |
| Production readiness or evaluation | [Production Guarantee Plan](PRODUCTION_GUARANTEE_PLAN.md) |
| Roadmap or future priorities | [Blueprint](BLUEPRINT.md) |

That rule is the main guardrail against documentation drift.

## Production Readiness

The project has reached **production-ready status** as documented in [Production Guarantee Plan](PRODUCTION_GUARANTEE_PLAN.md):

- ✅ Core protocol traits implemented (`ChainQuery`, `ChainSigner`, `ChainBroadcaster`, `ChainDeployer`, `ChainProofProvider`, `ChainRightOps`)
- ✅ All 5 chains use native SDKs per `NATIVE_SDK_COMPLIANCE.md`
- ✅ Unified facade pattern via `ChainFacade`
- ✅ CI guarantee gates enforce production standards
- ✅ Security audits pass (`cargo audit`)

Before claiming full production guarantee:

1. Run the CLI/wallet facade audit (check for direct adapter imports)
2. Verify all examples compile and run
3. Complete testnet integration test coverage

## Contributor checklist

Before you finish a change:

1. Run the smallest useful test command for the touched area.
2. Check whether the change affects a canonical doc.
3. Make sure README and docs still describe the current package layout.
4. Keep speculative roadmap material out of implementation docs.
