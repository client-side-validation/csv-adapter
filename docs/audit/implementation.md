# CSV Protocol — Audit Response & Implementation Plan

**Date:** 2026-05-21  
**Scope:** Agreed findings + concrete code-level implementations for each critical problem.

---

## Audit Verdict

The audit is **substantially correct**. The architectural intent is strong. The enforcement gaps are real and confirmed by code inspection. The two findings where I partially disagree are noted inline. Everything else is verified and actionable.

---

## Disagreements / Partial Disagreements

### Finding 15 — Cryptographic Agility: PARTIALLY DISAGREE

`csv-core/src/tagged_hash.rs` already implements BIP-340-style tagged hashing correctly. `csv_tagged_hash()` is the right primitive and it is used in `ReplayId::derive()`, canonical CBOR digests, and nullifier hashing. The audit is right that `CrossChainHashAlgorithm::hash_bytes()` still does raw hashing — that is the specific gap to fix.

### Finding 7 — Replay Protection "Operationally Weak": PARTIALLY DISAGREE

`csv-runtime/src/replay_db.rs` already specifies the right state machine (`Pending → Consumed → RolledBack`), CAS semantics via `insert_if_absent`, and append-only semantics. The code even documents the three unresolved distributed problems accurately. The gap is not conceptual — it is the missing distributed coordinator lease and the fact that `CrossChainRegistry` (in `csv-core`) is a plain in-memory `HashMap` that is completely independent of `ReplayDatabase`, so the double-spend guard vanishes on restart.

---

## Critical Finding Implementations

---

### 1. `CrossChainRegistry` Is In-Memory — Catastrophic Data Loss on Restart

**Location:** `csv-core/src/cross_chain.rs`

**Problem:** `CrossChainRegistry` uses `HashMap<Hash, CrossChainRegistryEntry>`. Every process restart loses all transfer records. The double-spend guard evaporates.

**Implementation:** Wire `CrossChainRegistry` to `ReplayDatabase`. Registry operations become durable CAS operations.

```rust
// csv-core/src/cross_chain.rs — replace CrossChainRegistry

use crate::proof::ReplayId;

/// Durable cross-chain registry backed by a ReplayDatabase.
/// Every recorded transfer is persisted atomically before control returns.
pub struct CrossChainRegistry<DB: ReplayDatabase> {
    db: DB,
    /// Secondary in-memory index for fast same-session lookup.
    /// Rebuilt from DB on startup via `CrossChainRegistry::restore`.
    session_index: std::sync::RwLock<HashMap<Hash, CrossChainRegistryEntry>>,
}

impl<DB: ReplayDatabase + Sync> CrossChainRegistry<DB> {
    pub fn new(db: DB) -> Self {
        Self {
            db,
            session_index: std::sync::RwLock::new(HashMap::new()),
        }
    }

    /// Restore session index from durable storage at startup.
    pub async fn restore(&self) -> Result<(), RuntimeError> {
        let entries = self.db.load_all_transfers().await?;
        let mut idx = self.session_index.write()
            .map_err(|_| RuntimeError::Internal("lock poisoned".into()))?;
        for entry in entries {
            idx.insert(entry.sanad_id, entry);
        }
        Ok(())
    }

    pub async fn record_transfer(
        &self,
        entry: CrossChainRegistryEntry,
    ) -> Result<(), CrossChainError> {
        // Build a deterministic ReplayId from (source_chain, source_seal, sanad_id)
        let replay_id = ReplayId::derive(
            entry.source_chain.as_str(),
            entry.lock_tx_hash.as_bytes(),
            entry.source_seal.as_bytes(),
            entry.destination_chain.as_str(),
            entry.sanad_id.as_bytes(),
        );

        // Atomic CAS insert — returns AlreadyExists if seen before.
        self.db
            .insert_if_absent(&replay_id, ReplayEntryState::Pending)
            .await
            .map_err(|e| match e {
                ReplayDbError::AlreadyExists => CrossChainError::AlreadyMinted,
                ReplayDbError::Storage(msg) => CrossChainError::StorageFailure(msg),
            })?;

        // Persist entry payload (separate from ReplayId — entry has richer data).
        self.db.store_transfer_entry(&entry).await
            .map_err(|e| CrossChainError::StorageFailure(e.to_string()))?;

        // Confirm consumed after successful payload persist.
        self.db.confirm_consumed(&replay_id).await
            .map_err(|e| CrossChainError::StorageFailure(e.to_string()))?;

        // Update in-memory index.
        let mut idx = self.session_index.write()
            .map_err(|_| RuntimeError::Internal("lock poisoned".into()))?;
        idx.insert(entry.sanad_id, entry);
        Ok(())
    }

    pub fn is_sanad_transferred(&self, sanad_id: &Hash) -> bool {
        self.session_index.read()
            .map(|idx| idx.contains_key(sanad_id))
            .unwrap_or(false)
    }
}
```

