# CSV Protocol — AI Agent Engineering Plan 2026

**Classification:** Internal Engineering Reference  
**Scope:** Strength preservation, weak-point remediation, WASM compilation fixes  
**Basis:** External principal-engineer audit (May 2026) + independent code audit of `repomix-output.xml`  

---

## Part 0 — How to Use This Document

This plan is written for an AI coding agent (or a human engineer following agent-style discipline).

**Rules of engagement:**

1. **Never touch a strength point without explicit instruction.** Strength sections are listed first in each area so the agent knows what to leave alone.
2. **Every fix section names the exact file, function, and change required.** No guessing.
3. **WASM fixes are purely additive** (feature gates, target-conditional deps). Zero native functionality is removed.
4. **Mock/placeholder verification code gets renamed, not deleted.** The test harness depends on them; they move to `_mock` / `_stub` / `_dev` suffixes and are feature-gated away from production builds.
5. **CI gates are added alongside every cryptographic fix.** A fix without a CI gate will regress.

---

## Part 1 — Strength Inventory (DO NOT MODIFY)

These modules are materially stronger than industry baseline. The agent must preserve them exactly.

### S-1 — `VerificationResult` Multi-Dimensional Model (`csv-core/src/verified.rs`)

The split of `InclusionStrength` / `FinalityStrength` / `VerificationAssurance` is the correct axis for a cross-chain protocol. Compressing to a boolean would require a complete protocol rewrite later.

**Preservation rule:** Never add a `bool verified` field. Never collapse `VerificationAssurance` levels. Never add a shortcut path that skips the level check.

### S-2 — Replay ID Domain-Separation Design (`csv-core/src/domains/replay_registry.rs`, `csv-core/src/replay_record.rs`)

Length-prefix encoding + multi-field binding + destination-chain inclusion is collision-safe by construction. This is unusually careful for an early protocol.

**Preservation rule:** Never change the encoding format without adding a migration version and snapshot tests.

### S-3 — Transfer State Machine with Compile-Fail Tests (`csv-core/tests/compile_fail/`)

The six compile-fail tests (`invalid_state_transition`, `locked_to_minting`, `skip_awaiting_finality`, etc.) are the primary guard against the "bridge exploit" failure mode. They are more valuable than any runtime test in this category.

**Preservation rule:** Never remove a compile-fail test. Any new state transition function must add a corresponding compile-fail test for every illegal predecessor.

### S-4 — `DeploymentProfile` Per-Component Thresholds (`csv-runtime/src/deployment_profile.rs`)

Profiles (`Local`, `Integration`, `Testnet`, `Staging`, `Production`) with typed minimum `InclusionStrength` and `FinalityStrength` are the correct approach. Scalar enum comparison would allow `Testnet >= Local` to pass where `Production` was required.

**Preservation rule:** Never add a `>= profile` comparison shortcut. All gating must go through `VerificationResult::meets_chain_thresholds`.

### S-5 — Bitcoin SPV and Merkle Path (`csv-bitcoin/src/spv.rs`, `csv-bitcoin/src/verifier.rs`)

The Bitcoin verifier uses real `verify_merkle_proof()` with actual branch traversal. The `BitcoinVerifier::verify_inclusion()` path already calls real Merkle validation rather than a hash checksum. This is one of the few fully implemented cryptographic paths.

**Preservation rule:** Do not simplify the Merkle branch logic. Do not replace `verify_merkle_proof` with a non-zero root check.

### S-6 — `ReplayDatabase` Trait CAS Semantics (`csv-runtime/src/replay_db.rs`)

The trait already documents all three unresolved concurrency problems (concurrent coordinators, partial-failure between insert and mint, Byzantine destination adapter). The trait interface itself is correct; only the concrete implementation is missing.

**Preservation rule:** Never change `insert_if_absent` to a non-atomic read-then-write. Never remove the documented concurrency warnings.

### S-7 — CI Architectural Boundary Enforcement (`.github/workflows/architectural-checks.yml`, `scripts/security/check_forbidden_patterns.sh`)

Forbidden-pattern detection for `mock_proof`, `fake.*hash`, `todo!()`, and `stub.*crypto` already runs in CI. This is the primary defense against placeholder code surviving into production builds.

**Preservation rule:** Do not loosen pattern regexes. New verification modules must be added to the scan scope.

### S-8 — Explorer Multi-Interface Architecture (`csv-explorer/`)

REST + GraphQL + WebSocket + indexing abstractions + wallet bridge is production-grade as architecture. The multi-interface approach is correct for partner integration.

**Preservation rule:** Do not collapse REST and GraphQL into a single handler. Keep indexer and API as separate crates.

---

## Part 2 — Critical Weakness Remediation

### W-1 — Ethereum Groth16 Verification is Simulated

**File:** `csv-ethereum/src/zk_verifier.rs`  
**Function:** `EthereumGroth16Verifier::verify_groth16()`  
**Current state:** Structural consistency check only. Proof bytes are inspected for well-formedness but no elliptic-curve pairing is computed. The comments say "For now, we simulate the verification logic."

#### Step 1 — Add arkworks dependency to `csv-ethereum/Cargo.toml`

```toml
[dependencies]
# Real Groth16 pairing verification
ark-bn254        = "0.4"
ark-groth16      = "0.4"
ark-serialize    = "0.4"
ark-ec           = "0.4"
ark-ff           = "0.4"

# Feature-gate so the mock path is unreachable in production
[features]
real-groth16 = ["dep:ark-bn254", "dep:ark-groth16", "dep:ark-serialize", "dep:ark-ec", "dep:ark-ff"]
```

#### Step 2 — Rename the mock function and introduce the real one

