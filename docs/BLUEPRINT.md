# CSV Adapter Blueprint

**Status:** Canonical product and engineering blueprint  
**Last updated:** May 2, 2026  
**Evaluation:** [Production Guarantee Plan](PRODUCTION_GUARANTEE_PLAN.md) - Production-Ready Status  
**Related docs:** [Motivation](MOTIVATION.md), [Architecture](ARCHITECTURE.md), [Specification](SPECIFICATION.md), [Developer Guide](DEVELOPER_GUIDE.md), [Production Guarantee Plan](PRODUCTION_GUARANTEE_PLAN.md)

---

## Purpose

CSV Adapter is a developer platform for portable, proof-verified rights across chains. It combines client-side validation, single-use seals, chain-native anchoring, and cross-chain proof workflows so applications can move rights, assets, credentials, and state commitments without turning every chain into the source of truth for everything.

This blueprint is the single planning document for the project. It replaces the older split between `PLAN.md` and `BLUEPRINT.md`.

It is intentionally opinionated:

- describe the product direction clearly
- preserve useful ideas from earlier planning
- avoid duplicate roadmaps
- avoid claiming production guarantees before the guarantee gates pass
- keep implementation acceptance criteria in [Production Guarantee Plan](PRODUCTION_GUARANTEE_PLAN.md)

---

## Current Position

The repository already has substantial foundations:

- `csv-adapter-core` contains protocol primitives for rights, seals, commitments, proofs, transitions, registries, and cross-chain abstractions.
- Chain adapter crates exist for Bitcoin, Ethereum, Sui, Aptos, and Solana.
- `csv-adapter` provides a unified Rust facade, though not every consumer uses it consistently yet.
- `csv-cli`, `csv-wallet`, and `csv-explorer` exist as working surfaces.
- Contracts/programs exist for Ethereum, Sui, Aptos, and Solana.
- The contracts are moving toward shared lifecycle names and shared metadata for tokens, NFTs, and advanced proofs.

The repository has reached production-candidate status. Per the [Production Evaluation](PRODUCTION_EVALUATION.md):

**Completed:**

- ✅ Strong protocol center in `csv-adapter-core` with canonical types
- ✅ Clean adapter boundaries via `AnchorLayer` and `FullChainAdapter` traits
- ✅ Native SDK compliance across all chains (Bitcoin, Ethereum, Sui, Aptos, Solana)
- ✅ Unified facade (`ChainFacade`) for CLI, wallet, and explorer
- ✅ Event schema standardization with shared `CsvEvent` types
- ✅ CI guarantee gates (8 phases) enforcing production standards

**Remaining Work (Non-Blocking):**

- ✅ CLI/wallet facade convergence audit - 100% facade usage verified
- ✅ Example cleanup - All 4 examples created
- ✅ Explorer indexer chain plugins - All 5 chains registered
- ⚠️ Testnet integration test execution (tests exist, need testnet runs)
- ⚠️ WASM wallet optimization (pending)

The production bar is defined in [Production Guarantee Plan](PRODUCTION_GUARANTEE_PLAN.md). This blueprint explains what the project should become; the guarantee plan explains how to prove it. The [Production Evaluation](PRODUCTION_EVALUATION.md) provides the comprehensive assessment leading to production-candidate status.

---

## Product Direction

CSV Adapter should become the default stack for applications that need portable rights across chains.

The platform should support:

- cross-chain token and NFT rights
- rights-backed DeFi applications
- proof-carrying credentials
- event tickets and memberships
- gaming assets
- supply-chain provenance
- privacy-preserving ownership flows
- AI-agent-operated cross-chain workflows
- explorer and wallet visibility into every state transition

The core promise:

> A Right can be created, transferred, consumed, proven, indexed, and displayed across chains using one shared protocol model and one shared implementation surface.

---

## Strategic Principles

### 1. Protocol First

`csv-adapter-core` is the conceptual source of truth. It should define:

- canonical rights and seal semantics
- chain trait contracts
- proof bundle formats
- event schemas
- domain-separated hashes
- replay and double-spend invariants
- feature maturity levels

Chain-specific crates implement the protocol. They should not redefine it.

### 2. One Concept, Many Surfaces

The same operation should be available through:

- Rust APIs
- CLI commands
- Dioxus wallet UI
- explorer APIs
- future TypeScript SDK
- future MCP/agent tools

Those surfaces must call the same implementation. If the CLI, wallet, and SDK each sign or broadcast differently, the architecture has failed.

### 3. Native Chains, Shared Semantics

Each chain adapter should use the best native tooling available:

- Bitcoin: `bitcoin`, BIP-32/39/86, Taproot/tapret-aware code, real broadcast clients
- Ethereum: Alloy for ABI, signing, providers, deployment, receipts, and EIP-1559 transactions
- Sui: Sui SDK or official JSON-RPC types for objects, checkpoints, packages, and transactions
- Aptos: Aptos SDK/REST types, BCS, Ed25519, resources, events, and module publishing
- Solana: Solana and Anchor crates, loader interfaces, RPC client, IDL/program bindings

The shared protocol should not erase chain-native strengths. It should make them composable.

### 4. Security Is a Product Feature

Security cannot be added later. Production code must fail closed when real signing, real proofs, real RPC, or real finality are unavailable.

Required posture:

- no fake tx hashes
- no fake proofs
- no silent mock success
- no plaintext key persistence
- no unauthenticated signing
- no replay-prone commitments
- no undocumented proof assumptions

Mocks and simulations belong in tests or explicitly non-production examples only.

### 5. Documentation Has One Home Per Topic

The documentation set should stay small and purposeful:

- [Motivation](MOTIVATION.md): why CSV exists
- [Specification](SPECIFICATION.md): protocol meaning
- [Architecture](ARCHITECTURE.md): current system shape
- [Developer Guide](DEVELOPER_GUIDE.md): how to work on the repo
- [Blueprint](BLUEPRINT.md): product and engineering direction
- [Production Guarantee Plan](PRODUCTION_GUARANTEE_PLAN.md): acceptance gates before production claims

Planning fragments should not multiply.

---

## Target Architecture

```text
csv-adapter-core
  Protocol types, traits, canonical schemas, validation logic, crypto policy.

csv-adapter-{chain}
  The only place for chain-specific implementation:
  - native SDK/RPC clients
  - transaction construction
  - signing payload formats
  - deployment/publishing
  - event decoding
  - inclusion/finality proofs
  - contract/program bindings

csv-adapter
  Unified facade:
  - CsvClient
  - ChainRegistry
  - Wallet service
  - Proof service
  - Deployment service
  - Explorer/event schema exports

csv-cli
  Command parsing, config, and output.
  Calls csv-adapter facade only.

csv-wallet
  UI, local session state, and human approval flows.
  Calls wasm-compatible csv-adapter facade only.

csv-explorer
  Indexing orchestration, storage, REST/GraphQL/WebSocket APIs, UI.
  Uses shared event schemas and per-chain indexer plugins.

typescript-sdk
  Future generated/thin SDK over stable facade schemas and WASM bindings.
```

Adding a new chain should require:

- one adapter crate
- one chain config file
- one explorer plugin
- registry metadata
- contract/program module only if needed
- integration tests

It should not require rewriting CLI, wallet, or SDK business logic.

---

## Shared Protocol Vocabulary

The project should standardize around these lifecycle events:

- `RightCreated`
- `RightConsumed`
- `CrossChainLock`
- `CrossChainMint`
- `CrossChainRefund`
- `RightTransferred`
- `NullifierRegistered`
- `RightMetadataRecorded`

Every chain should expose as much of this shared metadata as the native platform supports:

- `right_id`
- `commitment`
- `owner`
- `chain_id`
- `asset_class`
- `asset_id`
- `metadata_hash`
- `proof_system`
- `proof_root`
- `source_chain`
- `destination_chain`
- `tx_hash`
- `block_height`
- `finality_status`

This vocabulary is what lets the wallet, explorer, SDKs, and agents talk about the same thing.

---

## Developer Experience

CSV should feel simple from the outside even when the internals are serious.

Primary developer personas:

| Persona | Goal | What CSV must provide |
|---|---|---|
| TypeScript/Web developer | Add cross-chain rights to an app | clear SDK, browser wallet flow, examples, explorer links |
| Rust backend developer | Build reliable services | typed APIs, async clients, test fixtures, performance hooks |
| Protocol engineer | extend proofs/chains | precise traits, invariants, docs, fuzz/integration tests |
| AI agent | operate workflows from instructions | structured commands, machine-readable errors, deterministic status |