**Required addition to `ReplayDatabase` trait:**

```rust
// csv-runtime/src/replay_db.rs — add two methods to the trait

#[async_trait::async_trait]
pub trait ReplayDatabase: Send + Sync {
    // ... existing methods ...

    /// Persist full transfer entry payload for later restore.
    async fn store_transfer_entry(
        &self,
        entry: &CrossChainRegistryEntry,
    ) -> Result<(), RuntimeError>;

    /// Load all persisted transfer entries (called once at startup).
    async fn load_all_transfers(
        &self,
    ) -> Result<Vec<CrossChainRegistryEntry>, RuntimeError>;
}
```

---

### 2. Typed `FinalityGuarantee` — Replacing Opaque `FinalityProof`

**Location:** `csv-core/src/seal_protocol.rs`, `csv-core/src/cross_chain.rs`

**Problem:** `verify_finality()` returns `FinalityProof { finality_data: Vec<u8> }`. The runtime cannot reason about finality semantics — it must blindly trust adapters. Cross-chain transfers can finalize under incompatible trust assumptions.

**Implementation:**

```rust
// csv-core/src/finality.rs  (new file)

/// Canonical finality guarantee — typed, chain-agnostic.
/// Adapters produce this. The runtime reasons about it.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum FinalityGuarantee {
    /// Probabilistic finality: Bitcoin, pre-checkpoint Ethereum
    Probabilistic {
        /// Number of confirmations achieved
        confirmations: u64,
        /// Minimum required by protocol policy
        required: u64,
        /// Estimated reorg probability at this depth (0.0–1.0)
        reorg_probability: f64,
    },

    /// Deterministic finality: Solana root, Aptos quorum cert, Sui checkpoint
    Deterministic {
        /// Checkpoint/ledger hash that covers the anchor
        checkpoint_hash: [u8; 32],
        /// Checkpoint sequence number / ledger version
        sequence: u64,
        /// Quorum size that certified this checkpoint (2f+1 style)
        quorum_weight: Option<u64>,
    },

    /// Economic finality: slashing-backed (future EVM rollups)
    Economic {
        /// USD value of slashable stake backing this finality
        slash_cost_usd_cents: u128,
        /// Challenge period remaining in seconds (0 = challengeable now)
        challenge_window_secs: u64,
    },
}

impl FinalityGuarantee {
    /// Returns true if this guarantee meets the required policy.
    /// The policy is provided by the runtime — not the adapter.
    pub fn meets_policy(&self, policy: &FinalityPolicy) -> bool {
        match (self, policy) {
            (
                FinalityGuarantee::Probabilistic { confirmations, .. },
                FinalityPolicy::MinConfirmations(required),
            ) => confirmations >= required,

            (
                FinalityGuarantee::Deterministic { sequence, .. },
                FinalityPolicy::DeterministicCheckpoint { min_sequence },
            ) => sequence >= min_sequence,

            (
                FinalityGuarantee::Economic { challenge_window_secs, .. },
                FinalityPolicy::EconomicSettlement,
            ) => *challenge_window_secs == 0,

            _ => false, // type mismatch = reject
        }
    }
}

/// Runtime-owned finality policy. Adapters NEVER set this.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FinalityPolicy {
    MinConfirmations(u64),
    DeterministicCheckpoint { min_sequence: u64 },
    EconomicSettlement,
}
```

**Updated `SealProtocol` trait:**

```rust
// csv-core/src/seal_protocol.rs — replace FinalityProof associated type

pub trait SealProtocol {
    type SealPoint;
    type CommitAnchor;
    type InclusionProof;
    // REMOVED: type FinalityProof — was opaque Vec<u8>

    /// Returns a typed FinalityGuarantee the runtime can reason about.
    /// Adapters MUST NOT embed policy decisions here.
    /// Policy evaluation happens in the runtime against FinalityPolicy.
    fn verify_finality(
        &self,
        anchor: Self::CommitAnchor,
    ) -> Result<FinalityGuarantee>;
}
```

**Runtime enforcement:**