```rust
// csv-ethereum/src/zk_verifier.rs

/// Structural consistency check ONLY. Use only in dev/test builds.
/// In production the `real-groth16` feature gates this out.
#[cfg(not(feature = "real-groth16"))]
fn verify_groth16(
    &self,
    proof_bytes: &[u8],
    public_inputs: &ZkPublicInputs,
) -> Result<bool, ZkError> {
    // Renamed from the old body — structural check, NOT cryptographic.
    if proof_bytes.len() < 192 {
        return Err(ZkError::InvalidProof(
            "Groth16 proof too short (need 192 bytes minimum)".to_string(),
        ));
    }
    // DEV ONLY: deterministic consistency heuristic.
    let consistent = proof_bytes[0] != 0
        || (proof_bytes.len() >= 200 && proof_bytes[192..].contains(&0xAA));
    Ok(consistent)
}

/// Real Groth16 pairing verification via arkworks.
#[cfg(feature = "real-groth16")]
fn verify_groth16(
    &self,
    proof_bytes: &[u8],
    public_inputs: &ZkPublicInputs,
) -> Result<bool, ZkError> {
    use ark_bn254::{Bn254, Fr};
    use ark_groth16::{Groth16, Proof, VerifyingKey};
    use ark_serialize::CanonicalDeserialize;

    // 1. Deserialize verification key (loaded at construction time).
    let vk_bytes = self.verifier_key.as_ref().ok_or_else(|| {
        ZkError::VerifierNotFound("No verifier key loaded".to_string())
    })?;
    let vk = VerifyingKey::<Bn254>::deserialize_compressed(vk_bytes.as_slice())
        .map_err(|e| ZkError::InvalidProof(format!("VK deserialization failed: {e}")))?;

    // 2. Deserialize proof (A ∈ G1, B ∈ G2, C ∈ G1 — 192 bytes compressed).
    if proof_bytes.len() < 192 {
        return Err(ZkError::InvalidProof(
            "Groth16 proof must be at least 192 bytes".to_string(),
        ));
    }
    let proof = Proof::<Bn254>::deserialize_compressed(&proof_bytes[..192])
        .map_err(|e| ZkError::InvalidProof(format!("Proof deserialization failed: {e}")))?;

    // 3. Build public inputs vector from ZkPublicInputs.
    let mut hasher = sha2::Sha256::new();
    hasher.update(&public_inputs.seal_ref.id);
    hasher.update(public_inputs.block_hash.as_bytes());
    hasher.update(public_inputs.block_height.to_le_bytes());
    hasher.update(public_inputs.timestamp.to_le_bytes());
    let input_hash: [u8; 32] = hasher.finalize().into();
    // Interpret hash as a field element (big-endian scalar mod r).
    let scalar = Fr::from_be_bytes_mod_order(&input_hash);
    let inputs = vec![scalar];

    // 4. Pairing check: e(A, B) == e(alpha, beta) * e(inputs*gamma^-1, gamma) * e(C, delta).
    let valid = Groth16::<Bn254>::verify(&vk, &inputs, &proof)
        .map_err(|e| ZkError::VerificationFailed(format!("Pairing check failed: {e}")))?;

    Ok(valid)
}
```

#### Step 3 — Gate production CI on the `real-groth16` feature

Add to `.github/workflows/production-guarantee.yml`:

```yaml
- name: Groth16 real-pairing check
  run: |
    cargo check -p csv-ethereum --features real-groth16
    cargo test  -p csv-ethereum --features real-groth16 -- groth16
```

Add to `scripts/security/check_forbidden_patterns.sh`:

```bash
# Block any call to the structural mock in production crates
check_pattern \
  "verify_groth16_structural\|simulate.*verification\|mock.*groth16" \
  "simulated groth16" \
  "csv-ethereum/src" \
  "tests fuzz"
```

#### Step 4 — Add negative proof corpus test

```rust
// csv-ethereum/tests/groth16_corpus.rs  (new file)
#[cfg(feature = "real-groth16")]
#[test]
fn rejects_zero_proof() {
    let verifier = EthereumGroth16Verifier::new_with_key(test_verifier_key());
    let proof = ZkSealProof {
        proof_bytes: vec![0u8; 192],
        ..default_proof()
    };
    assert!(verifier.verify(&proof).is_err());
}

#[cfg(feature = "real-groth16")]
#[test]
fn rejects_truncated_proof() {
    let verifier = EthereumGroth16Verifier::new_with_key(test_verifier_key());
    let proof = ZkSealProof {
        proof_bytes: vec![0xABu8; 64], // too short
        ..default_proof()
    };
    assert!(matches!(verifier.verify(&proof), Err(ZkError::InvalidProof(_))));
}
```

---

### W-2 — `LedgerProof::verify()` is a Non-Zero Root Check

**File:** `csv-aptos/src/merkle.rs` (contains `LedgerProof`)  
**Function:** `LedgerProof::verify()`  
**Current state:** "For now, just check that the root is non-zero."

#### Step 1 — Rename placeholder

```rust
// Before
pub fn verify(&self) -> bool {
    // For now, just check that the root is non-zero
    self.root_hash != [0u8; 32]
}

// After
/// Structural pre-check only. Does NOT verify the Merkle accumulator path.
/// Use `verify_against_accumulator()` in production code.
#[cfg(any(test, feature = "dev-mocks"))]
pub fn verify_structure_only(&self) -> bool {
    self.root_hash != [0u8; 32]
}

/// Full Merkle accumulator proof verification.
/// Verifies that `self.state_root` is a valid leaf at position `self.version`
/// in the Aptos JMT accumulator rooted at `expected_accumulator_root`.
pub fn verify_against_accumulator(
    &self,
    expected_accumulator_root: [u8; 32],
) -> Result<bool, MerkleError> {
    if self.proof.is_empty() {
        return Err(MerkleError::EmptyProof);
    }
    // Walk the sibling hashes bottom-up using the Aptos accumulator hash rule:
    //   parent = sha3_256(0x01 || left || right)
    let mut current = self.root_hash;
    let mut pos = self.version;
    for sibling in &self.proof {
        let (left, right) = if pos % 2 == 0 {
            (current, sibling.hash)
        } else {
            (sibling.hash, current)
        };
        current = accumulator_node_hash(&left, &right);
        pos /= 2;
    }
    Ok(current == expected_accumulator_root)
}
```

#### Step 2 — Update all call-sites

Search for `ledger_proof.verify()` and `LedgerProof::verify()` across the workspace:

```bash
rg "\.verify\(\)" csv-aptos/src/ --include="*.rs" -n
```

Replace every production call-site with `verify_against_accumulator(expected_root)?`.

#### Step 3 — Add CI guard

