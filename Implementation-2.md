# Implementation.md

## Purpose

This document exists to prevent implementation drift.

The repository already contains strong primitives, but the current execution surface is too ambiguous for autonomous agents. Multiple crates expose overlapping concepts (`seal`, `commitment`, `proof`, `transfer`, `finality`, `replay`, `sanad`) without a strict operational contract.

This document defines:

- exact implementation boundaries
- mandatory execution order
- forbidden shortcuts
- invariants that MUST hold
- acceptable extension patterns
- failure handling behavior
- cross-crate ownership rules

If an AI agent cannot determine whether a change is safe, the correct action is:

> stop implementation and mark the area as unresolved.

Never infer cryptographic semantics from naming alone.

---

# 1. System Model

CSV is a deterministic cross-chain proof and seal protocol.

The repository is divided into layers:

| Layer | Responsibility | Must NEVER Do |
|---|---|---|
| `csv-core` | Canonical protocol logic | Chain RPC access |
| `csv-runtime` | Coordination and orchestration | Cryptographic rule definition |
| `csv-*` chain crates | Chain-specific adapters | Define canonical state rules |
| `csv-store` | Persistence | Verification |
| `csv-wallet` | UX and local state | Consensus decisions |
| `csv-explorer` | Read/index/query | Mutate canonical protocol state |
| `csv-sdk` | Stable external API | Internal orchestration |
| `csv-cli` | Operator tooling | Runtime authority |

The direction of authority is:

```text
csv-core
    ↑
chain adapters / runtime
    ↑
wallet / explorer / sdk / cli
```

Nothing outside `csv-core` may redefine protocol truth.

---

# 2. Mandatory Architectural Rules

## 2.1 Canonical Types Live Only in `csv-core`

The following concepts MUST originate in `csv-core`:

- commitments
- proofs
- replay protection
- transfer states
- sanad lifecycle
- verification results
- protocol invariants
- finality semantics

Chain crates may WRAP these.
They may NOT redefine them.

Bad:

```rust
pub enum TransferState {
    Pending,
    Done,
}
```

inside `csv-bitcoin`.

Correct:

```rust
use csv_core::transfer_state::TransferState;
```

---

## 2.2 Adapters Are Translators

Chain crates:

- fetch data
- serialize chain formats
- produce inclusion proofs
- produce finality observations
- submit transactions
- verify chain-native signatures

They MUST NOT:

- invent alternate commitment rules
- bypass replay registry
- bypass finality checks
- mark transfers complete
- mutate canonical state directly

---

## 2.3 Runtime Coordinates — Never Decides Truth

`csv-runtime`:

- schedules work
- retries operations
- coordinates transitions
- persists events
- handles leases
- detects reorgs

`csv-runtime` MUST NOT:

- decide proof validity independently
- override verifier failures
- skip replay detection
- fabricate state transitions

Verification authority remains inside `csv-core`.

---

# 3. Mandatory Transfer Lifecycle

This state order is STRICT.

```text
Locked
→ ProofBuilding
→ ProofValidated
→ AwaitingFinality
→ Minting
→ Completed
```

Rollback path:

```text
Any Active State
→ RolledBack
```

Compromise path:

```text
Any State
→ Compromised
```

Illegal transitions MUST hard fail.

Compile-fail tests already exist in:

```text
csv-core/tests/compile_fail/
```

Agents MUST preserve those guarantees.

---

# 4. Replay Protection Rules

Replay resistance is not optional.

Every proof pipeline MUST:

1. derive deterministic replay identity
2. check replay registry before acceptance
3. persist replay record atomically
4. reject duplicates

Never:

- "temporarily disable" replay checks
- add feature flags around replay protection
- allow runtime bypasses
- make replay checks advisory

Replay protection belongs to:

```text
csv-core/replay_registry.rs
csv-runtime/replay_db*
csv-store/replay_registry_store.rs
```

Any implementation touching proofs MUST audit these paths.

---

# 5. Finality Rules

Finality is chain-specific.

Never generalize across chains.

Examples already encoded in repository:

| Chain | Finality Model |
|---|---|
| Bitcoin | confirmation depth |
| Aptos | certified checkpoints |
| Ethereum | block confirmations / consensus |
| Solana | commitment levels |
| Sui | checkpoint finality |
|

Agents MUST NOT:

- create a universal finality threshold
- share finality structs across chains unless already canonicalized in `csv-core`
- equate "transaction observed" with finalized