```rust
// csv-runtime/src/coordinator.rs — finality check

fn enforce_finality(
    guarantee: &FinalityGuarantee,
    chain: &ChainId,
    policy_registry: &PolicyRegistry,
) -> Result<(), RuntimeError> {
    let policy = policy_registry
        .get(chain)
        .ok_or_else(|| RuntimeError::NoPolicyForChain(chain.clone()))?;

    if !guarantee.meets_policy(policy) {
        return Err(RuntimeError::FinalityNotMet {
            chain: chain.clone(),
            guarantee: guarantee.clone(),
            required: policy.clone(),
        });
    }
    Ok(())
}
```

---

### 3. Adapter Boundary — Core Must Own Verification

**Location:** `csv-core/src/seal_protocol.rs`, all adapter crates

**Problem:** `verify_inclusion()` is implemented per-adapter. The runtime trusts whatever the adapter returns. A malicious or buggy adapter can forge inclusion.

**Implementation:** Adapters return raw chain data. Core owns verification logic.

```rust
// csv-core/src/adapter.rs  (new file — replaces SealProtocol verification methods)

/// What an adapter is allowed to produce — raw, unverified chain data.
/// The core InclusionVerifier validates it.
pub trait ChainAdapter: Send + Sync {
    /// Fetch raw anchor data from the chain. No verification.
    fn fetch_anchor(&self, anchor_id: &AnchorId) -> Result<RawAnchorData>;

    /// Fetch raw inclusion proof bytes from the chain. No verification.
    fn fetch_inclusion_proof(&self, anchor: &RawAnchorData) -> Result<RawInclusionProof>;

    /// Translate chain-native finality signal into typed FinalityGuarantee.
    /// Adapters MAY compute this (they have chain context), but the runtime
    /// then validates it against FinalityPolicy — adapters cannot override policy.
    fn query_finality(&self, anchor: &RawAnchorData) -> Result<FinalityGuarantee>;

    /// Return chain metadata (ID, hash algorithm, signature scheme).
    fn chain_context(&self) -> &ChainContext;
}

/// Raw unverified anchor data from an adapter.
pub struct RawAnchorData {
    pub chain: ChainId,
    pub block_hash: [u8; 32],
    pub block_height: u64,
    pub tx_bytes: Vec<u8>,
}

/// Raw unverified inclusion proof bytes.
pub struct RawInclusionProof {
    pub proof_type: InclusionProofType,
    pub proof_bytes: Vec<u8>,
}

/// Core-owned verifier. Never delegates verification to adapters.
pub struct InclusionVerifier;

impl InclusionVerifier {
    /// Verify a Bitcoin SPV Merkle proof against the block header.
    pub fn verify_bitcoin(
        raw: &RawInclusionProof,
        anchor: &RawAnchorData,
        commitment: &[u8; 32],
    ) -> Result<VerifiedInclusion> {
        let proof: BitcoinMerkleProof = decode_proof(&raw.proof_bytes)?;

        // 1. Parse and validate block header structure
        let header = BitcoinBlockHeader::parse(&anchor.tx_bytes)?;
        if header.hash() != anchor.block_hash {
            return Err(ProtocolError::InclusionProofFailed(
                "block header hash mismatch".into(),
            ));
        }

        // 2. Verify Merkle branch against merkle_root in header
        verify_bitcoin_merkle_branch(
            &proof.txid,
            &proof.merkle_branch,
            &header.merkle_root,
        )?;

        // 3. Verify commitment is in the tx output (tapret or OP_RETURN)
        verify_commitment_in_tx(&proof.txid, commitment, &proof.tx_bytes)?;

        Ok(VerifiedInclusion {
            chain: anchor.chain.clone(),
            block_hash: anchor.block_hash,
            block_height: anchor.block_height,
        })
    }

    pub fn verify_ethereum_mpt(
        raw: &RawInclusionProof,
        anchor: &RawAnchorData,
        commitment: &[u8; 32],
    ) -> Result<VerifiedInclusion> {
        let proof: EthereumMPTProof = decode_proof(&raw.proof_bytes)?;

        // 1. Verify receipt root via MPT proof
        verify_mpt_proof(
            &proof.merkle_nodes,
            &proof.receipt_root,
            &proof.receipt_rlp,
        )?;

        // 2. Verify commitment appears in receipt log
        verify_commitment_in_receipt(&proof.receipt_rlp, commitment)?;

        Ok(VerifiedInclusion {
            chain: anchor.chain.clone(),
            block_hash: anchor.block_hash,
            block_height: anchor.block_height,
        })
    }
}
```

---

### 4. `CanonicalSanadEnvelope` with Protocol Versioning

