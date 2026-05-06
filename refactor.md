# CSV Protocol — Full Architectural Autopsy & Refactoring Plan

**Prepared by:** Principal Software Architect, Distributed & Decentralized Systems  
**Target audience:** Backend Engineering, Frontend Engineering, Protocol Design  
**Status:** For external validation  
**Date:** May 2026  
**Repository:** `client-side-validation/csv-adapter`  
**Version analyzed:** `0.4.0` (workspace Cargo.toml)

---

> **Thesis:** This codebase has the right instincts and a solid protocol kernel. What it lacks is architectural discipline in every layer above that kernel. The result is a system that cannot be reasoned about, cannot be safely extended, and cannot reach its 100-chain DeFi goal without a principled refactoring intervention. This document calls every problem by its real name.

---

## Part I — Executive Diagnosis

### What Works

The protocol kernel in `csv-adapter-core` is genuinely good:

- `Seal`, `Right`, `Commitment`, `ProofBundle`, `DAGSegment` are correct primitives with sound semantics
- The `PROTOCOL_INVARIANTS.md` is precise and accurate. Every invariant stated there is real
- Domain-separated commitment hashing is implemented correctly
- The cross-chain state machine (`Locked → AwaitingFinality → BuildingProof → ProofReady → Minting → Complete`) is the right model
- Fuzz targets exist for the critical serialization paths
- The `ConsignmentValidator` 5-step pipeline is architecturally sound

### What Is Broken

Everything above the kernel is in one of four states:

1. **Duplicated** — defined twice with different semantics, one definition silently winning
2. **Bypassed** — the facade exists but the surfaces (wallet, CLI, explorer) route around it
3. **Stubs promoted to production** — mock behavior gated on Cargo features that compile into production binaries under the wrong conditions
4. **Conceptually undefined** — nobody agrees on what a Runtime is, whether the explorer needs a backend, or what "plug-and-play for 100 chains" actually means at the binary boundary

The Cargo.toml workspace author field reads `"Amin Razavi, ALL FREE LLMs"`. This is not a joke — it is an accurate description of the development model: a principal engineer directing multiple LLM-assisted coding sessions with insufficient architectural review between them. Every inconsistency in this report traces back to that pattern.

---

## Part II — Critical Defects (Security & Correctness Class)

These defects must be resolved before any testnet deployment. They are not style issues.

### Defect C-1: Dual `AnchorLayer` Trait Definition (Protocol Fragmentation)

**Location:** `csv-adapter-core/src/interface.rs` (line 7025) AND `csv-adapter-core/src/traits.rs` (line 20193)

Two definitions of `AnchorLayer` exist in the same crate. They are not identical. `interface.rs` defines a generic trait with associated types (`SealRef`, `AnchorRef`, `InclusionProof`, `FinalityProof`) that is object-unsafe — it cannot be `Box<dyn AnchorLayer>`. This version is the correct cryptographic contract.

`traits.rs` defines a second `AnchorLayer` that the rest of the codebase actually uses as the trait object surface. Because Rust resolves trait names by path, implementations of one are silently not implementations of the other. Chain adapters may satisfy one definition while violating the other.

**Security impact:** A chain implementation that passes `traits.rs` validation may still violate the associated-type invariants in `interface.rs`. Neither the compiler nor the tests catch this because both definitions compile independently.

**Fix:** Delete `interface.rs`. Consolidate into `traits.rs` (renamed `seal_protocol.rs` — see Part IV). The object-unsafe version requires a shim layer using `SealProtocolExt` or an enum-dispatch approach — document this explicitly.

---

### Defect C-2: Mutex Locked Inside Async Context in Proof Verification

**Location:** `csv-adapter/src/facade.rs`, method `verify_proof_bundle`

```rust
let seal_checker = |seal_id: &[u8]| {
    let store = self.client.store.lock().unwrap(); // std::Mutex inside async fn
    ...
};
```

A synchronous `std::sync::Mutex` is locked inside an `async fn` that is called across `.await` points. This is a classic async deadlock vector. If any other task awaits while holding this lock, or if the tokio executor parks this future between the lock acquisition and release, executor threads will block. Under load with multiple concurrent proof verifications, this degrades to sequential execution or deadlock.

**Fix:** Replace with `tokio::sync::Mutex` and `await` the lock, or restructure to remove the closure capture of `self.client` entirely by pre-fetching store state before entering the proof verification pipeline.

---

### Defect C-3: Chain-Specific Encoding Inside the Facade (Adapter Pattern Violation)

**Location:** `csv-adapter/src/facade.rs`, method `build_contract_call`

```rust
let tx_data = match chain {
    Chain::Ethereum => encode_eth_contract_call(contract, function, args),
    Chain::Sui | Chain::Aptos => encode_move_contract_call(contract, function, args, from, nonce),
    Chain::Solana => encode_solana_contract_call(contract, function, args, from),
    Chain::Bitcoin => return Err(CsvError::CapabilityUnavailable { ... }),
    _ => return Err(CsvError::ChainNotSupported(chain))
};
```

The entire point of the adapter pattern is that the facade does not know which chain it is speaking to. This `match` statement means every new chain requires modifying `csv-adapter/src/facade.rs`. This is not plug-and-play — this is hard-coded dispatch. The ABI encoding helpers (`encode_eth_contract_call`, `encode_move_contract_call`, `encode_solana_contract_call`) live in the facade crate and will drift from the actual chain adapter implementations.

**Furthermore:** `encode_eth_contract_call` pads arguments to exactly 32 bytes using a manual loop. Ethereum ABI encoding is not this simple — tuples, dynamic types, and arrays require offset encoding. This implementation will produce incorrect calldata for any function with non-scalar arguments.

**Fix:** Move `build_contract_call` into each `ChainBackend` implementation (see Part IV for rename). The facade calls `backend.build_contract_call(function, args, from, nonce)` and knows nothing about encoding.

---

### Defect C-4: `get_transaction_count` Returns Wrong Data

**Location:** `csv-adapter/src/facade.rs`, method `get_transaction_count`

```rust
match adapter.get_chain_info().await {
    Ok(info) => {
        if let Some(nonce) = info.get("nonce").and_then(|n| n.as_u64()) {
            return Ok(nonce);
        }
        if let Some(sequence) = info.get("sequence_number").and_then(|n| n.as_u64()) {
            return Ok(sequence);
        }
        Err(CsvError::CapabilityUnavailable { ... })
    }
}
```

`get_chain_info()` returns global chain metadata — block height, network ID, chain configuration. It does not return account-specific nonce data. The `address` parameter passed to this function is completely ignored. This method will always return `CapabilityUnavailable` for every chain that correctly implements `get_chain_info`, silently breaking any flow that depends on nonce management (Ethereum EIP-1559 transactions, Aptos sequence numbers).

**Fix:** Add `get_account_nonce(address: &str) -> ChainOpResult<u64>` to the `ChainQuery` trait. Each backend implements it natively.

---

### Defect C-5: `#![allow(dead_code)]` in Production Library

**Location:** `csv-adapter-store/src/lib.rs`, line 4

A production persistence library with `#![allow(dead_code)]` is a confession: the API surface is larger than what is actually used. Dead code in a persistence layer means untested code paths that can corrupt state. It also means the crate's public API is lying — types and functions exist that no caller uses, and they will never be maintained.

**Fix:** Run `cargo +nightly udeps --all-targets` on the workspace. Prune every unused symbol. Gate remaining internal utilities behind `#[cfg(test)]`. Remove the global allow.

---

### Defect C-6: Mock RPC Bleed Risk

**Location:** `csv-adapter-ethereum/src/rpc.rs` — `MockEthereumRpc` is gated by `#[cfg(test)]` correctly. However, in `csv-adapter-bitcoin/src/rpc.rs` and others, mock implementations are gated by feature flags rather than `#[cfg(test)]`. Feature flags compose — a downstream crate enabling multiple features may inadvertently include mock RPC in a production binary.

