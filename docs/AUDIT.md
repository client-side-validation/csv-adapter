# CSV Protocol — Production Reality Audit (Replacement)

Version: 2.0
Date: May 2026
Audience: Principal engineers, maintainers, protocol architects, runtime engineers

---

# 0. Executive Reality Check

The repository is no longer in the “prototype pretending to be production” phase.
It already contains:

- meaningful protocol separation
- state-machine direction
- replay semantics
- multi-chain abstraction attempts
- reorg handling primitives
- compile-fail invariant tests
- runtime orchestration scaffolding
- explorer/indexer decomposition
- protocol documentation with actual invariant thinking

However:

**the system is still architecturally inconsistent in several critical dimensions.**

The original AUDIT.md solved the *obvious* category mistakes:

- bool-based verification
- scalar assurance collapse
- chain-name branching
- missing replay semantics
- missing rollback model
- weak capability typing
- non-deterministic transfer state progression
- adapter leakage into orchestration

Those are mostly addressed structurally.

The remaining problems are more dangerous because they are subtle:

- distributed consistency failures
- capability drift between docs/config/runtime
- false determinism assumptions
- adapter trust inflation
- RPC consensus ambiguity
- runtime split-brain risks
- proof provenance incompleteness
- state persistence atomicity gaps
- operational unsafety
- incomplete adversarial testing
- type-safe architecture with non-type-safe execution semantics

This document only discusses:

- what is still wrong
- why it is wrong
- concrete fixes
- exact implementation direction
- protocol-level consequences if ignored

No praise. No summaries. No generic recommendations.

---

# 1. The Biggest Remaining Architectural Problem

# Runtime Coordination Is Still Not Actually Single-Authority

The repository introduces `csv-runtime` and a `transfer_coordinator`, but the system still behaves as if:

- adapters are partially autonomous
- orchestration is advisory
- state mutation authority is not singular

This creates the exact class of bugs the runtime split was intended to eliminate.

## Evidence

Current structure still allows:

- adapter-local retry logic
- adapter-local proof verification decisions
- adapter-local RPC fallback behavior
- adapter-local confirmation interpretation
- adapter-local finality progression
- explorer/indexer independent chain interpretation
- wallet-triggered runtime bypass paths

The repository is therefore **logically centralized but operationally federated**.

That is the worst possible middle-ground.

---

# Required Fix

## Introduce Transfer Epoch Ownership

Every transfer must have exactly one active runtime authority.

Create:

`csv-runtime/src/lease.rs`

```rust
pub struct TransferLease {
    pub transfer_id: TransferId,
    pub epoch: u64,
    pub owner_runtime_id: RuntimeId,
    pub acquired_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
}
```

Runtime operations MUST require:

```rust
pub struct RuntimeExecutionContext {
    pub lease: TransferLease,
    pub runtime_instance: RuntimeId,
}
```

Every mutating operation:

- proof validation
- rollback
- mint authorization
- replay consumption
- retry scheduling
- finality transition

must verify lease ownership.

---

# Why This Matters

Without execution leases:

- two runtimes can both advance state
- explorer reprocessors can trigger duplicate transitions
- retry workers can race rollback workers
- HA deployments become unsafe
- Kubernetes restarts create split-brain progression

The current design assumes process-level uniqueness.
Production systems cannot assume this.

---

# Required Storage Contract

The replay DB and transfer store must support:

```sql
SELECT ... FOR UPDATE SKIP LOCKED
```

SQLite is therefore no longer acceptable for runtime coordination.

SQLite may remain:

- explorer local cache
- wallet cache
- demo/testing mode

But runtime coordination requires:

- PostgreSQL
- CockroachDB
- FoundationDB
- or etcd-backed lease coordination

---

# 2. The Three "Unsolved Problems" Actually Have Concrete Solutions

The previous audit treated three problems as fundamentally unsolved.
They are not unsolved.
They are merely expensive.

---

# Problem 1 — RPC Disagreement Consensus

## Current Situation

The system detects disagreement.
It does not actually resolve disagreement.

`RpcDisagreement` currently behaves like a terminal ambiguity.