**Location:** `csv-core/src/sanad.rs` (new type alongside existing `Sanad`)

**Problem:** No protocol_version field, no schema evolution mechanism, Sanad data is account-centric not envelope-centric.

**Implementation:**

```rust
// csv-core/src/envelope.rs  (new file)

/// The canonical, chain-agnostic, version-stable Sanad identity.
/// This is what gets hashed, committed, and verified — not chain account data.
/// Chain contracts store only `envelope_commitment` (the hash of this struct).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CanonicalSanadEnvelope {
    /// Protocol version that defines this envelope's schema.
    /// MUST be checked before decoding any other field.
    pub protocol_version: u16,

    /// Globally unique Sanad identity (content-addressed from body_hash + issuer_id).
    pub sanad_id: [u8; 32],

    /// Semantic type identifier (registered in the schema registry).
    pub sanad_type: TypeId,

    /// Issuer's canonical identity hash.
    pub issuer_id: [u8; 32],

    /// Hash of the Sanad body (rights, metadata, custom fields).
    pub body_hash: [u8; 32],

    /// Root of the metadata Merkle tree.
    pub metadata_root: [u8; 32],

    /// Root of the proof Merkle tree (historical proofs of state transitions).
    pub proof_root: [u8; 32],

    /// Hash of the seal commitment (current chain anchor).
    pub seal_root: [u8; 32],

    /// Monotonic creation timestamp (Unix seconds).
    pub timestamp: u64,

    /// Nonce preventing two identical envelopes with same timestamp.
    pub nonce: u64,

    /// Hashes of parent envelopes (for lineage / provenance DAG).
    pub parent_refs: Vec<[u8; 32]>,

    /// Hashes of dependency envelopes (e.g., rights this Sanad inherits from).
    pub dependency_refs: Vec<[u8; 32]>,

    /// Signature scheme used to sign this envelope.
    pub signature_scheme: SignatureScheme,

    /// Canonical serialization encoding (always CBOR for v1).
    pub canonical_encoding: EncodingType,
}

impl CanonicalSanadEnvelope {
    pub const CURRENT_VERSION: u16 = 1;

    /// Compute the canonical commitment hash.
    /// Identical on every chain and language implementation.
    pub fn commitment(&self) -> [u8; 32] {
        let cbor = to_canonical_cbor(self)
            .expect("CanonicalSanadEnvelope is always CBOR-serializable");
        csv_tagged_hash("sanad-envelope-v1", &cbor)
    }

    /// Validate version before processing.
    pub fn check_version(&self) -> Result<(), ProtocolError> {
        if self.protocol_version > Self::CURRENT_VERSION {
            return Err(ProtocolError::UnsupportedVersion {
                found: self.protocol_version,
                max_supported: Self::CURRENT_VERSION,
            });
        }
        Ok(())
    }
}

/// Version-aware decoder — the only entry point for deserializing envelopes.
pub fn decode_envelope(bytes: &[u8]) -> Result<CanonicalSanadEnvelope, ProtocolError> {
    // Peek at version field before full decode
    let version = peek_protocol_version(bytes)?;
    match version {
        1 => from_canonical_cbor::<CanonicalSanadEnvelope>(bytes),
        v => Err(ProtocolError::UnsupportedVersion {
            found: v,
            max_supported: CanonicalSanadEnvelope::CURRENT_VERSION,
        }),
    }
}

fn peek_protocol_version(bytes: &[u8]) -> Result<u16, ProtocolError> {
    // CBOR map — first key is always "protocol_version" in sorted canonical encoding
    // Safe minimal parse without full decode
    ciborium::de::from_reader::<serde_json::Value, _>(bytes)
        .map_err(|_| ProtocolError::MalformedEnvelope)?
        .get("protocol_version")
        .and_then(|v| v.as_u64())
        .and_then(|v| u16::try_from(v).ok())
        .ok_or(ProtocolError::MalformedEnvelope)
}
```

---

### 5. `CanonicalEvent` with Causality Chains

**Location:** `csv-sdk/src/events.rs` — add protocol-level event type separate from SDK events.

**Problem:** Events have no causality parent, no monotonic sequence, no deterministic event hash.

**Implementation:**

