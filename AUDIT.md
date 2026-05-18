# CSV Protocol — Engineering Plan to Production
**Target: Four publishable repositories — `csv-core` · `csv-runtime` · `csv-adapters` · `csv-apps`**
Version: 1.0 · May 2026

---

## 0. How to Read This Document

This plan is organized into **five sequential phases**. Each phase must be complete before the next begins, because later phases depend on the types and contracts defined in earlier ones. Within a phase, tasks may be parallelized across engineers as noted.

Every task entry specifies:
- The **file(s)** to create or modify
- The **exact implementation** required (not a description — actual function signatures, data types, and logic)
- The **test that proves it is done**

**Scope boundary**: Items from the audits already marked as resolved (`SV-01b`, P2P Nostr, MCP server validation) are not repeated here. This plan covers every remaining gap.

---

## 1. Repository Topology (Target State)

After this plan is complete, the monorepo is split into four separately publishable crates/packages. The split is not a file-copy operation — it is a logical boundary enforced by `Cargo.toml` dependencies.

```
csv-core             (crates.io: csv-core)
│  No external chain dependencies. No network I/O.
│  Contains: protocol invariants, proof formats, transfer state machine,
│            replay registry logic, finality policies, rollback semantics,
│            chain capability model, verification interfaces, typestate types.
│  Consumers: csv-runtime, csv-adapters/*, csv-apps/*

csv-runtime          (crates.io: csv-runtime)  ← NEW CRATE
│  Depends on: csv-core only (no chain adapter imports)
│  Contains: orchestration engine, proof pipeline coordinator,
│            transfer state machine executor, retry engine,
│            rollback executor, event bus, persistent registry.
│  Consumers: csv-apps/*

csv-adapters         (workspace, contains sub-crates)
│  csv-bitcoin    depends on: csv-core, csv-runtime
│  csv-ethereum   depends on: csv-core, csv-runtime
│  csv-solana     depends on: csv-core, csv-runtime
│  csv-aptos      depends on: csv-core, csv-runtime
│  csv-sui        depends on: csv-core, csv-runtime
│  csv-celestia   depends on: csv-core  (DA role only, no runtime dep)
│  csv-stark      depends on: csv-core  (experimental, gated)

csv-apps             (workspace, contains sub-crates)
│  csv-cli        depends on: csv-core, csv-runtime, csv-adapters/*
│  csv-wallet     depends on: csv-core, csv-runtime, csv-adapters/*
│  csv-explorer   depends on: csv-core, csv-adapters/*  (observational only)
│  csv-sdk        depends on: csv-core, csv-runtime, csv-adapters/*
│  csv-mcp-server depends on: csv-sdk
```

**Layer boundary rules** (enforced in CI with `cargo deny`):
- `csv-core` must never import any adapter or app crate. **Permanent.**
- `csv-runtime` must never import any adapter crate directly. **Phase 1–3 constraint.**
  This boundary is strict during the consolidation phase to force the single-coordinator
  property to actually hold. Revisit after Phase 3 integration tests are green and the
  need for capability-specialized fast paths, proof batching, or chain-aware scheduling
  is demonstrated empirically — not anticipated. Do not relax before then.
- `csv-wallet` and `csv-cli` must never import `csv-bitcoin`, `csv-ethereum`, etc. directly. **Permanent.**
- Explorer indexers must never import `csv-runtime`. **Permanent.**

---

## 2. Phase 1 — csv-core Security Foundation
**Priority: CRITICAL. Must complete before any other phase.**
**Parallelism: Tasks 1.1–1.4 are independent. Task 1.5 depends on 1.1–1.4.**

### 1.1 — Introduce `VerificationAssurance` Type System

**Files**: `csv-core/src/verifier.rs` (replace existing), new file `csv-core/src/verified.rs`

**Design note**: The previous draft used a single ordered enum with scalar discriminants
(`Structural=0, Cryptographic=1, ConsensusBound=2`) and a scalar gate
(`assurance >= ConsensusBound`). This is wrong. Inclusion strength and finality strength
are orthogonal axes, not a linear sequence. Bitcoin SPV can have strong inclusion proof
but probabilistic finality. Ethereum can have a finalized checkpoint but an incomplete
receipt MPT proof. A scalar comparison collapses these dimensions and produces a gate
that can be satisfied by being strong on one axis while weak on the other.

The correct model: `VerifiedComponents` carries typed strength per dimension.
The production gate checks each dimension independently against per-chain minimums
declared in `ChainCapabilities`. The `VerificationAssurance` enum remains useful
as a coarse UI display signal, but **must not be used as the mint authorization gate**.

The current codebase contains `Result<bool>` returns from verification functions. This is the root cause of silent verification bypass. Replace with typed wrappers.

**Create `csv-core/src/verified.rs`**:

```rust
//! Multi-dimensional verification result types.
//! Inclusion strength and finality strength are orthogonal — do not collapse
//! them into a single scalar. The production gate (`meets_chain_thresholds`)
//! checks each component against the per-chain minimum declared in
//! ChainCapabilities, not against a total ordering.

use serde::{Serialize, Deserialize};

/// Coarse assurance level. Useful for UI display and logging.
/// NOT a total ordering suitable for mint authorization — use
/// `VerificationResult::meets_chain_thresholds` for that.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VerificationAssurance {
    /// Proof structure was parsed but no cryptographic check performed.
    Structural,
    /// At least one cryptographic check passed (e.g. Merkle path or signature)
    /// but not all components are verified.
    PartialCryptographic,
    /// All cryptographic checks passed. Finality may still be pending.
    Cryptographic,
    /// All cryptographic checks passed AND finality confirmed per chain policy.
    ConsensusBound,
}

/// Typed strength for inclusion proof verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InclusionStrength {
    /// Not checked.
    None,
    /// Internal checksum only — not cryptographically binding.
    Checksum,
    /// Full Merkle branch or MPT path verified against a block/state root.
    MerklePath,
    /// Merkle path verified AND root anchored to a trusted state (light client).
    AnchoredMerklePath,
}

/// Typed strength for finality verification.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FinalityStrength {
    /// Not checked.
    None,
    /// Probabilistic finality: N confirmations on a PoW chain.
    Probabilistic { confirmations: u64 },
    /// Deterministic finality: BFT certificate or finalized checkpoint.
    Deterministic,
}

/// Per-component verification record. Each field is independently checked.
/// The production gate reads this struct directly — it does not reduce
/// components to a scalar before comparing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerifiedComponents {
    pub inclusion: InclusionStrength,
    pub finality: FinalityStrength,
    pub replay_checked: bool,
    pub ownership_signature: bool,
}

/// A strongly-typed verification result.
/// `valid: false` means the proof was checked and failed — not that it was skipped.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationResult {
    pub valid: bool,
    pub assurance: VerificationAssurance,
    pub verified_components: VerifiedComponents,
    pub error: Option<VerificationFailure>,
}

/// Explicit failure reason. Replaces `Ok(false)`, `Ok(vec![])`, `Err(String)`.
#[derive(Debug, Clone, Serialize, Deserialize, thiserror::Error)]
pub enum VerificationFailure {
    #[error("Inclusion proof Merkle path is invalid")]
    InvalidMerklePath,
    #[error("Proof of work does not meet target")]
    InvalidProofOfWork,
    #[error("Pairing equation check failed (Groth16)")]
    PairingCheckFailed,
    #[error("Seal has already been consumed (replay detected)")]
    ReplayDetected,
    #[error("Required finality depth not reached: need {required}, have {actual}")]
    FinalityNotReached { required: u64, actual: u64 },
    #[error("Chain reorg detected at height {0}")]
    ReorgDetected(u64),
    #[error("RPC nodes disagree on chain state")]
    RpcDisagreement,
    #[error("Required data is missing from proof bundle: {0}")]
    MissingData(String),
    #[error("Chain capability not supported: {0}")]
    UnsupportedCapability(String),
    #[error("Ownership signature verification failed")]
    InvalidOwnershipSignature,
    #[error("Chain ID mismatch: expected {expected}, got {actual}")]
    ChainIdMismatch { expected: String, actual: String },
    #[error("Seal ID mismatch in proof bundle")]
    SealIdMismatch,
}

impl VerificationResult {
    /// Check each component against the per-chain minimums declared in
    /// ChainCapabilities. This is the production mint authorization gate.
    /// Do NOT replace this with a scalar enum comparison.
    pub fn meets_chain_thresholds(
        &self,
        caps: &crate::chain_config::ChainCapabilities,
    ) -> Result<(), VerificationFailure> {
        if !self.valid {
            return Err(self.error.clone()
                .unwrap_or(VerificationFailure::InvalidMerklePath));
        }
        // Check inclusion independently of finality
        if !caps.inclusion_threshold_met(&self.verified_components.inclusion) {
            return Err(VerificationFailure::InvalidMerklePath);
        }
        // Check finality independently of inclusion
        if !caps.finality_threshold_met(&self.verified_components.finality) {
            return Err(VerificationFailure::FinalityNotReached {
                required: caps.finality_depth,
                actual: match self.verified_components.finality {
                    FinalityStrength::Probabilistic { confirmations } => confirmations,
                    FinalityStrength::Deterministic => caps.finality_depth,
                    FinalityStrength::None => 0,
                },
            });
        }
        Ok(())
    }
}
```

**Add to `ChainCapabilities`** — in `csv-core/src/chain_config.rs`:
```rust
impl ChainCapabilities {
    /// Returns true if the observed inclusion strength meets this chain's minimum.
    pub fn inclusion_threshold_met(&self, observed: &InclusionStrength) -> bool {
        match self.proof_model {
            ProofModel::SpvMerkle => matches!(observed,
                InclusionStrength::MerklePath | InclusionStrength::AnchoredMerklePath),
            ProofModel::MerklePatricia => matches!(observed,
                InclusionStrength::MerklePath | InclusionStrength::AnchoredMerklePath),
            ProofModel::AccumulatorPath | ProofModel::CheckpointMerkle => matches!(observed,
                InclusionStrength::MerklePath | InclusionStrength::AnchoredMerklePath),
            ProofModel::SlotConfirmation => matches!(observed,
                InclusionStrength::Checksum    // Solana: slot confirmation is the primitive
                | InclusionStrength::MerklePath),
            ProofModel::DaNamespace => matches!(observed,
                InclusionStrength::MerklePath | InclusionStrength::AnchoredMerklePath),
        }
    }

    /// Returns true if the observed finality strength meets this chain's minimum.
    pub fn finality_threshold_met(&self, observed: &FinalityStrength) -> bool {
        match (&self.finality_model, observed) {
            (FinalityModel::ProofOfWork { confirmations }, FinalityStrength::Probabilistic { confirmations: obs }) =>
                obs >= confirmations,
            (FinalityModel::FinalizedCheckpoint, FinalityStrength::Deterministic) => true,
            (FinalityModel::BftInstant, FinalityStrength::Deterministic) => true,
            (FinalityModel::OptimisticWithSlotExpiry { slots }, FinalityStrength::Probabilistic { confirmations: obs }) =>
                obs >= slots,
            _ => false,
        }
    }
}
```

**Enforcement**: The ban on `Result<bool>` in verification paths is enforced via a
compile-fail test, not grep. Add to `csv-core/tests/compile_fail/`:
```rust
// compile_fail/result_bool_in_verifier.rs
// This must not compile. VerificationResult, not bool, is the return type.
fn bad_verifier() -> Result<bool, ()> { //~ ERROR
    Ok(true)
}
```
Run with `cargo test --test compile_fail`. See §5.1 for the full enforcement tier strategy.