Developer experience goals:

- first successful local workflow in under 5 minutes
- one command to check environment health
- clear typed errors with suggested actions
- examples that are real enough to trust
- no hidden simulation in production commands
- reproducible local/testnet setup

Useful future commands:

```bash
csv doctor
csv chain list
csv wallet init --network testnet
csv right create --chain bitcoin
csv cross-chain transfer --from bitcoin --to sui --right-id <id>
csv proof verify --proof-file proof.json
```

`csv doctor` should become the canonical diagnostics entrypoint:

- toolchain versions
- config health
- RPC connectivity
- wallet/keystore status
- contract deployment status
- explorer/indexer status
- feature maturity warnings

---

## Agent Experience

AI agents should be able to inspect, operate, and explain CSV workflows without guessing.

Agent-facing requirements:

- machine-readable command schemas
- stable JSON output mode for CLI
- structured operation statuses
- typed error codes
- retryability hints
- suggested remediation
- links to relevant docs
- no ambiguous success states

A cross-chain transfer status should be representable as a structured state machine:

- `initiated`
- `locking`
- `waiting_for_finality`
- `generating_proof`
- `submitting_mint`
- `completed`
- `failed`

Every failure should answer:

- what failed
- whether retry is safe
- what capability was missing
- which chain/RPC/contract was involved
- what the next action is

This agent surface should reuse CLI and SDK business logic, not create another implementation.

---

## Priority Workstreams

### Workstream A: Production Architecture

Goal: make the architecture guaranteeable.

Focus:

- finish the single chain operation API
- force CLI/wallet/explorer through `csv-adapter`
- remove duplicate chain logic
- move mocks to test-only modules
- remove production placeholders and simulations

Acceptance criteria live in [Production Guarantee Plan](PRODUCTION_GUARANTEE_PLAN.md).

### Workstream B: Chain-Native Adapter Completion

Goal: each adapter performs real chain-backed operations or fails closed.

Focus:

- real balance queries
- real transaction construction
- real signing payloads
- real broadcast and confirmation
- real contract/program deployment or publishing
- real finality and inclusion proofs
- native event decoding

Adapters should expose capability errors for unsupported features, never fake success.

### Workstream C: Wallet and CLI Convergence

Goal: make CLI and wallet thin surfaces over one implementation.

Focus:

- keystore-only key persistence
- explicit human approval for signing/broadcast
- no duplicated signing code in wallet service modules
- no raw chain RPC in CLI command handlers
- same status/error model across CLI and wallet

The wallet should be production UI, not a demo dashboard.

### Workstream D: Explorer as Verification Surface

Goal: explorer shows what actually happened on chains.

Focus:

- chain indexer plugin model
- shared event schema
- efficient per-chain indexing
- finality-aware status
- rights/seals/proofs/transfers views
- wallet-compatible API contracts

Explorer should help users and agents audit rights history.

### Workstream E: SDK and Application Platform

Goal: make CSV usable in real applications.

Focus:

- TypeScript SDK
- WASM bindings where useful
- generated schemas from Rust source of truth
- application templates
- React components only after core APIs stabilize
- examples for NFTs, subscriptions, credentials, and DeFi

SDKs should be thin and boring. Protocol complexity belongs in core and adapters.

### Workstream F: Advanced Proofs and Privacy

Goal: prepare for privacy, ZK, and advanced proofs without compromising current correctness.

Focus:

- proof-system identifiers in shared metadata
- proof root / verification-key commitments
- fraud proofs for invalid cross-chain claims
- ZK proof compression
- selective disclosure
- future STARK/SNARK verification modules
- privacy-preserving ownership flows

These features should enter through explicit design documents and gated feature maturity labels.

---

## Application Directions

The strongest application ideas from earlier planning are still valuable, but they should be treated as product directions, not current-state claims.

### Cross-Chain NFTs

Portable NFT rights across chains, with metadata roots preserved through `RightMetadataRecorded`.

Why it fits:

- rights are naturally single-use
- transfers benefit from proof-carrying ownership
- wallet/explorer visibility matters

### Cross-Chain Subscriptions

Recurring access rights that can be consumed or transferred across chains.

Why it fits:

- clear business model
- easy to demonstrate cost reduction
- good CLI/wallet/explorer workflow

### Gaming Assets

Rights for game items that move across chains without central custody.

Why it fits:

- assets need portability
- ownership history matters
- explorer timeline is useful

### Event Ticketing and Memberships

Single-use or revocable rights for access.

Why it fits:

- seal consumption maps cleanly to ticket use
- refunds and transfers are understandable
- fraud prevention is visible

### Supply Chain Provenance

Rights and commitments for custody changes and attestations.

Why it fits:

- traceability is central
- proof history matters
- multiple chains may anchor different stages

### Credentials and Identity

Proof-carrying rights for claims, credentials, and identity attestations.

Why it fits:

- selective disclosure can be layered later
- ownership and revocation need strong semantics
- privacy roadmap becomes meaningful

### DeFi

Rights-backed cross-chain lending, DEX flows, yield aggregation, and insurance.

Why it fits later:

- requires production-grade proofs and finality first
- high security stakes
- strong demonstration of the CSV primitive once the base is hardened

---

## Advanced Research Tracks

These are promising but should not distract from production hardening.

### Fraud Proofs

Fraud proofs can challenge invalid cross-chain claims:

- missing source lock
- invalid finality
- duplicate mint
- malformed inclusion proof
- wrong owner or destination

They become important when relayers, agents, or third-party services submit claims.

### ZK and Privacy

Useful directions:

- proof compression
- private ownership
- selective disclosure
- confidential transfer metadata
- batch verification

Rule: privacy must not hide invalid state transitions from verifiers.

### MPC Wallets

MPC can improve custody for teams, agents, and applications.

Useful directions:

- threshold signing
- policy controls
- agent-limited signing sessions
- social recovery

Rule: MPC is a wallet/security layer, not a replacement for protocol proof verification.

### RGB and AluVM Compatibility

RGB and AluVM ideas remain relevant because CSV shares client-side validation DNA.

Useful directions:

- import/export compatibility
- proof translation
- VM-based validation experiments
- Bitcoin-native asset interoperability

Rule: keep this exploratory until core production guarantees are done.

### React and App UI Components

Reusable UI should come after API convergence.

Potential components:

- right card
- proof verifier
- transfer timeline
- chain status badge
- wallet approval panel
- explorer link panel

Rule: components must display real state, never mock application data as if live.

---

## Success Metrics

Track a small set of metrics that prove the architecture is getting healthier.

| Metric | Target | Why it matters |
|---|---:|---|
| Production audit findings | 0 | proves no stubs/placeholders remain in production code |
| Direct chain calls from CLI/wallet | 0 outside facade | proves single implementation |
| Time to first local workflow | < 5 minutes | proves onboarding works |
| Supported chains through registry | 100% | proves chain addition model |
| Wallet plaintext key persistence | 0 paths | proves key handling discipline |
| End-to-end transfer success on testnets | tracked per chain pair | proves real chain behavior |
| Explorer indexing lag | bounded per chain | proves operational usefulness |
| Agent JSON command success rate | > 95% | proves automation surface |
| Proof verification throughput | measured in CI/bench | proves scalability |
| Security audit critical findings | 0 unresolved | proves release readiness |

Do not use success metrics that are not measured.

---

## Documentation Strategy

The docs should follow a simple split:

- Tutorials: "get something working"
- How-to guides: "perform a task"
- Reference: "exact API/protocol meaning"
- Explanation: "why the system works this way"

Immediate documentation cleanup:

- keep this blueprint as the only roadmap-style file
- keep `CSV_DETAILED_PLAN.md` as a dated recovery diagnosis until its tasks are either completed or migrated
- keep `PRODUCTION_GUARANTEE_PLAN.md` as the hard acceptance checklist
- remove stale planning fragments when their content is absorbed
- avoid status claims like "complete" unless CI and acceptance gates prove them

---

## Definition of a Defendable Blueprint

This blueprint is defendable if it stays true to these rules:

1. It distinguishes current reality from future direction.
2. It keeps one source of truth for each topic.
3. It refuses production claims until acceptance gates pass.
4. It makes new chain support a registry/adapter problem, not a CLI/wallet rewrite.
5. It keeps security and cryptography in the critical path.
6. It preserves ambitious ideas without pretending they are already implemented.