```rust
// csv-core/src/event.rs  (new file)

/// A canonical, deterministically hashable protocol event.
/// Every state transition MUST emit exactly one CanonicalEvent.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CanonicalEvent {
    /// Deterministic event identity: tagged_hash("csv-event-v1", cbor(self minus event_id))
    pub event_id: [u8; 32],

    /// Hash of the event that causally precedes this one (None = genesis).
    pub causality_parent: Option<[u8; 32]>,

    /// Transfer this event belongs to.
    pub transfer_id: [u8; 32],

    /// Monotonically increasing sequence within this transfer.
    pub sequence: u64,

    /// Unix seconds when this event was emitted.
    pub emitted_at: u64,

    /// Event variant.
    pub event_type: EventType,

    /// Tagged hash of the event-specific payload.
    pub payload_hash: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EventType {
    SealCreated,
    SealLocked,
    InclusionVerified,
    FinalityConfirmed,
    TransferComplete,
    SealRolledBack { reorg_depth: u64 },
    ReplayRejected,
    AdapterError { retryable: bool },
}

impl CanonicalEvent {
    /// Construct and self-hash a new event.
    pub fn new(
        causality_parent: Option<[u8; 32]>,
        transfer_id: [u8; 32],
        sequence: u64,
        emitted_at: u64,
        event_type: EventType,
        payload: &[u8],
    ) -> Result<Self, ProtocolError> {
        let payload_hash = csv_tagged_hash("csv-event-payload-v1", payload);

        // Build without event_id first
        let mut event = Self {
            event_id: [0u8; 32],
            causality_parent,
            transfer_id,
            sequence,
            emitted_at,
            event_type,
            payload_hash,
        };

        // Self-hash
        let cbor = to_canonical_cbor(&event)?;
        event.event_id = csv_tagged_hash("csv-event-v1", &cbor);
        Ok(event)
    }
}

/// Append-only event log — the single source of truth for audit reconstruction.
pub trait EventLog: Send + Sync {
    /// Append an event. Returns error if causality_parent doesn't match last event.
    fn append(&self, event: CanonicalEvent) -> Result<(), ProtocolError>;

    /// Read events for a transfer in sequence order.
    fn events_for_transfer(
        &self,
        transfer_id: &[u8; 32],
    ) -> Result<Vec<CanonicalEvent>, ProtocolError>;

    /// Verify the full causality chain for a transfer.
    fn verify_causality(&self, transfer_id: &[u8; 32]) -> Result<(), ProtocolError>;
}
```

---

### 6. Eliminate Raw Hashing in Cross-Chain Operations

**Location:** `csv-core/src/cross_chain.rs` — `CrossChainHashAlgorithm::hash_bytes()`

**Problem:** `hash_bytes()` does raw SHA-256 / Keccak without domain separation. A proof from chain A could be replayed against chain B verification if the content matches.

**Implementation:**

```rust
// csv-core/src/cross_chain.rs — replace hash_bytes()

impl CrossChainHashAlgorithm {
    /// Hash bytes with chain-specific domain separation.
    /// The domain tag binds the hash to a specific chain and operation context.
    pub fn hash_bytes_domain(
        self,
        chain: &ChainId,
        domain: CrossChainDomain,
        bytes: &[u8],
    ) -> Hash {
        // Build domain tag: "csv-cross-chain-v1:{chain}:{domain}"
        let tag = format!(
            "csv-cross-chain-v1:{}:{}",
            chain.as_str(),
            domain.as_str()
        );

        // Apply the chain's native hash, then wrap with tagged_hash for domain separation
        let native_hash = self.raw_hash(bytes);
        let final_hash = csv_tagged_hash(&tag, &native_hash);
        Hash::new(final_hash)
    }

    /// Raw chain-native hash WITHOUT domain separation.
    /// ONLY for use when verifying chain-native Merkle proofs where the
    /// raw hash must match what the chain itself produced.
    pub(crate) fn raw_hash(self, bytes: &[u8]) -> [u8; 32] {
        match self {
            Self::DoubleSha256 => {
                let first = Sha256::digest(bytes);
                Sha256::digest(first).into()
            }
            Self::Sha256 => Sha256::digest(bytes).into(),
            Self::Keccak256 => Keccak256::digest(bytes).into(),
            Self::Sha3_256 => Sha3_256::digest(bytes).into(),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum CrossChainDomain {
    LockEventCommitment,
    StateRoot,
    ProofBinding,
    FinalityAttestation,
}

impl CrossChainDomain {
    fn as_str(self) -> &'static str {
        match self {
            Self::LockEventCommitment => "lock-commitment",
            Self::StateRoot => "state-root",
            Self::ProofBinding => "proof-binding",
            Self::FinalityAttestation => "finality-attestation",
        }
    }
}
```

---

### 7. Distributed Coordinator Lease — Solving Concurrent Coordinator Split-Brain

**Location:** `csv-runtime/src/replay_db.rs` — documented as "Unresolved problem 1"