Observed transaction ≠ finalized transaction.

A transfer may only enter:

```text
AwaitingFinality → Minting
```

after chain-specific finality verification succeeds.

---

# 6. Reorg Handling

Reorg handling is REQUIRED.

Every chain adapter MUST support:

- inclusion invalidation
- rollback detection
- replay reconciliation
- transfer reversal

Required flow:

```text
observe block
→ verify inclusion
→ monitor finality
→ detect reorg
→ invalidate proof
→ rollback transfer
→ reconcile replay state
```

Never assume:

- a transaction remains canonical forever
- observed inclusion is permanent
- mempool state is trustworthy

Relevant modules:

```text
csv-core/reorg/
csv-runtime/adversarial.rs
csv-runtime/transfer_coordinator.rs
```

---

# 7. Proof Pipeline Contract

The proof pipeline is deterministic.

Required order:

```text
collect chain evidence
→ build canonical proof material
→ verify inclusion
→ verify replay resistance
→ verify finality
→ verify signatures
→ build proof bundle
→ persist provenance
```

Forbidden:

```text
verify signatures after persistence
persist before replay checks
mint before finality
```

Relevant modules:

```text
csv-core/proof_pipeline.rs
csv-core/proof_material.rs
csv-core/proof_bundle.rs
csv-core/provenance.rs
```

---

# 8. Cryptographic Rules

## 8.1 Hashing

Never replace tagged hashing with raw SHA usage.

Use:

```rust
csv_tagged_hash(...)
```

where protocol-defined hashing exists.

Relevant:

```text
csv-core/tagged_hash.rs
csv-core/commit_mux.rs
```

---

## 8.2 Determinism

Proof generation MUST be deterministic.

Same inputs MUST produce:

- same proof bytes
- same commitment hashes
- same mux roots
- same replay identifiers

Property tests already enforce parts of this.

Do not introduce:

- random salts during verification
- nondeterministic map iteration
- timestamp-derived proof data
- unstable serialization

---

## 8.3 Serialization

Never change serialized layouts casually.

Before changing:

- CBOR layout
- binary ordering
- field ordering
- hash preimages
- proof bundle schema

agent MUST:

1. locate golden tests
2. regenerate fixtures intentionally
3. update versioning rules
4. verify backward compatibility

Relevant:

```text
csv-core/tests/golden/
protocol_version.rs
schema.rs
```

---

# 9. MPC Tree Rules

`commit_mux.rs` defines deterministic multi-protocol commitment aggregation.

Agents MUST preserve:

- leaf ordering semantics
- tagged hash usage
- odd-node promotion behavior
- branch verification rules
- proof determinism

Never:

- sort leaves implicitly
- parallelize with nondeterministic ordering
- replace promotion rules
- modify proof branch direction semantics

Changing mux semantics invalidates historical proofs.

Treat this area as consensus-critical.

---

# 10. Storage Rules

Storage layers are dumb persistence.

They MUST NOT:

- verify cryptography
- decide transfer state legality
- infer finality
- auto-heal protocol state

Allowed:

- indexing
- snapshots
- persistence
- caching
- replay record storage

If storage detects corruption:

- mark inconsistency
- escalate error
- stop transition

Never auto-correct silently.

---

# 11. Error Handling Rules

Never swallow protocol errors.

Forbidden:

```rust
let _ = verify(...);
```

Forbidden:

```rust
.unwrap_or_default()
```

on cryptographic or consensus-critical paths.

Required behavior:

```text
fail loudly
preserve context
attach chain identity
attach proof identity
persist failure provenance
```

Errors affecting proof integrity MUST bubble upward.

---

# 12. Async and Concurrency Rules

Concurrency is allowed only where determinism is preserved.

Safe:

- parallel RPC fetches
- concurrent indexing
- independent proof verification

Unsafe unless explicitly guarded:

- concurrent replay writes
- parallel transfer transitions
- unordered commitment aggregation
- shared mutable proof state

Every async mutation path MUST define:

- ownership boundary
- retry behavior
- idempotency guarantees
- rollback semantics

---

# 13. Lease System Rules

Leases prevent multiple coordinators from executing the same transfer.

Required guarantees:

- lease acquisition is atomic
- expired leases are detectable
- renewals are monotonic
- execution ownership is unique

Never:

- bypass lease acquisition in runtime
- allow dual coordinators
- execute proof pipelines without lease ownership