---

### 1.2 — Replace `ChainCapabilities` with Security-Aware Version

**File**: `csv-core/src/chain_config.rs`

The current `ChainCapabilities` only tracks UI features (NFT support, smart contract support). Replace with the security-relevant model required by the target architecture.

**Replace the existing `ChainCapabilities` struct entirely**:

```rust
/// Security-relevant capabilities of a chain adapter.
/// Transfer logic MUST depend on these capabilities, NOT on chain names.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainCapabilities {
    // --- State model ---
    pub state_model: StateModel,

    // --- Finality model ---
    pub finality_model: FinalityModel,
    /// Blocks/slots/checkpoints required for probabilistic finality
    pub finality_depth: u64,
    /// Whether finality is deterministic (BFT) vs probabilistic (PoW/PoS)
    pub deterministic_finality: bool,

    // --- Proof model ---
    pub proof_model: ProofModel,

    // --- Replay protection ---
    pub replay_protection: ReplayProtectionModel,
    /// Whether the chain supports atomic single-use seal semantics natively
    pub native_single_use_semantics: bool,

    // --- Reorg characteristics ---
    pub reorg_risk: ReorgRisk,
    /// Maximum reorg depth the adapter is designed to handle safely
    pub max_safe_reorg_depth: u64,

    // --- Verification capabilities ---
    pub supports_light_client_proofs: bool,
    pub supports_state_proofs: bool,
    pub supports_transaction_inclusion_proofs: bool,
    pub supports_offline_verification: bool,
    pub supports_zk_proofs: bool,

    // --- DA role ---
    pub chain_role: ChainRole,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum StateModel {
    Utxo,           // Bitcoin
    Account,        // Ethereum
    Object,         // Sui
    Resource,       // Aptos (Move resources)
    DataBlob,       // Celestia
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum FinalityModel {
    ProofOfWork { confirmations: u64 },      // Bitcoin
    FinalizedCheckpoint,                     // Ethereum post-merge
    BftInstant,                              // Aptos (HotStuff), Sui (Narwhal)
    OptimisticWithSlotExpiry { slots: u64 }, // Solana
    DataAvailabilityHeader,                  // Celestia
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProofModel {
    SpvMerkle,        // Bitcoin: double-SHA256 Merkle branch + header PoW
    MerklePatricia,   // Ethereum: MPT storage/receipt proof
    AccumulatorPath,  // Aptos: sparse Merkle accumulator
    CheckpointMerkle, // Sui: checkpoint Merkle path
    SlotConfirmation, // Solana: slot-based ledger proof
    DaNamespace,      // Celestia: namespace Merkle proof
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReplayProtectionModel {
    UtxoSpentCheck,          // Bitcoin: UTXO is spent = consumed
    SmartContractNullifier,  // Ethereum: nullifier mapping in contract
    PdaClosed,               // Solana: account closed = consumed
    ResourceDeleted,         // Aptos: Move resource moved away
    ObjectDeleted,           // Sui: object deleted = consumed
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ReorgRisk {
    High,    // Bitcoin (rare but deep reorgs possible)
    Medium,  // Ethereum (finalized checkpoints, but pre-finality risk)
    Low,     // Solana, Aptos, Sui (BFT or near-instant)
    None,    // Celestia (DA only)
}

/// A chain's role in the CSV protocol architecture.
/// Celestia is DA, not Settlement. This distinction must be enforced.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ChainRole {
    Settlement,          // Can hold and transfer value: BTC, ETH, SOL, APT, SUI
    DataAvailability,    // Can post commitment data: Celestia
    Verification,        // Can verify proofs on-chain
}
```

**Add capability constructors** in the same file:

```rust
impl ChainCapabilities {
    pub fn bitcoin() -> Self {
        Self {
            state_model: StateModel::Utxo,
            finality_model: FinalityModel::ProofOfWork { confirmations: 6 },
            finality_depth: 6,
            deterministic_finality: false,
            proof_model: ProofModel::SpvMerkle,
            replay_protection: ReplayProtectionModel::UtxoSpentCheck,
            native_single_use_semantics: true,
            reorg_risk: ReorgRisk::High,
            max_safe_reorg_depth: 6,
            supports_light_client_proofs: true,
            supports_state_proofs: false,
            supports_transaction_inclusion_proofs: true,
            supports_offline_verification: true,
            supports_zk_proofs: false,
            chain_role: ChainRole::Settlement,
        }
    }

    pub fn ethereum() -> Self {
        Self {
            state_model: StateModel::Account,
            finality_model: FinalityModel::FinalizedCheckpoint,
            finality_depth: 2, // epochs; ~12 min
            deterministic_finality: true,
            proof_model: ProofModel::MerklePatricia,
            replay_protection: ReplayProtectionModel::SmartContractNullifier,
            native_single_use_semantics: false,
            reorg_risk: ReorgRisk::Medium,
            max_safe_reorg_depth: 12,
            supports_light_client_proofs: true,
            supports_state_proofs: true,
            supports_transaction_inclusion_proofs: true,
            supports_offline_verification: true,
            supports_zk_proofs: true,
            chain_role: ChainRole::Settlement,
        }
    }

    pub fn solana() -> Self {
        Self {
            state_model: StateModel::Account,
            finality_model: FinalityModel::OptimisticWithSlotExpiry { slots: 32 },
            finality_depth: 32,
            deterministic_finality: false,
            proof_model: ProofModel::SlotConfirmation,
            replay_protection: ReplayProtectionModel::PdaClosed,
            native_single_use_semantics: true,
            reorg_risk: ReorgRisk::Low,
            max_safe_reorg_depth: 32,
            supports_light_client_proofs: false,
            supports_state_proofs: false,
            supports_transaction_inclusion_proofs: true,
            supports_offline_verification: false, // requires slot confirmation
            supports_zk_proofs: false,
            chain_role: ChainRole::Settlement,
        }
    }

    pub fn aptos() -> Self {
        Self {
            state_model: StateModel::Resource,
            finality_model: FinalityModel::BftInstant,
            finality_depth: 1,
            deterministic_finality: true,
            proof_model: ProofModel::AccumulatorPath,
            replay_protection: ReplayProtectionModel::ResourceDeleted,
            native_single_use_semantics: true,
            reorg_risk: ReorgRisk::Low,
            max_safe_reorg_depth: 0,
            supports_light_client_proofs: true,
            supports_state_proofs: true,
            supports_transaction_inclusion_proofs: true,
            supports_offline_verification: true,
            supports_zk_proofs: false,
            chain_role: ChainRole::Settlement,
        }
    }

    pub fn sui() -> Self {
        Self {
            state_model: StateModel::Object,
            finality_model: FinalityModel::BftInstant,
            finality_depth: 1,
            deterministic_finality: true,
            proof_model: ProofModel::CheckpointMerkle,
            replay_protection: ReplayProtectionModel::ObjectDeleted,
            native_single_use_semantics: true,
            reorg_risk: ReorgRisk::Low,
            max_safe_reorg_depth: 0,
            supports_light_client_proofs: true,
            supports_state_proofs: true,
            supports_transaction_inclusion_proofs: true,
            supports_offline_verification: true,
            supports_zk_proofs: false,
            chain_role: ChainRole::Settlement,
        }
    }

    pub fn celestia() -> Self {
        Self {
            state_model: StateModel::DataBlob,
            finality_model: FinalityModel::DataAvailabilityHeader,
            finality_depth: 1,
            deterministic_finality: true,
            proof_model: ProofModel::DaNamespace,
            replay_protection: ReplayProtectionModel::SmartContractNullifier,
            native_single_use_semantics: false,
            reorg_risk: ReorgRisk::None,
            max_safe_reorg_depth: 0,
            supports_light_client_proofs: true,
            supports_state_proofs: false,
            supports_transaction_inclusion_proofs: false,
            supports_offline_verification: false,
            supports_zk_proofs: false,
            chain_role: ChainRole::DataAvailability,
        }
    }

    /// Returns true if this chain may authorize a mint operation.
    /// DA-only chains (Celestia) may never mint.
    pub fn can_authorize_mint(&self) -> bool {
        self.chain_role == ChainRole::Settlement
    }

    /// Returns true if a proof from this chain supports full offline verification.
    pub fn supports_offline(&self) -> bool {
        self.supports_offline_verification && self.supports_transaction_inclusion_proofs
    }
}
```

**Each adapter's `declare_capabilities()` must return the matching constructor above** — not a generic default.

---

### 1.3 — Introduce `ProofState` Typestate and `ReplayId`

**File**: `csv-core/src/proof.rs` (extend existing)

```rust
/// Explicit proof lifecycle stages. A proof may only advance forward.
/// No transfer may mint unless the phase reaches `ConsensusBound`.
/// Authorization for mint is determined by `VerificationResult::meets_chain_thresholds`,
/// not by comparing this enum to `ConsensusBound` directly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ProofPhase {
    Constructed = 0,
    StructuralValidated = 1,
    CryptographicallyValidated = 2,
    FinalityValidated = 3,
    ReplayChecked = 4,
    ConsensusBound = 5,     // Only this phase may authorize mint
}

/// Globally unique transfer identity. Prevents replay across process restarts
/// and across chain reorganizations.
///
/// Every transfer MUST derive a ReplayId before any state transition.
/// The replay database is append-only; a ReplayId already present means
/// the transfer has been seen before.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ReplayId([u8; 32]);

impl ReplayId {
    /// Derive a ReplayId from all inputs that uniquely identify a transfer.
    /// The hash binds together source chain, transaction, seal, transition,
    /// and destination chain so that no two legitimate transfers share an ID.
    pub fn derive(
        source_chain: &str,
        source_txid: &[u8],
        source_output_index: u32,
        seal_id: &[u8],
        transition_id: &[u8],
        destination_chain: &str,
    ) -> Self {
        use sha2::{Sha256, Digest};
        let mut h = Sha256::new();
        // Domain separation prefix
        h.update(b"CSV_REPLAY_ID_V1\x00");
        // Encode each field with length prefix to prevent collisions
        let encode = |h: &mut Sha256, s: &[u8]| {
            h.update(&(s.len() as u32).to_le_bytes());
            h.update(s);
        };
        encode(&mut h, source_chain.as_bytes());
        encode(&mut h, source_txid);
        h.update(source_output_index.to_le_bytes());
        encode(&mut h, seal_id);
        encode(&mut h, transition_id);
        encode(&mut h, destination_chain.as_bytes());
        ReplayId(h.finalize().into())
    }

    pub fn as_bytes(&self) -> &[u8; 32] { &self.0 }
}
```

---

### 1.4 — Replace `FinalityVerifier` Trait in Core

**File**: `csv-core/src/finality/mod.rs`

The `FinalityVerifier` must be separated from inclusion proof verification. Inclusion proves the transaction happened. Finality proves it cannot be reversed.