Add to `scripts/security/check_forbidden_patterns.sh`:

```bash
check_pattern \
  "verify_structure_only\b" \
  "LedgerProof structural mock in production" \
  "csv-aptos/src csv-core/src" \
  "tests"
```

---

### W-3 — STARK Verifier is an Explicit Mock (`csv-stark/src/lib.rs`)

**Current state:** Module doc says "This module uses mock/stub implementations and is NOT ready for production." Both `StarkProver` and `StarkVerifier` are mock structs.

#### Step 1 — Isolate behind feature gate

```toml
# csv-stark/Cargo.toml
[features]
default = []
dev-stark = []          # enables the mock prover/verifier
sp1 = ["dep:sp1-sdk"]   # enables the real SP1 integration when available
```

#### Step 2 — Gate mock structs

```rust
// csv-stark/src/lib.rs
#![cfg_attr(not(feature = "dev-stark"), forbid(unsafe_code))]

/// A mock/stub STARK prover for development and testing.
/// 
/// SECURITY: compile-time gated. Cannot be instantiated in production builds.
#[cfg(feature = "dev-stark")]
pub struct MockStarkProver { /* ... */ }

#[cfg(feature = "dev-stark")]
pub struct MockStarkVerifier { /* ... */ }
```

#### Step 3 — Add compile-time production guard

```rust
// csv-stark/src/lib.rs

/// Production STARK interface.
/// Until `sp1` feature is complete, this panics at runtime if called.
#[cfg(not(any(feature = "dev-stark", feature = "sp1")))]
pub fn verify_stark_proof(_proof: &[u8]) -> Result<bool, StarkError> {
    // Compile succeeds; runtime panics with a clear message.
    // This surfaces any code path that reaches STARK verification in production
    // before the real implementation exists.
    Err(StarkError::NotImplemented(
        "STARK verification requires the `sp1` feature — \
         real implementation pending SP1 circuit integration".to_string()
    ))
}
```

---

### W-4 — Replay Database Has No Concrete Production Implementation

**File:** `csv-runtime/src/replay_db.rs`  
**Current state:** The trait `ReplayDatabase` is well-defined with CAS semantics. No concrete implementation is wired for production use. The trait documentation itself identifies three unresolved concurrency problems.

#### Step 1 — RocksDB CAS Implementation

```rust
// csv-runtime/src/replay_db_rocksdb.rs  (new file)

/// RocksDB-backed replay database with compare-and-swap semantics.
///
/// Uses `merge_operator` with a conflict-detecting custom merge to
/// approximate CAS on top of RocksDB's atomic batch writes.
/// Single-node CAS only. For multi-node deployments, use PostgreSQL advisory locks.
#[cfg(feature = "persistent")]
pub struct RocksReplayDb {
    db: Arc<rocksdb::DB>,
    cf: &'static str,
}

#[cfg(feature = "persistent")]
#[async_trait::async_trait]
impl ReplayDatabase for RocksReplayDb {
    async fn contains(&self, id: &ReplayId) -> Result<bool, RuntimeError> {
        let key = id.as_bytes();
        Ok(self.db.get_cf(self.cf_handle(), key)?.is_some())
    }

    async fn insert_if_absent(
        &self,
        id: &ReplayId,
        state: ReplayEntryState,
    ) -> Result<(), ReplayDbError> {
        let key  = id.as_bytes();
        let val  = bincode::serialize(&state).expect("infallible");
        // put_if_absent via a write batch with a merge operator that rejects
        // writes when the key already exists.
        let mut batch = rocksdb::WriteBatch::default();
        batch.merge_cf(self.cf_handle(), key, &val);
        self.db.write(batch).map_err(|e| {
            // The merge operator returns a sentinel error value when the key exists.
            if e.to_string().contains("REPLAY_CONFLICT") {
                ReplayDbError::AlreadyExists
            } else {
                ReplayDbError::Storage(e.to_string())
            }
        })?;
        Ok(())
    }

    async fn mark_consumed(&self, id: &ReplayId) -> Result<(), RuntimeError> {
        let key = id.as_bytes();
        let val = bincode::serialize(&ReplayEntryState::Consumed).expect("infallible");
        // Consumed is a terminal state; overwrite unconditionally.
        self.db.put_cf(self.cf_handle(), key, val)?;
        Ok(())
    }

    async fn mark_rolled_back(&self, id: &ReplayId) -> Result<(), RuntimeError> {
        let key = id.as_bytes();
        let val = bincode::serialize(&ReplayEntryState::RolledBack).expect("infallible");
        self.db.put_cf(self.cf_handle(), key, val)?;
        Ok(())
    }
}
```

#### Step 2 — PostgreSQL CAS Implementation (multi-node)

```rust
// csv-runtime/src/replay_db_postgres.rs  (new file)

/// PostgreSQL-backed replay database with advisory-lock CAS.
///
/// Uses `INSERT ... ON CONFLICT DO NOTHING RETURNING` to get true
/// server-side CAS semantics across multiple coordinator processes.
#[cfg(feature = "postgres")]
pub struct PostgresReplayDb {
    pool: sqlx::PgPool,
}

#[cfg(feature = "postgres")]
#[async_trait::async_trait]
impl ReplayDatabase for PostgresReplayDb {
    async fn insert_if_absent(
        &self,
        id: &ReplayId,
        state: ReplayEntryState,
    ) -> Result<(), ReplayDbError> {
        let hex_id = hex::encode(id.as_bytes());
        let state_str = format!("{:?}", state);
        let rows_affected = sqlx::query!(
            r#"
            INSERT INTO replay_entries (id, state, inserted_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (id) DO NOTHING
            "#,
            hex_id,
            state_str,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| ReplayDbError::Storage(e.to_string()))?
        .rows_affected();

        if rows_affected == 0 {
            Err(ReplayDbError::AlreadyExists)
        } else {
            Ok(())
        }
    }
    // ... other methods follow the same pattern
}
```

#### Step 3 — Required schema migration