That is insufficient.

---

# Correct Model: Byzantine RPC Quorum

Replace “RPC fallback” with weighted quorum consensus.

Create:

`csv-core/src/rpc/quorum.rs`

```rust
pub struct RpcObservation {
    pub endpoint: Url,
    pub chain_id: String,
    pub observed_height: u64,
    pub observed_hash: Hash,
    pub observed_finality: FinalityStrength,
    pub latency_ms: u64,
    pub timestamp: DateTime<Utc>,
}
```

Create:

```rust
pub struct QuorumDecision {
    pub canonical_hash: Hash,
    pub agreeing_nodes: usize,
    pub disagreeing_nodes: usize,
    pub confidence: f64,
}
```

---

# Required Rules

## Bitcoin

Require:

- longest accumulated work
- not longest height

Current architecture does not explicitly enforce this.
That is catastrophic.

A malicious RPC can present a higher-height lower-work fork.

---

## Ethereum

Use:

- finalized checkpoint root
- not latest block agreement

---

## Solana

Require:

- slot agreement
- commitment agreement
- blockhash agreement

because slot numbers alone are insufficient.

---

# Runtime Policy

If quorum confidence falls below threshold:

- freeze transfer progression
- mark transfer degraded
- persist quorum evidence
- retry later

Do NOT fallback to:

- fastest node
- first node
- majority by height

All three are unsafe.

---

# Problem 2 — Reorg Safety Beyond Local Observation

## Current State

Reorg handling exists.
Rollback exists.
Reconciliation exists.

But the runtime still assumes:

> “if we did not observe a reorg, it did not happen.”

This is false.

Cold restarts invalidate this assumption.

---

# Correct Solution: Canonical Chain Snapshot Anchoring

Persist:

```rust
pub struct FinalityAnchor {
    pub chain: ChainId,
    pub finalized_height: u64,
    pub finalized_hash: Hash,
    pub cumulative_work: Option<U256>,
    pub finalized_at: DateTime<Utc>,
}
```

On restart:

1. load latest anchor
2. re-query chain state
3. compare ancestor continuity
4. detect silent historical reorg
5. force rollback if mismatch

---

# Missing Critical Feature

The runtime currently lacks:

## historical continuity proofs

Without this:

- restart safety is incomplete
- long-range reorg detection is probabilistic
- database corruption can fake canonicality

---

# Required Addition

Persist:

- every finalized ancestor hash
- every consumed seal commitment
- proof root lineage

for at least:

- max_safe_reorg_depth * 4

---

# Problem 3 — Offline Verification Trust Bootstrap

The docs discuss offline verification.
The implementation still cheats.

Offline verification is meaningless unless:

- trust roots are versioned
- trust roots are authenticated
- chain checkpoints are pinned
- proof origin is attestable

Right now offline verification is only “offline cryptography”.
That is not enough.

---

# Correct Solution: Verification Trust Packages

Create:

`csv-core/src/trust_package.rs`

```rust
pub struct TrustPackage {
    pub chain_id: ChainId,
    pub trusted_checkpoint: Hash,
    pub checkpoint_height: u64,
    pub validator_commitment: Vec<u8>,
    pub generated_at: DateTime<Utc>,
    pub expires_at: DateTime<Utc>,
    pub package_signature: Signature,
}
```

Offline verification must require:

```rust
pub struct OfflineVerificationContext {
    pub trust_package: TrustPackage,
    pub verification_time: DateTime<Utc>,
}
```

---

# Why This Matters

Without trust packages:

offline verification silently degrades into:

> “someone gave me cryptographic blobs.”

That is not verification.
That is deserialization with signatures.

---

# 3. The Runtime/Event Architecture Is Still Incomplete

# Event Bus Is Not Durable

Current `event_bus.rs` appears to model in-process async propagation.

That is insufficient for:

- retries
- recovery
- HA
- crash recovery
- replayable orchestration
- auditability

---

# Required Architecture

The runtime requires:

## append-only event sourcing

Create:

```rust
pub struct RuntimeEventEnvelope {
    pub event_id: Uuid,
    pub transfer_id: TransferId,
    pub causation_id: Option<Uuid>,
    pub correlation_id: Uuid,
    pub event: RuntimeEvent,
    pub timestamp: DateTime<Utc>,
    pub runtime_id: RuntimeId,
}
```

Persist before publish.
Never publish before persist.

---

# Current Risk

Without durable events:

- runtime can emit without persistence
- persistence can happen without publish
- retries can duplicate progression
- rollback can lose causality
- crash windows create invisible state divergence

---

# Required Rule

The transfer state machine must be:

## event-derived

NOT:

## mutable-row-derived

Current architecture is still halfway between both.

Pick one.

Production systems use event-derived state.

---

# 4. Replay Protection Is Still Too Adapter-Centric

The repository correctly introduced replay semantics.
But replay consumption is still chain-local.

That is insufficient.

---

# Problem

A replay registry must be:

- globally canonical
- chain-independent
- causally ordered

Otherwise:

- cross-chain duplicate mint windows remain possible
- rollback races can resurrect seals
- eventual consistency breaks single-use guarantees

---

# Correct Architecture

Create:

```rust
pub struct GlobalReplayRecord {
    pub seal_id: SealId,
    pub originating_chain: ChainId,
    pub consumed_by_transfer: TransferId,
    pub consumption_proof_hash: Hash,
    pub state: ReplayState,
}
```

Where:

```rust
pub enum ReplayState {
    Pending,
    Finalized,
    RolledBack,
    Tombstoned,
}
```

---

# Critical Missing Rule

Replay consumption must be:

## idempotent

Current repository does not consistently enforce this.

Every replay mutation must support:

```rust
consume_if_unconsumed()
```

NOT:

```rust
check_then_consume()
```

The latter is race-prone.

---

# 5. Verification Pipeline Still Over-Trusts Adapters

The repository improved verification typing.
But adapters still retain too much authority.

---

# Adapters Must Become Pure Data Providers

Adapters should:

- fetch proofs
- fetch headers
- fetch state roots
- fetch chain metadata

Adapters should NOT:

- decide final validity
- authorize minting
- determine assurance thresholds
- determine replay status
- decide rollback necessity

Those belong to core/runtime.

---

# Current Smell

Files like:

- `csv-ethereum/src/verifier.rs`
- `csv-bitcoin/src/verifier.rs`
- `csv-solana/src/verifier.rs`

still likely contain protocol decisions.

That is architectural leakage.

---

# Required Refactor

Move all policy logic into:

`csv-core/src/proof_pipeline.rs`

Adapters should only implement:

```rust
trait ProofMaterialProvider {
    async fn fetch_proof_material(...)
        -> Result<ProofMaterialBundle>;
}
```

Verification authority belongs to core.

---

# 6. The Chain Config Layer Is Still Dangerous

The TOML chain configs are operationally unsafe.

Why?

Because runtime-critical semantics still exist outside typed capability enforcement.

Examples:

- confirmation_blocks
- commitment_level
- start_block
- network assumptions

These can drift from:

- docs
- runtime logic
- capability constructors
- adapter assumptions

---

# Required Fix

The TOML files must become:

## operational overlays only

NOT:

## protocol definition sources

---

# Required Rule

Security semantics must originate only from:

```rust
ChainCapabilities
```

Never from config files.

---

# Add Validation

Create:

`csv-core/src/config_validation.rs`

Enforce:

```rust
assert_eq!(
    config.confirmation_blocks,
    capabilities.finality_depth,
)
```

Fail startup on mismatch.

Do not warn.
Fail.

---

# 7. The Explorer Architecture Violates Trust Boundaries

The explorer currently mixes:

- observation
- interpretation
- operational truth

This is extremely dangerous.

---

# Explorer Must Never Be Authoritative

The explorer should:

- index
- cache
- visualize
- aggregate

It must NEVER:

- advance runtime state
- determine replay validity
- determine transfer completion
- decide rollback
- act as canonical source

---

# Current Risk

The existence of:

- sync repositories
- priority indexing
- index-derived status APIs

creates pressure toward explorer-authoritative state.

That must be prevented structurally.