**Problem:** Two coordinators simultaneously see a ReplayId absent, both proceed to insert + mint.

**Implementation:**

```rust
// csv-runtime/src/coordinator_lease.rs  (new file)

use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// A distributed coordinator lease preventing split-brain double-mints.
/// Only the coordinator holding a valid lease may attempt mints.
///
/// Implementation: Database row with advisory lock (PostgreSQL) or
/// single-writer token (RocksDB with flock).
pub trait CoordinatorLease: Send + Sync {
    /// Attempt to acquire or renew the lease for this coordinator.
    /// Returns Ok(lease_expiry_unix_secs) if acquired.
    /// Returns Err(LeaseConflict { held_by, expires_at }) if another coordinator holds it.
    async fn acquire_or_renew(
        &self,
        coordinator_id: &CoordinatorId,
        ttl: Duration,
    ) -> Result<u64, LeaseError>;

    /// Release the lease explicitly (best-effort, not required for correctness).
    async fn release(&self, coordinator_id: &CoordinatorId) -> Result<(), LeaseError>;

    /// Returns true if the lease is currently held by this coordinator and not expired.
    async fn is_held_by(&self, coordinator_id: &CoordinatorId) -> bool;
}

/// Guard that enforces lease before any mint operation.
pub struct LeaseGuard<'a, L: CoordinatorLease> {
    lease: &'a L,
    coordinator_id: CoordinatorId,
    valid_until: u64,
}

impl<'a, L: CoordinatorLease> LeaseGuard<'a, L> {
    pub async fn acquire(
        lease: &'a L,
        coordinator_id: CoordinatorId,
        ttl: Duration,
    ) -> Result<Self, LeaseError> {
        let valid_until = lease.acquire_or_renew(&coordinator_id, ttl).await?;
        Ok(Self { lease, coordinator_id, valid_until })
    }

    /// Verify lease is still valid before each mint step.
    /// Must be called before: insert_if_absent, mint_sanad, confirm_consumed.
    pub fn assert_valid(&self) -> Result<(), LeaseError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        if now >= self.valid_until {
            return Err(LeaseError::Expired {
                coordinator: self.coordinator_id.clone(),
                expired_at: self.valid_until,
            });
        }
        Ok(())
    }
}

/// Mint coordinator — only path to calling insert_if_absent + mint.
pub struct MintCoordinator<DB: ReplayDatabase, L: CoordinatorLease, M: MintProvider> {
    db: DB,
    lease: L,
    minter: M,
    coordinator_id: CoordinatorId,
    lease_ttl: Duration,
}

impl<DB, L, M> MintCoordinator<DB, L, M>
where
    DB: ReplayDatabase,
    L: CoordinatorLease,
    M: MintProvider,
{
    pub async fn execute_mint(
        &self,
        proof: &CrossChainTransferProof,
    ) -> Result<MintReceipt, RuntimeError> {
        // 1. Acquire lease before any state mutation.
        let guard = LeaseGuard::acquire(&self.lease, self.coordinator_id.clone(), self.lease_ttl)
            .await
            .map_err(RuntimeError::LeaseConflict)?;

        // 2. Derive and insert ReplayId — atomic CAS.
        let replay_id = ReplayId::from_proof(proof);
        guard.assert_valid().map_err(RuntimeError::LeaseExpired)?;
        self.db
            .insert_if_absent(&replay_id, ReplayEntryState::Pending)
            .await
            .map_err(|e| match e {
                ReplayDbError::AlreadyExists => RuntimeError::ReplayAttempt(replay_id),
                ReplayDbError::Storage(s) => RuntimeError::Storage(s),
            })?;

        // 3. Execute mint — lease still required.
        guard.assert_valid().map_err(|e| {
            // Lease expired between insert and mint — log for recovery coordinator.
            tracing::error!(
                replay_id = ?replay_id,
                "Lease expired after insert, before mint — recovery coordinator required"
            );
            RuntimeError::LeaseExpired(e)
        })?;

        let receipt = self.minter
            .mint_sanad(proof)
            .await
            .map_err(|e| {
                // Mint failed — ReplayId is Pending, recovery coordinator must resolve.
                tracing::error!(
                    replay_id = ?replay_id,
                    error = %e,
                    "Mint failed after insert — needs recovery"
                );
                RuntimeError::MintFailed { replay_id, cause: e.to_string() }
            })?;

        // 4. Confirm consumed only after on-chain verification.
        self.db.confirm_consumed(&replay_id).await
            .map_err(|e| RuntimeError::Storage(e.to_string()))?;

        Ok(receipt)
    }
}
```

