# AGENT.md — AI Agent Contribution Guide for CSV Protocol

## 1. Project Overview

CSV (Client-Side Validation) Protocol is a **cross-chain asset portability system** that uses client‑verified proofs instead of trusted bridges. A “Sanad” (digital deed) moves between chains by consuming a single‑use seal on the source chain and providing a self‑contained proof bundle for the destination chain. Verification happens **offline** (client‑side) – no validator set, no bridge, no external RPC required.

**Key concepts:**

- **Seal – chain‑enforced single‑use primitive** (Bitcoin UTXO, Sui object, Ethereum nullifier slot, Aptos resource, Solana PDA).
- **Commitment – off‑chain binding** that links seal consumption to a state transition.
- **Proof Bundle – inclusion + finality proofs** proving that a seal was consumed on the source chain.
- **Cross‑chain Transfer – lock, prove, verify, mint** pipeline with no trusted intermediary.

**Protocol Version:** v0.4.0  
**License:** MIT OR Apache‑2.0  
**Maintaining Organization:** Client‑Side Validation Foundation  

## 2. Getting Started

### Prerequisites

- Rust 1.92+ (edition 2024)
- System dependencies (for SQLite, etc.)
- For Solidity contracts: Foundry/forge
- For Move chains: Sui/Aptos CLI tools

### Build

```bash
# Build all workspace crates
cargo build --workspace
```

Key crate groups:

- `csv-core` – universal types, seal traits, commitment, proofs, DAG, state machine
- `csv-bitcoin` / `csv-ethereum` / `csv-sui` / `csv-aptos` / `csv-solana` – chain adapters
- `csv-sdk` – unified public SDK
- `csv-cli` – command line tool (wallet, proofs, transfers)
- `csv-wallet` – browser/desktop wallet UI (Dioxus)
- `csv-explorer` – multi‑chain indexer + GraphQL API + UI
- `csv-contracts` – on‑chain smart contracts (Solidity, Move, Anchor)
- `csv-celestia` – data availability adapter
- `csv-p2p` – Nostr‑based proof transport
- `csv-mcp-server` – AI agent integration (MCP)

### Testing

```bash
cargo test --workspace
cargo test -p csv-core    # core protocol tests
cargo test -p csv-bitcoin # bitcoin adapter tests
```

### Linting & Formatting

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets
```

## 3. Repository Structure

```
├── Cargo.toml                  # workspace root
├── csv-core/                   # 🔒 central protocol types & traits
│   ├── src/
│   │   ├── seal.rs             # SealPoint, CommitAnchor
│   │   ├── commitment.rs       # MPC‑aware commitment
│   │   ├── proof.rs            # ProofBundle, InclusionProof, FinalityProof
│   │   ├── seal_protocol.rs    # SealProtocol trait (the primary interface for chain adapters)
│   │   ├── verifier.rs         # verify_proof pipeline
│   │   ├── nullifier.rs        # cross‑chain seal registry
│   │   ├── transfer.rs         # state machine
│   │   ├── tagged_hash.rs      # domain‑separated hashing
│   │   ├── PROTOCOL_INVARIANTS.md  # non‑negotiable security rules
│   │   └── ...
├── csv-bitcoin/                # BTC adapter (UTXO seals, Taproot commitments)
├── csv-ethereum/               # ETH adapter (nullifier slots, CSVLock/CSVMint contracts)
├── csv-sui/                    # Sui adapter (object deletion)
├── csv-aptos/                  # Aptos adapter (resource destruction)
├── csv-solana/                 # Solana adapter (PDA closure)
├── csv-sdk/                    # unified SDK (builder, managers, runtime)
├── csv-cli/                    # CLI tool
├── csv-wallet/                 # Dioxus UI wallet
├── csv-explorer/               # Explorer (indexer + GraphQL + UI)
├── csv-contracts/              # smart contracts (Solidity, Anchor, Move)
├── csv-celestia/               # Celestia DA layer
├── csv-p2p/                    # Nostr proof transport
├── csv-mcp-server/             # MCP server for AI agents
├── csv-store/                  # persistence (SQLite, browser storage)
├── csv-keys/                   # keystore (BIP-39, BIP-44, AES encryption)
├── csv-stark/                  # STARK batch verification (IoT sensor streams)
└── docs/                       # architecture, audits, masterplan
```

## 4. Coding Conventions

### General

- **Rust edition 2042** with `#![warn(missing_docs)]`.
- Use `thiserror` for error types, implement `csv_core::mcp::HasErrorSuggestion` for agent‑friendly suggestions.
- Serialize with `serde`, BCS for Move chains, RLP for Ethereum.
- **Async runtime:** Tokio (multi‑thread) for all server‑side; synchronised wrappers for WASM.
- **No unsafe code in runtime paths** (see Rule G‑05).
- **Domain separation:** all hashing MUST go through `csv_core::tagged_hash` or a proper Domain trait. Never use bare `sha2::Sha256`, `sha3::Keccak256` directly in protocol logic.

