# AUDIT.md

# CSV Protocol — Deep Architecture, Security, Contract, and Runtime Audit

Date: 2026-05-21
Scope: `csv-core`, `csv-runtime`, adapters, contracts, explorer, SDK/runtime boundaries, deployment readiness, seal semantics, Sanad model, distributed-system behavior.

---

# Executive Assessment

The repository shows unusually strong architectural intent.

Several parts already demonstrate mature protocol thinking:

- explicit security invariants in `csv-core`
- separation between runtime and adapters
- chain-specific crates instead of conditional spaghetti
- replay-prevention awareness
- observability hooks
- seal semantics treated as protocol primitives
- multi-chain abstraction without collapsing into lowest-common-denominator APIs
- early attention to adversarial behavior
- deployment profile separation

The problem is not conceptual direction.

The problem is operational hardness.

Right now the repository behaves more like a protocol research platform than a production-grade settlement fabric.

There are strong signs of architectural ambition exceeding current enforcement.

Examples:

- security invariants are documented but not mechanically enforced
- distributed consistency assumptions exist but durability semantics are incomplete
- contracts are modeled as stable anchors, but schema/version evolution strategy is underdefined
- runtime abstraction exists, but deterministic execution guarantees are still weak
- there are many `unwrap()`/`expect()` calls across the codebase
- too many trust boundaries remain implicit
- chain adapters can still become leakage vectors into core semantics
- several crates are structurally present but operationally shallow

This means the protocol is currently vulnerable to:

- state divergence
- replay inconsistencies
- adapter-specific semantic corruption
- schema drift
- event ordering bugs
- non-deterministic proofs
- incomplete finality guarantees
- unsafe deployment upgrades
- weak auditability under incident response

The repository is not yet ready for independent external implementers.

A second team attempting to implement the protocol from contracts/specs alone would likely produce incompatible behavior.

That is the main architectural warning.

---

# Critical Findings

# 1. SECURITY INVARIANTS ARE DECLARATIVE, NOT ENFORCED

`csv-core/src/seal_protocol.rs` contains strong invariant documentation.

That is valuable.

But the invariants are still comments.

The system currently relies on adapter authors to “do the right thing”.

That fails under scale.

A malicious or careless adapter can still:

- accept malformed proofs
- bypass replay semantics
- weaken inclusion validation
- alter hash domain separation
- redefine finality semantics
- create silent cross-chain ambiguity

## Architectural Risk

The protocol currently trusts adapter correctness too early.

The runtime should trust proofs.
It should not trust adapters.

Adapters should produce canonical proof artifacts.
The runtime/core should validate them.

Right now too much verification responsibility sits inside adapters.

That creates chain-specific trust islands.

## Required Refactor

Move verification semantics into canonical core verification engines.

Adapters should only:

- fetch chain data
- normalize proofs
- expose chain metadata
- expose finality parameters

Core should own:

- seal lifecycle verification
- replay verification
- commitment verification
- proof verification
- state transition verification
- event canonicalization

## Required Structure

```rust
trait ChainAdapter {
    fn fetch_anchor(&self, anchor_id: AnchorId) -> Result<AnchorData>;
    fn fetch_proof(&self, proof_id: ProofId) -> Result<RawProof>;
    fn chain_context(&self) -> ChainContext;
}

trait ProofVerifier {
    fn verify(&self, proof: CanonicalProof) -> Result<VerifiedProof>;
}
```

The runtime should never accept “verified=true” from adapters.

---

# 2. MASSIVE `unwrap()` / `expect()` SURFACE

Repository scan shows:

- ~784 `unwrap()` usages
- ~129 `expect()` usages
- multiple `panic!()` paths
- several `unsafe` usages

This is one of the biggest production-readiness blockers.

In a distributed settlement system:

`unwrap()` is not merely a crash risk.

It becomes:

- consensus divergence risk
- event-stream truncation risk
- partial-transfer corruption risk
- replay-window corruption
- settlement deadlock trigger
- observability blind spot

## Example Failure Scenario

Node A crashes during replay DB persistence.

Node B commits the event.

A restarted coordinator replays partially persisted state.

Now the transfer graph diverges.

The repository already hints at replay semantics, but panic surfaces undermine the entire model.

## Required Enforcement

CI must reject:

- `unwrap()`
- `expect()`
- `panic!()`
- unchecked indexing
- `unsafe`

outside explicitly approved modules.

## Required Replacement Pattern

```rust
let proof = proof_store
    .load(id)
    .map_err(RuntimeError::ProofLoadFailure)?;
```

Every failure path must:

- preserve causality
- preserve trace identifiers
- preserve replay determinism
- emit structured telemetry
- classify retryability

---

# 3. CONTRACTS ARE NOT YET STABLE PROTOCOL ARTIFACTS

This is the single most important long-term architectural issue.

The contracts currently look like implementation containers.

They must become immutable protocol anchors.

Your own concern is correct:

changing contracts frequently after runtime/core evolution becomes operationally catastrophic.

Especially across:

- multiple chains
- proofs
- Sanad evolution
- third-party builders
- historical verification
- archival reconstruction

Right now the contracts do not yet expose a hardened canonical schema governance model.

---

# 4. SANAD DATA MODEL IS UNDER-SPECIFIED FOR LONG-TERM EVOLUTION

The Solana `SanadAccount` already includes:

- owner
- sanad_id
- commitment
- state_root
- metadata_hash
- proof_root
- nullifier
- asset class

That is good.

But the schema is still structurally fragile.

## Problems

## A. No explicit schema versioning

There is no strong evolution mechanism.

Future additions will break:

- proof compatibility
- historical verification
- external SDK implementations
- archival reconstruction

## B. No canonical serialization specification

Different chains/languages will serialize differently.

This destroys proof portability.

## C. Metadata semantics are weak

`metadata_hash` alone is insufficient.

You need:

- canonical metadata envelope
- detached metadata proofs
- recursive proof support
- canonical field ordering
- schema registries
- semantic type IDs

## D. No content-addressed proof envelope

The protocol needs immutable proof-addressable structures.

Current model is still too account-centric.

---

# 5. REQUIRED CANONICAL SANAD ENVELOPE

This should become the protocol center.

Not chain accounts.

Not runtime structs.

Not SDK DTOs.

A canonical envelope.

## Required Model

```rust
struct CanonicalSanadEnvelope {
    protocol_version: u16,
    sanad_id: Hash256,
    sanad_type: TypeId,
    issuer_id: Hash256,

    body_hash: Hash256,
    metadata_root: Hash256,
    proof_root: Hash256,
    seal_root: Hash256,

    timestamp: u64,
    nonce: u64,

    parent_refs: Vec<Hash256>,
    dependency_refs: Vec<Hash256>,

    signature_scheme: SignatureScheme,
    canonical_encoding: EncodingType,
}
```

This envelope must:

- serialize identically everywhere
- hash identically everywhere
- survive chain migration
- survive adapter rewrites
- survive SDK rewrites
- survive runtime rewrites

The chain contracts should store only commitments.

Never large semantic payloads.

---

# 6. NO STRONG EVENT CANONICALIZATION STRATEGY

The runtime already contains event abstractions.

But event semantics are still under-defined.

Distributed settlement systems fail from event ambiguity long before cryptography fails.

## Missing Pieces

- globally ordered event IDs
- causality chains
- replay checkpoints
- deterministic event hashing
- idempotency guarantees
- monotonic sequencing
- poison-event handling
- duplicate-event reconciliation
- tombstone semantics

## Required Event Structure

```rust
struct CanonicalEvent {
    event_id: Hash256,
    causality_parent: Option<Hash256>,
    transfer_id: Hash256,
    sequence: u64,
    emitted_at: u64,
    event_type: EventType,
    payload_hash: Hash256,
}
```

---

# 7. REPLAY PROTECTION IS CONCEPTUALLY GOOD BUT OPERATIONALLY WEAK

`ReplayDatabase` is one of the strongest architectural directions in the repository.

But replay prevention in distributed systems is not just key existence.

You need:

- atomic state transitions
- durable write-ahead logging
- monotonic checkpoints
- crash-safe commit semantics
- distributed fencing
- lease expiration semantics
- consensus-aware idempotency

## Missing Operational Guarantees

There is insufficient evidence of:

- WAL guarantees
- crash recovery invariants
- split-brain handling
- multi-coordinator fencing
- deterministic replay reconstruction

## Required Model

Replay state should become append-only.

Never mutable.

Example:

```rust
enum ReplayRecord {
    Pending,
    Locked,
    Anchored,
    Finalized,
    Rejected,
    Reverted,
}
```

Each transition should:

- include previous hash
- include monotonic sequence
- include actor identity
- be signed or MACed

---

# 8. ADAPTER ISOLATION IS INCOMPLETE

The architecture intends adapter isolation.

But the repository still risks semantic leakage.

Examples:

- chain-specific proof assumptions
- chain-native timing semantics
- chain-native account assumptions
- inconsistent finality behavior
- chain-native replay semantics