```sql
-- csv-runtime/migrations/0001_replay_entries.sql
CREATE TABLE IF NOT EXISTS replay_entries (
    id          TEXT        PRIMARY KEY,
    state       TEXT        NOT NULL CHECK (state IN ('Pending', 'Consumed', 'RolledBack')),
    inserted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at  TIMESTAMPTZ
);
CREATE INDEX IF NOT EXISTS idx_replay_state ON replay_entries (state);
```

#### Step 4 — Wire to TransferCoordinator

```rust
// csv-runtime/src/transfer_coordinator.rs
// Replace the in-memory replay check with the persistent DB:

// Before (existing pattern):
if self.seen_replays.contains(&replay_id) { return Err(...); }
self.seen_replays.insert(replay_id.clone());

// After:
self.replay_db.insert_if_absent(&replay_id, ReplayEntryState::Pending).await
    .map_err(|e| match e {
        ReplayDbError::AlreadyExists => RuntimeError::ReplayDetected(replay_id.clone()),
        other => RuntimeError::Storage(other.to_string()),
    })?;
```

---

### W-5 — Transfer State Machine Enforcement is Incomplete

**Files:** `csv-core/src/transfer_state/`  
**Current state:** States exist and compile-fail tests guard some transitions. But mutable status strings still exist in some coordinator paths, and not every illegal edge has a compile-fail test.

#### Step 1 — Audit existing compile-fail tests

```bash
ls csv-core/tests/compile_fail/
# Should contain tests for every illegal transition in the state graph:
# locked → minting (skip awaiting_finality, proof_building, proof_validated) ✓
# observed → completed (skip all intermediate states) ✓
# rollback → completed ✓
# Missing: proof_validated → locked (backward), minting → proof_building (backward)
```

#### Step 2 — Add missing backward-transition tests

```rust
// csv-core/tests/compile_fail/proof_validated_to_locked.rs
//! Backward transition from ProofValidated to Locked must be impossible.
//! 
//! ```compile_fail
//! use csv_core::transfer_state::{ProofValidated, Locked};
//! let state: Locked = ProofValidated::new_test().into(); // must not compile
//! ```
```

#### Step 3 — Eliminate mutable status strings in coordinator

Search for `status: String` or `state: String` in coordinator code:

```bash
rg "status.*String\|state.*String" csv-runtime/src/ --include="*.rs" -n
```

Replace each with a typed state enum. No string-based state anywhere in the runtime path.

---

### W-6 — Security Boundary Collapse at the UI/Display Layer

**Files:** `csv-wallet/src/pages/proofs/verify.rs`, `csv-wallet/src/components/seal_status.rs`, `csv-explorer/ui/src/pages/`  
**Risk:** UI displays ✅ Verified without showing which assurance level was reached.

#### Step 1 — Add explicit assurance label to `SealStatusBadge`

```rust
// csv-wallet/src/components/seal_status.rs

pub fn assurance_label(assurance: VerificationAssurance) -> &'static str {
    match assurance {
        VerificationAssurance::Structural          => "⚠️ Structure Only",
        VerificationAssurance::PartialCryptographic => "🔶 Partial Crypto",
        VerificationAssurance::Cryptographic        => "✅ Cryptographic",
        VerificationAssurance::ConsensusBound       => "✅ Consensus-Bound",
    }
}

// In the component render:
// BEFORE: rsx! { span { "✅ Verified" } }
// AFTER:
rsx! {
    span {
        class: assurance_css_class(assurance),
        "{assurance_label(assurance)}"
    }
}
```

#### Step 2 — Block mint gating below threshold

```rust
// csv-runtime/src/transfer_coordinator.rs

fn check_mint_threshold(
    result: &VerificationResult,
    profile: &DeploymentProfile,
) -> Result<(), RuntimeError> {
    if result.assurance < VerificationAssurance::Cryptographic {
        return Err(RuntimeError::AssuranceThresholdNotMet {
            required: VerificationAssurance::Cryptographic,
            actual:   result.assurance,
        });
    }
    if !result.meets_chain_thresholds(&chain_capabilities_for(profile)) {
        return Err(RuntimeError::FinalityThresholdNotMet);
    }
    Ok(())
}
```

---

### W-7 — Testnet RPC Dependency on Public Endpoints

**Files:** `chains/bitcoin.toml`, `chains/ethereum.toml`, `chains/solana.toml`, `chains/aptos.toml`, `chains/sui.toml`

#### Step 1 — Separate config profiles

```toml
# chains/bitcoin.testnet.toml  (controlled infrastructure)
[rpc]
url = "https://bitcoin-signet.internal.csv-protocol.dev"
auth = { env = "BITCOIN_SIGNET_RPC_AUTH" }
fallback_public = false  # fail loudly, do not silently fall back

# chains/bitcoin.ci.toml
[rpc]
url = "http://localhost:18443"  # local bitcoind in CI docker
auth = { user = "test", pass = "test" }
```

#### Step 2 — CI infrastructure compose file

```yaml
# docker-compose.ci.yml  (new file)
services:
  bitcoin-signet:
    image: lncm/bitcoind:v26.0
    command: ["-signet", "-rpcallowip=0.0.0.0/0", "-rpcbind=0.0.0.0"]
    ports: ["18443:18443"]

  ethereum-sepolia:
    image: ethereum/client-go:stable
    command: ["--sepolia", "--http", "--http.addr=0.0.0.0"]
    ports: ["8545:8545"]