The `PROTOCOL_INVARIANTS.md` is explicit: "no silent mock success." This is violated by the feature-flag gating pattern.

**Fix:** All mock types must be `#[cfg(test)]` only. Feature flags control network connectivity, not mock vs. real. Create a `csv-test-utils` crate for shared test fixtures that is never a dependency of production crates.

---

## Part III — Architectural Defects (Design Class)

### Defect A-1: Four Overlapping Chain Abstraction Layers

The codebase currently has four independent systems for representing "something that talks to a blockchain":

| Layer | Location | Purpose | Object-Safe |
|---|---|---|---|
| `AnchorLayer` (generic) | `interface.rs` | CSV protocol contract | No |
| `AnchorLayer` (object) | `traits.rs` | Actual trait object used | Yes |
| `ChainAdapter` | `chain_adapter.rs` | Creates RpcClient + Wallet | Yes |
| `FullChainAdapter` | `chain_operations.rs` | Composite of 6 sub-traits | Yes |

Each chain then also has its own per-crate RPC trait (`EthereumRpc`, `BitcoinRpc`, `SolanaRpc`, etc.) making **five** layers. A new chain implementor must satisfy all of them, with no single document explaining how they compose.

The `ChainAdapter` trait in `chain_adapter.rs` returns `Box<dyn RpcClient>` and `Box<dyn Wallet>` from async factory methods. The `RpcClient` trait in the same file duplicates methods already on `FullChainAdapter` (`get_latest_block`, `get_balance`, `is_transaction_confirmed`). These are not the same abstraction — they co-exist without either deprecating the other.

**Root cause:** Each sprint added a new abstraction without removing the previous one. The `ChainAdapter`/`RpcClient`/`Wallet` trio was Sprint 1. `FullChainAdapter` was the Production Guarantee Plan. Neither replaced the other.

**Fix:** See Part V, Canonical Architecture. The three-layer trait hierarchy is architecturally correct — it just needs precise names. See Part IV for the rename.

---

### Defect A-2: The `rpc.rs` / `real_rpc.rs` Split Is Semantically Wrong

Every chain adapter has:
- `rpc.rs` — defines a trait (`EthereumRpc`, `BitcoinRpc`, etc.)
- `real_rpc.rs` — implements that trait for production

The name "real_rpc" implies that `rpc.rs` is fake. Developers reading this for the first time will assume `rpc.rs` is the mock and `real_rpc.rs` is the production path. This is the opposite of the intended meaning.

Bitcoin additionally has `mempool_rpc.rs` as a third RPC variant for mempool.space HTTP API, making three RPC paths for one chain. The CLI's `chain_api.rs` hardcodes mempool.space URLs as the fallback for Bitcoin balance queries, bypassing the adapter entirely.

**Fix:** Rename `real_rpc.rs` → `node.rs`. The struct connecting to an actual chain node IS a node connection. Naming it `BitcoinNode`, `EthereumNode`, etc. states exactly what it is. Both `node.rs` and `mempool_space_rpc.rs` implement the same RPC trait, registered at construction time by configuration.

---

### Defect A-3: `csv-explorer` Is Three Systems Crammed Into One Cargo Member

The `csv-explorer` workspace member contains:

```
csv-explorer/
  api/         — axum HTTP server (REST + GraphQL + WebSocket)
  indexer/     — chain polling daemon (tokio background tasks)
  shared/      — shared types between api and indexer
  storage/     — SQLite repositories (sqlx)
  ui/          — Dioxus WASM/SSR frontend
  src/         — yet another source tree with indexing/, dashboard/, api/ subdirs
```

`src/` duplicates module structure from both `api/` and `indexer/`. There are three separate `main.rs` entry points that are not all clearly separated. The `Cargo.toml` for `csv-explorer` attempts to declare all four sub-crates as a single workspace member.

**The backend question is answered by the code, not by the blueprint:** The API server processes GraphQL queries against the storage layer. The indexer populates the storage layer. The wallet uses `WalletIndexerBridge` to request priority indexing from the same storage layer. A backend is required. The question was never about whether to have a backend — it was about whether the explorer UI could be purely client-side. It cannot, because cross-chain proof history requires indexed data across chains that individual browser clients cannot assemble.

**Fix:** Split into four separate workspace crates. See Part V.

---

### Defect A-4: The WASM Wallet's Core Divergence

`csv-wallet` targets WASM via Dioxus but `csv-adapter` (the facade) is not `wasm32` compatible. The wallet resolves this by maintaining its own:

- `wallet_core.rs` — duplicated address derivation for all 5 chains (using direct cryptographic libraries)
- `services/chain_api.rs` — duplicated chain configuration and RPC endpoint management
- `services/blockchain/` — 7 files of transaction building, signing, and submission that parallel what `csv-adapter` provides
- `context/wallet.rs` — its own state management separate from `csv-adapter-store`

This is not a "thin UI layer." This is a parallel implementation of the entire adapter stack.

The root problem: `csv-adapter` pulls `tokio` with `rt-multi-thread` features. WASM does not have multi-threading. The wallet cannot depend on `csv-adapter` without a WASM-compatible feature tree.

**Fix:** `csv-core` must become `no_std` / WASM-safe as a hard requirement. `csv-sdk` (the facade) must expose a `wasm` feature that replaces tokio with `wasm-bindgen-futures` and `gloo-timers`. `csv-keys` already has `browser_keystore.rs` — this is the correct pattern. The wallet's `wallet_core.rs`, `services/chain_api.rs`, and `services/blockchain/` must be deleted and replaced with the WASM-featured facade.

---

### Defect A-5: Three Overlapping Dynamic Chain Registration Systems

The codebase contains three independent systems for dynamically registering chains:

1. **`ChainRegistry`** in `chain_adapter.rs` — `HashMap<String, Box<dyn ChainAdapter>>`
2. **`ChainPluginRegistry`** in `chain_plugin.rs` — `HashMap<String, Box<dyn ChainPlugin>>`
3. **`AdapterFactory`** in `adapter_factory.rs` — `fn create_adapter(chain: Chain) -> Option<Box<dyn FullChainAdapter>>`

The explorer's indexer uses `IndexerPluginRegistry` which is a fourth system built independently.

None of these registries share data or code. A chain registered in `ChainRegistry` is not available in `ChainPluginRegistry`. The 100-chain goal requires one registry, one trait contract, one registration mechanism. Currently adding Bitcoin requires touching all four systems independently.

**Fix:** One `DriverRegistry`, one entry point. Three files (`adapter_factory.rs`, `chain_plugin.rs`, `chain_discovery.rs`) merge into `driver_registry.rs`. See Part V.

---

### Defect A-6: The Runtime Question (Answered Incorrectly)

The team is uncertain whether a Runtime is needed. The answer is: **yes, but it should be explicit, not emergent.**

Currently, runtime behavior is distributed across:
- The explorer indexer's `SyncCoordinator` (block polling, re-org detection)
- The `WalletIndexerBridge` (priority indexing for wallet-owned addresses)
- `ReorgMonitor` in `csv-adapter-core/src/monitor.rs` (chain re-org tracking)
- `PublicationTracker` in `monitor.rs` (commitment publication tracking)
- The `CircuitBreaker` in `hardening.rs` (per-RPC-endpoint failure isolation)

These components belong in a named `csv-runtime` crate that owns long-running tasks. The CLI does not need it (request-response). The wallet needs a lightweight version (the WASM runtime is `wasm-bindgen-futures`). The explorer needs the full version.

**Note on naming:** The top-level orchestrator struct (`ChainFacade`) should not yet be renamed to `CsvRuntime` — the decision of whether the runtime is a separate process, an embedded library, or a daemon is still open (see Part X Decision Register). The name must follow the decision.

---

## Part IV — Naming: Complete Diagnosis and Correction

Naming problems in this codebase are not cosmetic. They carry cognitive load into every future development session and into every external developer's first encounter with the codebase. There are three distinct naming problems that feed each other.

### The Three Root Naming Failures