---

### 8. `VerificationResult` — Replace All `fn verify() -> bool`

**Location:** `csv-core/src/verifier.rs` — `VerificationResult` already exists but returns `is_valid: bool`. Needs a richer failure type.

**Implementation:** `VerificationResult` already has the right shape. The gap is that callers check `result.is_valid` without inspecting error semantics (retryable vs permanent).

```rust
// csv-core/src/verifier.rs — extend VerificationResult

#[derive(Debug, Clone, Serialize)]
pub struct VerificationResult {
    pub is_valid: bool,
    pub level: VerificationLevel,
    pub errors: Vec<VerificationError>,  // CHANGED: typed errors, not Vec<String>
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct VerificationError {
    /// Machine-readable error code for routing.
    pub code: VerificationErrorCode,
    /// Human-readable description.
    pub message: String,
    /// Whether retrying may succeed (transient vs permanent).
    pub retryable: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum VerificationErrorCode {
    SealReplay,
    SignatureInvalid,
    InclusionProofInvalid,
    FinalityNotReached,
    DomainMismatch,
    MalformedProof,
    ProofTooLarge,
    AnchorInvalid,
    InternalError,
}

impl VerificationError {
    pub fn seal_replay(seal_id: &[u8]) -> Self {
        Self {
            code: VerificationErrorCode::SealReplay,
            message: format!("Seal {:?} already consumed — replay attempt", seal_id),
            retryable: false,
        }
    }

    pub fn finality_not_reached(confirmations: u64, required: u64) -> Self {
        Self {
            code: VerificationErrorCode::FinalityNotReached,
            message: format!("{confirmations} confirmations, need {required}"),
            retryable: true,  // Retry after more blocks
        }
    }
}
```

---

### 9. CI Expansion — Cover All Adapter Crates

**Location:** `.github/workflows/architectural-checks.yml`

**Problem:** The unwrap/expect CI check only covers `csv-runtime/src` and `csv-core/src`. All adapter crates (`csv-bitcoin`, `csv-solana`, `csv-ethereum`, etc.) are unguarded.

**Implementation:**

```yaml
# .github/workflows/architectural-checks.yml — replace forbidden-patterns job

  forbidden-patterns:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Check for unwrap() in all production crates
        run: |
          # All non-test, non-example Rust source under src/
          DIRS="csv-core/src csv-runtime/src csv-bitcoin/src csv-solana/src \
                csv-ethereum/src csv-aptos/src csv-sui/src csv-celestia/src \
                csv-sdk/src csv-cli/src"
          FOUND=0
          for dir in $DIRS; do
            if [ -d "$dir" ]; then
              # Exclude #[cfg(test)] blocks — use ripgrep with negative lookahead
              if rg --type rust '\.unwrap\(\)' "$dir" \
                   --glob '!**/tests/**' \
                   --glob '!**/examples/**' \
                   -l; then
                echo "ERROR: unwrap() found in $dir"
                FOUND=1
              fi
            fi
          done
          [ "$FOUND" -eq 0 ] || exit 1

      - name: Check for expect() in all production crates
        run: |
          DIRS="csv-core/src csv-runtime/src csv-bitcoin/src csv-solana/src \
                csv-ethereum/src csv-aptos/src csv-sui/src csv-celestia/src \
                csv-sdk/src csv-cli/src"
          FOUND=0
          for dir in $DIRS; do
            if [ -d "$dir" ]; then
              if rg --type rust '\.expect\(' "$dir" \
                   --glob '!**/tests/**' \
                   --glob '!**/examples/**' \
                   -l; then
                echo "ERROR: expect() found in $dir"
                FOUND=1
              fi
            fi
          done
          [ "$FOUND" -eq 0 ] || exit 1

      - name: Check for raw hashing without domain separation
        run: |
          # Sha256::digest / Keccak256::digest must not appear outside adapter raw_hash()
          if rg --type rust 'Sha256::digest|Keccak256::digest' \
               csv-core/src csv-runtime/src csv-sdk/src \
               --glob '!**/tests/**'; then
            echo "ERROR: Raw hashing found in core/runtime/sdk — use csv_tagged_hash()"
            exit 1
          fi
```

---

### 10. Append-Only `ReplayRecord` with Hash-Chain

**Location:** `csv-runtime/src/replay_db_postgres.rs`

**Problem:** The trait has state transitions but no tamper-evidence. A corrupted DB row cannot be detected.