### Traits & Adapters

- Each chain adapter implements `SealProtocol` (in its `seal_protocol.rs`) and a `ChainBackend` (in `ops.rs` / `backend.rs`).
- Use the unified `csv_sdk::ChainRuntime` for all chain operations; do not call adapter‑specific methods directly.
- Adapter creation is done via `AdapterBuilder` in `csv-sdk/src/runtime.rs`.

### State Machine

- Use **typestate transitions** for transfer lifecycle. Never mutate enum variants directly. The new typestate API is pending, but any new code should follow that pattern.
- States: `Locked` → `AwaitingFinality` → `ProofReady` → `Minting` → `Complete`.

## 5. Security Invariants (from PROTOCOL_INVARIANTS.md)

These are **non‑negotiable**. Any code change must respect these:

1. **Seal IDs Must Come From Real Blockchain Transactions** – never fabricate seal IDs from timestamps or random bytes.
2. **Commitments Must Be Published On‑Chain Before Proof Building** – never build a `ProofBundle` without a real `CommitAnchor`.
3. **Sanads Must Pass ConsignmentValidator Before Entering AppState** – all 5 validation steps required.
4. **Balances Are Stored as u64 Native Units** – no floating point for monetary amounts.
5. **Cross‑Chain Transfers Must Follow the TransferState Machine** – no skipping stages.
6. **SealRegistry Must Be Checked Before Accepting Any Transfer** – double‑spend protection mandatory.
7. **Domain Separation Must Be Used for All Hashes** – prevent cross‑chain replay.

**Audit checklist in PR template must reference these.**

## 6. Architecture Overview

### Seal → Anchor → Proof

```
Create Seal (on chain) → Publish Commitment → [Chain enforces single‑use]
                                                ↓
                       Inclusion Proof (tx in block)
                                                +
                       Finality Proof (enough confirmations)
                                                ↓
                                          Proof Bundle
```

### Cross‑chain Transfer

1. **Lock** Sanad on source chain (seal consumed, event emitted)
2. **Prove** – generate inclusion + finality proofs
3. **Verify** – destination client validates proofs offline
4. **Mint** new Sanad on destination chain

See `csv-core/src/cross_chain.rs` and `csv-sdk/src/transfers.rs`.

### Domain Separation

All hashing is tagged with `csv_tagged_hash("urn:lnp-bp:csv:" || name, data)`. The replacement with a proper `Domain` trait is upcoming; until then, use `csv_core::tagged_hash::csv_tagged_hash`.

## 7. Testing & Quality

- **Unit tests** in each crate.
- **Property tests** using `proptest` for invariants (seal replay, state transitions, serialization roundtrips).
- **Fuzz targets** for `ProofBundle::decode`, RPC parsers, ABI decoders.
- **Integration tests** for cross‑chain flows in `csv-cli` (some already exist).
- **CI** must enforce: no `TODO`/`FIXME` in security modules, fuzz corpus must pass, property tests must succeed, no warnings.