**Problem 1 — "Adapter" means nothing at the domain level.**
An adapter is a small connector between two incompatible interfaces. This codebase is a full protocol SDK with a wallet, explorer, cross-chain proof system, and contract schema library. "Adapter" describes none of that. It also appears at three different levels with three different meanings: `ChainAdapter` (plugin descriptor), `AnchorLayer` (protocol trait), `FullChainAdapter` (combined operations). The word does different things each time it appears.

**Problem 2 — The three-layer trait hierarchy has no clear names.**

```
ChainAdapter       ← "basic plugin descriptor" (chain_id, capabilities, create_client)
AnchorLayer        ← "the actual seal protocol" (create_seal, publish, verify_inclusion, enforce_seal)
FullChainAdapter   ← "combined: Query + Signer + Broadcaster + Deployer + ProofProvider + RightOps"
```

These three are architecturally correct and distinct. They just share words ("adapter", "layer") that reveal nothing about what each one does.

**Problem 3 — `SealRef` and `Right` are ambiguous in their own contexts.**
`SealRef` clashes with Rust's `&SealRef` reference syntax every time it appears in function signatures. `Right` collides with the English word "correct" and the directional word, causing readers to re-parse every sentence.

---

### Repository and Crate Renames

#### Rename repo: `csv-adapter` → `csv-protocol`

"CSV" to any outsider means comma-separated values. The `client-side-validation` GitHub org rescues it — but the repo name cannot assume the org is always visible. "Adapter" is documented above.

```
github.com/client-side-validation/csv-protocol   ← monorepo
github.com/client-side-validation/csv-wallet     ← (after split)
github.com/client-side-validation/csv-explorer   ← (after split)
github.com/client-side-validation/csv-ts         ← TypeScript SDK
github.com/client-side-validation/csv-mcp        ← MCP server
```

Keep the GitHub org name exactly as is. `client-side-validation` is precise, unique, and googleable.

#### Crate renames — drop "adapter" from every path

| Current crate name | New crate name | Rationale |
|---|---|---|
| `csv-adapter-core` | `csv-core` | Core protocol primitives. Mirrors `bitcoin`, `lightning`, `rgb-core`. |
| `csv-adapter-bitcoin` | `csv-bitcoin` | "CSV protocol, Bitcoin backend." Reads correctly. |
| `csv-adapter-ethereum` | `csv-ethereum` | Same pattern. |
| `csv-adapter-sui` | `csv-sui` | Same. |
| `csv-adapter-aptos` | `csv-aptos` | Same. |
| `csv-adapter-solana` | `csv-solana` | Same. |
| `csv-adapter-store` | `csv-store` | Storage layer. |
| `csv-adapter-keystore` | `csv-keys` | Key management. Shorter and equally clear. |
| `csv-adapter` | `csv-sdk` | The unified runtime crate. "SDK" correctly signals this is what integrators import. |
| `csv-cli` | `csv-cli` | Already correct. |
| `csv-wallet` | `csv-wallet` | Already correct. |
| `csv-explorer` | `csv-explorer` | Already correct. |

---

### The Three Trait Layers — Precise New Names

This is the architectural heart of the rename. Three traits, three precise names.

#### Layer 1: `ChainAdapter` → `ChainDriver`

**What it does:** Plugin descriptor for a chain. Provides `chain_id()`, `chain_name()`, `capabilities()`, `create_client()`, `create_wallet()`. How a chain registers itself into the system.

**Why "Driver"?** A device driver is the minimal interface that allows an OS to use a piece of hardware. `ChainDriver` is exactly that — the minimal interface that allows the protocol to use a blockchain. It does not do protocol operations; it describes the chain and creates the tools to interact with it.

```rust
// Before
pub trait ChainAdapter: Send + Sync { ... }
pub trait ChainAdapterExt: ChainAdapter { ... }

// After
pub trait ChainDriver: Send + Sync { ... }
pub trait ChainDriverExt: ChainDriver { ... }
```

**File rename:** `chain_adapter.rs` → `driver.rs`

---

#### Layer 2: `AnchorLayer` → `SealProtocol`

**What it does:** THE core protocol trait. Defines:
- `create_seal()` — open a new single-use seal on-chain
- `publish()` — anchor a commitment to a seal
- `verify_inclusion()` — prove commitment is in a block
- `verify_finality()` — prove block is final
- `enforce_seal()` — consume/close the seal

**Why "SealProtocol"?** This trait IS the single-use seal protocol. Every method is a step in the seal lifecycle. "Anchor" in the current name focuses on where commitments land; "SealProtocol" names what the trait is responsible for: the full lifecycle of a seal.

The associated types get clearer names too:

```rust
pub trait SealProtocol {
    type SealPoint: Debug + Clone + Eq;       // was: SealRef
    type AnchorPoint: Debug + Clone + Eq;     // was: AnchorRef
    type InclusionProof: Debug + Clone;       // unchanged — already clear
    type FinalityProof: Debug + Clone;        // unchanged
}
```

Chain implementations:

```rust
// Before                      // After
struct EthereumAnchorLayer     struct EthereumSealProtocol
struct BitcoinAnchorLayer      struct BitcoinSealProtocol
struct SuiAnchorLayer          struct SuiSealProtocol
struct AptosAnchorLayer        struct AptosSealProtocol
struct SolanaAnchorLayer       struct SolanaSealProtocol
```

**File rename:** `traits.rs` → `seal_protocol.rs`

---

#### Layer 3: `FullChainAdapter` → `ChainBackend`

**What it does:** The complete chain implementation combining:
- `ChainQuery` — read chain state
- `ChainSigner` — sign transactions
- `ChainBroadcaster` — submit transactions
- `ChainDeployer` — deploy contracts
- `ChainProofProvider` — build proof bundles
- `ChainSanadOps` — operate on Sanads (was `ChainRightOps`)

**Why "Backend"?** A backend is a full, complete implementation. "Full" in `FullChainAdapter` already signals completeness; "Backend" names what it is: the complete chain-side implementation that the protocol runtime talks to.

```rust
// Before
pub trait FullChainAdapter: ChainQuery + ChainSigner + ChainBroadcaster +
    ChainDeployer + ChainProofProvider + ChainRightOps { ... }

// After
pub trait ChainBackend: ChainQuery + ChainSigner + ChainBroadcaster +
    ChainDeployer + ChainProofProvider + ChainSanadOps { ... }
```

Chain implementations:

```
Before                          After
────────────────────────────────────────────────────────────
BitcoinChainOperations          BitcoinBackend
EthereumChainOperations         EthereumBackend
SuiChainOperations              SuiBackend
AptosChainOperations            AptosBackend
SolanaChainOperations           SolanaBackend
```

**File split:** `chain_adapter.rs` splits into `driver.rs` (Layer 1) and `backend.rs` (Layer 3). These two concepts share nothing and must not share a file.

---

#### Three-Layer Summary

```
Before                     After                   File
──────────────────────────────────────────────────────────────
ChainAdapter               ChainDriver             driver.rs
AnchorLayer                SealProtocol            seal_protocol.rs
FullChainAdapter           ChainBackend            backend.rs
```

---

### Core Type Renames

#### `SealRef` → `SealPoint`

**Problem:** `SealRef` clashes with Rust reference syntax. `&SealRef` in a function signature is genuinely ambiguous for a millisecond — "is this a reference to something, or the SealRef type?" Also, it is not "a reference to a seal" — it IS the seal identifier itself.

**Why "SealPoint"?** Bitcoin uses `OutPoint` (txid + vout) to identify a specific output. A Bitcoin seal IS an OutPoint. `SealPoint` generalizes this: a specific point on any chain that acts as a seal. Precise, has blockchain precedent, does not clash with Rust syntax.

```rust
// Before
pub struct SealRef {
    pub seal_id: Vec<u8>,
    pub nonce: Option<u64>,
}

// After
pub struct SealPoint {
    pub id: Vec<u8>,           // was: seal_id (redundant prefix now that type is SealPoint)
    pub nonce: Option<u64>,
}
```

Chain-specific variants:

```rust
// Before                     // After
BitcoinSealRef                BitcoinSealPoint
AptosSealRef                  AptosSealPoint
EthereumSealRef               (uses SealPoint directly — nullifier hash)
```

Fuzz target: `fuzz_seal_ref_from_bytes.rs` → `fuzz_seal_point.rs`

---

#### `AnchorRef` → `CommitAnchor`

**Problem:** `AnchorRef` suffers the same ref-confusion as `SealRef`. Also ambiguous with the proposed rename of `AnchorLayer`.

**Why "CommitAnchor"?** It is where a commitment was anchored on-chain. Reads as a noun: "the anchor for a commitment."

```rust
// Before                        // After
pub struct AnchorRef { ... }     pub struct CommitAnchor { ... }
pub struct BitcoinAnchorRef      pub struct BitcoinCommitAnchor
pub struct AptosAnchorRef        pub struct AptosCommitAnchor
```

---

#### `Right` → `Sanad`

**Problem:** "Right" is legally accurate (a right in property law is exclusive and transferable) but has two practical problems:

1. Collides with English: "turn right", "that's right", "right-click" — every developer mentally re-parses it on first encounter in code.
2. `OwnedState` already exists in `state.rs`. The relationship between `Right` and `OwnedState` is unclear when names look unrelated.

**Why "Sanad"?** A property deed (سَنَد) is:
- A legal document proving ownership
- Exclusive — one title per property
- Transferable — deeds change hands
- The record of provenance — chain of title

"Chain of title" even has a legal meaning matching the commitment chain exactly.

```rust
// Before                           // After
pub struct Right { ... }            pub struct Sanad { ... }
pub type RightId = Hash;            pub type SanadId = Hash;
RightOperationResult                SanadOperationResult
ChainRightOps                       ChainSanadOps
```

**File:** `right.rs` → `title.rs`

---

#### `MpcTree` → `CommitMux`

**Problem:** "MPC" stands for Multi-Protocol Commitment (the doc comment says so). But every developer will read "MPC" as Multi-Party Computation — a completely different cryptographic concept. The confusion is not hypothetical; it will mislead every new contributor.

**Why "CommitMux"?** A multiplexer combines multiple signals into one output. `CommitMux` combines multiple protocol commitments into one on-chain output. "Mux" has technical precision without collision with MPC terminology.

```rust
// Before          // After
MpcTree            CommitMux
MpcLeaf            MuxLeaf
MpcProof           MuxProof
```

**File:** `mpc.rs` → `commit_mux.rs`

---

#### `CrossChainSealRegistry` → `SealNullifier`

The name "registry" suggests a lookup table. What this actually does is enforce that a seal cannot be consumed twice. That is a nullifier — the ZK term for "a value that, once revealed, can never be used again." Using the standard term aligns with Phase 5 ZK work where actual ZK nullifiers will be used.

```rust
// Before
pub struct CrossChainSealRegistry { ... }

// After
pub struct SealNullifier { ... }
```

**File:** `seal_registry.rs` → `nullifier.rs`

---

#### `real_rpc.rs` → `node.rs` (across all five chains)

The struct connecting to an actual chain node IS a node connection. "Real" is a quality judgment, not a domain concept.

```
csv-bitcoin/src/real_rpc.rs    → csv-bitcoin/src/node.rs     (BitcoinNode)
csv-ethereum/src/real_rpc.rs   → csv-ethereum/src/node.rs    (EthereumNode)
csv-sui/src/real_rpc.rs        → csv-sui/src/node.rs         (SuiNode)
csv-aptos/src/real_rpc.rs      → csv-aptos/src/node.rs       (AptosNode)
csv-solana/src/real_rpc.rs     → csv-solana/src/node.rs      (SolanaNode)
```

---

#### `AdapterFactory` + `ChainPluginRegistry` + `ChainDiscovery` → `DriverRegistry`

Three files in core overlap in responsibility. Merge all three into `driver_registry.rs`:

```rust
// Before: three files
adapter_factory.rs     → DriverRegistry::register(), DriverRegistry::get()
chain_plugin.rs        → ChainPluginRegistry, ChainPluginMetadata
chain_discovery.rs     → discovery logic

// After: one file
driver_registry.rs     → DriverRegistry (merged), DriverMetadata (was ChainPluginMetadata)
```

---

### Complete File Rename Map

#### `csv-core/` (was `csv-adapter-core/`)

| Current path | New path |
|---|---|
| `src/traits.rs` | `src/seal_protocol.rs` |
| `src/chain_adapter.rs` | `src/driver.rs` + `src/backend.rs` (split) |
| `src/right.rs` | `src/title.rs` |
| `src/seal_registry.rs` | `src/nullifier.rs` |
| `src/mpc.rs` | `src/commit_mux.rs` |
| `src/adapter_factory.rs` | `src/driver_registry.rs` (merged with chain_plugin + chain_discovery) |
| `src/chain_plugin.rs` | *(merged into driver_registry.rs)* |
| `src/chain_discovery.rs` | *(merged into driver_registry.rs)* |
| `src/rgb_compat.rs` | `src/rgb.rs` |
| `src/advanced_commitments.rs` | `src/commitments_ext.rs` |
| `src/adapters/mod.rs` | `src/drivers/mod.rs` |
| `src/adapters/test.rs` | `src/drivers/mock.rs` |
| `src/proof_verify.rs` | `src/verifier.rs` |
| `src/agent_types.rs` | `src/mcp.rs` |
| `examples/basic_right.rs` | `examples/basic_title.rs` |
| `fuzz/fuzz_targets/fuzz_right_from_canonical_bytes.rs` | `fuzz/fuzz_targets/fuzz_title.rs` |
| `fuzz/fuzz_targets/fuzz_seal_ref_from_bytes.rs` | `fuzz/fuzz_targets/fuzz_seal_point.rs` |

Files to keep exactly as-is (names are correct):
`commitment.rs`, `commitment_chain.rs`, `consignment.rs`, `cross_chain.rs`, `dag.rs`, `error.rs`, `events.rs`, `genesis.rs`, `hardening.rs`, `hash.rs`, `monitor.rs`, `performance.rs`, `proof.rs`, `protocol_version.rs`, `schema.rs`, `seal.rs`, `signature.rs`, `state.rs`, `state_store.rs`, `store.rs`, `tagged_hash.rs`, `tapret_verify.rs`, `transition.rs`, `validator.rs`, `vm/`, `zk_proof.rs`

#### Chain crates — all five, same pattern

| Current file | New file |
|---|---|
| `adapter.rs` | `seal_protocol.rs` |
| `chain_adapter_impl.rs` | `backend.rs` |
| `chain_operations.rs` | `ops.rs` |
| `real_rpc.rs` | `node.rs` |
| `rpc.rs` | `rpc.rs` (keep — it IS the RPC trait) |

#### `csv-sdk/` (was `csv-adapter/`)

| Current | New |
|---|---|
| `src/facade.rs` | `src/runtime.rs` |
| `src/errors.rs` | `src/error.rs` (singular convention) |

#### `csv-wallet/src/`

| Current | New |
|---|---|
| `seals/manager.rs` | `seals/registry.rs` |
| `pages/rights/` | `pages/titles/` |
| `hooks/use_assets.rs` | `hooks/use_titles.rs` |
| `services/blockchain/` | `services/chain/` |
| `components/seal_visualizer.rs` | `components/seal_view.rs` |
| `components/proof_inspector.rs` | `components/proof_view.rs` |

---

### Complete Struct/Enum Rename Table