**Implementation:** Each state transition links to the previous hash.

```rust
// csv-runtime/src/replay_db_postgres.rs — row schema

/// A single immutable append to the replay log.
/// Each row links to the previous row via prev_hash.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReplayLogRow {
    /// Replay ID this row concerns.
    pub replay_id: [u8; 32],

    /// Row sequence (1-based, monotonically increasing per replay_id).
    pub seq: u32,

    /// New state after this transition.
    pub state: ReplayEntryState,

    /// Unix seconds when this row was written.
    pub written_at: u64,

    /// Identity of coordinator that wrote this row.
    pub coordinator_id: String,

    /// Hash of the previous row for this replay_id (None = first row).
    pub prev_hash: Option<[u8; 32]>,

    /// Self-hash: tagged_hash("csv-replay-log-v1", cbor(self minus row_hash))
    pub row_hash: [u8; 32],
}

impl ReplayLogRow {
    pub fn compute_hash(&self) -> [u8; 32] {
        #[derive(Serialize)]
        struct Digest<'a> {
            replay_id: &'a [u8; 32],
            seq: u32,
            state: &'a ReplayEntryState,
            written_at: u64,
            coordinator_id: &'a str,
            prev_hash: Option<&'a [u8; 32]>,
        }
        let d = Digest {
            replay_id: &self.replay_id,
            seq: self.seq,
            state: &self.state,
            written_at: self.written_at,
            coordinator_id: &self.coordinator_id,
            prev_hash: self.prev_hash.as_ref(),
        };
        let cbor = to_canonical_cbor(&d).unwrap_or_default();
        csv_tagged_hash("csv-replay-log-v1", &cbor)
    }

    pub fn verify_integrity(&self) -> bool {
        self.row_hash == self.compute_hash()
    }
}

// PostgreSQL insert (atomic, exploits unique constraint for CAS semantics):
// INSERT INTO replay_log (replay_id, seq, state, written_at, coordinator_id, prev_hash, row_hash)
// VALUES ($1, $2, $3, $4, $5, $6, $7)
// ON CONFLICT (replay_id, seq) DO NOTHING
// RETURNING row_hash;
// -- If no row returned: conflict = AlreadyExists
```

---

## Phase 1 Hardening Checklist (Updated)

| # | Finding | Confirmed | Implementation | Priority |
|---|---------|-----------|----------------|----------|
| 1 | `CrossChainRegistry` in-memory | ✅ | Wire to `ReplayDatabase` (§1 above) | **P0** |
| 2 | Typed `FinalityGuarantee` | ✅ | Replace `FinalityProof` opaque bytes (§2 above) | **P0** |
| 3 | Adapter owns verification | ✅ | `ChainAdapter` + `InclusionVerifier` (§3 above) | **P0** |
| 4 | `unwrap()`/`expect()` in adapters | ✅ | CI expanded to all crates (§9 above) | **P0** |
| 5 | Concurrent coordinator split-brain | ✅ | `CoordinatorLease` + `MintCoordinator` (§7 above) | **P0** |
| 6 | Raw hashing in `CrossChainHashAlgorithm` | ✅ | Domain-separated `hash_bytes_domain()` (§6 above) | **P1** |
| 7 | No protocol versioning in envelope | ✅ | `CanonicalSanadEnvelope` (§4 above) | **P1** |
| 8 | Event causality missing | ✅ | `CanonicalEvent` with parent hash (§5 above) | **P1** |
| 9 | `VerificationResult` weak typing | ✅ | Typed `VerificationError` with retryability (§8 above) | **P1** |
| 10 | Replay log not tamper-evident | ✅ | Hash-chained `ReplayLogRow` (§10 above) | **P2** |
| 15 | Cryptographic agility | Partial | `tagged_hash.rs` is good; fix `hash_bytes()` only | **P1** |
| 7 | Replay protection | Partial | `replay_db.rs` design is good; fix distributed lease | **P0** |

---

## What Is Already Strong (Do Not Regress)

- `csv-core/src/tagged_hash.rs` — BIP-340 tagged hashing, correct and used.
- `csv-core/src/verifier.rs` — 9-step pipeline order is correct.
- `csv-runtime/src/replay_db.rs` — CAS semantics, append-only, documented problems.
- `csv-core/src/nullifier.rs` — double-spend forensic recording is correct.
- `TransferState` enum in `cross_chain.rs` — explicit state machine is the right pattern; extend it to include `LeaseAcquired` and `MintVerified` states.
- CI coverage of `csv-core/src` and `csv-runtime/src` — extend, don't replace.
