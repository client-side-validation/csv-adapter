# The Complete Renaming Plan
## CSV Adapter Project — Every Name, Every Level

---

## The Core Diagnosis

The project has three distinct naming problems that feed each other:

**Problem 1 — "Adapter" means nothing.**  
An adapter is a small connector between two incompatible interfaces. This codebase 
is a full protocol SDK with a wallet, explorer, cross-chain proof system, and contract 
schema library. "Adapter" describes none of that. It also appears at three different 
levels with three different meanings: `ChainAdapter` (plugin), `AnchorLayer` (protocol 
trait), `FullChainAdapter` (combined operations). The word does different things each 
time it appears.

**Problem 2 — The three-layer trait hierarchy has no clear names.**  
```
ChainAdapter       ← "basic plugin descriptor" (chain_id, capabilities, create_client)
AnchorLayer        ← "the actual seal protocol" (create_seal, publish, verify_inclusion, enforce_seal)
FullChainAdapter   ← "combined: Query + Signer + Broadcaster + Deployer + ProofProvider + RightOps"
```
These three are architecturally correct and distinct. They just share a word ("adapter" 
or "layer") that reveals nothing about what each one does.

**Problem 3 — "Right" and "SealRef" are ambiguous in their own contexts.**  
`SealRef` clashes with Rust's `&SealRef` reference syntax every time it appears in 
function signatures. `Right` collides with the English word "correct" and the 
directional word, causing readers to re-parse every sentence.

---

## Part 1 — Project and Repository Name

### Current: `csv-adapter`
### Verdict: Wrong on both words

"CSV" to any outsider means comma-separated values. The `client-side-validation` 
GitHub org rescues it — but the repo name cannot assume the org is always visible. 
"Adapter" is documented above.

### Recommendation: Rename repo to `csv-protocol`

```
github.com/client-side-validation/csv-protocol   ← monorepo
github.com/client-side-validation/csv-wallet     ← (after split)
github.com/client-side-validation/csv-explorer   ← (after split)
github.com/client-side-validation/csv-ts         ← TypeScript SDK
github.com/client-side-validation/csv-mcp        ← MCP server
```

`csv-protocol` reads as "the Rust implementation of the CSV protocol" — accurate, 
clear, searchable. The org name `client-side-validation` provides the expansion for 
anyone who needs it.

Keep the GitHub org name exactly as is. `client-side-validation` is precise, unique, 
and googleable. Do not shorten it.

---

## Part 2 — Crate Names

The `csv-adapter-*` naming layers the word "adapter" into every import path. 
Drop it entirely.

| Current crate name | New crate name | Rationale |
|-------------------|---------------|-----------|
| `csv-adapter-core` | `csv-core` | Core protocol primitives. Clean. Mirrors `bitcoin`, `lightning`, `rgb-core`. |
| `csv-adapter-bitcoin` | `csv-bitcoin` | "CSV protocol, Bitcoin backend." Reads correctly. |
| `csv-adapter-ethereum` | `csv-ethereum` | Same pattern. |
| `csv-adapter-sui` | `csv-sui` | Same. |
| `csv-adapter-aptos` | `csv-aptos` | Same. |
| `csv-adapter-solana` | `csv-solana` | Same. |
| `csv-adapter-store` | `csv-store` | Storage layer. |
| `csv-adapter-keystore` | `csv-keys` | Key management. `keystore` as a word is fine but `csv-keys` is shorter and equally clear. |
| `csv-adapter` | `csv-sdk` | The unified runtime crate. "SDK" correctly signals this is what integrators import. |
| `csv-cli` | `csv-cli` | Already correct. |
| `csv-wallet` | `csv-wallet` | Already correct. |
| `csv-explorer` | `csv-explorer` | Already correct. |
| *(future)* | `csv-schemas` | Contract schema library. |

**`Cargo.toml` workspace package name:**
```toml
[workspace.package]
# was: name not set explicitly, version = "0.4.0"
# all crates get their own name field with the new names above
```

**Crate rename note:** `cargo` crate names use hyphens, Rust import paths use underscores. 
`csv-core` → `use csv_core::...`. This is already the pattern in your codebase.