The runtime must never understand:

- Solana slots
- Ethereum confirmations
- Aptos epochs
- Bitcoin mempool semantics

Adapters should translate all of this into canonical protocol semantics.

Right now the abstraction boundary is still porous.

---

# 9. FINALITY MODEL IS UNDERDEFINED

This is extremely dangerous.

Different chains expose:

- probabilistic finality
- deterministic finality
- optimistic finality
- checkpointed finality
- economic finality

A universal “verify_finality()” abstraction is insufficient.

The runtime requires a canonical finality confidence model.

## Required Structure

```rust
enum FinalityGuarantee {
    Probabilistic {
        confirmations: u64,
        reorg_probability: f64,
    },

    Deterministic {
        checkpoint_hash: Hash256,
    },

    Economic {
        slash_cost: u128,
    },
}
```

Without this:

cross-chain transfers can finalize under incompatible trust assumptions.

---

# 10. NO CLEAR GOVERNANCE MODEL FOR CONTRACT EVOLUTION

This is critical.

You already understand the operational danger:

contracts cannot change frequently.

But the repository lacks a visible governance architecture for:

- schema evolution
- contract migration
- proof evolution
- deprecation windows
- capability negotiation
- feature flags
- verifier upgrades

## Required Strategy

Every deployed contract must expose:

```rust
struct ContractCapabilities {
    protocol_version: u16,
    supported_proof_versions: Vec<u16>,
    supported_hash_algorithms: Vec<HashAlg>,
    supported_seal_versions: Vec<u16>,
}
```

Runtime nodes must negotiate capabilities.

Never assume compatibility.

---

# 11. EXPLORER ARCHITECTURE IS NOT YET INCIDENT-READY

Explorer/indexer systems become forensic infrastructure during attacks.

Right now the explorer appears operationally helpful but not forensic-grade.

Missing:

- immutable event snapshots
- forensic replay tooling
- proof lineage tracing
- causal graph visualization
- seal ancestry reconstruction
- transfer rollback simulation
- tamper-evident indexing

The explorer should become a verification instrument.

Not merely a UI.

---

# 12. OBSERVABILITY EXISTS BUT IS NOT PROTOCOL-CENTRIC

Most systems add metrics.

Settlement systems require causality observability.

You need:

- transfer lineage IDs
- cross-chain trace propagation
- deterministic replay traces
- proof verification spans
- state-transition audit trails
- cryptographic operation timing
- finality lag metrics
- chain drift metrics

## Required Trace Fields

Every operation should include:

```text
transfer_id
seal_id
proof_id
chain_id
event_sequence
replay_checkpoint
causality_parent
runtime_epoch
```

Without this:

incident reconstruction becomes guesswork.

---

# 13. DEPLOYMENT MODEL IS UNDERHARDENED

The repository contains deployment abstractions.

But distributed protocol deployment requires:

- deterministic bootstrap
- immutable manifests
- reproducible builds
- chain capability verification
- schema compatibility checks
- cryptographic deployment attestations
- rollback-safe migrations

## Required Artifact

Every deployment should emit:

```text
manifest.json
proof-manifest.json
contract-hashes.json
adapter-capabilities.json
runtime-policy.json
```

Signed.

Content-addressed.

Immutable.

---

# 14. CONTRACT STORAGE DESIGN STILL LEANS TOO HEAVILY ON MUTABLE STATE

The Solana contract already improved by removing O(n) lock vectors.

That is good.

But the broader architecture still models state too mutably.

Protocol-grade systems should prefer:

- append-only histories
- immutable commitments
- content-addressed state
- event sourcing
- deterministic reconstruction

instead of:

- mutable account records
- overwrite-style updates
- stateful branching

---

# 15. CRYPTOGRAPHIC AGILITY IS PRESENT BUT NOT FULLY SAFE

The repository hints at tagged hashing and proof-root abstractions.

Good direction.

But cryptographic agility becomes dangerous without strict domain governance.

## Missing

- algorithm registry
- forbidden algorithm registry
- mandatory domain separation rules
- hash namespace governance
- canonical transcript construction
- proof transcript versioning

## Required Rule

No direct raw hashing anywhere.

All hashing must go through:

```rust
TaggedHash::new(
    Domain::SealCommitment,
    version,
    payload,
)
```

---

# 16. RUNTIME DETERMINISM IS NOT YET GUARANTEED

This is extremely important.

Distributed settlement runtimes must behave deterministically under:

- retries
- crashes
- concurrent coordinators
- duplicate events
- reordered delivery
- delayed finality
- chain reorgs