```

---

## Part 3 — WASM Compilation Fixes for `csv-wallet`

The `csv-wallet` crate targets `wasm32-unknown-unknown` (Dioxus web). The compilation fails because several transitive dependencies pull in C libraries, OS threads, or openssl.

### Conflict Map

| Symptom | Root Cause | Fix |
|---|---|---|
| `openssl` link error | `sqlx` with `runtime-tokio-native-tls` in `csv-runtime` | Use `rustls` variant + feature-gate |
| `mio` not supported | `tokio` with `net` feature via `csv-runtime` default | Already guarded; `csv-wallet` must not pull `csv-runtime` unconditionally |
| `rocksdb` C compile fails | `csv-runtime` default feature `persistent` | Make `csv-runtime` optional in `csv-wallet`; use no-op runtime for wasm |
| `dirs` crate no wasm support | Direct dep in `csv-sdk` | Feature-gate behind `native` |
| `getrandom` version conflict | v0.2 + v0.3 coexist with different `js` feature names | Pin and alias via `workspace.dependencies` |
| `secp256k1 rand-std` | Uses `getrandom` v0.2; needs `js` feature for wasm | Already has `js` — ensure workspace patches propagate |
| `tokio` net in `csv-sdk` bitcoin feature | `csv-bitcoin` uses async RPC with tokio full | Async RPC must be `cfg(not(wasm32))` |
| `reqwest` native feature default | `csv-wallet` features = `["native"]` default pulls reqwest which links openssl | On wasm, use fetch API only |

---

### Fix W32-1 — Remove Unconditional `csv-runtime` from `csv-wallet`

`csv-runtime` unconditionally enables `persistent` (RocksDB) and `tokio = { features = ["full"] }` on native. On wasm this chain pulls mio → epoll → C code.

**`csv-wallet/Cargo.toml`:**

```toml
# BEFORE
csv-runtime = { path = "../csv-runtime" }

# AFTER — native only
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
csv-runtime = { path = "../csv-runtime", default-features = false }

[target.'cfg(target_arch = "wasm32")'.dependencies]
# No csv-runtime on wasm — wallet uses in-memory state only
```

Any wallet code that uses `csv_runtime` types must be `#[cfg(not(target_arch = "wasm32"))]` gated.

---

### Fix W32-2 — Remove Unconditional `tokio` from `csv-wallet`

`csv-wallet/Cargo.toml` declares `tokio` directly with `rt + macros + sync + time`. This is correct on wasm. But `csv-sdk` is pulled with `features = ["bitcoin", "tokio", "wallet"]`; the `tokio` feature in csv-sdk pulls `dep:tokio` with `rt, macros, sync, time` — also fine. The problem is `csv-bitcoin` transitively pulling tokio with `net` features for async RPC.

**`csv-wallet/Cargo.toml`:**

```toml
# BEFORE
csv-sdk = { path = "../csv-sdk", features = ["bitcoin", "tokio", "wallet"] }

# AFTER
csv-sdk = { path = "../csv-sdk", features = ["wallet"] }

# bitcoin feature in csv-sdk on wasm must not pull csv-bitcoin's async RPC
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
csv-sdk = { path = "../csv-sdk", features = ["bitcoin", "tokio", "wallet"] }

[target.'cfg(target_arch = "wasm32")'.dependencies]
csv-sdk = { path = "../csv-sdk", features = ["wasm", "wallet"] }
```

---

### Fix W32-3 — openssl via `sqlx` in `csv-runtime`

`csv-runtime/Cargo.toml` uses:

```toml
# CURRENT (causes openssl on any platform where openssl is not native-tls-free)
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-native-tls", ...], optional = true }
```

**Fix:**

```toml
# csv-runtime/Cargo.toml
[dependencies]
# Use rustls so openssl is never linked
sqlx = { version = "0.7", features = ["postgres", "runtime-tokio-rustls", "chrono", "uuid"], optional = true }
```

Also update `csv-explorer/storage/Cargo.toml`:

```toml
# BEFORE
sqlx = { version = "0.7", features = ["sqlite", "postgres", "runtime-tokio-native-tls", "chrono"] }

# AFTER
sqlx = { version = "0.7", features = ["sqlite", "postgres", "runtime-tokio-rustls", "chrono"] }
```

---

### Fix W32-4 — RocksDB on `csv-runtime` Default Feature

RocksDB has no wasm32 support. On wasm, `csv-wallet` must not activate the `persistent` feature of `csv-runtime`.

**`csv-runtime/Cargo.toml`** is already structured with:

```toml
[features]
default = ["persistent"]
persistent = ["dep:rocksdb"]
```

Since Fix W32-1 makes `csv-runtime` a native-only dependency of `csv-wallet`, this is resolved transitively. However, if any code path in `csv-wallet` re-introduces `csv-runtime` on wasm, the guard is:

```toml
# csv-runtime/Cargo.toml — add explicit wasm guard
[target.'cfg(target_arch = "wasm32")'.dependencies]
# rocksdb MUST NOT appear on wasm — this section intentionally left empty
# to document the decision.
```

And in `csv-runtime/src/lib.rs`:

```rust
#[cfg(all(target_arch = "wasm32", feature = "persistent"))]
compile_error!(
    "`persistent` (RocksDB) feature is not supported on wasm32. \
     Use `default-features = false` when depending on csv-runtime from a wasm crate."
);
```

---

### Fix W32-5 — `dirs` Crate Has No wasm Support

`csv-sdk/Cargo.toml` and `csv-wallet/Cargo.toml` both pull `dirs = "5.0"` unconditionally. `dirs` depends on OS-specific path resolution which does not compile for wasm.

**`csv-sdk/Cargo.toml`:**

```toml
# BEFORE
dirs = "5.0"

# AFTER
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
dirs = "5.0"
```

**`csv-wallet/Cargo.toml`:**

```toml
# BEFORE
dirs = "5.0"

# AFTER
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
dirs = "5.0"
```

Any code referencing `dirs::home_dir()` or `dirs::config_dir()` must be wrapped:

```rust
fn config_path() -> Option<std::path::PathBuf> {
    #[cfg(not(target_arch = "wasm32"))]
    { dirs::config_dir().map(|d| d.join("csv")) }
    #[cfg(target_arch = "wasm32")]
    { None } // wasm storage goes through web_sys::Storage
}
```

---

### Fix W32-6 — `getrandom` Version Conflict

The workspace currently has both `getrandom 0.2` (with `js` feature) and `getrandom 0.3` (with `wasm_js` feature) aliased in `hakari`. These must be unified via workspace-level patches.

**Root `Cargo.toml`:**

```toml
[workspace.dependencies]
# Pin getrandom 0.2 series; all crates that need wasm RNG use this.
getrandom = { version = "0.2", features = ["js"] }

[patch.crates-io]
# If any transitive dep pulls 0.3, redirect to 0.2-compat shim until upgraded.
# Remove once all deps are on 0.2 or a single 0.3.
```

And remove the hakari alias entries for the dual getrandom versions — they were workarounds for the conflict and are no longer needed once unified.

---