| Before | After | Location |
|---|---|---|
| `SealRef` | `SealPoint` | seal.rs, all consumers |
| `AnchorRef` | `CommitAnchor` | commitment.rs, all consumers |
| `Right` | `Sanad` | title.rs |
| `RightId` | `SanadId` | title.rs |
| `AnchorLayer` | `SealProtocol` | seal_protocol.rs |
| `FullChainAdapter` | `ChainBackend` | backend.rs |
| `ChainAdapter` | `ChainDriver` | driver.rs |
| `ChainAdapterExt` | `ChainDriverExt` | driver.rs |
| `MpcTree` | `CommitMux` | commit_mux.rs |
| `MpcLeaf` | `MuxLeaf` | commit_mux.rs |
| `MpcProof` | `MuxProof` | commit_mux.rs |
| `CrossChainSealRegistry` | `SealNullifier` | nullifier.rs |
| `AdapterError` | `ProtocolError` | error.rs |
| `ChainRightOps` | `ChainSanadOps` | backend.rs |
| `RightOperationResult` | `SanadOperationResult` | backend.rs |
| `ChainPluginMetadata` | `DriverMetadata` | driver_registry.rs |
| `ChainPluginRegistry` | `DriverRegistry` | driver_registry.rs |
| `ChainVerificationResult` | `VerificationResult` | (drop redundant "Chain" prefix) |
| `BitcoinAnchorLayer` | `BitcoinSealProtocol` | csv-bitcoin |
| `BitcoinChainOperations` | `BitcoinBackend` | csv-bitcoin |
| `BitcoinSealRef` | `BitcoinSealPoint` | csv-bitcoin |
| `BitcoinAnchorRef` | `BitcoinCommitAnchor` | csv-bitcoin |
| `EthereumAnchorLayer` | `EthereumSealProtocol` | csv-ethereum |
| `EthereumChainOperations` | `EthereumBackend` | csv-ethereum |
| `SuiAnchorLayer` | `SuiSealProtocol` | csv-sui |
| `SuiChainOperations` | `SuiBackend` | csv-sui |
| `AptosAnchorLayer` | `AptosSealProtocol` | csv-aptos |
| `AptosChainOperations` | `AptosBackend` | csv-aptos |
| `AptosSealRef` | `AptosSealPoint` | csv-aptos |
| `AptosAnchorRef` | `AptosCommitAnchor` | csv-aptos |
| `SolanaAnchorLayer` | `SolanaSealProtocol` | csv-solana |
| `SolanaChainOperations` | `SolanaBackend` | csv-solana |
| `use_assets` hook | `use_titles` | csv-wallet |
| `AssetService` | `SanadService` | csv-wallet |

---

### Names That Must Stay Exactly As-Is

These names are correct and must not be changed:

- `Seal`, `SealRecord`, `SealStatus` — correct RGB-compatible term from the single-use seal literature (Peter Todd). Deviation breaks RGB ecosystem alignment.
- `Consignment`, `ConsignmentError`, `ConsignmentValidator` — standard CSV/RGB literature term. Precise: you consign proof data to the receiver for independent verification.
- `ProofBundle` — specific, accurate, not confusing.
- `CommitmentChain` — technically precise. A hash-linked list of commitments.
- `TapretCommitment`, `OpretCommitment` — Bitcoin-specific standard terms (taproot-embedded, OP_RETURN-embedded). Standard in RGB/CSV Bitcoin literature.
- `Genesis`, `Schema`, `Transition`, `Validator` — clear domain terms.

---

### Naming Conventions Going Forward

Establish these rules in `CONTRIBUTING.md` to prevent drift:

**Rule 1: No pattern names as prefixes in domain type names.**
`ChainAdapter` → `ChainDriver`. `AnchorLayer` → `SealProtocol`. `RealRpc` → `ChainNode`. Name the concept, not the GoF pattern.

**Rule 2: No `Ref` suffix on non-reference types.**
Rust uses `Ref` for borrowed smart pointers. Domain types that identify something use `Point`, `Id`, `Anchor`, `Locator`.

**Rule 3: Error types use `Error` suffix, not `Errors`. Singular.**
`errors.rs` → `error.rs`. `CsvErrors` → `ProtocolError`.

**Rule 4: File name = primary type name.**
`seal_protocol.rs` contains `SealProtocol`. `title.rs` contains `Sanad`. If a file contains multiple types with no primary, use a descriptive noun: `types.rs`, `ops.rs`.

**Rule 5: `Backend` for full implementations, `Driver` for descriptors.**
`ChainBackend` = full implementation. `ChainDriver` = minimal plugin descriptor. Never use "Adapter", "Facade", "Factory" in domain type names again.

---

## Part V — Canonical Target Architecture

### Principle: Three Abstraction Levels, No Shortcuts Between Them

```
Level 1: Protocol (csv-core)
  ├── Cryptographic primitives (Seal, Commitment, Hash, DAGSegment, ProofBundle)
  ├── Domain traits (SealProtocol — one definition, object-safe via enum dispatch or dyn-safe redesign)
  ├── Chain operation contracts (ChainBackend replacing FullChainAdapter)
  ├── State machine types (TransferState, ConsignmentValidator)
  └── Protocol invariants (compile-time enforced where possible)

Level 2: Chain Implementations (csv-{chain})
  ├── One crate per chain
  ├── Implements: SealProtocol, ChainBackend
  ├── Contains: native RPC client (node.rs), transaction builder, contract bindings, proof builder
  ├── Never knows about other chains
  └── Registers itself via DriverMetadata at startup

Level 3: Surfaces (csv-sdk, csv-cli, csv-wallet, csv-explorer)
  ├── csv-sdk: the unified Rust facade + WASM bindings
  ├── csv-cli: thin CLI over sdk
  ├── csv-wallet: Dioxus UI, WASM-compiled, uses sdk with wasm feature
  └── csv-explorer: split into 4 sub-crates (see below)
```

### Canonical Workspace Structure

```toml
# Root Cargo.toml
[workspace]
members = [
  # Protocol layer
  "core",                    # was csv-adapter-core

  # Chain implementations
  "chains/bitcoin",          # was csv-adapter-bitcoin
  "chains/ethereum",         # was csv-adapter-ethereum
  "chains/sui",              # was csv-adapter-sui
  "chains/aptos",            # was csv-adapter-aptos
  "chains/solana",           # was csv-adapter-solana

  # Infrastructure
  "store",                   # was csv-adapter-store (no dead code)
  "keystore",                # was csv-adapter-keystore
  "runtime",                 # NEW — long-running task coordinator

  # Surfaces
  "sdk",                     # was csv-adapter (facade)
  "cli",                     # was csv-cli
  "wallet",                  # was csv-wallet

  # Explorer (four crates)
  "explorer/shared",         # shared types
  "explorer/indexer",        # chain polling daemon
  "explorer/api",            # axum HTTP/WebSocket/GraphQL server
  "explorer/ui",             # Dioxus frontend
]
```

### The Single Chain Registration Contract

Replace all four registration systems with one:

```rust
// In core/src/driver_registry.rs
pub struct DriverMetadata {
    pub chain_id: &'static str,
    pub display_name: &'static str,
    pub account_model: AccountModel,
    pub finality_model: FinalityModel,
    pub capabilities: CapabilitySet,
    pub default_networks: &'static [NetworkDescriptor],
}

pub trait ChainDriver: Send + Sync + 'static {
    fn metadata(&self) -> &'static DriverMetadata;
    fn build_backend(&self, config: &ChainConfig) -> ChainOpResult<Arc<dyn ChainBackend>>;
    fn build_indexer(&self, config: &ChainConfig, rpc: RpcManager)
        -> ChainOpResult<Arc<dyn ChainIndexer>>;
}

// Registration at binary startup:
let mut registry = DriverRegistry::new();
registry.register(BitcoinDriver::new());
registry.register(EthereumDriver::new());
// Adding chain 6..100: register one driver, done.
```

This single entry point replaces `ChainAdapter`, `ChainPlugin`, `AdapterFactory`, `ChainRegistry`, and `IndexerPluginRegistry` simultaneously.

### The Unified SealProtocol (Object-Safe)

The generic `SealProtocol` (formerly `AnchorLayer`) with associated types cannot be used as a trait object. The fix is not to delete the generic version — it encodes important type safety. The fix is to provide a type-erased wrapper:

```rust
// Keep in core/src/seal_protocol.rs
pub trait SealProtocol: Send + Sync {
    fn chain_id(&self) -> &'static str;
    fn create_seal(&self, value: Option<u64>) -> SealResult<SealPoint>;
    fn publish(&self, commitment: &Commitment, seal: &SealPoint) -> SealResult<CommitAnchor>;
    fn verify_inclusion(&self, anchor: &CommitAnchor) -> SealResult<InclusionProof>;
    fn verify_finality(&self, anchor: &CommitAnchor) -> SealResult<FinalityProof>;
    fn enforce_seal(&self, seal: &SealPoint) -> SealResult<()>;
    fn domain_separator(&self) -> [u8; 32];
    fn signature_scheme(&self) -> SignatureScheme;
    fn build_proof_bundle(
        &self, anchor: &CommitAnchor, dag: &DAGSegment
    ) -> SealResult<ProofBundle>;
}
```

The generic version in the deleted `interface.rs` becomes an internal implementation detail that chain backends use. It never appears in the public API.

### WASM Compatibility Plan

`csv-core` must be `no_std` compatible. Currently it uses `std::sync::Mutex`, `std::collections::HashMap`, and `std::error::Error` throughout. The path:

1. Replace `std::sync::Mutex` with `spin::Mutex` (already `no_std`) for core types
2. Gate `HashMap` behind `std` feature; use `BTreeMap` in `no_std` mode
3. `thiserror` works in `no_std` via `core::error::Error` in Rust 1.81+
4. `csv-sdk` gets two feature trees: `native` (tokio) and `wasm` (wasm-bindgen-futures + web-sys)

The wallet depends only on `csv-sdk` with `wasm` feature. All duplicated `wallet_core.rs`, `services/blockchain/`, and `services/chain_api.rs` are deleted.

---

## Part VI — The DeFi Endgame Gap Analysis

The stated ultimate goal: low-latency DeFi, off-chain proofs, Celestia DA, ZK proofs, 100 chains, plug-and-play. Honest gap analysis follows.

### Celestia Data Availability — Status: Absent

There is zero Celestia integration in the codebase. The `zk_proof.rs` module exists but contains only type stubs. The intended architecture for a DA-backed CSV protocol would be:

```
Transaction Flow:
  User signs Sanad transfer
  → CSV proof bundle generated (off-chain)
  → Proof bundle posted to Celestia namespace
  → Celestia block header committed on destination chain
  → Destination chain verifies: (a) DA proof from Celestia, (b) CSV proof bundle
  → Sanad minted on destination
```

This requires a `csv-celestia` crate implementing the Celestia light client (namespace merkle proofs, data root inclusion) and modifications to `SealProtocol::verify_inclusion` to support Celestia-anchored proofs alongside chain-native proofs. The `FinalityProofType` enum in `commitments_ext.rs` has a placeholder for this but nothing more.

**Phase gate:** Celestia integration is not a sprint task. It is a 3-6 month track requiring: celestia-node integration, blob submission, namespace proof verification circuits, and chain-side verifier contracts on all 5 current chains. Do not represent this as "planned" — represent it as "scoped and unstarted."

### ZK Proofs — Status: Scaffolding Only