---

# Required Rule

Explorer data must always carry:

```rust
pub enum DataAuthority {
    RuntimeDerived,
    ChainObserved,
    ExplorerInferred,
}
```

The UI must visibly distinguish them.

Otherwise operators will trust inferred state.

---

# 8. SQLite Usage Is Beyond Its Safe Operational Envelope

SQLite appears across:

- explorer
- runtime-adjacent stores
- replay persistence
- sync tracking

SQLite is acceptable for:

- local wallet
- offline tooling
- embedded cache
- tests

It is NOT acceptable for:

- distributed replay coordination
- runtime orchestration
- concurrent rollback
- durable event processing

---

# Required Database Split

## Runtime

Use PostgreSQL.

Require:

- serializable isolation
- advisory locks
- SKIP LOCKED
- transactional event persistence
- partial indexes
- JSONB evidence persistence

---

## Explorer

SQLite acceptable.

---

## Wallet

SQLite acceptable.

---

# 9. State Machine Typestate Is Good — Persistence Semantics Are Not

Compile-fail state tests exist.
That is good.

But the runtime still risks:

## persisted illegal transitions

because persistence is not strongly coupled to typestate transitions.

---

# Required Fix

Persist state transitions atomically with proofs.

Create:

```rust
pub struct PersistedTransition<S1, S2> {
    pub from: PhantomData<S1>,
    pub to: PhantomData<S2>,
    pub transition_event: RuntimeEventEnvelope,
    pub persisted_at: DateTime<Utc>,
}
```

Only persistence layer may construct this.

---

# Critical Missing Invariant

The following must become impossible:

- state updated without proof persisted
- proof persisted without replay update
- replay updated without event persisted

These are currently possible during crash windows.

---

# 10. Proof Bundles Are Missing Provenance Chains

Current proof bundles appear verification-focused.
They are not provenance-complete.

---

# Missing Metadata

Every proof bundle must include:

```rust
pub struct ProofProvenance {
    pub fetched_from: Vec<Url>,
    pub observed_at: DateTime<Utc>,
    pub rpc_quorum_hash: Hash,
    pub runtime_version: String,
    pub adapter_version: String,
    pub trust_package_hash: Option<Hash>,
}
```

---

# Why This Matters

Without provenance:

- incident forensics become impossible
- invalid proofs cannot be traced
- malicious RPC injection cannot be reconstructed
- historical disagreement analysis becomes impossible

---

# 11. CI Is Not Actually Enforcing Architectural Boundaries

The workflows build and lint.
That is not architecture enforcement.

---

# Required CI Tiering

## Tier 1 — Dependency Boundaries

Add:

```bash
cargo deny check bans
cargo hakari verify
```

Disallow:

- runtime importing adapters
- explorer importing runtime
- wallet importing adapters directly

---

# Tier 2 — Compile-Fail Architecture Tests

Add tests for:

- adapter policy leakage
- forbidden bool verifier returns
- forbidden direct hashing
- forbidden unchecked replay consumption

---

# Tier 3 — Adversarial Runtime Simulation

Introduce:

```bash
cargo nextest run --profile chaos
```

with:

- RPC disagreement injection
- crash-after-persist-before-publish
- rollback during minting
- replay double-consume race
- delayed finality
- conflicting checkpoints

---

# Tier 4 — Determinism Certification

All proof pipelines must pass:

```rust
same_input -> identical_output
```

byte-for-byte.

No timestamps.
No unordered maps.
No nondeterministic serialization.

---

# 12. Solana Handling Is Still Conceptually Weak

The repository treats Solana as “low reorg”.
That is too simplistic.

Solana’s problem is not merely reorg depth.
It is:

- optimistic confirmation ambiguity
- RPC inconsistency
- ledger pruning
- slot visibility divergence
- historical data instability

---

# Required Solution

Create explicit:

```rust
pub enum SolanaCommitmentGrade {
    Processed,
    Confirmed,
    Finalized,
}
```

Never collapse these into:

```rust
u64 confirmations
```

That is semantically wrong.

---

# Additional Requirement

Persist:

- slot
- blockhash
- commitment grade
- parent slot

for every observation.

---

# 13. Ethereum Finality Is Still Incorrectly Abstracted

Ethereum finality is not binary.

The current model simplifies:

```rust
FinalizedCheckpoint
```

This is insufficient.

---

# Missing Concepts

Ethereum requires distinction between:

- safe head
- justified checkpoint
- finalized checkpoint

These are operationally different.

---

# Required Addition

```rust
pub enum EthereumFinalityStage {
    UnsafeHead,
    SafeHead,
    Justified,
    Finalized,
}
```

Proof validation should not authorize minting below:

```rust
Finalized
```

unless explicitly operating in degraded mode.

---

# 14. The Runtime Lacks Degraded Mode Semantics

Production systems need explicit degradation behavior.

Currently failures become:

- retries
- errors
- rollback

This is incomplete.

---

# Required State

```rust
pub enum RuntimeHealth {
    Healthy,
    Degraded {
        reason: DegradedReason,
    },
    Unsafe,
}
```

---

# Required Degraded Reasons

- RPC disagreement
- quorum collapse
- historical continuity failure
- replay registry unavailable
- event persistence lag
- clock drift
- partial partition
- trust package expiry

---

# Operational Rule

Unsafe mode must freeze:

- mint authorization
- replay consumption
- proof finalization

but may still allow:

- observation
- indexing
- proof acquisition

---

# 15. Observability Is Still Too Application-Centric

The observability crate exists.
But metrics are still mostly infrastructure-level.

You need protocol-level observability.

---

# Required Metrics

## Verification

```text
verification_failures_total{
  chain,
  reason,
  assurance_level
}
```

---

## Replay

```text
replay_double_consume_attempts_total
```

---

## Runtime

```text
runtime_split_brain_prevented_total
```

---

## Reorg

```text
rollback_depth_histogram
```

---

## RPC

```text
rpc_quorum_disagreement_ratio
```

---

# Missing Critical Capability

The system currently cannot answer:

> “Which transfers were verified under degraded trust assumptions?”

That is unacceptable.

Persist verification context.

---

# 16. Wallet Security Model Is Still Under-Specified

The wallet architecture is broad but insufficiently isolated.

---

# Missing Security Domains

Separate:

- viewing keys
- signing keys
- transfer authorization
- proof verification trust
- replay registry trust

Currently they are conceptually mixed.

---

# Required Capability Separation

Introduce:

```rust
pub struct WalletCapabilities {
    pub can_sign: bool,
    pub can_verify: bool,
    pub can_mint: bool,
    pub can_export_keys: bool,
}
```

---

# Hardware Wallet Support Is Architecturally Missing

The repository still assumes software signing.

Production-grade settlement systems require:

- Ledger
- Trezor
- HSM
- remote signer

support.

---

# Required Refactor

Create:

```rust
trait SigningProvider {
    async fn sign(...);
}
```

Never let runtime depend on local private key material.

---

# 17. The SDK Boundary Is Too Thin

`csv-sdk` currently looks like a convenience layer.

It should become:

## the ONLY public integration boundary

---

# Required Rule

External applications must never import:

- runtime
- adapters
- core internals

Directly.

---

# Required Enforcement

Mark internal modules:

```rust
pub(crate)
```

far more aggressively.

The current repository exposes too much surface area.

---

# 18. The Repository Is Missing Formal Version Negotiation

You have protocol version files.
You do not yet have negotiated compatibility.

---

# Required Addition

```rust
pub struct CompatibilityMatrix {
    pub protocol_version: SemVer,
    pub minimum_runtime_version: SemVer,
    pub minimum_adapter_versions: HashMap<ChainId, SemVer>,
}
```

---

# Runtime Rule

Refuse transfer execution when:

- proof schema version mismatches
- adapter capability version mismatches
- replay semantics differ
- finality semantics differ

---

# 19. Fuzzing Exists But Is Too Narrow

Current fuzzing focuses on parsing.

Production failures will occur in:

- state interaction
- replay races
- rollback sequencing
- malformed causal ordering
- partial persistence

---

# Required Stateful Fuzzing