```rust
use crate::verified::{VerificationResult, VerificationFailure};

/// Proof that a specific chain state has been finalized and cannot be reversed.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalityProof {
    pub chain_id: String,
    pub block_height: u64,
    pub finality_evidence: FinalityEvidence,
    pub confirmations: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FinalityEvidence {
    /// Bitcoin: block header with accumulated work
    CumulativeWork { header_hash: [u8; 32], cumulative_work: u128 },
    /// Ethereum: finalized checkpoint epoch number and root
    FinalizedCheckpoint { epoch: u64, checkpoint_root: [u8; 32] },
    /// Solana: finalized slot with bank hash
    FinalizedSlot { slot: u64, bank_hash: [u8; 32] },
    /// Aptos/Sui: validator-quorum certificate
    ValidatorCertificate { round: u64, certificate_hash: [u8; 32] },
    /// Celestia: DA header inclusion
    DaHeaderInclusion { height: u64, data_hash: [u8; 32] },
}

/// Separate from ChainVerifier. Adapters implement this independently.
/// This trait MUST be implemented before an adapter may be used in production.
pub trait FinalityVerifier: Send + Sync {
    /// Verify that the given block/slot/checkpoint is finalized.
    /// Returns FinalityProof on success; VerificationFailure on any failure.
    /// MUST NOT return Ok if finality is uncertain.
    fn verify_finality(
        &self,
        block_height: u64,
        chain_id: &str,
    ) -> Result<FinalityProof, VerificationFailure>;

    /// The chain capability this verifier satisfies.
    fn capabilities(&self) -> &crate::chain_config::ChainCapabilities;
}
```

---

### 1.5 — Eliminate `default_verifier_registry()` from Non-Test Code

**File**: `csv-core/src/zk_proof.rs`

The function `default_verifier_registry()` registers placeholder verifier keys with zero-length data. Any ZK proof validation using these keys either trivially passes or trivially fails — neither is correct.

**Action**: Move `default_verifier_registry()` to `#[cfg(test)]` scope only.

```rust
// In csv-core/src/zk_proof.rs:
// BEFORE (dangerous): available in any context
pub fn default_verifier_registry() -> ZkVerifierRegistry { ... }

// AFTER: test-only
#[cfg(test)]
pub fn default_verifier_registry() -> ZkVerifierRegistry { ... }

// Add a production constructor that panics if called without real keys:
pub fn verifier_registry_from_chain_config(
    chain_id: &str,
    verifier_key_bytes: &[u8],
) -> Result<ZkVerifierRegistry, ZkError> {
    if verifier_key_bytes.is_empty() {
        return Err(ZkError::InvalidVerifierKey(
            "Verifier key may not be empty in production. \
             Load from chain-anchored registry.".to_string()
        ));
    }
    // ... actual key loading
}
```

**Add CI gate** in `.github/workflows/production-guarantee.yml`:
```yaml
- name: No placeholder verifier keys in production paths
  run: |
    rg "default_verifier_registry\(\)" \
      --glob "!**/tests/**" --glob "!**/test*.rs" \
      csv-core/src csv-sdk/src csv-wallet/src csv-cli/src && exit 1 || true
```

---

### 1.6 — Fix `SignatureScheme::default()` Feature Mismatch

**File**: `csv-core/src/signature.rs`

`SignatureScheme::default()` returns `MlDsa65`, but ML-DSA-65 requires the `pq` feature. When `pq` is not compiled in, verification of any proof signed with the default scheme silently fails. This creates a build-target-specific verification gap.

```rust
// BEFORE (dangerous):
impl Default for SignatureScheme {
    fn default() -> Self { Self::MlDsa65 }
}

// AFTER:
impl Default for SignatureScheme {
    fn default() -> Self {
        // Use Secp256k1 as default. ML-DSA-65 is an opt-in per-chain
        // configuration. See PROTOCOL_INVARIANTS.md §Signature Schemes.
        Self::Secp256k1
    }
}

// Add a const to record the intended post-quantum default:
/// The intended post-quantum default signature scheme.
/// Not yet the runtime default — requires the `pq` feature and explicit
/// per-seal configuration. See PLAN.md §Fancy Task 3.
pub const PQ_DEFAULT_SCHEME: SignatureScheme = SignatureScheme::MlDsa65;
```

**Add to PROTOCOL_INVARIANTS.md**:
> Signature scheme derivation for cross-chain verification: The scheme used to verify ownership of a transfer proof MUST be derived from `CrossChainHashAlgorithm::for_chain(&source_chain)`, NOT from the `scheme` field inside the proof payload. A malicious actor can omit or forge the payload field. The canonical scheme is chain-determined.

---

### Phase 1 — Completion Test

```bash
# All of the following must pass before Phase 2 begins:
cargo test -p csv-core --all-features
cargo clippy -p csv-core -- -D warnings -D clippy::unwrap_used -D clippy::expect_used
scripts/security/check_forbidden_patterns.sh
# Specifically: no Result<bool> in verifier.rs
# default_verifier_registry only in #[cfg(test)]
# SignatureScheme::default() = Secp256k1
```

---

## 3. Phase 2 — Create `csv-runtime` (New Crate)
**Priority: CRITICAL. No adapter or app code may be refactored before this exists.**
**All engineers: this is the primary blocker for parallel work.**

### 2.0 — Crate Scaffolding

Create `csv-runtime/Cargo.toml`:
```toml
[package]
name = "csv-runtime"
version = "0.1.0"
edition = "2021"

[dependencies]
csv-core = { path = "../csv-core" }
tokio = { version = "1", features = ["full"] }
serde = { version = "1", features = ["derive"] }
thiserror = "1"
tracing = "0.1"
rocksdb = { version = "0.21", optional = true }

[features]
default = ["persistent"]
persistent = ["dep:rocksdb"]
```

The crate has **no** dependency on any chain adapter. Adapter-specific logic is injected via trait objects.

---

### 2.1 — Transfer Coordinator (Extracted from `csv-sdk`)

**File**: `csv-runtime/src/transfer_coordinator.rs`

The current transfer orchestration is duplicated across `csv-sdk/src/cross_chain.rs`, `csv-wallet/src/services/blockchain.rs`, and `csv-cli/src/commands/cross_chain/transfer.rs`. This is the canonical, single implementation.

```rust
use csv_core::{
    cross_chain::CrossChainTransfer,
    verified::{VerificationResult, VerificationFailure},
    proof::ReplayId,
    finality::FinalityProof,
};
use crate::{
    replay_db::ReplayDatabase,
    proof_pipeline::ProofPipeline,
    event_bus::{EventBus, TransferEvent},
};

/// The single source of truth for cross-chain transfer execution.
/// All applications (CLI, wallet, SDK) MUST use this coordinator.
/// No application may implement its own transfer execution.
pub struct TransferCoordinator {
    replay_db: Box<dyn ReplayDatabase>,
    event_bus: EventBus,
}

impl TransferCoordinator {
    /// Execute a cross-chain transfer through the complete state machine.
    ///
    /// Preconditions checked by this function:
    /// 1. ReplayId is unique (not in replay_db)
    /// 2. Source chain capabilities permit cross-chain source
    /// 3. Destination chain capabilities permit mint
    ///
    /// This function is the ONLY place that may call `mint_sanad_on_chain`.
    pub async fn execute(
        &self,
        transfer: CrossChainTransfer,
        adapter_registry: &dyn AdapterRegistry,
    ) -> Result<TransferReceipt, TransferCoordinatorError> {
        // Step 1: Compute ReplayId and check for replay
        let replay_id = ReplayId::derive(
            transfer.source_chain.as_str(),
            &transfer.lock_tx_hash,
            transfer.lock_output_index,
            transfer.sanad_id.as_bytes(),
            transfer.transition_id.as_bytes(),
            transfer.destination_chain.as_str(),
        );

        if self.replay_db.contains(&replay_id).await? {
            return Err(TransferCoordinatorError::ReplayDetected(replay_id));
        }

        // Step 2: Verify source chain capabilities
        let src_caps = adapter_registry
            .capabilities(&transfer.source_chain)
            .ok_or(TransferCoordinatorError::UnknownChain(transfer.source_chain.clone()))?;

        if !src_caps.can_authorize_mint() {
            return Err(TransferCoordinatorError::UnsupportedOperation(
                format!("{} cannot be a cross-chain source", transfer.source_chain)
            ));
        }

        // Step 3: Verify destination chain capabilities
        let dst_caps = adapter_registry
            .capabilities(&transfer.destination_chain)
            .ok_or(TransferCoordinatorError::UnknownChain(transfer.destination_chain.clone()))?;

        if !dst_caps.can_authorize_mint() {
            return Err(TransferCoordinatorError::UnsupportedOperation(
                format!("{} cannot be a cross-chain destination", transfer.destination_chain)
            ));
        }

        // Step 4: Lock on source chain and await finality
        self.event_bus.emit(TransferEvent::Locking { transfer_id: transfer.id.clone() }).await;
        let lock_result = adapter_registry
            .lock_sanad(&transfer.source_chain, &transfer)
            .await?;

        self.event_bus.emit(TransferEvent::AwaitingFinality { transfer_id: transfer.id.clone() }).await;
        let finality_proof = self.await_finality(
            &transfer.source_chain,
            lock_result.block_height,
            &src_caps,
            adapter_registry,
        ).await?;

        // Step 5: Build inclusion proof
        self.event_bus.emit(TransferEvent::BuildingProof { transfer_id: transfer.id.clone() }).await;
        let proof_bundle = adapter_registry
            .build_inclusion_proof(&transfer.source_chain, &lock_result)
            .await?;

        // Step 6: Verify proof before mint (full cryptographic + finality + replay)
        let verification = adapter_registry
            .verify_proof_bundle(&proof_bundle, &self.replay_db)
            .await?;

        verification.require_consensus_bound()
        verification.meets_chain_thresholds(&src_caps)
            .map_err(TransferCoordinatorError::VerificationFailed)?;

        // Step 7: Record ReplayId BEFORE minting (prevents duplicate mints on retry)
        self.replay_db.insert(&replay_id).await?;

        // Step 8: Mint on destination chain
        self.event_bus.emit(TransferEvent::Minting { transfer_id: transfer.id.clone() }).await;
        let mint_result = adapter_registry
            .mint_sanad(&transfer.destination_chain, &transfer, &proof_bundle)
            .await?;

        self.event_bus.emit(TransferEvent::Complete {
            transfer_id: transfer.id.clone(),
            mint_tx_hash: mint_result.tx_hash.clone(),
        }).await;

        Ok(TransferReceipt {
            transfer_id: transfer.id,
            replay_id,
            lock_tx_hash: lock_result.tx_hash,
            mint_tx_hash: mint_result.tx_hash,
            proof_bundle,
            finality_proof,
            verification,
        })
    }
}
```

---

### 2.2 — Durable Replay Database

**File**: `csv-runtime/src/replay_db.rs`

The `CrossChainRegistry` in `csv-core` is in-memory only. The `PersistentTransferRegistry`
in `csv-sdk` wraps SQLite correctly, but the wallet bypasses it.

**⚠ INCOMPLETE — requires formal distributed treatment before production.**
The trait below is necessary but not sufficient. Three unresolved problems are called
out explicitly below and must be resolved before any multi-process or multi-region
deployment. Treating this section as "done" after implementing the trait would
introduce a fund-loss risk.

**Unresolved problem 1 — concurrent coordinators**: Two coordinator processes checking
the same ReplayId simultaneously both find it absent, both proceed to `insert`, both
attempt to mint. A local `RocksDB` insert is not a distributed compare-and-swap. The
trait's `insert` MUST provide compare-and-swap semantics — return an error if the key
already exists at the moment of write, not just at the moment of read.

**Unresolved problem 2 — partial failure between insert and mint**: The coordinator
inserts the ReplayId before minting (intentionally — to block duplicate mints on retry).
If the mint then fails, the transfer is permanently poisoned with no recovery path. The
rollback protocol for this case requires: (a) a separate `pending` state before `consumed`,
(b) a timeout-based expiry for `pending` entries, (c) a recovery coordinator that can
promote `pending` → `consumed` after verifying the mint on-chain, or demote
`pending` → `available` after confirming the mint never landed. This protocol is not
specified here and must be designed before production.