### Fix W32-7 — `reqwest` Default `native` Feature in `csv-wallet`

`csv-wallet` has `features = ["native"]` as default, which pulls `reqwest` with `rustls-tls`. On wasm, reqwest must not be active because the browser provides `fetch` natively.

**`csv-wallet/Cargo.toml`:**

```toml
# BEFORE
reqwest = { version = "0.12.12", default-features = false, features = ["json", "rustls-tls"], optional = true }

[features]
default = ["native"]
native = ["dep:reqwest"]

# AFTER
# reqwest: native-only HTTP client (never on wasm — use fetch API)
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
reqwest = { version = "0.12.12", default-features = false, features = ["json", "rustls-tls"], optional = true }

[features]
default = []    # no default features — each platform opts in explicitly
native = ["dep:reqwest"]
```

All HTTP calls in `csv-wallet/src/services/` must be dispatched through an abstraction:

```rust
// csv-wallet/src/services/http.rs  (new file)

#[cfg(not(target_arch = "wasm32"))]
pub async fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> anyhow::Result<T> {
    Ok(reqwest::get(url).await?.json().await?)
}

#[cfg(target_arch = "wasm32")]
pub async fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> anyhow::Result<T> {
    use wasm_bindgen::JsCast;
    use wasm_bindgen_futures::JsFuture;
    let window = web_sys::window().ok_or_else(|| anyhow::anyhow!("no window"))?;
    let resp: web_sys::Response = JsFuture::from(window.fetch_with_str(url))
        .await
        .map_err(|e| anyhow::anyhow!("{:?}", e))?
        .dyn_into()?;
    let json = JsFuture::from(resp.json().map_err(|e| anyhow::anyhow!("{:?}", e))?)
        .await
        .map_err(|e| anyhow::anyhow!("{:?}", e))?;
    Ok(serde_wasm_bindgen::from_value(json)?)
}
```

---

### Fix W32-8 — `tokio-tungstenite` WebSocket on WASM

`csv-wallet/Cargo.toml` has `tokio-tungstenite` as optional. On wasm, native WebSocket must be replaced with `web_sys::WebSocket`.

```toml
# csv-wallet/Cargo.toml

# BEFORE
tokio-tungstenite = { version = "0.20", optional = true }

# AFTER
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
tokio-tungstenite = { version = "0.20", optional = true }

# wasm WebSocket comes from web-sys (already listed as a dep)
# web-sys features needed: WebSocket, MessageEvent, CloseEvent
```

---

### Fix W32-9 — `csv-wallet/Cargo.toml` Target Configuration Summary

Below is the corrected target-split pattern for the full `csv-wallet/Cargo.toml`:

```toml
[dependencies]
# Always present — pure Rust, wasm-safe
csv-core  = { path = "../csv-core", default-features = false, features = ["std"] }
csv-store = { path = "../csv-store", default-features = false, features = ["browser-storage", "encrypted-storage"] }
csv-keys  = { path = "../csv-keys", features = ["wasm", "production"] }

dioxus         = { version = "0.7", features = ["web", "router"] }
dioxus-router  = "0.7"
serde          = { version = "1.0", features = ["derive"] }
serde_json     = "1.0"
thiserror      = "1.0"
chrono         = { version = "0.4", features = ["serde"] }
uuid           = { version = "1.0", features = ["v4", "serde", "js"] }
hex            = "0.4.3"
sha2           = "0.10"
sha3           = "0.10"
blake2         = "0.10"
bs58           = "0.5"
aes-gcm        = "0.10"
argon2         = "0.5"
zeroize        = { version = "1.7", features = ["derive"] }
futures        = "0.3"
tracing        = "0.1"
bcs            = "0.1"
base64         = "0.21"
rlp            = "0.5"
ethereum-types = "0.14"

# secp256k1 + ed25519: pure-rust, wasm-safe when getrandom/js is active
secp256k1    = { version = "0.29", features = ["rand-std", "global-context"] }
ed25519-dalek = { version = "2.0", features = ["rand_core"] }
rand          = { version = "0.8", features = ["std"] }
bip32         = { version = "0.5", features = ["std", "mnemonic"] }
bitcoin       = { version = "0.32", default-features = false, features = ["serde", "rand-std", "std"] }

# wasm JS interop
console_error_panic_hook = "0.1"
wasm-bindgen             = "0.2.93"
wasm-bindgen-futures     = "0.4"
js-sys                   = "0.3.70"
serde-wasm-bindgen       = "0.6"
getrandom                = { version = "0.2", features = ["js"] }
web-sys = { workspace = true, features = [
    "Window", "Storage", "Document", "Element",
    "HtmlInputElement", "FileReader", "Blob", "BlobPropertyBag",
    "Url", "HtmlAnchorElement", "File", "FileList",
    "ProgressEvent", "Navigator", "DataTransfer",
    "Event", "DragEvent", "Clipboard",
    "WebSocket", "MessageEvent", "CloseEvent",
] }
gloo-timers = { workspace = true, features = ["futures"] }


# ─── Native-only deps ────────────────────────────────────────────────────────
[target.'cfg(not(target_arch = "wasm32"))'.dependencies]
csv-runtime = { path = "../csv-runtime", default-features = false }
csv-sdk     = { path = "../csv-sdk", features = ["bitcoin", "tokio", "wallet"] }
csv-bitcoin = { path = "../csv-bitcoin", optional = true }
csv-ethereum = { path = "../csv-ethereum", optional = true }
tokio            = { version = "1.0", features = ["macros", "rt", "sync", "time"] }
tokio-tungstenite = { version = "0.20", optional = true }
reqwest          = { version = "0.12.12", default-features = false, features = ["json", "rustls-tls"], optional = true }
dirs             = "5.0"

# ─── WASM-only deps ──────────────────────────────────────────────────────────
[target.'cfg(target_arch = "wasm32")'.dependencies]
csv-sdk = { path = "../csv-sdk", features = ["wasm", "wallet"] }
# tokio wasm subset — no net, no io-util, no fs
tokio   = { version = "1.0", features = ["rt", "macros", "sync", "time"] }

[features]
default = []
native  = ["dep:reqwest"]
csv-bitcoin  = ["dep:csv-bitcoin"]
csv-ethereum = ["dep:csv-ethereum"]
```