Add:

```rust
proptest! {
    fn runtime_never_double_consumes(...)
}
```

and:

```rust
arbitrary event ordering
```

for:

- retries
- crashes
- reorgs
- delayed RPCs

---

# 20. The Docs Are Ahead of the Runtime

This is now the biggest meta-problem.

The documentation increasingly describes:

- protocol intent
- invariant expectations
- architecture purity

that the runtime does not yet fully enforce.

This creates:

## documentation-induced trust inflation

which is dangerous.

---

# Required Rule

Every invariant in:

- `PROTOCOL_INVARIANTS.md`
- `OFFLINE_VERIFICATION_MODEL.md`
- `PLAN.md`

must map to:

- compile-fail test
- property test
- runtime assertion
- or integration certification

If not:

the invariant is aspirational, not enforced.

---

# 21. Required New Test Categories

# A. Byzantine RPC Simulation

Simulate:

- inconsistent headers
- invalid Merkle proofs
- stale checkpoints
- conflicting finality
- delayed responses

---

# B. Runtime Crash Windows

Inject crash after:

- replay consume before persist
- persist before publish
- rollback before replay reset
- proof validation before finality update

---

# C. Long-Range Reorg Recovery

Simulate:

- runtime offline during reorg
- restart after canonicality divergence
- replay resurrection attempts

---

# D. Multi-Runtime HA Races

Simulate:

- lease expiry
- split brain
- stale lock ownership
- network partitions

---

# E. Deterministic Proof Generation

Ensure:

same input => same serialized proof bundle

across:

- OS
- architecture
- runtime restarts
- parallel execution

---

# 22. Required Repository Restructuring

The current repo split is directionally correct.
But not yet strict enough.

---

# csv-core

Must contain:

- invariants
- verification semantics
- proof schemas
- replay semantics
- state machine
- rollback semantics
- chain capability model

Must NOT contain:

- RPC
- async networking
- runtime orchestration
- adapter logic

The current RPC quorum code inside core is already borderline.

Prefer trait contracts only.

---

# csv-runtime

Must contain:

- orchestration
- event sourcing
- lease coordination
- retries
- rollback execution
- persistence

Must NOT contain:

- chain-specific logic
- proof policy
- cryptographic verification decisions

---

# adapters

Must become:

## dumb acquisition layers

Nothing else.

---

# explorer

Must become:

## observational only

Never operational.

---

# 23. Hard Truth About Production Readiness

The repository is not “almost production”.

It is:

## architecturally promising but operationally incomplete.

The remaining work is not cosmetic.
It is the hard part:

- distributed consistency
- authority ownership
- runtime determinism
- failure semantics
- operational correctness
- Byzantine tolerance
- replay race elimination
- provenance accountability

The codebase already solved many beginner mistakes.

Now it must solve the problems real distributed systems fail on.

That transition is the difference between:

- a sophisticated prototype
and
- an actual settlement protocol.

---

# 24. Immediate Highest-Priority Action List

In order:

1. Introduce runtime execution leases
2. Replace SQLite runtime coordination
3. Implement durable event sourcing
4. Add RPC quorum consensus engine
5. Add restart-safe finality anchoring
6. Add trust packages for offline verification
7. Make replay consumption idempotent and atomic
8. Remove policy authority from adapters
9. Add degraded/unsafe runtime modes
10. Add adversarial HA/reorg/race testing
11. Add provenance-complete proof bundles
12. Enforce architectural boundaries in CI
13. Add deterministic proof certification
14. Split operational config from protocol semantics
15. Make explorer explicitly non-authoritative

Everything else is secondary.

---

# 25. Final Engineering Principle

The repository currently relies too heavily on:

- type safety
- clean abstractions
- architectural intent
- protocol documentation

Those are necessary.
They are not sufficient.

Distributed settlement systems fail because of:

- crash timing
- partial persistence
- stale observations
- coordination ambiguity
- replay races
- authority confusion
- operational drift
- trust bootstrap failures

The next phase of this repository must therefore optimize for:

## hostile operational reality

not architectural elegance.

Only then does the architecture become trustworthy.