**Unresolved problem 3 — Byzantine destination adapter**: The destination chain adapter
returns a `MintReceipt` claiming success. The coordinator currently trusts this. Before
marking a transfer complete, the coordinator must independently verify the mint
transaction is present on-chain — not via the same adapter instance that performed the
mint, but via a quorum verification call against independent RPC nodes.

```rust
use csv_core::proof::ReplayId;
use crate::error::RuntimeError;

/// Behavioral invariants for any ReplayDatabase implementation:
/// 1. Append-only: entries are never deleted, only marked rolled-back.
/// 2. Compare-and-swap insert: `insert` must fail if the key exists at write time,
///    not only at read time. A read-then-write with a gap is not sufficient.
/// 3. Durability: a crash after a successful `insert` must not lose the entry.
/// 4. The implementation must document its behavior under concurrent writers.
///
/// The specific storage backend (RocksDB, SQLite, PostgreSQL with advisory locks,
/// distributed CAS store) is left to the implementation. Do not freeze the backend
/// choice in this interface.
#[async_trait::async_trait]
pub trait ReplayDatabase: Send + Sync {
    /// Returns true if this ReplayId has been seen before.
    /// This check alone is not sufficient for concurrent-safe replay prevention —
    /// use `insert_if_absent` for the atomic check-and-record operation.
    async fn contains(&self, id: &ReplayId) -> Result<bool, RuntimeError>;

    /// Atomically record a ReplayId as pending mint.
    /// Returns `Err(AlreadyExists)` if the key is present at the moment of write.
    /// This is a compare-and-swap operation, not a blind insert.
    /// Callers must handle `AlreadyExists` as a replay attempt, not a transient error.
    async fn insert_if_absent(
        &self,
        id: &ReplayId,
        state: ReplayEntryState,
    ) -> Result<(), ReplayDbError>;

    /// Promote a `Pending` entry to `Consumed` after mint is confirmed on-chain.
    async fn confirm_consumed(&self, id: &ReplayId) -> Result<(), ReplayDbError>;

    /// Mark a `Pending` entry as rolled-back (append-only: entry remains, state changes).
    async fn mark_rolled_back(&self, id: &ReplayId) -> Result<(), RuntimeError>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayEntryState {
    /// Insert recorded; mint has not yet been confirmed on-chain.
    Pending,
    /// Mint confirmed on-chain. Terminal state.
    Consumed,
    /// Transfer failed after insert; recovery coordinator may retry.
    RolledBack,
}

#[derive(Debug, thiserror::Error)]
pub enum ReplayDbError {
    #[error("ReplayId already present — replay attempt or concurrent insert")]
    AlreadyExists,
    #[error("Storage error: {0}")]
    Storage(String),
}

/// The storage backend (RocksDB, SQLite, PostgreSQL + advisory lock, etc.) is
/// an implementation decision that should not be frozen at this phase.
/// Each deployment context (single-process CLI, multi-process server, distributed
/// coordinator) requires a different backend. The trait above specifies the
/// behavioral contract; the implementation must document its concurrency model.
```

---

### 2.3 — `AdapterRegistry` Trait (Dependency Injection for Adapters)

**File**: `csv-runtime/src/adapter_registry.rs`

The runtime does not import any chain adapter directly. Chain adapters register themselves via this trait.

```rust
use csv_core::chain_config::ChainCapabilities;
use crate::transfer_coordinator::{LockResult, MintResult};
use csv_core::verified::VerificationResult;

/// The interface between csv-runtime and csv-adapters.
/// Each adapter implements this for its chain.
/// The TransferCoordinator calls through this — never directly into adapters.
#[async_trait::async_trait]
pub trait ChainAdapter: Send + Sync {
    fn chain_id(&self) -> &str;
    fn capabilities(&self) -> &ChainCapabilities;

    async fn lock_sanad(
        &self,
        transfer: &CrossChainTransfer,
    ) -> Result<LockResult, AdapterError>;

    async fn mint_sanad(
        &self,
        transfer: &CrossChainTransfer,
        proof_bundle: &ProofBundle,
    ) -> Result<MintResult, AdapterError>;

    async fn build_inclusion_proof(
        &self,
        lock_result: &LockResult,
    ) -> Result<ProofBundle, AdapterError>;

    async fn verify_inclusion_proof(
        &self,
        proof: &ProofBundle,
    ) -> Result<VerificationResult, AdapterError>;

    async fn verify_finality(
        &self,
        block_height: u64,
    ) -> Result<FinalityProof, AdapterError>;

    async fn verify_seal_registry(
        &self,
        seal_id: &[u8],
    ) -> Result<SealRegistryStatus, AdapterError>;
}

pub struct AdapterRegistryImpl {
    adapters: std::collections::HashMap<String, Arc<dyn ChainAdapter>>,
}

impl AdapterRegistryImpl {
    pub fn register(&mut self, adapter: Arc<dyn ChainAdapter>) {
        self.adapters.insert(adapter.chain_id().to_string(), adapter);
    }

    pub fn get(&self, chain_id: &str) -> Option<&Arc<dyn ChainAdapter>> {
        self.adapters.get(chain_id)
    }
}
```

---

### 2.4 — Event Bus

**File**: `csv-runtime/src/event_bus.rs`

```rust
#[derive(Debug, Clone)]
pub enum TransferEvent {
    Locking { transfer_id: String },
    AwaitingFinality { transfer_id: String },
    BuildingProof { transfer_id: String },
    ProofReady { transfer_id: String },
    Minting { transfer_id: String },
    Complete { transfer_id: String, mint_tx_hash: String },
    RollbackTriggered { transfer_id: String, reason: String },
    ReplayDetected { transfer_id: String },
    VerificationDowngraded { transfer_id: String, from: VerificationAssurance },
}

/// Emits structured events for observability. Applications subscribe to these
/// to update UI, metrics, and logs. The coordinator never calls UI directly.
pub struct EventBus {
    subscribers: Vec<Box<dyn Fn(TransferEvent) + Send + Sync>>,
}
```

---

### Phase 2 — Completion Test

```bash
cargo test -p csv-runtime --all-features
# Specific: verify no chain adapter import in csv-runtime
cargo deny check --manifest-path csv-runtime/Cargo.toml
# Run a simulated transfer through TransferCoordinator with mock adapter
cargo test -p csv-runtime transfer_coordinator_replay_detection
cargo test -p csv-runtime transfer_coordinator_capability_gate
```

---

## 4. Phase 3 — csv-adapters: Fix All Chain-Specific Gaps
**Priority: CRITICAL. Tasks within this phase may be parallelized by chain.**

### 3.1 — Bitcoin: Real SPV Merkle Verification

**File**: `csv-bitcoin/src/verifier.rs`

**Problem**: `BitcoinVerifier::verify_inclusion` validates a self-computed checksum but does NOT verify the SPV Merkle branch against the block header's `merkle_root`. A crafted proof with valid checksums but invalid Merkle path will pass.

**Complete replacement of the inclusion verification logic**:

```rust
use bitcoin::{
    BlockHeader,
    consensus::deserialize,
    hashes::Hash,
    merkle_tree::TxMerkleNode,
};

pub fn verify_bitcoin_spv_inclusion(
    txid_bytes: &[u8; 32],
    merkle_branch: &[[u8; 32]],          // Each node in the branch
    branch_side_bits: u32,               // Bitmask: 0=left, 1=right for each level
    block_header_bytes: &[u8; 80],
    lock_block_height: u64,
    current_chain_height: u64,
    required_confirmations: u64,
) -> Result<VerificationResult, VerificationFailure> {
    // 1. Deserialize the block header from raw bytes
    let header: BlockHeader = deserialize(block_header_bytes)
        .map_err(|_| VerificationFailure::MissingData(
            "Cannot deserialize 80-byte block header".to_string()
        ))?;

    // 2. Verify proof of work (header hash ≤ target)
    header.validate_pow(header.target())
        .map_err(|_| VerificationFailure::InvalidProofOfWork)?;

    // 3. Compute Merkle root by walking the branch
    let computed_root = walk_merkle_branch(txid_bytes, merkle_branch, branch_side_bits)?;

    // 4. Compare computed root with header's merkle_root field
    if computed_root != header.merkle_root.to_byte_array() {
        return Err(VerificationFailure::InvalidMerklePath);
    }

    // 5. Check confirmation depth
    let confirmations = current_chain_height.saturating_sub(lock_block_height);
    if confirmations < required_confirmations {
        return Err(VerificationFailure::FinalityNotReached {
            required: required_confirmations,
            actual: confirmations,
        });
    }

    Ok(VerificationResult {
        valid: true,
        assurance: VerificationAssurance::ConsensusBound,
        verified_components: VerifiedComponents {
            inclusion_proof: true,
            finality: true,
            replay_checked: false, // Caller must check via ReplayDatabase
            ownership_signature: false,
            merkle_path: true,
        },
        error: None,
    })
}

/// Walk a Bitcoin double-SHA256 Merkle branch.
/// `side_bits`: bit i = 0 means txid is left child at level i,
///              bit i = 1 means txid is right child at level i.
fn walk_merkle_branch(
    txid: &[u8; 32],
    branch: &[[u8; 32]],
    side_bits: u32,
) -> Result<[u8; 32], VerificationFailure> {
    use sha2::{Sha256, Digest};

    let double_sha256 = |a: &[u8; 32], b: &[u8; 32]| -> [u8; 32] {
        let first: [u8; 32] = Sha256::digest(
            &[a.as_slice(), b.as_slice()].concat()
        ).into();
        Sha256::digest(first).into()
    };

    let mut current = *txid;
    for (i, sibling) in branch.iter().enumerate() {
        let is_right = (side_bits >> i) & 1 == 1;
        current = if is_right {
            double_sha256(sibling, &current)
        } else {
            double_sha256(&current, sibling)
        };
    }
    Ok(current)
}
```

**Fix Bitcoin Taproot address encoding** in `csv-wallet/src/chains/bitcoin.rs`:

```rust
// BEFORE (wrong: P2WPKH hex, not a valid address):
// format!("bc1q{}", hex::encode(&pubkey_bytes[..20]))

// AFTER (correct: bech32m Taproot P2TR):
use bitcoin::{
    key::XOnlyPublicKey,
    taproot::TaprootSpendInfo,
    Address, Network,
    secp256k1::Secp256k1,
};

pub fn format_bitcoin_address(pubkey_bytes: &[u8], network: Network) -> Result<String, String> {
    let internal_key = XOnlyPublicKey::from_slice(&pubkey_bytes[..32])
        .map_err(|e| format!("Invalid x-only pubkey: {}", e))?;
    let secp = Secp256k1::verification_only();
    // Key-path-only spend (no script path): tweak with empty merkle root
    let (tweaked_key, _parity) = internal_key.tap_tweak(&secp, None);
    let address = Address::p2tr_tweaked(tweaked_key, network);
    Ok(address.to_string())
}
// Produces bc1p... prefix (bech32m, SegWit v1)
```

**Fix Bitcoin SP1 ZK prover** — `csv-bitcoin/src/zk_prover.rs`:

```rust
// BEFORE: emits fake "SP1_BTC_SPV_" prefix bytes even when sp1_available = true
// AFTER: hard error when real SP1 is unavailable in production builds

#[cfg(not(feature = "sp1"))]
pub fn prove_seal_consumption(...) -> Result<Vec<u8>, ZkError> {
    Err(ZkError::ProverUnavailable(
        "SP1 ZK proving requires the 'sp1' feature. \
         Build with --features sp1 for production proving. \
         This function MUST NOT fall back to fake proof bytes.".to_string()
    ))
}

#[cfg(feature = "sp1")]
pub fn prove_seal_consumption(
    witness: &SpvWitness,
    prover_key: &sp1_sdk::SP1ProvingKey,
) -> Result<Vec<u8>, ZkError> {
    use sp1_sdk::{ProverClient, SP1Stdin};
    let client = ProverClient::new();
    let mut stdin = SP1Stdin::new();
    stdin.write(witness);
    let (proof, _) = client.prove(
        include_bytes!("../elf/csv-bitcoin-spv"),
        stdin,
        prover_key,
    ).map_err(|e| ZkError::ProvingFailed(e.to_string()))?;
    Ok(proof.bytes())
}
```

---

### 3.2 — Ethereum: Real Groth16 and Signature Verification

**File**: `csv-ethereum/src/verifier.rs`

**Problem**: `EthereumVerifier` has simplified/mock Groth16 verification that checks structure but not the pairing equation. Also, `validate_transaction` skips ECDSA signature recovery.

**Replace mock Groth16 verification**:

```rust
// Add to Cargo.toml:
// ark-groth16 = "0.4"
// ark-bn254 = "0.4"
// ark-serialize = "0.4"

use ark_groth16::{Groth16, Proof, VerifyingKey};
use ark_bn254::{Bn254, Fr};
use ark_serialize::CanonicalDeserialize;
use ark_snark::SNARK;

pub fn verify_groth16_proof(
    proof_bytes: &[u8],
    verifying_key_bytes: &[u8],
    public_inputs: &[Fr],
) -> Result<VerificationResult, VerificationFailure> {
    // Deserialize verifying key
    let vk = VerifyingKey::<Bn254>::deserialize_compressed(verifying_key_bytes)
        .map_err(|_| VerificationFailure::MissingData(
            "Cannot deserialize Groth16 verifying key".to_string()
        ))?;

    // Deserialize proof
    let proof = Proof::<Bn254>::deserialize_compressed(proof_bytes)
        .map_err(|_| VerificationFailure::InvalidMerklePath)?;

    // Run the pairing equation check — this is the actual cryptographic verification
    let prepared_vk = Groth16::<Bn254>::process_vk(&vk)
        .map_err(|_| VerificationFailure::PairingCheckFailed)?;

    let valid = Groth16::<Bn254>::verify_with_processed_vk(
        &prepared_vk,
        public_inputs,
        &proof,
    ).map_err(|_| VerificationFailure::PairingCheckFailed)?;

    if !valid {
        return Err(VerificationFailure::PairingCheckFailed);
    }

    Ok(VerificationResult {
        valid: true,
        assurance: VerificationAssurance::Cryptographic,
        verified_components: VerifiedComponents {
            inclusion_proof: false, // MPT proof checked separately
            finality: false,        // Finality checked separately
            replay_checked: false,
            ownership_signature: false,
            merkle_path: true,      // Groth16 encodes the Merkle path
        },
        error: None,
    })
}
```

**Fix `validate_transaction` signature check** in `csv-ethereum/src/ops.rs`:

```rust
// BEFORE: "TODO: Fix signature validation once Alloy API is stable"
// AFTER:

pub async fn validate_transaction(
    &self,
    tx_envelope: &alloy_consensus::TxEnvelope,
) -> Result<[u8; 20], ChainOpError> {
    // recover_signer() verifies the ECDSA signature and returns the sender address.
    // This is the standard Alloy API — no instability concerns.
    let signer_address = tx_envelope
        .recover_signer()
        .map_err(|e| ChainOpError::InvalidInput(
            format!("Transaction signature recovery failed: {}", e)
        ))?;
    Ok(signer_address.into())
}
```

**Fix Ethereum finality to use config** in `csv-ethereum/src/verifier.rs`:

```rust
// BEFORE: hardcoded 12
// if confirmations < 12 { return Err(...) }

// AFTER: read from config
let required_depth = self.config.finality_depth
    .unwrap_or(12); // 12 is correct post-merge default, but config overrides
if confirmations < required_depth {
    return Err(VerificationFailure::FinalityNotReached {
        required: required_depth,
        actual: confirmations,
    });
}
```

**Fix `CSVLock.sol` access control** in `csv-contracts/ethereum/contracts/src/CSVLock.sol`:

```solidity
// Add to state:
address public owner;

// Add to constructor:
constructor(address _mintContract) {
    owner = msg.sender;
    mintContract = _mintContract;
}

// Add modifier:
modifier onlyOwner() {
    require(msg.sender == owner, "CSVLock: caller is not owner");
    _;
}

// Apply to both:
function setLockContract(address _lockContract) external onlyOwner {
    lockContract = _lockContract;
}

function registerNullifier(bytes32 nullifier) external onlyOwner {
    // Only the protocol owner (or a designated relayer after further auth)
    // may register nullifiers. This prevents nullifier-griefing attacks.
    usedNullifiers[nullifier] = true;
}
```

**Fix `CSVMint.sol` `batchMintSanads`** — add the missing `trustedVerifier` check:

```solidity
function batchMintSanads(
    bytes32[] calldata sanadIds,
    bytes32[] calldata commitments,
    // ... other params
) external {
    // MISSING: this check exists in mintSanad but was absent in batchMintSanads
    require(msg.sender == trustedVerifier, "CSVMint: caller is not trusted verifier");
    // ... rest of implementation
}
```

**Fix EIP-1559 fee fields** in `csv-ethereum/src/tx_builder.rs` (or equivalent):

```rust
// Remove legacy gas_price field from all transaction builders.
// Use EIP-1559 fields exclusively:
use alloy::rpc::types::eth::TransactionRequest;

let base_fee = self.rpc.get_block_by_number(BlockNumberOrTag::Pending)
    .await?
    .base_fee_per_gas
    .ok_or(ChainOpError::RpcError("Block has no base fee".to_string()))?;

let priority_fee: u128 = 1_000_000_000; // 1 Gwei

let tx = TransactionRequest::default()
    .max_fee_per_gas(base_fee * 2 + priority_fee)
    .max_priority_fee_per_gas(priority_fee)
    .gas_limit(estimated_gas);
```

---

### 3.3 — Add Bitcoin and Aptos to `mint_sanad_on_chain`

**File**: `csv-sdk/src/cross_chain.rs` (eventually move to `csv-runtime/src/transfer_coordinator.rs`)

The current dispatch missing Bitcoin, Ethereum (partially), and Aptos destinations.

**Bitcoin destination — tapret output**:

```rust
"bitcoin" => {
    #[cfg(feature = "bitcoin")]
    {
        use csv_bitcoin::tapret::{build_tapret_output, TapretCommitment};
        use csv_bitcoin::tx_builder::BitcoinTxBuilder;

        // Create a Taproot commitment output embedding the Sanad proof
        let commitment = TapretCommitment {
            sanad_id: sanad_id.to_bytes(),
            source_proof_hash: proof_bundle.bundle_hash(),
        };
        let (script_pubkey, proof) = build_tapret_output(
            &destination_pubkey,
            &commitment,
            network,
        ).map_err(|e| CrossChainError::ProtocolError(e.to_string()))?;

        let tx = BitcoinTxBuilder::new(rpc)
            .add_output(script_pubkey, mint_amount_sats)
            .build_and_broadcast()
            .await
            .map_err(|e| CrossChainError::MintFailed(e.to_string()))?;

        Ok(MintReceipt { tx_hash: tx.txid().to_string(), proof_of_inclusion: None })
    }
    #[cfg(not(feature = "bitcoin"))]
    Err(CrossChainError::FeatureNotEnabled("bitcoin".to_string()))
}
```

**Aptos destination — Move entry function**:

```rust
"aptos" => {
    #[cfg(feature = "aptos")]
    {
        use csv_aptos::entry_function::EntryFunctionBuilder;
        use csv_aptos::rpc::AptosRpc;

        let entry_fn = EntryFunctionBuilder::new(
            &aptos_contract_address,
            "csv_seal",
            "mint_sanad",
        )
        .arg_bytes32(sanad_id.to_bytes())
        .arg_bytes32(commitment.to_bytes())
        .arg_string(source_chain.as_str())
        .arg_bytes(source_seal_proof.as_bytes())
        .arg_bytes(proof_bundle.to_bytes())
        .build();

        let tx_hash = rpc
            .submit_transaction(&signer, entry_fn)
            .await
            .map_err(|e| CrossChainError::MintFailed(e.to_string()))?;

        Ok(MintReceipt { tx_hash, proof_of_inclusion: None })
    }
    #[cfg(not(feature = "aptos"))]
    Err(CrossChainError::FeatureNotEnabled("aptos".to_string()))
}
```

---

### 3.4 — Fix Solana Hash Algorithm

**File**: `csv-core/src/cross_chain.rs` — `CrossChainHashAlgorithm::for_chain()`

```rust
// BEFORE (wrong: Solana uses SHA256, not Keccak256):
"solana" => CrossChainHashAlgorithm::Keccak256,

// AFTER:
"solana" => CrossChainHashAlgorithm::Sha256,
// Rationale: Solana's ledger hashing uses SHA256. The Keccak256 mapping
// was copied from Ethereum and is incorrect. Verified against
// csv-solana/src/proofs.rs which calls SHA256 for slot hash verification.
```

Also update `csv-solana/src/proofs.rs` to confirm it uses SHA256 and document this alignment explicitly.

---

### 3.5 — Explorer: Sync State Persistence

**File**: `csv-explorer/storage/src/repositories/sync.rs`

All chain indexers return `Ok(0)` from `get_latest_synced_block()`, causing full re-index from genesis on every restart.

**Add SQL schema** to `csv-explorer/storage/src/schema.sql`:
```sql
CREATE TABLE IF NOT EXISTS sync_state (
    chain       TEXT        PRIMARY KEY,
    last_block  BIGINT      NOT NULL DEFAULT 0,
    updated_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

**Implement in `sync.rs`**:
```rust
pub async fn get_latest_synced_block(
    pool: &PgPool,
    chain: &str,
) -> Result<u64, StorageError> {
    let row = sqlx::query!(
        "SELECT last_block FROM sync_state WHERE chain = $1",
        chain
    )
    .fetch_optional(pool)
    .await?;
    Ok(row.map(|r| r.last_block as u64).unwrap_or(0))
}