## 8. Agent Guidelines

When contributing, AI agents MUST obey these rules (from the Principal Engineer execution plan): tasks to be done completely, not to be postponed, Implementation should be audit level, not simplified methods and algorithms, not leave stub, not leave placeholders, not partially implementations, full and complete for all related and applicable places in repo, do not evade complex tasks.

- **No partial validation:** if you can't complete a security verification, return an error; never downgrade to a warning.
- **No silent fallbacks:** never substitute a default RPC provider or fallback crypto without explicit logging and opting in.
- **Unskippable security APIs:** embed verification inside the protocol call; don't rely on the caller to validate.
- **No raw hashing:** use domain‑separated hash functions exclusively.
- **No unsafe constructors in runtime paths:** `new_unchecked` and similar are forbidden outside tests.
- **Determinism:** all operations must be deterministic given the same inputs.
- **Explicit error handling:** use `CsvError` with chain and message context; do not panic.
- **Feature‑gated optionality:** internal experiments must be behind `#[cfg(feature = "experimental")]`.

### Global Repository Guardrails

#### 1. Zero-Placeholder Policy (ZPP)

    The Rule: You are strictly forbidden from using todo!(), unimplemented!(), // FIXME, or empty/mock return values (Ok(vec![]), true) to resolve compilation errors in core, sdk, or adapter modules. Also any synonym for these placeholders, such as "In production we should", " The proper Implementation should be done later", " Real implementation will be done later".

    The Constraint: If a logic block is too complex to pass the borrow checker or type system, you must refactor the architecture to accommodate the complexity rather than deleting the logic to satisfy the compiler.

    If a logic is necessary for production, it must be implemented properly, even if it means refactoring the code to make it work. 

#### 2. Mandatory Debugging Protocol

When encountering a compilation error, follow this sequence:

    Analyze the Invariant: Check docs/PROTOCOL_INVARIANTS.md. Does your proposed fix weaken a security invariant? If yes, the fix is invalid.

    Type-Preservation: You may not simplify data structures (e.g., converting a struct Proof to a Vec<u8>) just to bypass lifetime or trait bound issues.

    Traceability: Every time you resolve a compiler error by changing a function signature or logic flow, you must append a "Refactor Note" to the PR/Commit explaining why the new structure still satisfies the original cryptographic requirements.

#### 3. "Compiling != Correct" Checkpoint

Before declaring a bug fixed:
    Verification Check: Ensure that any verify() or validate() function still contains at least one cryptographic operation (Hashing, Merkle Proof, or Signature Check).
    Negative Test Requirement: You must write (or update) a test case that fails if the logic is simplified. For example, if you are fixing a Verifier, you must provide a test with a malformed proof that your "fix" correctly rejects.

#### 4. Forbidden Phrases in Code

The following patterns will trigger an immediate "Needs Revision" status:

    return Ok(Default::default()); // Temporary fix for build

    // Logic removed to satisfy borrow checker; restore later

    assert!(true); // TODO: actual verification

#### 5. Forbidden Runtime Patterns

The following patterns are forbidden in production runtime code:

```rust
unwrap()
expect()
new_unchecked()
unsafe
Sha256::digest
Keccak256::digest
blake3::hash
```

Allowed only in:

```text
/tests
/fuzz
/benches
```

#### 6. Approved Unsafe Modules

Unsafe MAY exist only in:

```text
csv-crypto/
csv-zk/
```

All unsafe blocks MUST include:

```rust
// SAFETY:
```

Without exception.

### Cryptographic Guardrails

- **No `Ok(true)` in verification paths.** Every verification path must return `Err` if data is unavailable, not a passing result.
- **No mock signatures in production code.** The production guarantee CI (once paths are fixed) must catch `fake_sig`, `mock_proof`, `[0u8; 64]` patterns in non-test code.
- **Seal registry check is mandatory.** `verify_proof()` must always call the seal registry callback. Never skip it with `//todo`.
- **Empty proof bundles are rejected.** Zero-length `inclusion_proof` bytes must fail, not pass.