Current architecture suggests awareness of this.

But deterministic execution guarantees are not yet enforceable.

## Required Rule

All state transitions must become:

```text
pure(current_state, event) -> next_state
```

No hidden IO.

No implicit clocks.

No ambient state.

No adapter mutation.

---

# 17. MISSING FORMAL SPECIFICATION LAYER

This is one of the largest ecosystem blockers.

The repository currently acts as:

- implementation
n- partial specification
- protocol definition

all simultaneously.

That does not scale.

External implementers need:

- canonical protocol specification
- proof format specification
- event semantics specification
- canonical serialization specification
- failure semantics specification
- replay semantics specification
- chain capability specification
- deterministic test vectors

Without this:

every independent implementation becomes a fork.

---

# REQUIRED PRIORITY ROADMAP

# PHASE 1 — HARDENING

Mandatory before ecosystem expansion.

## Required

- eliminate unwrap/expect/panic paths
- freeze canonical serialization
- introduce protocol-versioned envelopes
- centralize proof verification
- enforce deterministic replay semantics
- add malicious adapter test suites
- introduce append-only replay logs
- define canonical event model
- forbid non-domain-separated hashing
- freeze proof envelope format

---

# PHASE 2 — CONTRACT STABILIZATION

## Required

- immutable schema governance
- capability negotiation
- canonical proof commitments
- contract upgrade governance
- version compatibility matrix
- deployment attestation manifests
- chain-independent proof semantics

---

# PHASE 3 — ECOSYSTEM READINESS

## Required

- formal protocol specification
- interoperability test harness
- deterministic reference vectors
- SDK conformance tests
- independent verifier implementation
- formal replay simulator
- adversarial fuzz infrastructure

---

# CODE-LEVEL RECOMMENDATIONS

# Replace Boolean Verification APIs

Forbidden:

```rust
fn verify() -> bool
```

Required:

```rust
enum VerificationResult {
    Valid,
    Invalid(VerificationFailure),
    Retryable(TransientFailure),
}
```

---

# Introduce Canonical IDs Everywhere

Use strongly typed IDs.

```rust
struct TransferId(Hash256);
struct SealId(Hash256);
struct ProofId(Hash256);
struct EventId(Hash256);
```

Avoid raw `[u8; 32]` everywhere.

Raw byte arrays eventually create semantic corruption.

---

# Replace Mutable State Machines

Avoid:

```rust
sanad.locked = true;
```

Prefer:

```rust
enum SealState {
    Created,
    Locked,
    Anchored,
    Finalized,
    Refunded,
}
```

with explicit transition validation.

---

# Add Deterministic Serialization Layer

Mandatory.

Example:

```rust
trait CanonicalSerialize {
    fn canonical_encode(&self) -> Vec<u8>;
}
```

Every proof, event, seal, and Sanad must use identical encoding across:

- Rust
- TypeScript
- WASM
- Solana
- EVM
- Sui
- Aptos

---

# Add Contract Capability Discovery

Every contract should expose:

```rust
fn capabilities() -> ContractCapabilities
```

This prevents silent incompatibility.

---

# Introduce Immutable Audit Trail Hashing

Every replay/event record should include:

```rust
prev_event_hash
record_hash
state_hash
```

This creates tamper-evident lineage.

---

# Add Protocol Test Vector Repository

You need a dedicated repository containing:

- canonical proof vectors
- malformed proof vectors
- replay attack vectors
- chain reorg vectors
- serialization vectors
- malicious adapter vectors
- finality edge cases

Without vectors:

external implementations will drift.

---

# FINAL ASSESSMENT

The repository contains the foundation of a serious protocol.

The architectural intent is substantially above average.

The strongest parts are:

- protocol-oriented thinking
- replay awareness
- seal semantics
- adapter separation
- multi-chain abstraction direction
- explicit adversarial awareness
- event-driven runtime direction

The weakest parts are:

- enforcement
- determinism
- schema governance
- contract immutability strategy
- canonical serialization
- operational replay guarantees
- runtime purity
- formal specification boundaries
- ecosystem reproducibility

The repository is currently:

- strong enough for internal iteration
- strong enough for controlled prototyping
- not yet strong enough for independent ecosystem implementation
- not yet hardened enough for irreversible settlement-grade deployment

The core architectural transition still missing is this:

from:

"a runtime with chain adapters"

into:

"a deterministic cryptographic protocol with canonical semantics independent of chains"

That shift changes everything:

- proofs
- contracts
- replay semantics
- observability
- serialization
- governance
- ecosystem compatibility
- auditability
- long-term survivability.