pub async fn update_synced_block(
    pool: &PgPool,
    chain: &str,
    block: u64,
) -> Result<(), StorageError> {
    sqlx::query!(
        "INSERT INTO sync_state (chain, last_block, updated_at) VALUES ($1, $2, NOW())
         ON CONFLICT (chain) DO UPDATE SET last_block = $2, updated_at = NOW()",
        chain,
        block as i64,
    )
    .execute(pool)
    .await?;
    Ok(())
}
```

**Each chain indexer** must call `get_latest_synced_block` on startup and `update_synced_block` after each block batch. Wire this into `csv-explorer/indexer/src/sync.rs`.

**Explorer indexers are OBSERVATIONAL ONLY** — add to `csv-explorer/api/src/server.rs`:
```rust
// Add response header to all explorer API responses:
response.headers_mut().insert(
    "X-CSV-Authority",
    HeaderValue::from_static("informational-only-not-authoritative"),
);
```

---

### 3.6 — Explorer: Transfer Indexing for Bitcoin, Ethereum, Aptos

**File**: `csv-explorer/indexer/src/bitcoin.rs`:
```rust
async fn index_transfers(&self, block: u64) -> ChainResult<Vec<TransferRecord>> {
    // Parse OP_RETURN outputs matching CSV_XFER_ magic prefix
    let block_data = self.rpc.get_block(block).await?;
    let mut transfers = Vec::new();
    for tx in &block_data.transactions {
        for output in &tx.outputs {
            if output.script.starts_with(b"OP_RETURN CSV_XFER_") {
                if let Ok(record) = parse_csv_transfer_op_return(&output.script) {
                    transfers.push(record);
                }
            }
        }
    }
    Ok(transfers)
}
```

**File**: `csv-explorer/indexer/src/ethereum.rs`:
```rust
async fn index_transfers(&self, block: u64) -> ChainResult<Vec<TransferRecord>> {
    // Filter for CrossChainLock and SanadMinted events
    let lock_logs = self.rpc.get_logs(block, &[sig_cross_chain_lock()]).await?;
    let mint_logs = self.rpc.get_logs(block, &[sig_sanad_minted()]).await?;
    let mut transfers = parse_lock_events(lock_logs)?;
    transfers.extend(parse_mint_events(mint_logs)?);
    Ok(transfers)
}
```

**File**: `csv-explorer/indexer/src/aptos.rs`:
```rust
async fn index_transfers(&self, block: u64) -> ChainResult<Vec<TransferRecord>> {
    let events = self.rpc.get_events_by_type(
        &format!("{}::csv_seal::CrossChainLockEvent", self.module_address),
        block,
    ).await?;
    Ok(events.into_iter().map(parse_aptos_lock_event).collect::<Result<_, _>>()?)
}
```

---

## 5. Phase 4 — csv-apps: Application Layer Fixes
**Priority: HIGH. These are user-facing correctness bugs, some of which are fraud vectors.**

### 4.1 — Wallet: Delete Fake Transfer Functions

**File**: `csv-wallet/src/services/blockchain.rs`

Delete `BlockchainService::transfer_sanad_local()` and `execute_cross_chain_transfer()`. These functions return fake `TransferResult` structs with `"pending"` hashes. **Users initiate transfers; nothing goes on-chain.**

**Replacement**: All wallet transfer calls route to `csv-runtime::TransferCoordinator` via the SDK client:

```rust
// In csv-wallet/src/pages/cross_chain/transfer.rs (replaces inline logic):
pub async fn execute_transfer(
    runtime: &TransferCoordinator,
    adapter_registry: &AdapterRegistryImpl,
    transfer_params: TransferParams,
) -> Result<TransferReceipt, TransferCoordinatorError> {
    let transfer = CrossChainTransfer::from_params(transfer_params);
    runtime.execute(transfer, adapter_registry).await
}
```

---

### 4.2 — Wallet: Use Durable Storage Backend

**File**: `csv-wallet/src/services/blockchain.rs`

```rust
// BEFORE (all state lost on restart):
CsvClientBuilder::new()
    .with_store_backend(csv_sdk::builder::StoreBackend::InMemory)
    .build()

// AFTER (encrypted durable storage):
let storage_path = platform_storage_dir()?.join("csv-wallet-store");
CsvClientBuilder::new()
    .with_store_backend(StoreBackend::Encrypted {
        path: storage_path,
        passphrase: wallet_key_manager.derive_storage_key()?,
    })
    .build()
```

The `EncryptedStorage` infrastructure in `csv-wallet/src/core/storage.rs` already exists. Wire it to the SDK client builder.

---

### 4.3 — Wallet: Fix `ParallelVerifyService` — Currently a No-Op

**File**: `csv-wallet/src/services/parallel_verify.rs`

```rust
// BEFORE (verifies nothing, all seals report valid):
async fn verify_single_seal(&self, _seal: &SealRecord) -> Result<(), SealError> {
    Ok(())
}

// AFTER (real verification via chain adapter):
async fn verify_single_seal(&self, seal: &SealRecord) -> Result<VerificationResult, SealError> {
    let adapter = self.adapter_registry
        .get(seal.chain.as_str())
        .ok_or(SealError::UnknownChain(seal.chain.clone()))?;

    // Check seal is not consumed on-chain
    let status = adapter
        .verify_seal_registry(seal.seal_id.as_bytes())
        .await
        .map_err(|e| SealError::VerificationFailed(e.to_string()))?;

    // Check proof cryptographic validity
    let proof_result = adapter
        .verify_inclusion_proof(&seal.proof_bundle)
        .await
        .map_err(|e| SealError::VerificationFailed(e.to_string()))?;

    // Require at minimum Cryptographic assurance for UI display
    if !proof_result.is_production_acceptable() {
        return Err(SealError::InsufficientAssurance {
            got: proof_result.assurance,
            required: VerificationAssurance::Cryptographic,
        });
    }

    Ok(proof_result)
}

// Use JoinSet for real parallelism:
pub async fn verify_all(&self, seals: &[SealRecord]) -> Vec<SealVerificationStatus> {
    let mut join_set = tokio::task::JoinSet::new();
    for seal in seals {
        let seal = seal.clone();
        let verifier = self.clone();
        join_set.spawn(async move {
            let result = verifier.verify_single_seal(&seal).await;
            SealVerificationStatus { seal_id: seal.seal_id.clone(), result }
        });
    }
    let mut results = Vec::new();
    while let Some(r) = join_set.join_next().await {
        if let Ok(status) = r { results.push(status); }
    }
    results
}
```

---

### 4.4 — Wallet: Fix `PersistentTransferRegistry` Not Used

**File**: `csv-wallet/src/core/wallet.rs`

The wallet uses the in-memory `CrossChainRegistry`, meaning double-spend protection does not survive a restart. It must use `PersistentTransferRegistry`.

```rust
// In wallet initialization:
let registry_path = storage_dir.join("transfer-registry.db");
let persistent_registry = PersistentTransferRegistry::open(&registry_path)?;
persistent_registry.load_into_registry(&mut core_registry)?;
// ... after any transfer execution ...
persistent_registry.save_from_registry(&core_registry)?;
```

---

### 4.5 — Wallet: Seal Consumption Guard

**File**: `csv-wallet/src/pages/seals/consume.rs`

Before showing the Consume button as active, check chain state:

```rust
// Add async check before rendering the consume button:
async fn check_seal_consumable(
    seal: &SealRecord,
    adapter_registry: &AdapterRegistryImpl,
) -> SealConsumableStatus {
    let adapter = match adapter_registry.get(seal.chain.as_str()) {
        Some(a) => a,
        None => return SealConsumableStatus::UnknownChain,
    };
    match adapter.verify_seal_registry(seal.seal_id.as_bytes()).await {
        Ok(SealRegistryStatus::Unspent) => SealConsumableStatus::CanConsume,
        Ok(SealRegistryStatus::Spent) => SealConsumableStatus::AlreadyConsumed,
        Err(_) => SealConsumableStatus::CheckFailed,
    }
}
```

---

### 4.6 — Wallet: QR Code Implementation

**File**: `csv-wallet/src/pages/validate/offline.rs`

```rust
// Add to Cargo.toml: qrcode = "0.13"
use qrcode::{QrCode, EcLevel};
use qrcode::render::svg;

pub fn render_proof_qr(proof_bundle: &ProofBundle) -> String {
    let payload = proof_bundle.to_base64_url();
    let code = QrCode::with_error_correction_level(
        payload.as_bytes(),
        EcLevel::M,
    ).expect("Proof bundle always fits in QR code");

    code.render::<svg::Color>()
        .min_dimensions(200, 200)
        .build()
}
```

---

### 4.7 — Wallet: Proof Export Implementation

**File**: `csv-wallet/src/pages/validate/offline.rs`

```rust
// TODO: Implement proof export functionality
// Replace with:
pub fn export_proof_bundle(proof: &ProofBundle) -> Vec<u8> {
    // Canonical serialization — deterministic, always the same bytes for the same proof
    proof.to_bytes()
}

// For browser WASM targets, trigger a download via web_sys:
#[cfg(target_arch = "wasm32")]
pub fn trigger_browser_download(bytes: &[u8], filename: &str) -> Result<(), JsValue> {
    use web_sys::{Blob, BlobPropertyBag, Url, HtmlAnchorElement};
    use wasm_bindgen::JsCast;
    use js_sys::{Uint8Array, Array};

    let array = Uint8Array::from(bytes);
    let blob_parts = Array::new();
    blob_parts.push(&array);
    let mut props = BlobPropertyBag::new();
    props.type_("application/octet-stream");
    let blob = Blob::new_with_u8_array_sequence_and_options(&blob_parts, &props)?;
    let url = Url::create_object_url_with_blob(&blob)?;

    let document = web_sys::window().unwrap().document().unwrap();
    let a: HtmlAnchorElement = document.create_element("a")?.dyn_into()?;
    a.set_href(&url);
    a.set_download(filename);
    a.click();
    Url::revoke_object_url(&url)?;
    Ok(())
}
```

---

### 4.8 — CLI: Fix Always-False Seal Registry Closure

**File**: `csv-cli/src/commands/validate.rs`

```rust
// BEFORE (never detects double-spends):
let seal_registry = |_seal_id: &[u8]| -> bool { false };
let result = verify_proof(&bundle, seal_registry)?;

// AFTER (queries real chain state):
let chain_id = bundle.anchor_ref.chain_id.as_str();
let adapter = client.chain_runtime()
    .get_adapter(chain_id)
    .ok_or_else(|| CliError::UnknownChain(chain_id.to_string()))?;

let seal_registry = |seal_id: &[u8]| -> bool {
    // Run the async check synchronously from CLI context
    tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(async {
            match adapter.verify_seal_registry(seal_id).await {
                Ok(SealRegistryStatus::Unspent) => false,   // Not consumed = not replayed
                Ok(SealRegistryStatus::Spent) => true,      // Consumed = replay detected
                Err(_) => {
                    // On RPC error, fail safe: treat as consumed (conservative)
                    eprintln!("Warning: Cannot verify seal registry, treating as consumed");
                    true
                }
            }
        })
    })
};