### Key Management Guardrails

- **No raw private key bytes in logs.** Add a lint rule: `grep -r "private_key\|secret_key\|signing_key" --include="*.rs" | grep "println\|info!\|debug!"` must return zero results.
- **Zeroize on drop.** Any type holding a private key must implement `Zeroize` + `ZeroizeOnDrop`. Verify with `#[derive(ZeroizeOnDrop)]`.
- **No key material in state files.** `unified_storage.json` must never contain fields with names matching `key|secret|mnemonic|seed|private`.

### Chain Integration Guardrails

- **Testnet by default.** Any new chain integration must default to testnet in config. Mainnet requires explicit `--network mainnet` flag.
- **RPC endpoints via env vars only.** No hardcoded endpoints in any config file checked into the repo.
- **Finality depth is never zero.** `min_confirmations = 0` must fail at config parse time.

### State Machine Guardrails

- **Transfer status is append-only.** A transfer record cannot go from `Completed` back to `Pending`. Add validation in `UnifiedStateManager::update_transfer()`.
- **Sanad status is monotonic.** `Active → Transferred → Consumed`. Reverse transitions must panic/error.
- **Double-consume fails loudly.** Attempting to consume an already-consumed Sanad must return a specific error, not silently succeed or update the record.

### CI Guardrails

- **Production guarantee gates must pass on every PR.** No merges to `main` with failing gates.
- **`cargo audit` on every push.** Already in CI — keep it.
- **`cargo clippy -- -D warnings` blocks merge.** Already in CI — keep it.
- **No `unwrap()` in production paths.** Add clippy lint `#![deny(clippy::unwrap_used)]` to `csv-core/src/lib.rs`, `csv-keys/src/lib.rs`.
- **CI MUST fail on forbidden patterns:** TODO, FIXME, unwrap(), expect(), unsafe (outside approved modules), raw hashing (outside crypto module), mock proofs (production code), manual ABI encoding (EVM adapters).

When adding new chain adapters, follow the pattern:

- `seal_protocol.rs` – `SealProtocol` implementation
- `ops.rs` / `chain_operations.rs` – `ChainBackend` implementation
- `rpc.rs` – trait + mock for RPC
- `types.rs` – seal/anchor/proof types

## 9. Other Methods to Make the Repository AI‑Friendly

Beyond this file, consider adding:

- **`.github/copilot-instructions.md`** – specifically for GitHub Copilot coding style rules.
- **`.cursor/rules/`** – guidelines for Cursor AI (e.g., rules for inviolable invariants).
- **`.github/CODEOWNERS`** – to protect critical paths (e.g., `csv-core/src/verifier.rs`).
- **`docs/architecture/`** with Mermaid diagrams for state machines, seal lifecycle, etc.
- **Structured comments** in every `lib.rs` and critical module explaining purpose and security considerations.
- **A `CONTRIBUTING.md`** that includes the security checklist from `PROTOCOL_INVARIANTS.md`.
- **Git hooks** that run checks (`cargo fmt`, `cargo clippy`, `cargo test -p csv-core`) before push.
- **A `deno.json` or `biome.json` for TypeScript** (the MCP server is TypeScript).

The single most impactful addition: **a strong, machine‑readable set of invariant checks in CI** that automatically reject any PR that introduces raw hashing, unsafe code in runtime, or unverified proof acceptance.

## 10. References

- Repository: <https://github.com/client-side-validation/csv-protocol>
- `csv-core/src/PROTOCOL_INVARIANTS.md` – mandatory reading
- `docs/CONSULTING.md` and `docs/MASTERPLAN.md` for deeper design
- `docs/AUDIT.md` and `docs/AUDIT2.md` for security audit status
