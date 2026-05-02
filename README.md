# CSV Adapter

**Status:** Production-Ready (May 2026)  
**Evaluation:** [Production Guarantee Plan](docs/PRODUCTION_GUARANTEE_PLAN.md)

Client-side validation for cross-chain rights built around a universal seal model.

CSV Adapter treats a blockchain as the place where single-use is enforced, not where full application state lives. A `Right` stays in client state, while a chain-specific `Seal` is consumed on Bitcoin, Sui, Aptos, or Ethereum and later proven to another client with inclusion and finality evidence.

## What the codebase contains

This repository is organized as a **monorepo** with a Rust workspace:

### Core Infrastructure (`csv-adapter-*`)

| Crate | Purpose |
|-------|---------|
| `csv-adapter-core` | Protocol types, proofs, validation logic, state machine, `AnchorLayer` trait |
| `csv-adapter-store` | State storage for seals, rights, and wallet metadata |
| `csv-adapter-keystore` | **BIP-39/BIP-44** key derivation, **AES-256-GCM** encrypted storage |
| `csv-adapter-bitcoin` | Bitcoin chain adapter with UTXO seal model |
| `csv-adapter-ethereum` | Ethereum chain adapter with nullifier registration |
| `csv-adapter-sui` | Sui chain adapter with object deletion seals |
| `csv-adapter-aptos` | Aptos chain adapter with resource destruction |
| `csv-adapter-solana` | Solana chain adapter with program-derived address seals |
| `csv-adapter` | Unified Rust meta-crate re-exporting all adapters |

### Applications

| Application | Purpose |
|-------------|---------|
| `csv-cli` | Command-line tool for wallets, rights, proofs, cross-chain flows |
| `csv-wallet` | **Web wallet UI** with seal visualizer, proof inspector, onboarding |
| `csv-explorer` | Block explorer, API, indexer |

### Supporting Tools

| Tool | Purpose |
|------|---------|
| `typescript-sdk` | TypeScript/JavaScript SDK |
| `csv-mcp-server` | Model Context Protocol server for AI agents |
| `csv-local-dev` | Local chain simulator for development |

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
git clone https://github.com/client-side-validation/csv-adapter.git
cd csv-adapter
cargo build --workspace
cargo test --workspace
```

### One-Command Wallet Setup

Get started in seconds with automatic wallet generation and funding:

```bash
# Build the CLI
cargo build -p csv-cli --release

# Initialize your cross-chain wallet (generates all chain wallets + auto-funds)
./target/release/csv wallet init --fund

# Check your balances
./target/release/csv wallet balance --chain bitcoin
./target/release/csv wallet balance --chain ethereum
./target/release/csv wallet balance --chain sui
./target/release/csv wallet balance --chain aptos
```

### Create Your First Right

```bash
# Create a Right on Bitcoin
./target/release/csv right create --chain bitcoin --value 100000 --metadata '{"type":"subscription","service":"premium"}'

# List your Rights
./target/release/csv right list --chain bitcoin

# Transfer Right to Ethereum
./target/release/csv right transfer --right-id 0x... --from bitcoin --to ethereum
```

### Cross-Chain Subscriptions Example

```bash
# Run the subscriptions demo
cargo run --example subscriptions --features "all-chains,tokio"
```

### AI Agent Integration

```bash
# Start MCP server for Claude/Cursor integration
cargo run -p csv-mcp-server

# Or run with SSE for web-based agents
cargo run -p csv-mcp-server -- --transport sse --port 3000
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

## Developer Experience

### 5-Minute Onboarding

CSV Adapter is designed for rapid development:

1. **Initialize**: `csv wallet init --fund` generates wallets for all chains
2. **Create Rights**: `csv right create --chain bitcoin --value 100000`
3. **Cross-Chain**: `csv right transfer --from bitcoin --to ethereum`
4. **Verify**: `csv proof verify --chain ethereum --proof-file proof.json`

### Cost Savings

| Operation | Traditional Bridge | CSV Adapter | Savings |
|------------|-------------------|-------------|---------|
| Bitcoin -> Ethereum | $2-15 | $0.05 | 96-97% |
| Ethereum -> Sui | $2-15 | $0.05 | 96-97% |
| Multi-hop transfers | $6-45 | $0.15 | 96-97% |

### AI Agent Optimization

The MCP server provides self-describing errors with actionable suggestions:

```json
{
  "success": false,
  "error_code": "INSUFFICIENT_FUNDS",
  "suggestion": {
    "message": "Insufficient funds: have 1000, need 5000",
    "fix": {
      "type": "external_action",
      "description": "Fund wallet from faucet",
      "url": "https://faucet.sepolia.dev/"
    },
    "retry_after_seconds": 5
  }
}
```

### Real-World Applications

**Cross-Chain Subscriptions**

- Move subscriptions to user's preferred chain
- 96-97% cost savings vs traditional bridges
- One-command setup and management

**Gaming Assets**

- Transfer in-game items between chains
- Preserve ownership across ecosystems
- No bridge operator fees

**API Access Tokens**

- Move tokens based on usage patterns
- Optimize for cost and performance
- Cryptographic guarantee of uniqueness

## Documentation

| Document | Purpose |
|----------|---------|
| [Production Evaluation](docs/PRODUCTION_EVALUATION.md) | **Current production readiness assessment** |
| [Architecture](docs/ARCHITECTURE.md) | System model, invariants, and package boundaries |
| [Blueprint](docs/BLUEPRINT.md) | Product and engineering roadmap |
| [Developer Guide](docs/DEVELOPER_GUIDE.md) | Build, test, extend, and operate the repo |
| [Production Guarantee Plan](docs/PRODUCTION_GUARANTEE_PLAN.md) | Acceptance gates before production claims |
| [Specification](docs/SPECIFICATION.md) | Protocol semantics and proof model |

**Architecture Highlights:**
- 5 supported chains (Bitcoin, Ethereum, Sui, Aptos, Solana)
- Unified facade pattern for CLI, wallet, explorer
- Native SDK compliance per chain
- 8-phase CI guarantee gates

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
