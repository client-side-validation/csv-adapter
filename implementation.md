# IMPLEMENTATION.md

# Purpose

This document is an execution-grade engineering implementation plan intended for AI agents and human engineers.

It contains:

- exact architectural restructuring
- crate decomposition
- file-level refactors
- protocol hardening
- serialization redesign
- proof-system redesign
- verification centralization
- contract hardening
- distributed-system corrections
- testing mandates
- governance systems
- runtime minimization
- scalability guardrails
- implementation ordering

This document assumes:

- current architecture is NOT stable
- runtime/core MUST NOT freeze yet
- contracts MUST be redesigned before immutability
- verification MUST become canonical
- complex Sanads MUST NOT ship yet

---

# PHASE 0 — HARD FREEZE & PROTOCOL LOCKDOWN

## Goal

Prevent semantic drift while refactoring.

---

## Tasks

### 0.1 Freeze New Features

AI AGENT:

- Reject all new feature PRs.
- Only allow:
  - refactors
  - invariant fixes
  - serialization fixes
  - proof fixes
  - testing
  - contracts stabilization

---

### 0.2 Create Protocol RFC System

Create:

```text
/docs/rfcs/
```

Add:

```text
RFC-0001-canonical-serialization.md
RFC-0002-proof-taxonomy.md
RFC-0003-finality-model.md
RFC-0004-replay-model.md
RFC-0005-content-model.md
RFC-0006-contract-abi.md
RFC-0007-hash-registry.md
RFC-0008-state-machine.md
RFC-0009-schema-registry.md
RFC-0010-adapter-capabilities.md
```

MANDATORY RULE:

No protocol change without RFC.

---

### 0.3 Create Protocol Constitution Tests

Create:

```text
/tests/protocol_constitution/
```

Add invariant tests:

```rust
#[test]
fn protocol_hashes_are_stable();

#[test]
fn serialization_is_canonical();

#[test]
fn replay_is_impossible();

#[test]
fn forbidden_state_transitions_fail();

#[test]
fn all_proofs_are_domain_separated();
```

These tests MUST NEVER BREAK.

---

# PHASE 1 — REPOSITORY RESTRUCTURING

# Goal

Destroy monolith behavior.

---

## Target Workspace Structure

REPLACE current architecture with:

```text
/crates
    /csv-protocol
    /csv-codec
    /csv-hash
    /csv-proof
    /csv-verifier
    /csv-schema
    /csv-content
    /csv-runtime
    /csv-storage
    /csv-adapters
        /adapter-bitcoin
        /adapter-ethereum
        /adapter-solana
        /adapter-aptos
        /adapter-sui
    /csv-sdk
    /csv-wallet-core
    /csv-wallet-ui
    /csv-cli
    /csv-explorer
    /csv-contract-bindings
    /csv-testkit
```

---

## Rules

### csv-protocol

ONLY:

- state machines
- protocol constants
- protocol types
- protocol invariants
- replay semantics
- transition legality
- versioning

FORBIDDEN:

- networking
- storage
- RPC
- async
- chain code
- runtime orchestration

---

### csv-codec

ONLY:

- canonical serialization
- deterministic decoding
- schema validation
- byte ordering

NO business logic.

---

### csv-hash

ONLY:

- hash registry
- tagged hashing
- domain separation
- Merkle construction
- proof commitments

---

### csv-proof

ONLY:

- proof types
- proof composition
- proof validation primitives
- proof DAGs

---

### csv-verifier

ONLY canonical verifier.

Every SDK, wallet, runtime, explorer MUST use this.

NO duplicate verification logic anywhere else.

---

### csv-runtime

ONLY:

- orchestration
- queues
- scheduling
- coordination

NO protocol semantics.

---

### csv-storage

ONLY:

- persistence abstractions
- snapshots
- checkpointing
- transactional persistence

---

### csv-wallet-core

ONLY:

- key management
- signing
- encrypted local storage
- sync
- verification calls

NO rendering.

---

# PHASE 2 — CANONICAL SERIALIZATION

# Goal

Eliminate protocol ambiguity permanently.

---

## 2.1 Delete ALL Ad Hoc Serialization

AI AGENT TASK:

Search for:

```text
to_vec
serialize
deserialize
bincode
serde_json
manual byte pushes
```

Delete protocol-level manual serialization.

---

## 2.2 Introduce Canonical Codec Layer

Create:

```text
/crates/csv-codec/
```

Files:

```text
src/
    canonical.rs
    encode.rs
    decode.rs
    schema.rs
    versioning.rs
    error.rs
```

---

## 2.3 Define Canonical Encoding Rules

MANDATORY:

```text
- little endian ONLY
- field ordering fixed
- no optional implicit fields
- explicit enum tags
- explicit version tags
- deterministic maps
- deterministic arrays
- canonical UTF-8 normalization
- no floating points in protocol state
```

---

## 2.4 Add Cross-Language Golden Vectors

Create:

```text
/tests/golden_vectors/
```

Generate:

```text
Rust
Go
TS
Python
Solidity
Move
```

All hashes MUST match.

---

## 2.5 Add Serialization Differential Fuzzing

Create:

```rust
proptest! {
    fn encoding_roundtrip_is_identical(...)
}
```

Add malformed corpus tests.

---

# PHASE 3 — HASHING & DOMAIN SEPARATION

# Goal

Freeze cryptographic semantics.

---

## 3.1 Create Hash Registry

Create:

```text
/crates/csv-hash/src/registry.rs
```

Example:

```rust
pub enum HashDomain {
    SanadHeader,
    SanadContent,
    ProofBundle,
    ReplayNullifier,
    SealCommitment,
    TransitionCommitment,
    MerkleLeaf,
    MerkleInternal,
}
```

NO freeform tags allowed.

---

## 3.2 Create Typed Hash Wrappers

FORBIDDEN:

```rust
Vec<u8>
String
```

REPLACE WITH:

```rust
pub struct ContentHash([u8; 32]);
pub struct ProofHash([u8; 32]);
pub struct SealHash([u8; 32]);
```

Prevent semantic confusion.

---

## 3.3 Implement Canonical Merkle Trees

Create:

```text
src/merkle/
    tree.rs
    proof.rs
    verifier.rs
    streaming.rs
```

MANDATORY:

- ordered hashing
- leaf tagging
- internal node tagging
- deterministic balancing
- proof compression

---

# PHASE 4 — SANAD REDESIGN

# Goal

Support future complex content safely.

---

## 4.1 Replace Flat Sanad Content

CURRENT:

Likely:

```rust
struct Sanad {
    payload: Vec<u8>
}
```

DELETE.

---

## 4.2 Introduce Merkleized Content Trees

Create:

```rust
pub struct Sanad {
    header: SanadHeader,
    content_root: ContentRoot,
    proof_root: ProofRoot,
    schema_id: SchemaId,
    encoding_id: EncodingId,
    attachment_root: AttachmentRoot,
}
```

---

## 4.3 Create Content Tree System

Create:

```text
/crates/csv-content/
```

Files:

```text
content_tree.rs
claims.rs
attachments.rs
rights.rs
participants.rs
encryption.rs
redaction.rs
streaming.rs
```

---

## 4.4 Add Selective Disclosure Proofs

MANDATORY:

Users must prove subtree validity without exposing whole content.

Implement:

```rust
DisclosureProof
RedactedMerkleProof
EncryptedSubtreeProof
```

---

## 4.5 Add Attachment Model

NEVER store large blobs directly in Sanads.

Use:

```rust
pub struct AttachmentRef {
    cid: ContentAddress,
    media_type: MediaType,
    size: u64,
    hash: ContentHash,
}
```

---

## 4.6 Add Resource Accounting

Every verification path MUST calculate:

```rust
VerificationCost {
    cpu,
    memory,
    io,
    recursion_depth,
}
```

Reject pathological content.

---

# PHASE 5 — PROOF SYSTEM REDESIGN

# Goal

Unify proof semantics.

---

## 5.1 Create Formal Proof Taxonomy

Create:

```rust
pub enum Proof {
    Inclusion,
    Finality,
    Ownership,
    Transition,
    Replay,
    Execution,
    ZK,
    Composite,
}
```

---

## 5.2 Remove Proof Duplication

Search/delete:

```text
proof_bundle
proof_material
proof_pipeline
proof_context
```

Consolidate.

---

## 5.3 Build Proof DAG System

Proofs must become composable DAGs.

Create:

```rust
pub struct ProofNode {
    id: ProofId,
    dependencies: Vec<ProofId>,
    proof: Proof,
}
```

---

## 5.4 Create Canonical Verifier

Create:

```text
/crates/csv-verifier/
```

ALL verification routes through:

```rust
Verifier::verify()
```

Adapters NEVER verify protocol semantics.

---

## 5.5 Add Verification Context

Create:

```rust
pub struct VerificationContext {
    protocol_version: ProtocolVersion,
    chain_capabilities: CapabilitySet,
    trusted_roots: Vec<TrustedRoot>,
    replay_policy: ReplayPolicy,
    finality_policy: FinalityPolicy,
}
```

Mandatory for ALL verification.

---

# PHASE 6 — REPLAY & FINALITY

# Goal

Fix distributed consistency.

---

## 6.1 Create Replay Constitution

Create:

```text
/docs/replay-model.md
```

Define:

- replay domains
- replay scope
- replay invalidation
- rollback semantics
- chain-local replay
- cross-chain replay

---

## 6.2 Centralize Replay Registry

Create:

```text
/crates/csv-protocol/src/replay/
```

ONLY protocol controls replay.

Adapters cannot override.

---

## 6.3 Create Finality Abstraction

Create:

```rust
pub enum FinalityType {
    Probabilistic,
    Economic,
    Checkpoint,
    Quorum,
    Instant,
}
```

---

## 6.4 Chain Capability Model

Create:

```rust
pub trait ChainCapabilities {
    fn supports_state_proofs();
    fn supports_finality_proofs();
    fn supports_event_proofs();
    fn supports_objects();
}
```

NO semantic flattening.

---

## 6.5 Add Reorg Simulation Tests

Create:

```text
/tests/reorg/
```

Simulate:

- deep reorgs
- delayed finality
- replay rollback
- proof invalidation
- partial state corruption

---

# PHASE 7 — CONTRACT HARDENING

# Goal

Freeze immutable chain anchors safely.

---

## 7.1 Rewrite Contract Semantics

Contracts MUST:

- anchor commitments
- anchor replay nullifiers
- anchor proof roots
- anchor schema ids
- emit canonical events
- enforce replay uniqueness

NOT business logic.

---

## 7.2 Add ABI Constitution

Create:

```text
/docs/contracts/ABI_CONSTITUTION.md
```

Freeze:

- event ordering
- field ordering
- event hashing
- serialization
- topic indexing

---

## 7.3 Create Canonical Event Schema

ALL chains emit identical semantic events.

Example:

```rust
event SanadAnchored {
    protocol_version,
    schema_id,
    content_root,
    proof_root,
    replay_nullifier,
    transition_type,
}
```

---

## 7.4 Add Contract Equivalence Tests

Create:

```text
/tests/contracts_equivalence/
```

Verify:

- all chains emit same semantics
- same hashes
- same replay behavior
- same commitment rules

---

## 7.5 Deployment Framework

Create:

```text
/deployment/
```

Add:

```text
manifest.json
chain-configs/
checksums/
reproducibility/
```

Deployment must verify:

- bytecode checksum
- RPC consistency
- chain ID
- ABI compatibility
- deployment provenance

---

# PHASE 8 — RUNTIME MINIMIZATION

# Goal

Prevent runtime from becoming consensus layer.

---

## 8.1 Remove Protocol Semantics from Runtime

DELETE from runtime:

- replay semantics
- proof semantics
- state legality
- finality rules
- verification logic

Move to:

```text
csv-protocol
csv-verifier
```

---

## 8.2 Runtime ONLY Coordinates

Runtime responsibilities:

```text
- task scheduling
- orchestration
- retries
- queueing
- workflow execution
```

Nothing else.

---

## 8.3 Add Failure Domains

Create:

```rust
pub enum FailureDomain {
    Rpc,
    Verification,
    Storage,
    Replay,
    Finality,
    Consensus,
    Serialization,
}
```

All failures classified.

---

## 8.4 Add Deterministic Recovery

Recovery must use:

```rust
RecoveryCheckpoint
ReplayCheckpoint
VerificationCheckpoint
```

No implicit reconstruction.

---

# PHASE 9 — WALLET REWRITE

# Goal

Prevent wallet complexity explosion.

---

## 9.1 Separate Rendering from Verification

Wallet UI MUST NEVER verify.

Verification only via:

```text
csv-verifier
```

---

## 9.2 Create Schema Plugin System

Complex Sanads require:

```rust
trait SchemaRenderer {
    fn render();
    fn validate();
    fn sanitize();
}
```

---

## 9.3 Sandbox Attachments

MANDATORY:

- MIME allowlists
- decompression limits
- rendering isolation
- size limits
- recursion limits

---

## 9.4 Add Secure Local Storage

Implement:

```rust
EncryptedStore
IntegrityCheckpoint
RecoverySnapshot
```

