# Implementation Status

Related docs: [Architecture](ARCHITECTURE.md), [Cross-Chain Specification](CROSS_CHAIN_SPEC.md), [Developer Guide](DEVELOPER_GUIDE.md), [Testnet E2E Report](TESTNET_E2E_REPORT.md)

## Snapshot

As of this documentation pass, the implementation is best understood as:

- a mature core protocol crate
- four implemented chain adapters
- a CLI that exposes end-to-end workflows
- a broader ecosystem of wallet, explorer, SDK, MCP, and local-dev packages

## Implemented areas

### Core protocol

`csv-adapter-core` already carries the main protocol surface:

- rights and commitments
- inclusion and finality proof types
- verification helpers
- commitment chains and DAG segments
- seal registry and validation logic
- experimental support modules for broader protocol evolution

### Chain adapters

| Crate | Primary responsibility | Current role |
|-------|------------------------|--------------|
| `csv-adapter-bitcoin` | UTXO-based seals, Tapret-related transaction logic, proof generation | Source-chain implementation for structural seals |
| `csv-adapter-ethereum` | Nullifier-based seals, contracts, Ethereum proof handling | Contract-enforced seal model |
| `csv-adapter-sui` | Object-based seals, checkpoint handling, Move contracts | Structural object model |
| `csv-adapter-aptos` | Resource-based seals, ledger proof handling, Move contracts | Type-enforced resource model |

### Unified access and orchestration

| Package | Responsibility |
|---------|----------------|
| `csv-adapter` | Unified Rust client surface and builder |
| `csv-cli` | User-facing orchestration for wallet, right, proof, and transfer operations |
| `csv-adapter-store` | Persistence for seal and anchor state |

### Broader ecosystem already present in the repo

These packages exist today and are part of the real project surface, even though older docs underemphasized them:

- `csv-wallet`
- `typescript-sdk`
- `csv-mcp-server`
- `csv-local-dev`
- `csv-explorer`
- `csv-vscode`
- `csv-tutorial`
- `create-csv-app`

## Operational flow implemented in the CLI

The CLI command tree in `csv-cli/src/main.rs` currently exposes:

- `chain`
- `wallet`
- `right`
- `proof`
- `cross-chain`
- `contract`
- `seal`
- `test`
- `validate`

That makes `csv-cli` the most practical top-level integration point for exercising the system end to end.

## Where implementation is strongest

### 1. Architectural consistency

The `AnchorLayer` trait gives all four chain adapters the same high-level contract, which keeps the implementation coherent despite very different underlying chains.

### 2. Protocol-centered factoring

The workspace is organized around protocol concerns rather than around one monolithic application:

- protocol and proofs in `csv-adapter-core`
- chain-specific enforcement in adapter crates
- user workflows in `csv-cli`
- higher-level client ergonomics in `csv-adapter`

### 3. Ecosystem direction

The existence of the TypeScript SDK, local-dev tooling, explorer, and MCP server shows the project is growing beyond a library into a developer platform.

## Known documentation-era gaps and risks

The biggest issues discovered in this pass were not missing code so much as missing canonicality:

- some documents described future UX as if it were already shipped
- multiple planning files covered the same DX and agent topics
- older links referenced files not present in this checkout

Those documentation gaps make implementation maturity harder to judge than the code itself.

## Practical gap list

These are the main areas contributors should still treat carefully:

| Area | Why it needs attention |
|------|------------------------|
| End-to-end operational verification | Real testnet flows depend on funded wallets, RPC quality, and contract deployment state |
| Ecosystem alignment | Rust workspace, SDK, MCP, wallet, and explorer docs should evolve together |
| Experimental modules | VM, MPC, RGB compatibility, and AluVM-oriented work need clearer maturity boundaries |
| Source-of-truth discipline | Canonical docs must stay tied to live manifests and entry points |

## How to read status going forward

- For protocol meaning, trust [Cross-Chain Specification](CROSS_CHAIN_SPEC.md).
- For system boundaries, trust [Architecture](ARCHITECTURE.md).
- For local setup and commands, trust [Developer Guide](DEVELOPER_GUIDE.md).
- For forward-looking work, trust [Blueprint](BLUEPRINT.md).

This document should stay short and current; it is the implementation snapshot, not a historical changelog.