let result = verify_proof(&bundle, seal_registry)?;
// Display the assurance level to the user:
println!("Assurance: {:?}", result.assurance);
if !result.is_production_acceptable() {
    eprintln!("Warning: Proof verification did not reach Cryptographic assurance level");
    std::process::exit(1);
}
```

**Also add `--offline` flag** to `csv-cli validate` — when `--offline` is passed, use `|_| false` (conservative, no network) but display a clear warning that double-spend check was skipped.

---

### 4.9 — SDK: Publish `@csv-protocol/sdk` via WASM

**File**: `csv-sdk/src/lib.rs` and `csv-mcp-server/src/index.ts`

The WASM chain_id bug (SV-04) must be fixed before npm publish. Locate the bug:

```bash
# Find the chain_id WASM bug:
grep -n "chain_id\|ChainId\|wasm_bindgen" csv-sdk/src/*.rs | grep -v "//\|test"
```

The typical issue: `ChainId` is serialized as an opaque Rust struct across the WASM boundary instead of a plain string. Fix:

```rust
// In csv-sdk/src/lib.rs, for the WASM-exported functions:
#[wasm_bindgen]
pub fn create_seal(chain_id: &str, ...) -> Result<JsValue, JsValue> {
    // Accept chain_id as plain &str, not as ChainId struct
    let chain = ChainId::new(chain_id);
    // ...
}
```

After the fix, run `wasm-pack build --target bundler` and verify the TypeScript types match.

---

### 4.10 — MCP Server: Complete All 7 Tool Implementations

**File**: `csv-mcp-server/src/index.ts`

The MCP server defines tool schemas but some implementations are stubs. All 7 tools must have real implementations that call through to the SDK:

```typescript
// Tool: verify_proof
server.tool("verify_proof", {
    proof_bundle_b64: z.string(),
    offline: z.boolean().default(false),
}, async ({ proof_bundle_b64, offline }) => {
    const bundle = ProofBundle.fromBase64(proof_bundle_b64);
    const result = offline
        ? await sdk.verifyProofOffline(bundle)
        : await sdk.verifyProof(bundle);
    return {
        valid: result.valid,
        assurance: result.assurance,
        components: result.verifiedComponents,
        error: result.error ?? null,
    };
});

// Tool: monitor_transfer (real polling, not mock)
server.tool("monitor_transfer", {
    transfer_id: z.string(),
}, async ({ transfer_id }) => {
    const status = await sdk.transfers().getStatus(transfer_id);
    if (!status) throw new Error(`Transfer ${transfer_id} not found`);
    return {
        transfer_id,
        status: status.state,
        lock_tx_hash: status.lockTxHash,
        mint_tx_hash: status.mintTxHash ?? null,
        proof_phase: status.proofPhase,
    };
});
```

Each of the 7 tools must have input validation (already present per audit) and real SDK calls (currently some are stubs).

---

## 6. Phase 5 — CI Enforcement and Deployment Hardening
**Priority: HIGH. Must be complete before any public network deployment.**

### 5.1 — Forbidden Patterns CI Gates

**File**: `.github/workflows/production-guarantee.yml`

Enforcement is organized into four tiers by durability. Grep-based checks are temporary
migration scaffolding, not permanent governance. Each grep check has an explicit
graduation path to a stronger enforcement mechanism.

**Tier 1 — Type-state enforcement (strongest; no CI step required)**
These invariants are enforced at compile time by the type system itself. No CI check
needed because a violation cannot compile.
- `VerificationResult` cannot be constructed by external code (private fields, builder only)
- `ReplayId` cannot be compared with `==` except via `ReplayDatabase::insert_if_absent`
- `FinalityProof` cannot be constructed without a matching `FinalityVerifier` implementation

**Tier 2 — Clippy lints (strong; add to `csv-core`, `csv-runtime`, all adapters)**
```rust
// Add to the crate root of csv-core, csv-runtime, csv-bitcoin, csv-ethereum,
// csv-solana, csv-aptos, csv-sui:
#![deny(clippy::unwrap_used)]
#![deny(clippy::expect_used)]
#![deny(clippy::panic)]
#![deny(unused_must_use)]
// clippy::unwrap_used catches the most common silent-bypass pattern.
// unused_must_use catches discarded VerificationResult values.
```

**Tier 3 — Compile-fail tests (strong; catches misuse of core types)**
```rust
// csv-core/tests/compile_fail/result_bool_in_verifier.rs
// error[E0308]: mismatched types
fn rejects_bool_return() -> Result<bool, ()> { Ok(true) } //~ ERROR

// csv-core/tests/compile_fail/scalar_assurance_comparison.rs  
// Verify that VerificationAssurance cannot be compared with < or >=
// (PartialOrd is not derived — see §1.1)
let a = VerificationAssurance::Cryptographic;
let b = VerificationAssurance::ConsensusBound;
let _ = a < b; //~ ERROR: binary operation `<` cannot be applied
```
Add `compiletest_rs` or use `trybuild` crate. Run in CI as:
```yaml
- name: Compile-fail invariants
  run: cargo test -p csv-core --test compile_fail
```

**Tier 4 — Grep checks (temporary; each has a named graduation target)**
These are migration-period scaffolding. They will produce false positives as the
codebase grows. Each must be replaced by a Tier 1–3 mechanism within one release cycle.

```yaml
jobs:
  security-gates:
    steps:
      # Graduation target: replace with Tier 3 compile-fail test
      - name: "[TEMP] No placeholder verifier keys"
        run: |
          rg "default_verifier_registry\(\)" \
            --glob "!**/tests/**" --glob "!*test*.rs" \
            csv-core/src csv-sdk/src csv-wallet/src csv-cli/src && exit 1 || true

      # Graduation target: replace with SP1 feature-gate that returns hard error
      - name: "[TEMP] No SP1 mock proof bytes in production"
        run: |
          rg "SP1_BTC_SPV_" --glob "!*test*" && exit 1 || true

      # Graduation target: replace with compile-fail test on TransferResult construction
      - name: "[TEMP] No pending placeholder tx hashes"
        run: |
          rg '"pending"' csv-wallet/src csv-cli/src \
            --glob "!*test*" && exit 1 || true

      # Graduation target: replace with sealed SealRegistryCallback trait
      - name: "[TEMP] No always-false seal registry closures"
        run: |
          rg "\|_\| false" csv-core/src csv-sdk/src csv-cli/src \
            --glob "!*test*" && exit 1 || true

      # This one is appropriate as grep — the experimental flag is a string in TOML
      - name: No experimental feature in production Cargo profiles
        run: |
          if grep -r 'features.*experimental' Cargo.toml \
            --include="Cargo.toml" | grep -v '#'; then exit 1; fi

      # Graduation target: replace with ChainRole::DataAvailability type gate in runtime
      - name: "[TEMP] No Celestia in Settlement transfer paths"
        run: |
          rg 'ChainRole::Settlement.*celestia\|celestia.*ChainRole::Settlement' \
            --glob "*.rs" && exit 1 || true

      # This one is appropriate as grep — presence of a function name is unambiguous
      - name: All adapters declare capabilities
        run: |
          for crate in csv-bitcoin csv-ethereum csv-solana csv-aptos csv-sui; do
            grep -rl "fn declare_capabilities" $crate/src/ || \
              (echo "MISSING declare_capabilities in $crate" && exit 1)
          done
```

---

### 5.2 — Deployment Profiles

**File**: `.cargo/config.toml` and each crate's `Cargo.toml`

```toml
# .cargo/config.toml
[profile.production]
inherits = "release"
panic = "abort"
overflow-checks = true
# Production builds MUST have these features disabled:
# - experimental (fake STARK proofs)
# - mock (any mock implementation)

[profile.testnet]
inherits = "release"
# Can include rpc-devnet feature
```

Add runtime profile detection in `csv-runtime/src/lib.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeploymentProfile {
    Local,
    Integration,
    Testnet,
    Staging,
    Production,
}

impl DeploymentProfile {
    /// Returns the minimum per-component thresholds for this deployment profile.
    /// Note: these are per-component requirements, not a scalar enum comparison.
    /// `VerificationAssurance` is a display-only signal; production gating must
    /// use `VerificationResult::meets_chain_thresholds` with `ChainCapabilities`.
    pub fn minimum_inclusion(&self) -> InclusionStrength {
        match self {
            Self::Local | Self::Integration => InclusionStrength::Checksum,
            Self::Testnet | Self::Staging => InclusionStrength::MerklePath,
            Self::Production => InclusionStrength::MerklePath,
        }
    }

    pub fn minimum_finality(&self) -> FinalityStrength {
        match self {
            Self::Local => FinalityStrength::None,
            Self::Integration => FinalityStrength::Probabilistic { confirmations: 1 },
            Self::Testnet | Self::Staging => FinalityStrength::Probabilistic { confirmations: 3 },
            Self::Production => FinalityStrength::Deterministic, // or chain-specific
        }
    }

    pub fn from_env() -> Self {
        match std::env::var("CSV_DEPLOY_PROFILE").as_deref() {
            Ok("production") => Self::Production,
            Ok("staging") => Self::Staging,
            Ok("testnet") => Self::Testnet,
            Ok("integration") => Self::Integration,
            _ => Self::Local,
        }
    }
}
```

---

### 5.3 — Observability Stack

**File**: `csv-observability/src/metrics/mod.rs`

The following events MUST be observable in production. The exact metric names,
label cardinality, and Prometheus library version are implementation decisions —
do not freeze them here. What must be enforced is that every named event has a
corresponding metric emission in the code path where it occurs.

**Required observable events** (wire these; choose your own names and library):
- Proof verification attempt: dimensions = chain, outcome (pass/fail), inclusion strength, finality strength
- Replay detection: dimensions = chain, source of detection (pre-insert check vs insert conflict)
- Rollback triggered: dimensions = chain, trigger reason
- RPC quorum disagreement: dimensions = chain, diverging node count
- Proof pipeline latency: from lock confirmation to mint confirmation, per chain
- Verification component failure: which specific component failed (inclusion/finality/ownership), per chain

**Do not use `lazy_static!`** — use `std::sync::OnceLock` or `std::sync::LazyLock`
(stable since Rust 1.80). `lazy_static` is a dependency that can be eliminated.

**Do not freeze metric label values** in this document. Label cardinality must be
reviewed against your Prometheus cardinality budget before going to production.
`&["chain", "result", "assurance"]` with open-ended `chain` labels is safe at
five chains; review if chain count grows.

---

### 5.4 — Explicit RPC Trust Stance

**This section did not exist in the original plan. It is added because the plan
implied philosophical alignment with "proof-verifying not RPC-trusting" while
specifying implementations that still rely on RPC-delivered evidence. The gap
was not acknowledged. It is acknowledged here.**

**The position**: The quorum RPC model in `csv-runtime` (multiple independent nodes;
disagreement causes rejection) is the Stage 1 operational stance. It is
**RPC-minimized, not RPC-free**. It reduces trust surface compared to a single
trusted node but does not eliminate it. A quorum of colluding or compromised RPC
nodes can still deliver false finality evidence.

**This is an accepted pragmatic position for Stage 1**, not a protocol violation.
Truly trust-minimized verification requires embedded light clients per chain:
- Bitcoin: a header chain validator that follows cumulative PoW from genesis
- Ethereum: a consensus layer client validating BLS signatures from the validator set
- Solana: a ledger hash chain follower
- Aptos/Sui: BFT certificate validator with known validator set rotation

These are multi-month engineering efforts per chain. They are Stage 3 targets, not
Stage 1 requirements. The `FinalityVerifier` trait is the abstraction point for
swapping in light client implementations as they become available — the coordinator
does not need to change.

**Required documentation**: `PROTOCOL_INVARIANTS.md` must include a section titled
"RPC Trust Model" that states this position explicitly, names the quorum parameters,
and lists which chains have light client verification on the roadmap. This is not
optional — the absence of this documentation is what allowed the philosophy-operations
gap to go unnoticed in the original plan.

**The invariant that must hold regardless of RPC trust level**: No transfer completes
without an independent on-chain confirmation of the mint transaction that does not
come from the same adapter instance that submitted it. This is the minimum bar even
under the quorum model.

### 5.4 — Proof Corpus Test Infrastructure

Create the following directory and CI pipeline:

```
proof-corpus/
  valid/
    btc-to-sol-2026-05-01.proof.bin     # Real proof bundle from testnet
    eth-to-sui-2026-05-10.proof.bin
  malformed/
    truncated-header.proof.bin           # 79-byte header (1 byte short)
    wrong-merkle-root.proof.bin          # Valid header, wrong merkle root
    zero-length-proof.proof.bin          # Empty proof bytes
  adversarial/
    replay-attempt.proof.bin             # Valid proof, already consumed seal
    wrong-chain-id.proof.bin             # Chain ID mismatch
    fake-finality.proof.bin              # Finality claimed but not reached
  stale/
    expired-bitcoin-6conf.proof.bin      # Bitcoin proof below 6 confirmations
```

**CI step**:
```yaml
- name: Proof corpus regression
  run: |
    cargo test -p csv-core proof_corpus -- --test-threads=1
    cargo test -p csv-bitcoin spv_corpus -- --test-threads=1
    cargo test -p csv-ethereum mpt_corpus -- --test-threads=1
```

---

### 5.5 — Contracts: Fix Deployment Manifest

**File**: `csv-contracts/ethereum/scripts/update_manifest.rs`

```rust
// BEFORE: literal "TODO" strings in the manifest
// AFTER:
use sha3::{Keccak256, Digest};
use std::fs;

fn compute_bytecode_hash(bytecode_path: &str) -> String {
    let bytes = fs::read(bytecode_path)
        .expect(&format!("Cannot read bytecode from {}", bytecode_path));
    format!("0x{}", hex::encode(Keccak256::digest(&bytes)))
}

// In main():
let lock_hash = compute_bytecode_hash("out/CSVLock.sol/CSVLock.json");
let mint_hash = compute_bytecode_hash("out/CSVMint.sol/CSVMint.json");
let verifier_address = std::env::var("CSV_VERIFIER_ADDRESS")
    .expect("CSV_VERIFIER_ADDRESS must be set before running update_manifest");
```

**File**: `csv-contracts/solana/build.rs`

```rust
// BEFORE: String::new() silent empty placeholder
// AFTER: explicit failure
fn embed_program_bytecode() -> String {
    match std::env::var("ANCHOR_BUILD") {
        Ok(_) => {
            // Run anchor build and embed the result
            std::process::Command::new("anchor")
                .args(["build"])
                .status()
                .expect("anchor build failed");
            let so_path = "target/deploy/csv_seal.so";
            std::fs::read_to_string(so_path)
                .expect(&format!("Anchor build succeeded but {} not found", so_path))
        }
        Err(_) => {
            // Check for committed artifact
            let committed_path = "artifacts/csv_seal.so.b64";
            std::fs::read_to_string(committed_path)
                .expect(
                    "Solana program bytecode not found. Either:\n\
                     1. Set ANCHOR_BUILD=1 to build from source, or\n\
                     2. Commit the compiled artifact to artifacts/csv_seal.so.b64\n\
                     This build MUST NOT produce an empty artifact."
                )
        }
    }
}
```

---

## 7. Adversarial Test Requirements

These tests do not exist yet. Every adapter must pass all of them before being marked production-ready.

### Byzantine RPC Simulator

**File**: `csv-core/src/rpc/quorum_client.rs` (extend existing)

```rust
#[cfg(test)]
pub enum FaultyRpcMode {
    InvalidProof,     // Returns a valid-looking but cryptographically invalid proof
    WrongFinality,    // Returns finality = true when block is not finalized
    PartialState,     // Returns partial/truncated state
    EmptyResult,      // Returns Ok(vec![]) for all queries
    Timeout,          // Never responds
    Reorg,            // Returns a reorged chain view
    FakeReceipt,      // Returns a receipt with wrong log data
}

// Every adapter integration test must include:
#[tokio::test]
async fn test_bitcoin_rejects_byzantine_rpc() {
    for mode in [
        FaultyRpcMode::InvalidProof,
        FaultyRpcMode::WrongFinality,
        FaultyRpcMode::EmptyResult,
        FaultyRpcMode::FakeReceipt,
    ] {
        let faulty_rpc = FaultyBitcoinRpc::new(mode);
        let verifier = BitcoinVerifier::new(faulty_rpc, test_config());
        let result = verifier.verify_inclusion(&test_proof()).await;
        assert!(result.is_err(),
            "BitcoinVerifier must reject faulty RPC mode {:?}", mode);
    }
}
```

### Reorg Simulation Tests (Mandatory)

**File**: `csv-bitcoin/tests/reorg_tests.rs`:
```rust
// Scenarios required before mainnet:
// 1-block reorg, 3-block reorg, 6-block deep reorg, conflicting SPV proof
```

**File**: `csv-ethereum/tests/reorg_tests.rs`:
```rust
// Scenarios: finalized vs non-finalized mismatch, uncle/orphan behavior
```

**File**: `csv-solana/tests/reorg_tests.rs`:
```rust
// Scenarios: optimistic confirmation rollback, fork switch
```

---

## 8. Four-Repository Publication Checklist

Before splitting into four repos, every item must be ✅.

### csv-core Checklist
- [ ] `VerificationAssurance` enum in `verified.rs` — note: NOT `PartialOrd/Ord`, not used as mint gate
- [ ] `VerifiedComponents` with typed `InclusionStrength` and `FinalityStrength` fields
- [ ] `meets_chain_thresholds(&ChainCapabilities)` replaces scalar `require_consensus_bound()`
- [ ] `ChainCapabilities::inclusion_threshold_met()` and `finality_threshold_met()` implemented
- [ ] No `Result<bool>` in security paths — enforced by compile-fail test, not grep
- [ ] `ChainCapabilities` with security-relevant fields (StateModel, FinalityModel, ProofModel, etc.)
- [ ] `ProofPhase` typestate enum in `proof.rs`
- [ ] `ReplayId::derive()` with domain separation
- [ ] `FinalityVerifier` trait with `FinalityProof` return
- [ ] `VerificationFailure` enum (replaces all `Result<bool>` and `Err(String)`)
- [ ] `default_verifier_registry()` moved to `#[cfg(test)]`
- [ ] `SignatureScheme::default()` = `Secp256k1`
- [ ] All compile-fail tests pass (`csv-core/tests/compile_fail/`)
- [ ] Property tests pass (`csv-core/tests/properties/`)
- [ ] Zero `unwrap()` or `expect()` in non-test code
- [ ] Docs: `PROTOCOL_INVARIANTS.md` updated with signature scheme policy

### csv-runtime Checklist
- [ ] `TransferCoordinator` with full 8-step execution pipeline
- [ ] `ReplayDatabase` trait with `insert_if_absent` (CAS) semantics
- [ ] `ReplayEntryState` (Pending / Consumed / RolledBack) state machine
- [ ] Recovery coordinator design document exists (partial-failure-after-insert protocol)
- [ ] Mint confirmation verified independently of submitting adapter (anti-Byzantine-destination)
- [ ] Concurrency model documented in impl (single-process only, or multi-process CAS)
- [ ] `AdapterRegistry` trait (dependency injection interface for adapters)
- [ ] `EventBus` for observability events
- [ ] `DeploymentProfile` with per-component minimum thresholds (not scalar assurance)
- [ ] No imports of any chain adapter crate (`cargo deny` passes)
- [ ] Unit tests: replay detection, capability gating, finality enforcement
- [ ] Integration test: full transfer simulation with mock adapters
- [ ] `PROTOCOL_INVARIANTS.md` RPC Trust Model section written

### csv-adapters Checklist
- [ ] **Bitcoin**: Real SPV Merkle verification (not checksum only); Taproot address `bc1p`; SP1 hard error when unavailable; `declare_capabilities()` returning `ChainCapabilities::bitcoin()`
- [ ] **Ethereum**: Real Groth16 pairing check via `ark-groth16`; ECDSA recovery via `TxEnvelope::recover_signer()`; finality from config not hardcode; EIP-1559 fee fields; `CSVLock.sol` `onlyOwner` on `setLockContract` and `registerNullifier`; `CSVMint.sol` `trustedVerifier` on `batchMintSanads`
- [ ] **Solana**: Hash algorithm corrected to SHA256; committed or built `.so` artifact; Anchor instruction coverage tests (lock, mint, refund)
- [ ] **Aptos**: `mint_sanad_on_chain` arm implemented; Move contract address not placeholder `0x1234`
- [ ] **Sui**: Duplicate Move files removed (one canonical source); `package_id` required not optional
- [ ] **Celestia**: `ChainRole::DataAvailability` declared; never appears in Settlement transfer paths
- [ ] All adapters: `declare_capabilities()` implemented and verified by CI
- [ ] All adapters: Byzantine RPC simulator tests pass
- [ ] All adapters: Reorg simulation tests pass

### csv-apps Checklist
- [ ] **csv-wallet**: `BlockchainService::transfer_sanad_local()` and `execute_cross_chain_transfer()` deleted; `EncryptedStorage` backend wired; `ParallelVerifyService` implements real chain verification; `PersistentTransferRegistry` loaded/saved around every transfer; seal consumption guard; QR code via `qrcode` crate; proof export implemented; MetaMask connect via `web_sys`
- [ ] **csv-cli**: `cmd_validate` queries real seal registry (no `|_| false`); `--offline` flag with warning; assurance level displayed in output
- [ ] **csv-explorer**: All chains persist sync state via `sync_state` table; transfer indexing for Bitcoin/Ethereum/Aptos; `X-CSV-Authority: informational-only` header on all API responses; `get_latest_synced_block()` never returns 0 after first block
- [ ] **csv-sdk**: WASM chain_id bug (SV-04) fixed; `@csv-protocol/sdk` published to npm; deprecated `build_solana_transaction` removed
- [ ] **csv-mcp-server**: All 7 tools have real implementations (not stubs); input validation on all tools

---

## 9. Parallel Work Assignment Map

The following tracks which phases can be worked in parallel once their prerequisites are met.

```
Week 1-2 (all hands):
  Engineer A: Phase 1 tasks (csv-core types)
  Engineer B: Phase 2 scaffolding (csv-runtime crate structure)

Week 2-3:
  Engineer A: Phase 3 — Bitcoin SPV + Taproot (unblocked after Phase 1)
  Engineer B: Phase 3 — Ethereum Groth16 + sig recovery
  Engineer C: Phase 3 — Solana hash fix + contract tests
  Engineer D: Phase 3 — Aptos/Sui mint dispatch + contract cleanup

Week 3-4:
  Engineer A: Phase 4 — Wallet fake functions deleted + storage wired
  Engineer B: Phase 4 — Wallet parallel verify + seal guard
  Engineer C: Phase 4 — CLI validate fix + offline flag
  Engineer D: Phase 4 — Explorer sync state + transfer indexing

Week 4-5:
  All engineers: Phase 5 (CI gates, deployment profiles, observability)
  
Week 5 (integration):
  Full cross-chain transfer test: Bitcoin → Solana on signet/devnet
  Offline verification demo wiring (PLAN.md §Part I)
  scripts/test-cross-chain.sh added to CI
```

---

## 10. What Not to Build During This Plan

Per `docs/PLAN.md` §Part X and the audit findings:

- **Do not implement ZK Pedersen commitments** until Phase 5 is fully complete. Segment D (DeFi identity) is blocked on this; it is a Stage 2 item.
- **Do not implement Atomic Seal Swap** (Bitcoin Tapscript hash-lock) until all Phase 3 chain work is done. Stage 2 conference demo only.
- **Do not enable the `experimental` feature** in any CI profile until `csv-stark` has a real STARK prover implementation (not `MockStarkProver`).
- **Do not add a new chain** until `ChainCapabilities` is enforced by the capability system. Adding a chain via the old system defeats the entire capability architecture.
- **Do not ship the NFT Gallery page** until the chain queries are implemented. Remove it from wallet navigation now; add it back when complete.
- **Do not mark anything as "post-quantum by default"** until ML-DSA-65 is in the default feature set and the `pq` flag is always compiled in for production builds.
```