---

## Part 3 — The Three Trait Layers (Most Important)

This is the architectural heart. Three traits, three precise new names.

### Layer 1: `ChainAdapter` → `ChainDriver`

**Location:** `csv-adapter-core/src/chain_adapter.rs`  
**What it does:** Plugin descriptor for a chain. Provides `chain_id()`, 
`chain_name()`, `capabilities()`, `create_client()`, `create_wallet()`. 
This is how a chain registers itself into the system.

**Why "Driver"?** A device driver is the minimal interface that allows an OS to use 
a piece of hardware. `ChainDriver` is exactly that — the minimal interface that allows 
the protocol to use a blockchain. It does not do protocol operations; it describes the 
chain and creates the tools to interact with it.

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

### Layer 2: `AnchorLayer` → `SealProtocol`

**Location:** `csv-adapter-core/src/traits.rs`  
**What it does:** THE core protocol trait. Defines:
- `create_seal()` — open a new single-use seal on-chain
- `publish()` — anchor a commitment to a seal
- `verify_inclusion()` — prove commitment is in a block
- `verify_finality()` — prove block is final
- `enforce_seal()` — consume/close the seal

**Why "SealProtocol"?** This trait IS the single-use seal protocol. Every method 
is a step in the seal lifecycle. "Anchor" in the current name focuses on where 
commitments land; "SealProtocol" names what the trait is responsible for managing: 
the full lifecycle of a seal.

The implementations become:
```rust
// Before
struct EthereumAnchorLayer
struct BitcoinAnchorLayer
struct SuiAnchorLayer

// After
struct EthereumSealProtocol   // or just: Ethereum (within csv-ethereum crate)
struct BitcoinSealProtocol
struct SuiSealProtocol
```

**File rename:** `traits.rs` → `seal_protocol.rs`

The associated types get clearer names too:
```rust
pub trait SealProtocol {
    type SealPoint: Debug + Clone + Eq;       // was: SealRef
    type AnchorPoint: Debug + Clone + Eq;     // was: AnchorRef  
    type InclusionProof: Debug + Clone;       // unchanged — already clear
    type FinalityProof: Debug + Clone;        // unchanged
}
```

---

### Layer 3: `FullChainAdapter` → `ChainBackend`

**Location:** `csv-adapter-core/src/chain_adapter.rs` (currently shares file with Layer 1)  
**What it does:** The complete chain implementation combining:
- `ChainQuery` — read chain state
- `ChainSigner` — sign transactions
- `ChainBroadcaster` — submit transactions
- `ChainDeployer` — deploy contracts
- `ChainProofProvider` — build proof bundles
- `ChainRightOps` — operate on Rights

**Why "Backend"?** A backend is a full, complete implementation. "Full" in 
`FullChainAdapter` already signals completeness; "Backend" names what it is: 
the complete chain-side implementation that the protocol runtime talks to.

```rust
// Before
pub trait FullChainAdapter: ChainQuery + ChainSigner + ChainBroadcaster + 
    ChainDeployer + ChainProofProvider + ChainRightOps { ... }

// After
pub trait ChainBackend: ChainQuery + ChainSigner + ChainBroadcaster +
    ChainDeployer + ChainProofProvider + ChainTitleOps { ... }
// (ChainRightOps → ChainTitleOps because Right → Title, see Part 4)
```

**Separate files:** Split `chain_adapter.rs` into:
- `driver.rs` — `ChainDriver` (Layer 1)
- `backend.rs` — `ChainBackend` + all sub-traits (Layer 3)

These two concepts share nothing and should not share a file.

---

### Summary of Three Layers

```
Before                     After                   File
──────────────────────────────────────────────────────────────
ChainAdapter               ChainDriver             driver.rs
AnchorLayer                SealProtocol            seal_protocol.rs
FullChainAdapter           ChainBackend            backend.rs
```

The chain implementations become:

```
Before                          After
────────────────────────────────────────────────────────────
BitcoinAnchorLayer              BitcoinSealProtocol
BitcoinChainOperations          BitcoinBackend
EthereumAnchorLayer             EthereumSealProtocol
EthereumChainOperations         EthereumBackend
SuiAnchorLayer                  SuiSealProtocol
SuiChainOperations              SuiBackend
AptosAnchorLayer                AptosSealProtocol
AptosChainOperations            AptosBackend
SolanaAnchorLayer               SolanaSealProtocol
SolanaChainOperations           SolanaBackend
```

---

## Part 4 — Core Type Names

### `SealRef` → `SealPoint`

**Problem:** `SealRef` clashes with Rust reference syntax. `&SealRef` in a 
function signature is genuinely ambiguous for a millisecond — "is this a 
reference to something, or the SealRef type?" Also, it's not "a reference to 
a seal" — it IS the seal identifier itself.

**Why "SealPoint"?** Bitcoin uses `OutPoint` (txid + vout) to identify a specific 
output. A Bitcoin seal IS an OutPoint. `SealPoint` generalizes this: a specific 
point on any chain that acts as a seal. It's precise, has blockchain precedent, 
and does not clash with Rust syntax.

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
// Before                        // After
BitcoinSealRef                   BitcoinSealPoint
AptosSealRef                     AptosSealPoint
EthereumSealRef                  (uses SealPoint directly — nullifier hash)
```

The fuzz target `fuzz_seal_ref_from_bytes.rs` → `fuzz_seal_point_from_bytes.rs`

---

### `AnchorRef` → `CommitAnchor`

**Problem:** `AnchorRef` suffers the same ref-confusion as `SealRef`. Also 
ambiguous with the proposed rename of `AnchorLayer`.

**Why "CommitAnchor"?** It's where a commitment was anchored on-chain. 
"CommitAnchor" reads as a noun: "the anchor for a commitment."

```rust
// Before
pub struct AnchorRef { ... }
pub struct BitcoinAnchorRef { ... }
pub struct AptosAnchorRef { ... }

// After
pub struct CommitAnchor { ... }
pub struct BitcoinCommitAnchor { ... }
pub struct AptosCommitAnchor { ... }
```

---

### `Right` → `Title`

**The question:** Is "Right" a good name?

It is legally accurate (a "right" in property law is exclusive and transferable), 
short, and maps to the concept. But it has two practical problems:

1. Collides with English: "turn right", "that's right", "right-click" — every 
   developer mentally re-parses "right" the first time they encounter it in code.
2. In your own codebase, `OwnedState` already exists in `state.rs` and represents 
   raw owned state. The relationship between `Right` and `OwnedState` is unclear 
   when names look unrelated.

**Why "Title"?** A property title is:
- A legal document proving ownership ✓
- Exclusive — one title per property ✓  
- Transferable — deeds change hands ✓
- The record of provenance — chain of title ✓

"Chain of title" even has a legal meaning matching your commitment chain exactly.

```rust
// Before
pub struct Right { ... }
pub type RightId = Hash;