---

# PHASE 10 — CLI REWRITE

# Goal

Turn CLI into protocol engineering tool.

---

## 10.1 Add Deterministic Output Modes

Add:

```bash
csv verify --json
csv verify --canonical
csv verify --proof-tree
```

---

## 10.2 Add Protocol Inspection Commands

Implement:

```bash
csv inspect sanad
csv inspect proof
csv inspect replay
csv inspect merkle
```

---

## 10.3 Add Schema Tooling

Implement:

```bash
csv schema validate
csv schema compile
csv schema diff
```

---

# PHASE 11 — DISTRIBUTED SYSTEM HARDENING

# Goal

Define consistency semantics explicitly.

---

## 11.1 Create Consistency Constitution

Create:

```text
/docs/consistency-model.md
```

Define:

- authoritative state
- rollback semantics
- ownership finality
- proof invalidation
- replay rollback
- eventual consistency guarantees

---

## 11.2 Add Byzantine RPC Protection

Implement:

```rust
RpcQuorum
ResponseConsensus
EndpointScoring
```

Reject inconsistent responses.

---

## 11.3 Add Deterministic Event Ordering

MANDATORY:

All event processing uses:

```text
(block_height, tx_index, log_index)
```

No timestamp ordering.

---

# PHASE 12 — TESTING & FORMAL VERIFICATION

# Goal

Make protocol freeze safe.

---

## 12.1 Create Adversarial Test Framework

Create:

```text
/crates/csv-testkit/
```

Capabilities:

- fake chains
- Byzantine RPC
- corrupted proofs
- malformed serialization
- rollback simulation
- delayed finality
- duplicate replay

---

## 12.2 Add State Machine Verification

Generate:

```rust
transition_graph.rs
```

Automatically test:

- all legal transitions
- all forbidden transitions
- liveness
- deadlocks
- rollback legality

---

## 12.3 Add Model Checking

Mandatory:

```text
TLA+
Alloy
```

Models:

- replay safety
- ownership uniqueness
- rollback recovery
- proof consistency

---

## 12.4 Add Differential Verification Testing

Run:

```text
Rust verifier
TS verifier
Go verifier
```

All MUST produce identical results.

---

## 12.5 Add Fuzz Corpus

Create:

```text
/fuzz/corpus/
```

Include:

- malformed proofs
- recursive Merkle trees
- corrupted attachments
- replay duplicates
- invalid signatures
- malformed events
- invalid schemas

---

# PHASE 13 — GOVERNANCE

# Goal

Prevent ecosystem fragmentation.

---

## 13.1 Create Schema Governance

Create:

```text
/docs/governance/schema-governance.md
```

Define:

- schema lifecycle
- schema deprecation
- schema compatibility
- schema IDs
- schema migration

---

## 13.2 Create Hash Governance

Create:

```text
/docs/governance/hash-governance.md
```

No unregistered hash domains.

---

## 13.3 Create Capability Governance

Define:

- how new chains integrate
- required proof guarantees
- capability registration
- compliance testing

---

# PHASE 14 — FINAL FREEZE CHECKLIST

DO NOT FREEZE CORE/RUNTIME UNTIL:

- canonical serialization complete
- hash registry frozen
- proof taxonomy frozen
- verifier centralized
- replay semantics formalized
- finality semantics formalized
- contract ABI frozen
- content model frozen
- differential tests passing
- adversarial tests passing
- formal verification passing
- cross-language vectors stable
- chain equivalence tests stable

---

# FINAL IMPLEMENTATION PRIORITY

EXECUTION ORDER:

```text
1. RFC system
2. serialization
3. hashing
4. verifier centralization
5. replay/finality
6. protocol extraction
7. proof redesign
8. Sanad redesign
9. contracts rewrite
10. runtime minimization
11. wallet rewrite
12. distributed hardening
13. formal verification
14. freeze core/runtime
15. ecosystem expansion
```

---

# ABSOLUTE RULES

## RULE 1

NO duplicate verification logic.

---

## RULE 2

NO manual serialization.

---

## RULE 3

NO protocol semantics in adapters.

---

## RULE 4

NO runtime ownership of protocol invariants.

---

## RULE 5

NO mutable protocol hashes after freeze.

---

## RULE 6

NO chain-specific semantic divergence.

---

## RULE 7

NO complex Sanads before Merkleized content.

---

## RULE 8

NO ecosystem integrations before verifier stabilization.

---

# END