---

### Fix W32-10 — `.cargo/config.toml` WASM Build Target

Add the WASM target configuration to prevent `rustflags = ["-Dwarnings"]` from propagating into wasm builds that have legitimate platform conditionals:

```toml
# .cargo/config.toml  — add after existing [build] section

[target.wasm32-unknown-unknown]
rustflags = [
    "-Dwarnings",
    "-Dunused_imports",
    # Do NOT include "-Dunused_variables" here — cfg-gated variables
    # that are unused on wasm will trigger false positives.
]
```

---

### Fix W32-11 — CI WASM Check Job

Add to `.github/workflows/ci.yml`:

```yaml
  wasm-check:
    name: WASM Compilation Check
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          targets: wasm32-unknown-unknown

      - name: Install wasm-pack
        run: cargo install wasm-pack --locked

      - name: Check csv-wallet for wasm32
        run: |
          cargo check -p csv-wallet \
            --target wasm32-unknown-unknown \
            --no-default-features \
            2>&1 | tee wasm-check.log
          if grep -i "error\[" wasm-check.log; then
            echo "WASM compilation errors detected"
            exit 1
          fi

      - name: Check csv-core for wasm32
        run: |
          cargo check -p csv-core \
            --target wasm32-unknown-unknown \
            --no-default-features \
            --features std

      - name: Check csv-store for wasm32
        run: |
          cargo check -p csv-store \
            --target wasm32-unknown-unknown \
            --no-default-features \
            --features browser-storage,encrypted-storage
```

---

## Part 4 — Additional Findings (Independent Audit)

The following issues were not in the external audit but were found during code review.

### A-1 — `csv-core` pulls `secp256k1` with `rand-std` Unconditionally

`csv-core/Cargo.toml`:

```toml
# CURRENT — always links std RNG
secp256k1 = { version = "0.28", features = ["rand-std"] }
```

On `no_std` targets this fails. For wasm it works only if `getrandom/js` is active (which it is, but only if the workspace patch propagates).

**Fix:** Feature-gate `rand-std` to the `std` feature:

```toml
secp256k1 = { version = "0.28", features = [] }

[features]
std = ["secp256k1/rand-std", ...]
```

---

### A-2 — `csv-core` pulls `reqwest 0.11` (Old) While `csv-wallet` uses `reqwest 0.12`

```toml
# csv-core/Cargo.toml
reqwest = { version = "0.11", features = ["json"], optional = true }

# csv-wallet/Cargo.toml
reqwest = { version = "0.12.12", ... }
```

This causes two incompatible reqwest versions in the dependency graph, doubling binary size and potentially causing TLS version conflicts.

**Fix:** Upgrade `csv-core`'s optional reqwest to `0.12`:

```toml
reqwest = { version = "0.12", default-features = false, features = ["json", "rustls-tls"], optional = true }
```

---

### A-3 — `csv-wallet` Clipboard Access has a Stub Comment

```rust
// csv-wallet/src/components/hash_display.rs  (approx line 14444)
// Clipboard access via web_sys is feature-gated; for now just log
```

**Fix:**

```rust
#[cfg(target_arch = "wasm32")]
async fn copy_to_clipboard(text: &str) -> Result<(), JsValue> {
    let window = web_sys::window().ok_or(JsValue::from_str("no window"))?;
    let navigator = window.navigator();
    let clipboard = navigator.clipboard();
    JsFuture::from(clipboard.write_text(text)).await?;
    Ok(())
}

#[cfg(not(target_arch = "wasm32"))]
async fn copy_to_clipboard(text: &str) -> Result<(), String> {
    // Desktop: use arboard crate or OS clipboard
    arboard::Clipboard::new()
        .and_then(|mut cb| cb.set_text(text))
        .map_err(|e| e.to_string())
}
```

---

### A-4 — `csv-solana` Uses `// Use a default hash for now` in Transaction Builder

```rust
// csv-solana/src/*.rs  (approx line 27025)
// Use a default hash for now - would need actual recent blockhash
```

Solana transactions require a valid recent blockhash or they are rejected by the network. This is a functional (not just security) bug that will cause all Solana transactions to fail on testnet.

**Fix:** Always fetch the recent blockhash from RPC before building a transaction. No in-memory default is acceptable.

```rust
async fn build_transaction(&self, ...) -> Result<Transaction, SolanaError> {
    let recent_blockhash = self.rpc
        .get_latest_blockhash()
        .await
        .map_err(|e| SolanaError::RpcError(format!("blockhash fetch failed: {e}")))?;
    // use recent_blockhash in transaction construction
}
```

---

### A-5 — Fuzz Targets Don't Cover the Verification Pipeline Entrypoint

```
fuzz/fuzz_targets/
  abi_decoder.rs
  consignment_decode.rs
  finality_parser.rs
  proof_bundle_decode.rs     ← decodes only; does not call verify_proof_bundle()
  rpc_parser.rs
```

The primary attack surface is `csv-core/src/verifier.rs::verify_proof_bundle()`. No fuzz target exercises it end-to-end.

**Fix — add `fuzz/fuzz_targets/verify_pipeline.rs`:**

```rust
#![no_main]
use libfuzzer_sys::fuzz_target;
use csv_core::verifier::verify_proof_bundle;
use csv_core::proof::ProofBundle;
use csv_core::signature::SignatureScheme;

fuzz_target!(|data: &[u8]| {
    if let Ok(bundle) = bincode::deserialize::<ProofBundle>(data) {
        let _ = verify_proof_bundle(
            &bundle,
            |_seal_id| false, // treat all seals as unused
            SignatureScheme::Secp256k1,
        );
    }
});
```

---

### A-6 — `csv-stark/src/lib.rs` Homomorphic Addition Comment is Dangerous

```rust
// Line 55633 (approx):
/// This is a simulated homomorphic addition using hash concatenation.
```

Hash concatenation is not homomorphic addition. This is a named security property (homomorphic encryption) being falsely claimed in doc comments. Any downstream code that assumes real HE properties will be wrong.

**Fix:** Rename and re-document:

```rust
/// Commitment binding via hash concatenation.
/// 
/// This is NOT homomorphic addition. It combines two commitment values
/// via SHA-256 concatenation for binding purposes only.
/// Real homomorphic operations require a proper HE scheme (Paillier, BFV, etc.).
fn bind_commitments(a: &[u8], b: &[u8]) -> [u8; 32] {
    // ...
}
```

---

## Part 5 — Implementation Sequencing for the AI Agent

Execute in this exact order. Each step has a clear done condition.

### Sprint 1 — WASM Unblock (Days 1–3)

| Task | File | Done Condition |
|---|---|---|
| Apply Fix W32-1 | `csv-wallet/Cargo.toml` | `cargo check -p csv-wallet --target wasm32-unknown-unknown` passes |
| Apply Fix W32-2 | `csv-wallet/Cargo.toml` | No `mio` in WASM dependency graph (`cargo tree`) |
| Apply Fix W32-3 | `csv-runtime/Cargo.toml`, `csv-explorer/storage/Cargo.toml` | No `openssl` in `cargo tree -p csv-runtime` |
| Apply Fix W32-4 | `csv-runtime/src/lib.rs` | `compile_error!` fires on `cargo check --target wasm32 --features persistent` |
| Apply Fix W32-5 | `csv-sdk/Cargo.toml`, `csv-wallet/Cargo.toml` | `dirs` absent from wasm dep tree |
| Apply Fix W32-6 | Root `Cargo.toml` | `cargo tree -d` shows zero duplicate getrandom versions |
| Apply Fix W32-7 | `csv-wallet/Cargo.toml` | `reqwest` absent from wasm dep tree |
| Apply Fix W32-8 | `csv-wallet/Cargo.toml` | `tokio-tungstenite` absent from wasm dep tree |
| Apply Fix W32-9 | `csv-wallet/Cargo.toml` | Complete target-split config in place |
| Apply Fix W32-10 | `.cargo/config.toml` | WASM rustflags section present |
| Apply Fix W32-11 | `.github/workflows/ci.yml` | WASM CI job green |

### Sprint 2 — Cryptographic Hardening (Days 4–14)

| Task | File | Done Condition |
|---|---|---|
| W-1 Step 1–2 | `csv-ethereum/Cargo.toml`, `csv-ethereum/src/zk_verifier.rs` | `ark-groth16` pairing impl compiles under `real-groth16` feature |
| W-1 Step 3 | `.github/workflows/production-guarantee.yml` | CI fails on missing `real-groth16` feature |
| W-1 Step 4 | `csv-ethereum/tests/groth16_corpus.rs` | Zero-proof and truncated-proof tests pass |
| W-2 Step 1–2 | `csv-aptos/src/merkle.rs`, call-sites | `verify_structure_only` is test-only; `verify_against_accumulator` used everywhere |
| W-2 Step 3 | `scripts/security/check_forbidden_patterns.sh` | Pattern scan added and passing |
| W-3 Step 1–3 | `csv-stark/Cargo.toml`, `csv-stark/src/lib.rs` | Mock structs are `dev-stark` feature only |
| A-6 | `csv-stark/src/lib.rs` | No "homomorphic" claim in non-HE code |

### Sprint 3 — Runtime Hardening (Days 15–21)

| Task | File | Done Condition |
|---|---|---|
| W-4 Step 1 | `csv-runtime/src/replay_db_rocksdb.rs` | RocksDB CAS impl passes unit test with concurrent write simulation |
| W-4 Step 2 | `csv-runtime/src/replay_db_postgres.rs` | Postgres `INSERT ... ON CONFLICT` impl passes integration test |
| W-4 Step 3 | `csv-runtime/migrations/` | Migration SQL file exists and is idempotent |
| W-4 Step 4 | `csv-runtime/src/transfer_coordinator.rs` | In-memory replay `HashSet` replaced with `ReplayDatabase` |
| W-5 Step 2 | `csv-core/tests/compile_fail/` | Two new backward-transition compile-fail tests |
| W-5 Step 3 | `csv-runtime/src/` | Zero `status: String` or `state: String` fields in coordinator |
| A-4 | `csv-solana/src/` | No `default hash for now` comment; RPC blockhash fetch is mandatory |

### Sprint 4 — UI Safety and Observability (Days 22–28)

| Task | File | Done Condition |
|---|---|---|
| W-6 Step 1 | `csv-wallet/src/components/seal_status.rs` | `assurance_label()` renders level, not generic checkmark |
| W-6 Step 2 | `csv-runtime/src/transfer_coordinator.rs` | Mint gating rejects `< Cryptographic` assurance |
| W-7 Step 1 | `chains/*.testnet.toml`, `docker-compose.ci.yml` | CI hits local nodes, not public RPCs |
| A-3 | `csv-wallet/src/components/hash_display.rs` | Clipboard uses `web_sys::Clipboard` on wasm |
| A-5 | `fuzz/fuzz_targets/verify_pipeline.rs` | Fuzzer builds and runs for 60s without crash |

---

## Part 6 — Invariants the Agent Must Never Violate

1. **`verify()` on any struct named `*Verifier` must perform real cryptographic verification or return `Err(NotImplemented)` — never silently return `true`.**
2. **`VerificationAssurance::Cryptographic` is the minimum threshold for any action that moves value. Do not add a parameter to lower this.**
3. **Compile-fail tests in `csv-core/tests/compile_fail/` are permanent. Never delete them. Never `#[ignore]` them.**
4. **`ReplayDatabase::insert_if_absent` must be a compare-and-swap. Never replace it with `contains()` + `insert()`.**
5. **The `wasm32` CI job must stay green. Every PR that changes a `Cargo.toml` must re-run the WASM check.**
6. **`VerificationAssurance` levels must not be collapsed. If a new level is added, every match statement in the codebase must be updated (use `#[non_exhaustive]` is not sufficient — use exhaustive matches).**
7. **Feature flags that gate mock code (`dev-mocks`, `dev-stark`) must never appear in the `default` feature set.**
8. **The `DeploymentProfile::Production` minimum thresholds must never be lowered. They can only be raised.**

---

*End of Engineering Plan — Version 1.0 — May 2026*