// After
pub struct Title { ... }
pub type TitleId = Hash;
```

Supporting type renames:
```rust
// Before                       After
RightOperationResult            TitleOperationResult
ChainRightOps                   ChainTitleOps
RightId                         TitleId
fuzz_right_from_canonical_bytes fuzz_title_from_canonical_bytes
basic_right.rs (example)        basic_title.rs
```

**File:** `csv-adapter-core/src/right.rs` → `csv-adapter-core/src/title.rs`

---

### `MpcTree` → `CommitMux`

**Problem:** "MPC" stands for Multi-Protocol Commitment (the doc comment says so). 
But every developer will read "MPC" as Multi-Party Computation — a completely 
different cryptographic concept. The confusion is not hypothetical; it will mislead 
every new contributor.

**Why "CommitMux"?** A multiplexer (mux) combines multiple signals into one output. 
`CommitMux` combines multiple protocol commitments into one on-chain output. 
"Mux" has technical precision without collision with MPC terminology.

```rust
// Before                    After
MpcTree                      CommitMux
MpcLeaf                      MuxLeaf
MpcProof                     MuxProof
MerkleBranchNode             MuxBranchNode  (or keep MerkleBranchNode — it's clear)
```

**File:** `csv-adapter-core/src/mpc.rs` → `csv-adapter-core/src/commit_mux.rs`

---

### `CrossChainSealRegistry` → `SealNullifier`

The name "registry" suggests a lookup table. What this actually does is enforce 
that a seal cannot be consumed twice. That's a nullifier — the ZK term for "a 
value that, once revealed, can never be used again." Using the standard term 
aligns with Phase 5 ZK work where actual ZK nullifiers will be used.

```rust
// Before
pub struct CrossChainSealRegistry { ... }

// After
pub struct SealNullifier { ... }
```

**File:** `csv-adapter-core/src/seal_registry.rs` → `csv-adapter-core/src/nullifier.rs`

---

### `ChainFacade` → `CsvRuntime`

**Location:** `csv-adapter/src/facade.rs`  
This is the top-level orchestrator: routes all protocol operations to the correct 
chain backend, manages registered backends, provides the unified API that CLI, 
wallet, and explorer use.

"Facade" is a GoF design pattern name — not a domain concept. "Runtime" is precise: 
it is the runtime environment that manages chain connections and executes protocol 
operations.

```rust
// Before
pub struct ChainFacade { ... }

// After
pub struct CsvRuntime { ... }
```

**File:** `csv-adapter/src/facade.rs` → `csv-sdk/src/runtime.rs`  
(after the crate rename from `csv-adapter` to `csv-sdk`)

---

### `RealRpc` → `ChainNode`

In each chain adapter there are two files:
- `rpc.rs` — the trait defining what RPC calls are available
- `real_rpc.rs` — the actual HTTP/WS client implementation

"Real" implies there is also a "fake" (there is: the mock in `adapters/test.rs`). 
But naming the production struct "Real" is odd in production code.

**Better:** The struct connecting to an actual chain node IS a node connection.

```rust
// Before (in csv-adapter-bitcoin/src/)
real_rpc.rs  →  contains: BitcoinRealRpc

// After
node.rs      →  contains: BitcoinNode
```

Same pattern for all five chains:
```
csv-bitcoin/src/real_rpc.rs    → csv-bitcoin/src/node.rs     (BitcoinNode)
csv-ethereum/src/real_rpc.rs   → csv-ethereum/src/node.rs    (EthereumNode)
csv-sui/src/...                → csv-sui/src/node.rs          (SuiNode)
csv-aptos/src/real_rpc.rs      → csv-aptos/src/node.rs       (AptosNode)
csv-solana/src/real_rpc.rs     → csv-solana/src/node.rs      (SolanaNode)
```

---

### `AdapterFactory` → `DriverRegistry`

**Location:** `csv-adapter-core/src/adapter_factory.rs`  
**What it does:** Registers and instantiates chain drivers by chain ID.

Since `ChainAdapter` → `ChainDriver`, the factory that creates them is a registry 
of drivers. Three files in core currently overlap in responsibility:
- `adapter_factory.rs` — creates drivers
- `chain_plugin.rs` — plugin registry
- `chain_discovery.rs` — discovers available chains

Merge all three into `driver_registry.rs`:

```rust
// Before: three files
adapter_factory.rs     → DriverRegistry::register(), DriverRegistry::get()
chain_plugin.rs        → ChainPluginRegistry, ChainPluginMetadata
chain_discovery.rs     → discovery logic

// After: one file
driver_registry.rs     → DriverRegistry (merged), DriverMetadata (was ChainPluginMetadata)
```

---

## Part 5 — File Renames (Complete Map)

### `csv-adapter-core/` → `csv-core/`

| Current path | New path | Reason |
|---|---|---|
| `src/traits.rs` | `src/seal_protocol.rs` | Contains SealProtocol trait |
| `src/chain_adapter.rs` | `src/driver.rs` | ChainDriver (Layer 1 only) |
| `src/right.rs` | `src/title.rs` | Right → Title |
| `src/seal_registry.rs` | `src/nullifier.rs` | SealNullifier |
| `src/mpc.rs` | `src/commit_mux.rs` | CommitMux |
| `src/adapter_factory.rs` | `src/driver_registry.rs` | Merged with chain_plugin + chain_discovery |
| `src/chain_plugin.rs` | *(merged into driver_registry.rs)* | Redundant split |
| `src/chain_discovery.rs` | *(merged into driver_registry.rs)* | Redundant split |
| `src/rgb_compat.rs` | `src/rgb.rs` | Shorter, still clear |
| `src/advanced_commitments.rs` | `src/commitments_ext.rs` | "advanced" is not a domain concept |
| `src/adapters/mod.rs` | `src/drivers/mod.rs` | Folder rename matches trait rename |
| `src/adapters/test.rs` | `src/drivers/mock.rs` | "mock" is the standard term for test doubles |
| `src/proof_verify.rs` | `src/verifier.rs` | The module is the verifier, not the action |
| `src/agent_types.rs` | `src/mcp.rs` | AI agent types belong to MCP context |
| `examples/basic_right.rs` | `examples/basic_title.rs` | Follows Title rename |
| `fuzz/fuzz_targets/fuzz_right_from_canonical_bytes.rs` | `fuzz/fuzz_targets/fuzz_title.rs` | Shorter |
| `fuzz/fuzz_targets/fuzz_seal_ref_from_bytes.rs` | `fuzz/fuzz_targets/fuzz_seal_point.rs` | Follows SealPoint rename |

Files to keep exactly as-is (names are correct):
```
src/commitment.rs
src/commitment_chain.rs
src/consignment.rs
src/cross_chain.rs
src/dag.rs
src/error.rs
src/events.rs
src/genesis.rs
src/hardening.rs
src/hash.rs
src/interface.rs
src/monitor.rs
src/performance.rs
src/proof.rs
src/protocol_version.rs
src/schema.rs
src/seal.rs
src/signature.rs
src/state.rs
src/state_store.rs
src/store.rs
src/tagged_hash.rs
src/tapret_verify.rs
src/transition.rs
src/validator.rs
src/vm/          (entire directory)
src/zk_proof.rs
```

### Chain Adapter Crates (all five — same pattern)

Each `csv-adapter-{chain}/src/` gets:

| Current file | New file | Reason |
|---|---|---|
| `adapter.rs` | `seal_protocol.rs` | Contains {Chain}SealProtocol impl |
| `chain_adapter_impl.rs` | `backend.rs` | Contains {Chain}Backend impl |
| `chain_operations.rs` | `ops.rs` | ChainOperations is verbose; it IS ops |
| `real_rpc.rs` | `node.rs` | ChainNode — connection to a real node |
| `rpc.rs` | `rpc.rs` | Keep — it IS the RPC trait definition |
| `seal.rs` | `seal.rs` | Keep — seal-specific logic |
| `proofs.rs` | `proofs.rs` | Keep |
| `signatures.rs` | `signatures.rs` | Keep |
| `types.rs` | `types.rs` | Keep |
| `config.rs` | `config.rs` | Keep |
| `error.rs` | `error.rs` | Keep |
| `deploy.rs` | `deploy.rs` | Keep |

Bitcoin-specific files (keep all, names are good):
```
bip341.rs, spv.rs, tapret.rs, tx_builder.rs, 
mempool_rpc.rs, wallet.rs, testnet_deploy.rs
```

Ethereum-specific (keep):
```
seal_contract.rs, finality.rs, mpt.rs
```

Aptos-specific (keep):
```
checkpoint.rs, merkle.rs
```

### `csv-adapter/` → `csv-sdk/`

| Current | New | Reason |
|---|---|---|
| `src/facade.rs` | `src/runtime.rs` | CsvRuntime |
| `src/client.rs` | `src/client.rs` | Keep — CsvClient is fine |
| `src/errors.rs` | `src/error.rs` | Singular convention (matches all other crates) |

### `csv-adapter-keystore/` → `csv-keys/`

| Current | New |
|---|---|
| `src/keystore.rs` | `src/keystore.rs` — Keep |
| `src/browser_keystore.rs` | `src/browser.rs` — shorter |
| `src/bip39.rs` | Keep |
| `src/bip44.rs` | Keep |
| `src/memory.rs` | Keep |

### `csv-wallet/src/`

| Current | New | Reason |
|---|---|---|
| `wallet_core.rs` | *(deleted — merged into wallet/ module)* | Phase 0.3 consolidation |
| `seals/manager.rs` | `seals/registry.rs` | It's the local seal registry |
| `pages/rights/` | `pages/titles/` | Follows Title rename |
| `pages/rights/journey.rs` | `pages/titles/provenance.rs` | Better: shows provenance |
| `pages/rights/consume.rs` | `pages/titles/consume.rs` | Keep verb |
| `pages/rights/transfer.rs` | `pages/titles/transfer.rs` | Keep verb |
| `services/blockchain/` | `services/chain/` | "blockchain" is verbose for a folder |
| `context/state.rs` | `context/state.rs` | Keep — AppState is fine |
| `components/seal_status.rs` | `components/seal_status.rs` | Keep |
| `components/seal_visualizer.rs` | `components/seal_view.rs` | Shorter |
| `components/proof_inspector.rs` | `components/proof_view.rs` | Consistent `-view` suffix |
| `hooks/use_assets.rs` | `hooks/use_titles.rs` | Follows Title rename |
| `assets/` (whole folder) | `titles/` | Follows Title rename |

---

## Part 6 — Struct and Enum Names Inside Files

These do not require file renames but require find-replace across the codebase:

### In `csv-adapter-core/`

| Before | After | File |
|---|---|---|
| `SealRef` | `SealPoint` | seal.rs, all consumers |
| `AnchorRef` | `CommitAnchor` | commitment.rs, all consumers |
| `Right` | `Title` | title.rs (renamed from right.rs) |
| `RightId` | `TitleId` | title.rs |
| `AnchorLayer` | `SealProtocol` | seal_protocol.rs |
| `FullChainAdapter` | `ChainBackend` | backend.rs |
| `ChainAdapter` | `ChainDriver` | driver.rs |
| `ChainAdapterExt` | `ChainDriverExt` | driver.rs |
| `MpcTree` | `CommitMux` | commit_mux.rs |
| `MpcLeaf` | `MuxLeaf` | commit_mux.rs |
| `MpcProof` | `MuxProof` | commit_mux.rs |
| `CrossChainSealRegistry` | `SealNullifier` | nullifier.rs |
| `AdapterError` | `ProtocolError` | error.rs — "Adapter" gone from error names too |
| `ChainRightOps` | `ChainTitleOps` | backend.rs |
| `RightOperationResult` | `TitleOperationResult` | backend.rs |
| `ChainPluginMetadata` | `DriverMetadata` | driver_registry.rs |
| `ChainPluginRegistry` | `DriverRegistry` | driver_registry.rs |
| `ChainVerificationResult` | `VerificationResult` | (drop "Chain" prefix — context is clear) |

### Chain-specific structs

| Before | After |
|---|---|
| `BitcoinAnchorLayer` | `BitcoinSealProtocol` |
| `BitcoinChainOperations` | `BitcoinBackend` |
| `BitcoinSealRef` | `BitcoinSealPoint` |
| `BitcoinAnchorRef` | `BitcoinCommitAnchor` |
| `EthereumAnchorLayer` | `EthereumSealProtocol` |
| `EthereumChainOperations` | `EthereumBackend` |
| `SuiAnchorLayer` | `SuiSealProtocol` |
| `SuiChainOperations` | `SuiBackend` |
| `AptosAnchorLayer` | `AptosSealProtocol` |
| `AptosChainOperations` | `AptosBackend` |
| `AptosAnchorRef` | `AptosCommitAnchor` |
| `AptosSealRef` | `AptosSealPoint` |
| `SolanaAnchorLayer` | `SolanaSealProtocol` |
| `SolanaChainOperations` | `SolanaBackend` |

### In `csv-adapter/` (becomes `csv-sdk/`)

| Before | After |
|---|---|
| `ChainFacade` | `CsvRuntime` |
| `CsvError` | `SdkError` |

### In `csv-wallet/`

| Before | After |
|---|---|
| `use_assets` hook | `use_titles` |
| `AssetService` | `TitleService` |
| All `right` references in AppState | `title` |

---

## Part 7 — What "Seal" and "Consignment" Should Stay As

### `Seal` — Keep exactly as-is

"Seal" is used by RGB. It is the correct term from the single-use seal literature 
(Peter Todd's original construction). Any deviation from this term:
1. Breaks alignment with the RGB ecosystem you want interoperability with
2. Loses the theoretical precision — a "seal" in this context has a specific 
   mathematical meaning that differentiates it from a "lock" or "commitment"

The word "seal" is fine. `seal.rs`, `SealStatus`, `SealRecord` — all keep "Seal".  
Only `SealRef` changes (to `SealPoint`) because "Ref" is the broken part, not "Seal".

### `Consignment` — Keep exactly as-is

RGB uses "consignment" for the package of data shipped from sender to receiver 
containing the proof history. The term is:
- Standard in the CSV/RGB literature
- Precise: a consignment contains goods (proof data) for delivery
- Self-describing: you consign the proof to the receiver for independent verification

`consignment.rs`, `Consignment`, `ConsignmentError`, `ConsignmentValidator` — all keep these names.

### `ProofBundle` — Keep

"ProofBundle" is specific, accurate, and not confusing. The bundle of proofs. 
Alternative `Certificate` would lose the word "proof" which matters.

### `CommitmentChain` — Keep

Technically precise. A chain (hash-linked list) of commitments. Keep.

### `TapretCommitment`, `OpretCommitment` — Keep

These are Bitcoin-specific terms (Tapret = taproot-embedded, Opret = OP_RETURN-embedded). 
They are standard in the RGB/CSV Bitcoin literature.

---

## Part 8 — Naming Conventions Going Forward

Establish these rules in `CONTRIBUTING.md` to prevent drift:

**Rule 1: No protocol terms as prefixes in type names**  
`ChainAdapter` → `ChainDriver` (drop "Adapter")  
`AnchorLayer` → `SealProtocol` (name the concept, not the pattern)  
`RealRpc` → `ChainNode` (name the thing, not that it's "real")

**Rule 2: No "Ref" suffix on non-reference types**  
Rust uses `Ref` for borrowed smart pointers. Domain types that happen to 
identify something use `Point`, `Id`, `Anchor`, `Locator`.

**Rule 3: Error types use `Error` suffix, not `Errors`**  
`errors.rs` → `error.rs`. `CsvErrors` → `SdkError`. Singular.

**Rule 4: File name = primary type name**  
`seal_protocol.rs` contains `SealProtocol`. `title.rs` contains `Title`.  
If a file contains multiple types with no primary, it becomes `mod.rs` or a 
descriptive noun: `types.rs`, `ops.rs`.

**Rule 5: "Backend" for full implementations, "Driver" for descriptors**  
`ChainBackend` = full implementation. `ChainDriver` = minimal plugin descriptor.  
Never use "Adapter", "Facade", "Factory" in domain type names again.

---

## Part 9 — Migration Execution Order

Do NOT rename everything at once. Rename in this order to keep CI green at each step:

```
Step 1 — Crate renames (Cargo.toml only, no code changes)
  csv-adapter-core   → csv-core
  csv-adapter        → csv-sdk  
  csv-adapter-store  → csv-store
  csv-adapter-keystore → csv-keys
  All five chain crates: csv-adapter-{chain} → csv-{chain}
  Update all path deps in workspace Cargo.toml
  CI must pass after this step alone.

Step 2 — SealRef → SealPoint (highest impact, many call sites)
  Global find-replace: SealRef → SealPoint, seal_id → id (within SealPoint only)
  Fuzz target rename
  CI must pass.

Step 3 — AnchorRef → CommitAnchor
  Global find-replace. Fewer call sites than SealRef.
  CI must pass.

Step 4 — Trait renames (most impactful architecturally)
  AnchorLayer → SealProtocol
  FullChainAdapter → ChainBackend  
  ChainAdapter → ChainDriver
  All impl blocks across 5 chain crates
  File renames: traits.rs → seal_protocol.rs, chain_adapter.rs → driver.rs + backend.rs
  CI must pass.

Step 5 — Right → Title
  right.rs → title.rs
  Global find-replace: Right → Title, RightId → TitleId
  Wallet pages/rights/ → pages/titles/
  Wallet hooks/use_assets → use_titles
  CI must pass.

Step 6 — MpcTree → CommitMux
  mpc.rs → commit_mux.rs
  Global find-replace within file and consumers
  CI must pass.

Step 7 — CrossChainSealRegistry → SealNullifier  
  seal_registry.rs → nullifier.rs
  CI must pass.

Step 8 — ChainFacade → CsvRuntime
  facade.rs → runtime.rs (within csv-sdk)
  CI must pass.

Step 9 — Remaining file renames (lower impact)
  adapter_factory.rs → driver_registry.rs (merged)
  chain_plugin.rs → (deleted, merged)
  chain_discovery.rs → (deleted, merged)
  real_rpc.rs → node.rs (across all 5 chain crates)
  rgb_compat.rs → rgb.rs
  proof_verify.rs → verifier.rs
  CI must pass.

Step 10 — Repository rename
  GitHub: csv-adapter → csv-protocol
  Update CODEBASE_OWNERS.md, README, all docs
  Update docs.rs links if published
```

Each step is a single PR. No feature changes in any step. CI is the gate.

---

## Summary Reference Card

```
PROJECT
  GitHub repo:        csv-adapter       → csv-protocol
  Crate prefix:       csv-adapter-*     → csv-*  (drop "adapter")
  Unified crate:      csv-adapter       → csv-sdk

CRATES
  csv-adapter-core    → csv-core
  csv-adapter-bitcoin → csv-bitcoin
  csv-adapter-ethereum→ csv-ethereum
  csv-adapter-sui     → csv-sui
  csv-adapter-aptos   → csv-aptos
  csv-adapter-solana  → csv-solana
  csv-adapter-store   → csv-store
  csv-adapter-keystore→ csv-keys
  csv-adapter         → csv-sdk

THREE TRAIT LAYERS
  ChainAdapter        → ChainDriver         (plugin descriptor)
  AnchorLayer         → SealProtocol        (seal lifecycle protocol)
  FullChainAdapter    → ChainBackend        (complete implementation)

TYPES
  SealRef             → SealPoint           (not a Rust reference)
  AnchorRef           → CommitAnchor        (where a commitment is anchored)
  Right               → Title               (a property title, chain of title)
  RightId             → TitleId
  MpcTree             → CommitMux           (not multi-party computation)
  MpcLeaf             → MuxLeaf
  MpcProof            → MuxProof
  CrossChainSealRegistry → SealNullifier    (ZK standard term)
  ChainFacade         → CsvRuntime
  AdapterError        → ProtocolError

CHAIN IMPLS  
  {Chain}AnchorLayer  → {Chain}SealProtocol
  {Chain}ChainOperations → {Chain}Backend
  {Chain}SealRef      → {Chain}SealPoint
  {Chain}AnchorRef    → {Chain}CommitAnchor

KEY FILES
  traits.rs           → seal_protocol.rs
  chain_adapter.rs    → driver.rs + backend.rs (split)
  right.rs            → title.rs
  mpc.rs              → commit_mux.rs
  seal_registry.rs    → nullifier.rs
  facade.rs           → runtime.rs
  real_rpc.rs         → node.rs (each chain)
  adapter_factory.rs  → driver_registry.rs (merged with chain_plugin + chain_discovery)
  rgb_compat.rs       → rgb.rs
  proof_verify.rs     → verifier.rs

KEEP EXACTLY AS-IS
  Seal, SealRecord, SealStatus     (correct RGB-compatible term)
  Consignment, ConsignmentValidator (standard CSV literature term)
  ProofBundle                       (precise and clear)
  CommitmentChain                   (precise and clear)
  TapretCommitment, OpretCommitment (Bitcoin-specific standard terms)
  Genesis                           (universal protocol term)
  Schema, Transition, Validator     (clear domain terms)
```