Relevant:

```text
csv-runtime/coordinator_lease.rs
csv-runtime/coordinator_lease_postgres.rs
```

---

# 14. Testing Requirements

Every protocol-affecting change MUST include:

| Change Type | Required Tests |
|---|---|
| state transition | compile-fail + runtime test |
| serialization | golden fixture test |
| replay logic | replay property test |
| reorg logic | rollback/reorg test |
| chain proof | inclusion verification test |
| hash semantics | deterministic property test |
| async coordination | concurrency/idempotency test |

Agents MUST search existing tests before introducing new abstractions.

Avoid duplicate testing frameworks.

---

# 15. Forbidden Implementation Patterns

## Never Add "Temporary" Security Bypasses

Forbidden:

```rust
if cfg!(test) {
    return Ok(());
}
```

inside proof verification.

---

## Never Add Silent Fallback Verification

Forbidden:

```rust
primary_verifier.or_else(|_| backup_accept())
```

---

## Never Convert Consensus Failures into Warnings

Bad:

```rust
warn!("proof invalid");
Ok(())
```

Correct:

```rust
return Err(...)
```

---

## Never Infer Chain Semantics from Strings

Bad:

```rust
if chain.contains("bitcoin")
```

Correct:

explicit enums or typed configuration.

---

## Never Introduce Dynamic Typing Into Core Verification

Forbidden:

```rust
serde_json::Value
HashMap<String, Value>
```

inside canonical proof logic.

Use typed structures.

---

# 16. Extension Rules for New Chains

Adding a new chain REQUIRES:

```text
csv-newchain/
    config.rs
    proofs.rs
    verifier.rs
    seal.rs
    seal_protocol.rs
    types.rs
    rpc.rs
    ops.rs
```

The adapter MUST implement:

- inclusion proof generation
- finality verification
- replay compatibility
- reorg handling
- canonical serialization
- signature verification

The adapter MUST NOT:

- fork core transfer semantics
- redefine proof bundles
- introduce alternate replay identity formats

---

# 17. Agent Execution Procedure

When implementing any feature:

## Step 1 — Identify Layer

Determine:

- protocol logic?
- runtime orchestration?
- chain adapter?
- storage?
- UI?

Never mix layers.

---

## Step 2 — Locate Existing Invariants

Search for:

```text
tests/
compile_fail/
property tests
protocol docs
```

before coding.

---

## Step 3 — Identify Consensus-Critical Paths

Consensus-critical means:

- hashes
- proof layouts
- replay identity
- state transitions
- mux trees
- finality

Changes here require maximum caution.

---

## Step 4 — Implement Minimally

Prefer:

- extending existing types
- small deterministic functions
- explicit transitions
- typed errors

Avoid:

- framework rewrites
- generic abstractions
- trait explosions
- unnecessary async

---

## Step 5 — Verify Against Existing Tests

Mandatory:

```bash
cargo test --workspace --all-features
```

If serialization changes:

```text
verify golden fixtures manually
```

---

# 18. Priority Files

The following files are high-authority references.

Agents SHOULD read these before major implementation.

## Core Protocol

```text
csv-core/src/transfer_state/
csv-core/src/proof_pipeline.rs
csv-core/src/replay_registry.rs
csv-core/src/finality/
csv-core/src/commit_mux.rs
csv-core/src/proof.rs
csv-core/src/verifier.rs
```

## Runtime

```text
csv-runtime/src/transfer_coordinator.rs
csv-runtime/src/event_store.rs
csv-runtime/src/lease.rs
```

## Reorg Handling

```text
csv-core/src/reorg/
```

## Tests

```text
csv-core/tests/compile_fail/
csv-core/tests/properties/
```

---

# 19. Non-Goals

This repository is NOT:

- a generic blockchain SDK
- a speculative execution engine
- an eventually-consistent proof system
- a best-effort bridge
- a trust-me middleware layer

Correctness is preferred over throughput.

Determinism is preferred over convenience.

Explicit failure is preferred over silent recovery.

---

# 20. Definition of Done

An implementation is complete only if:

- invariants remain preserved
- replay safety remains intact
- finality semantics remain chain-correct
- reorg behavior is handled
- deterministic tests pass
- serialization compatibility is verified
- no forbidden shortcuts were introduced
- error paths are explicit
- state transitions remain legal

If any of those are uncertain, implementation is incomplete.