`csv-bitcoin/src/zk_prover.rs` and `csv-core/src/zk_proof.rs` exist. The bitcoin zk_prover references SP1 (Succinct's zkVM) in `sp1_guest/`. This is the right choice for a production ZK stack — SP1 generates recursion-friendly STARK proofs from Rust guest programs.

What is missing:
- The SP1 guest program for SPV proof (exists as `sp1_guest/spv.rs` but is a stub)
- The verifier contract on EVM chains
- Proof aggregation for batching multiple Sanad transfers
- The recursive proof composition needed for Celestia DA proofs

**Recommendation:** ZK proofs for Bitcoin SPV verification are the highest-value first target. A real SP1-based Bitcoin inclusion proof eliminates the need for trust assumptions in Bitcoin → EVM cross-chain transfers. This should be Workstream F Phase 1.

### 100 Chains — Status: Architecturally Possible, Currently Impossible

The current architecture requires, for each new chain:
- New Cargo crate added to workspace
- New feature flag in `csv-sdk/Cargo.toml`
- New match arm in `facade.rs` (Defect C-3)
- New branch in `wallet_core.rs` address derivation (Defect A-4)
- New indexer registered in `IndexerPluginRegistry`
- New chain variant in `Chain` enum in `protocol_version.rs`

Adding chain 6 to chain 100 requires 6 separate coordinated code changes. With the canonical architecture from Part V (single `ChainDriver` trait + `DriverRegistry`), adding a new chain requires: one new crate implementing `ChainDriver`, one `registry.register(NewChainDriver::new())` call at startup. Nothing else changes.

The `Chain` enum in `protocol_version.rs` is the hardest dependency. It must become either a newtype over a string identifier or an open enum via a numeric identifier with string registration. The latter is the correct long-term answer for 100 chains.

### Low-Latency DeFi — Status: Latency Not Measured

No benchmarks exist for end-to-end transfer latency. The `performance.rs` module in core defines `PerformanceMetrics` and `ProofCache` but these are not wired to the actual proof generation path. No latency SLAs are defined.

For DeFi the critical path is:

```
Lock on source chain → Finality confirmation → Proof generation → DA posting → Mint on destination
```

Bitcoin finality: 6 confirmations ≈ 60 minutes (irremovable bottleneck without fraud proofs)
Ethereum finality: ~13 minutes (2 epoch checkpoints)
Sui/Aptos/Solana: 2-4 seconds

A DeFi application built on Bitcoin source locking will have a 60-minute settlement time. This is not low-latency. The architecture must support **optimistic minting** (mint immediately, slash on fraud proof) for acceptable UX. This requires the fraud proof infrastructure described in the blueprint's research track — which is currently entirely unimplemented.

**Recommendation:** For the DeFi goal, prioritize Solana/Aptos/Sui as source chains first. Their sub-5-second finality makes real-time DeFi possible. Bitcoin becomes a store-of-value anchor, not a DeFi source chain, in the initial product.

---

## Part VII — Phased Refactoring Plan

### Phase 0 — Stabilize (Weeks 1-2)

**Goal:** Stop the bleeding. No new features until these are done.

- Fix Defect C-1: merge dual `SealProtocol` definitions, delete `interface.rs`
- Fix Defect C-2: replace `std::sync::Mutex` in async proof verification with `tokio::sync::Mutex`
- Fix Defect C-3: move encoding into each `ChainBackend` implementation
- Fix Defect C-4: add `get_account_nonce(address)` to `ChainQuery`; implement per chain
- Fix Defect C-5: remove `#![allow(dead_code)]` from csv-store; purge dead symbols
- Fix Defect C-6: gate all mock types to `#[cfg(test)]`; create `csv-test-utils`
- Fix security: persist `SealNullifier` to store on every mutation; reload at startup
- Fix security: change `seal_checker` closure error path from `false` to `Err`

**Output:** Green CI. No known security-class defects. Single `SealProtocol` definition.

---

### Phase 1 — Naming & Structure (Weeks 3-5)

**Goal:** Engineers can navigate the codebase without a guide. Every name describes its domain concept.

Execute in this order to keep CI green at each step:

**Step 1 — Crate renames (Cargo.toml only, no code changes)**
- `csv-adapter-core` → `csv-core`
- `csv-adapter` → `csv-sdk`
- `csv-adapter-store` → `csv-store`
- `csv-adapter-keystore` → `csv-keys`
- All five chain crates: `csv-adapter-{chain}` → `csv-{chain}`
- Update all path deps in workspace Cargo.toml
- CI must pass after this step alone.

**Step 2 — `SealRef` → `SealPoint` (highest call-site count)**
- Global find-replace: `SealRef` → `SealPoint`, `seal_id` → `id` within `SealPoint` only
- Fuzz target rename: `fuzz_seal_ref_from_bytes.rs` → `fuzz_seal_point.rs`
- CI must pass.

**Step 3 — `AnchorRef` → `CommitAnchor`**
- Global find-replace. Fewer call sites than `SealRef`.
- CI must pass.

**Step 4 — Trait renames (most architecturally impactful)**
- `AnchorLayer` → `SealProtocol`
- `FullChainAdapter` → `ChainBackend`
- `ChainAdapter` → `ChainDriver`
- All impl blocks across 5 chain crates
- File renames: `traits.rs` → `seal_protocol.rs`, `chain_adapter.rs` → `driver.rs` + `backend.rs`
- CI must pass.

**Step 5 — `Right` → `Sanad`**
- `right.rs` → `title.rs`
- Global find-replace: `Right` → `Sanad`, `RightId` → `SanadId`
- Wallet: `pages/rights/` → `pages/titles/`, `use_assets` → `use_titles`
- CI must pass.

**Step 6 — `MpcTree` → `CommitMux`**
- `mpc.rs` → `commit_mux.rs`
- Global find-replace within file and consumers
- CI must pass.

**Step 7 — `CrossChainSealRegistry` → `SealNullifier`**
- `seal_registry.rs` → `nullifier.rs`
- CI must pass.

**Step 8 — Remaining file renames**
- `adapter_factory.rs` → `driver_registry.rs` (merged with `chain_plugin.rs` + `chain_discovery.rs`)
- `real_rpc.rs` → `node.rs` (across all 5 chain crates)
- `rgb_compat.rs` → `rgb.rs`
- `proof_verify.rs` → `verifier.rs`
- `advanced_commitments.rs` → `commitments_ext.rs`
- `agent_types.rs` → `mcp.rs`
- CI must pass.

**Step 9 — `scalable_builder.rs` deleted; merged into `builder.rs`**
- CI must pass.

**Step 10 — Repository rename**
- GitHub: `csv-adapter` → `csv-protocol`
- Update `CODEBASE_OWNERS.md`, README, all docs
- Update docs.rs links if published

Each step is a single PR. No feature changes in any step. CI is the gate.

**Output:** Every file name describes what it contains. No duplicate-named types. No "Adapter" anywhere in domain names.

---

### Phase 2 — Registry Unification (Weeks 6-7)

**Goal:** One registration mechanism for all chain capabilities.

- Implement `ChainDriver` trait as specified in Part V
- Implement unified `DriverRegistry` consuming `ChainDriver`
- Port Bitcoin, Ethereum, Sui, Aptos, Solana to implement `ChainDriver`
- Wire `DriverRegistry` into sdk, cli, explorer indexer
- Delete old `AdapterFactory`, `ChainPluginRegistry`, `IndexerPluginRegistry`
- Change `Chain` enum to string-ID approach or open numeric enum

**Output:** Adding a new chain = one crate + one `registry.register()` call. Measured with a test chain.

---

### Phase 3 — WASM Unification (Weeks 8-10)

**Goal:** Wallet uses the same code as CLI and backend services.

- Make `csv-core` fully `no_std` compatible
- Add `wasm` feature to `csv-sdk` (facade)
- Implement WASM-compatible async runtime shims
- Delete `csv-wallet/src/wallet_core.rs` (address derivation migrated to csv-keys)
- Delete `csv-wallet/src/services/blockchain/` (replaced by sdk)
- Delete `csv-wallet/src/services/chain_api.rs` (replaced by sdk)
- Validate: wallet WASM binary size budget (< 5MB gzipped), load time (< 3s on 4G)

**Output:** Single implementation of address derivation, signing, and chain queries across CLI, backend, and WASM.

---

### Phase 4 — Explorer Decomposition (Weeks 11-12)

**Goal:** Explorer is four independently deployable systems.

- Split `csv-explorer` into `explorer/shared`, `explorer/indexer`, `explorer/api`, `explorer/ui`
- Define API contracts between indexer → storage → api → ui as explicit types in `explorer/shared`
- Create `csv-runtime` crate for `ReorgMonitor`, `PublicationTracker`, `CircuitBreaker`, `SyncCoordinator`
- Wire `WalletIndexerBridge` through `csv-runtime` (not through the explorer crate)
- Remove `src/` top-level directory from explorer (dead code)

**Output:** `explorer/indexer` and `explorer/api` are independently deployable Docker containers. Explorer UI is a static WASM binary.

---

### Phase 5 — ZK & Celestia Track (Months 3-6)

**Goal:** Real ZK proofs for Bitcoin SPV. Celestia DA integration scoped.

- Complete SP1 guest program for Bitcoin SPV inclusion proof
- Deploy verifier contracts on Ethereum (Solidity) and Sui (Move)
- Integrate SP1 proof generation into `BitcoinSealProtocol::build_proof_bundle`
- Define Celestia namespace scheme for CSV proof bundles
- Implement Celestia blob submission in `csv-runtime`
- Implement Celestia light client verification as a `SealProtocol` implementation

**Output:** Bitcoin → Ethereum transfer with ZK SPV proof (no trust assumptions). First Celestia-anchored proof bundle on testnet.

---

### Phase 6 — Repository Split (Month 6+)

Crates should be split into separate repositories when:
1. They have a stable, published API (no breaking changes for 3 months)
2. They have independent release cadences
3. External teams depend on them directly

Recommended split order:
1. `csv-core` → own repo first (protocol types are the most stable)
2. `csv-keys` → own repo (useful independent of the full stack)
3. `csv-{chain}` crates → own repos when chain implementations stabilize
4. `csv-sdk` → own repo when the facade API stabilizes
5. CLI, wallet, explorer stay in a monorepo (they co-evolve)

Do not split before stability. Premature repository fragmentation makes cross-cutting refactoring (like the WASM migration in Phase 3) require coordinated multi-repo PRs.

---

## Part VIII — Scalability & Performance Assessment

### Current State

The `PerformanceMetrics` struct and `ProofCache` exist in `csv-core/src/performance.rs` but are not integrated into any hot path. The `BloomFilter` for seal registry lookups is defined but not used in `SealNullifier` or its optimized variant. There are no load tests, no latency benchmarks, and no throughput targets defined.

### What 100-Chain DeFi Actually Requires

A DeFi application with 100 supported chains and meaningful trading volume will experience:

**Proof verification:** Each incoming cross-chain transfer requires one inclusion proof verification + one finality proof verification + one seal nullifier double-spend check. At 1,000 TPS (modest DeFi volume), that is 3,000 cryptographic operations per second. The sequential `SequentialVerifier` in `performance.rs` will not scale.

**Required:** Parallel proof verification using a thread pool (native) or `FuturesUnordered` (WASM). The nullifier double-spend check must be O(1) — the `BloomFilter` must be integrated.

**RPC fanout:** Polling 100 chains simultaneously with naive per-chain HTTP polling will hit rate limits within minutes. The `RpcManager` in the explorer has circuit breaker support (`CircuitBreaker` in `hardening.rs`) but it is not wired to the actual polling loop.

**Required:** Adaptive polling intervals per chain based on block time. WebSocket subscriptions where supported (Ethereum, Solana). Exponential backoff with jitter on rate limit responses.

**Storage:** SQLite is used for the explorer storage layer. SQLite's write lock prevents concurrent indexer writes across chains. Under a 100-chain workload with parallel indexing, all 100 chains will serialize on the write lock.

**Required:** PostgreSQL for explorer storage when chain count exceeds ~10. The schema is clean and the migration is mechanical, but it must be planned before the chain count scales.

---

## Part IX — Security Architecture Assessment

### What Is Sound

- Keystore reference pattern (wallet never stores plaintext keys, only references)
- Domain-separated commitment hashing (Invariant 7 is implemented correctly)
- `zeroize` dependency is declared for key material
- The `ConsignmentValidator` 5-step pipeline enforces all protocol invariants before state mutation
- `#[cfg(test)]` gating of `MockEthereumRpc` (Ethereum only — see Defect C-6 for others)

### What Is Not Sound

**The `SealNullifier` is not authoritative.** `CrossChainSealRegistry` (renamed `SealNullifier`) is an in-memory structure. If the process restarts, the nullifier set is lost. A cross-chain double-spend attack requires only that the attacker submit the second spend between process restarts. The nullifier must be persisted to the store and reloaded at startup.

**The proof verification error path defaults to non-consumed.** In Defect C-2's context, the `seal_checker` closure returns `false` (seal not consumed) on store errors. "When in doubt, assume not consumed to avoid blocking valid transactions" is explicitly stated in a comment. This is backwards: fail closed, not open. A store error should halt the transfer, not admit it.

**There is no rate limiting on proof submission.** The REST API in `explorer/api/src/rest/handlers.rs` does not show request rate limiting. A DeFi application will be attacked — without rate limiting, proof submission endpoints become free DoS attack surfaces.

**Key derivation uses `OsRng` directly in `WalletData::generate_test_key`.** This function is named "test key" but has no `#[cfg(test)]` gate. It is callable in production contexts.

---

## Part X — Decision Register

These decisions must be made by the team and recorded. The architecture cannot stabilize without them.

| Decision | Options | Recommendation | Urgency |
|---|---|---|---|
| Project name | Keep "CSV", choose a new name | Choose a new name before any public SDK release | High |
| `Chain` enum vs string IDs | Keep closed enum, open with string IDs | String IDs for 100-chain goal | High |
| Runtime process model | Embedded library, separate daemon, or per-surface | Library (`csv-runtime`) composed differently per surface; **resolve before naming `ChainFacade`** | High |
| Bitcoin DeFi strategy | Source chain, anchor-only, or fraud-proof-backed optimistic | Anchor-only initially; fraud proofs in Phase 5 | Medium |
| DA layer | Celestia, EigenDA, Avail, none | Celestia (best Rust client ecosystem) | Medium |
| ZK backend | SP1, RISC Zero, Noir | SP1 (already referenced in code) | Medium |
| Explorer storage for scale | SQLite (current), PostgreSQL | PostgreSQL at >10 chains | Medium |
| Wallet architecture | Keep Dioxus WASM, switch to React | Keep Dioxus after Phase 3 WASM unification | Low |
| Repository split timing | Now, Phase 6+, never | Phase 6+ after stability | Low |

---

## Appendix A — File Deletion List (Dead Code, Phase 0-1)

The following files are candidates for deletion (verify no living callers first):

- `csv-adapter-core/src/interface.rs` — duplicate `SealProtocol` definition; delete after consolidation
- `csv-adapter-core/src/chain_discovery.rs` — discovery stub, nothing calls it; merge into `driver_registry.rs`
- `csv-adapter-core/src/chain_plugin.rs` — merge into `driver_registry.rs`
- `csv-adapter/src/scalable_builder.rs` — duplicate of `builder.rs`
- `csv-explorer/src/` (entire top-level `src/`) — duplicates `api/` and `indexer/`
- `csv-wallet/src/wallet_core.rs` — after Phase 3
- `csv-wallet/src/services/blockchain/` (all 7 files) — after Phase 3
- `csv-wallet/src/services/chain_api.rs` — after Phase 3
- `csv-bitcoin/src/testnet_deploy.rs` — appears to be a one-time script, not library code
- `csv-explorer/.pids` — committed PID file, belongs in `.gitignore`

---

## Appendix B — Immediate Action Items (Backend Team)

Ordered by impact. Weeks 1-2 only.

1. `git grep "AnchorLayer"` — map all uses; merge to single `SealProtocol` definition
2. Replace `std::sync::Mutex` in `facade.rs` `verify_proof_bundle` with `tokio::sync::Mutex`
3. Move `encode_eth_contract_call`, `encode_move_contract_call`, `encode_solana_contract_call` into respective `ChainBackend` implementations
4. Add `get_account_nonce(address: &str)` to `ChainQuery` trait; implement per chain
5. In `seal_checker` closure: change `false` on error to return `Err` (fail closed)
6. Persist `SealNullifier` to store on every mutation; reload at startup
7. Add `#[cfg(test)]` to all mock RPC types not already gated (Bitcoin, Solana, Aptos, Sui)
8. Run `cargo +nightly udeps --all-targets`; remove every unused dependency and symbol
9. Remove `#![allow(dead_code)]` from `csv-store/src/lib.rs`
10. Gate `OsRng` key generation in `WalletData::generate_test_key` to `#[cfg(test)]`

---

## Appendix C — Immediate Action Items (Frontend/Wallet Team)

1. Audit every `reqwest::Client::new()` call in `csv-wallet/src/services/` — these cannot exist in a WASM target; they must be behind `#[cfg(not(target_arch = "wasm32"))]`
2. Audit `csv-wallet/src/wallet_core.rs` — every address derivation function duplicates `csv-keys`; create a tracking issue
3. The wallet's `storage.rs` uses `web_sys::window()?.local_storage()` — correct for browser, but MUST be feature-gated so it compiles for desktop/testing targets
4. The Dioxus version is `0.5` but `dioxus-web` `0.5` requires explicit WASM pack configuration. Verify the WASM build pipeline end-to-end, not just `cargo build`
5. The `TAILWIND_CSS` is `include_str!` embedded — this forces a full Tailwind build before compilation. Ensure CI runs the Tailwind build step before `cargo build`

---

## Quick Reference Card

```
REPOSITORY
  GitHub repo:         csv-adapter        → csv-protocol
  Crate prefix:        csv-adapter-*      → csv-*  (drop "adapter")
  Unified crate:       csv-adapter        → csv-sdk

CRATES
  csv-adapter-core     → csv-core
  csv-adapter-bitcoin  → csv-bitcoin
  csv-adapter-ethereum → csv-ethereum
  csv-adapter-sui      → csv-sui
  csv-adapter-aptos    → csv-aptos
  csv-adapter-solana   → csv-solana
  csv-adapter-store    → csv-store
  csv-adapter-keystore → csv-keys
  csv-adapter          → csv-sdk

THREE TRAIT LAYERS
  ChainAdapter         → ChainDriver         (plugin descriptor, driver.rs)
  AnchorLayer          → SealProtocol        (seal lifecycle, seal_protocol.rs)
  FullChainAdapter     → ChainBackend        (complete impl, backend.rs)

CORE TYPES
  SealRef              → SealPoint           (not a Rust reference)
  AnchorRef            → CommitAnchor        (where a commitment is anchored)
  Right                → Sanad               (a property deed, chain of title)
  RightId              → SanadId
  MpcTree              → CommitMux           (not multi-party computation)
  MpcLeaf              → MuxLeaf
  MpcProof             → MuxProof
  CrossChainSealRegistry → SealNullifier     (ZK standard term)
  AdapterError         → ProtocolError

CHAIN IMPLS
  {Chain}AnchorLayer   → {Chain}SealProtocol
  {Chain}ChainOperations → {Chain}Backend
  {Chain}SealRef       → {Chain}SealPoint
  {Chain}AnchorRef     → {Chain}CommitAnchor

KEY FILES
  traits.rs            → seal_protocol.rs
  chain_adapter.rs     → driver.rs + backend.rs  (split)
  right.rs             → title.rs
  mpc.rs               → commit_mux.rs
  seal_registry.rs     → nullifier.rs
  real_rpc.rs          → node.rs  (each chain)
  adapter_factory.rs   → driver_registry.rs  (merged with chain_plugin + chain_discovery)
  rgb_compat.rs        → rgb.rs
  proof_verify.rs      → verifier.rs
  advanced_commitments.rs → commitments_ext.rs
  agent_types.rs       → mcp.rs

KEEP EXACTLY AS-IS
  Seal, SealRecord, SealStatus        (correct RGB-compatible term)
  Consignment, ConsignmentValidator   (standard CSV literature term)
  ProofBundle                         (precise and clear)
  CommitmentChain                     (precise and clear)
  TapretCommitment, OpretCommitment   (Bitcoin-specific standard terms)
  Genesis, Schema, Transition, Validator
```

---

*This document is intended for engineering team internal use and external architectural review. It should be updated as defects are resolved and decisions are recorded.*
