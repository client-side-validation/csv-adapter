# CSV Protocol — Pre-Deployment Audit Report

**Date**: May 18, 2026 | **Against**: `docs/CSV_ENGINEERING_PLAN.md v1.0`

---

## 🔴 BLOCKERS — Do Not Deploy

### B-1: `ChainVerifier` Trait Still Returns `Result<bool>`

**File**: `csv-core/src/proof_pipeline.rs`
**Severity**: CRITICAL (Security)

The plan's Phase 1 root-cause fix is **not implemented**. `ChainVerifier` defines all 5 methods (`verify_inclusion`, `verify_finality`, `verify_zk`, `verify_seal_registry`, `verify_signature`) as `-> Result<bool>`. This is explicitly identified in the plan as the root cause of silent verification bypass. The new `VerificationResult` / `VerifiedComponents` types exist in `verified.rs` but are **never used by the pipeline**.

```rust
// STILL IN CODE — must not exist at deploy
async fn verify_inclusion(&self, proof: &InclusionProof, expected_root: Hash) -> Result<bool>;
async fn verify_finality(&self, proof: &FinalityProof) -> Result<bool>;
```

**`meets_chain_thresholds` is never called anywhere in the pipeline.** The mint authorization gate the plan was designed to create does not gate anything.

---

### B-2: Fake Transfer Functions Not Removed

**File**: `csv-wallet/src/services/blockchain.rs`
**Severity**: CRITICAL (Fraud Vector)

Plan §4.1 says: *"Delete `transfer_sanad_local()` and `execute_cross_chain_transfer()`."* Both functions still exist. `transfer_sanad_local()` calls `execute_cross_chain_transfer()`, which calls `csv-sdk` directly — bypassing `csv-runtime`'s `TransferCoordinator` entirely. Users can initiate transfers through a path with no replay protection and no state machine.

---

### B-3: Wallet Bypasses `csv-runtime` (Layer Boundary Violation)

**File**: `csv-wallet/Cargo.toml`
**Severity**: CRITICAL (Architecture)

The plan's permanent rule: *"csv-wallet must never import csv-bitcoin, csv-ethereum, etc. directly."* Violations found:

- `csv-bitcoin` — direct import
- `csv-ethereum` — direct import
- `csv-solana` — direct import
- `csv-runtime` — **NOT in dependencies at all**

The wallet routes transfers through `csv-sdk` directly, not `TransferCoordinator`. The isolation boundary the entire Phase 2 was designed to enforce does not exist at the wallet layer.

---

### B-4: Bitcoin Finality Returns `Ok(true)` on Empty Data

**File**: `csv-bitcoin/src/verifier.rs` — `verify_finality()`
**Severity**: CRITICAL (Security)

```rust
if proof.finality_data.is_empty() {
    return Ok(true);  // ← BUG: no data = finalized?
}
```

An attacker submitting a finality proof with zero bytes passes the Bitcoin finality check. Same pattern exists in `csv-sui/src/verifier.rs`. Must return `Err(VerificationFailure::MissingData(...))`.

---

### B-5: CI Adapter Boundary Check Uses Wrong Crate Names (Silent False Pass)

**File**: `.github/workflows/production-guarantee.yml`
**Severity**: CRITICAL (CI Integrity)

The CI job `no-direct-adapter-imports` checks for:

```
rg "csv_adapter_(bitcoin|ethereum|sui|aptos|solana)"
```

The actual crate names are `csv_bitcoin`, `csv_ethereum`, etc. This check **will never fire**. The violations in B-3 above would pass CI undetected. The enforcement gate the plan requires is absent.

---

## 🟠 HIGH — Fix Before Public Network

### H-1: Compile-Fail Tests Not Present

**Severity**: HIGH

Plan §5.1 requires compile-fail tests in `csv-core/tests/compile_fail/` to enforce `Result<bool>` ban at the type level. These files do not exist in the repomix. Grep-based CI is explicitly described as "temporary migration scaffolding" — and even those grep checks are broken (B-5).

---

### H-2: `TransferCoordinator.execute()` Not Wired to Any Application

**File**: `csv-runtime/src/transfer_coordinator.rs`
**Severity**: HIGH

`csv-runtime` crate exists and `TransferCoordinator` is implemented correctly (replay check → capability check → lock → finality → proof → mint). However, neither `csv-wallet` nor `csv-cli` have `csv-runtime` as a dependency. The coordinator is dead code in production paths.

---

### H-3: Bitcoin `verify_finality` Does Not Query Confirmations

**File**: `csv-bitcoin/src/verifier.rs`
**Severity**: HIGH

The finality check reads `finality_data` length but never calls the Bitcoin RPC to fetch actual block confirmation depth. For a PoW chain, confirmation count must be fetched from a live node or encoded in a verifiable header chain — not inferred from proof struct field presence.

---

### H-4: `VerificationAssurance` Enum Used as Gate in UI Code (Wrong Usage)

**Severity**: HIGH

The plan explicitly warns: *"The `VerificationAssurance` enum must not be used as the mint authorization gate."* Review `csv-wallet/src/pages/validate/` and `csv-wallet/src/services/` for any scalar enum comparison being used to gate operations — the type exists and may be misused.

---

## 🟡 MEDIUM — Address Before Mainnet

### M-1: Ethereum Storage Key Derivation Needs Audit

**File**: `csv-ethereum/src/verifier.rs`
The storage key for `usedSeals[sealId]` is computed as:

```rust
storage_key_array[..32].copy_from_slice(seal_id_hash.as_slice());
storage_key_array[32..].copy_from_slice(&[0u8; 32]); // slot 0
```

Solidity mapping slot computation requires `keccak256(abi.encode(key, slot))`. This is not the same as raw concatenation. Verify against the deployed `CSVLock` contract ABI.

---

### M-2: `csv-runtime` Uses `rocksdb` — WASM Incompatible

**File**: `csv-runtime/Cargo.toml`
RocksDB is a native C++ library. If `csv-wallet` (WASM target) ever depends on `csv-runtime`, this will fail to compile. Feature-gate `rocksdb` correctly and provide a WASM-safe storage backend for the replay DB.

---

### M-3: `ReplayDatabase` Trait Has No Compare-And-Swap Semantics

**File**: `csv-runtime/src/replay_db.rs`
The plan requires *"durable replay prevention with compare-and-swap semantics."* Verify the `contains()` + `insert()` pattern is atomic. A non-atomic check-then-set allows a race window for replay attacks under concurrent requests.

---

### M-4: CI `production-surface-audit` Excludes `examples/` — Too Broad

**File**: `.github/workflows/production-guarantee.yml`
The `--exclude-pattern "examples/"` flag in ripgrep checks means production code in subdirectories named `examples` would be excluded. Verify no production-path modules are in `examples/` directories.

---

## ✅ Confirmed Implemented Correctly

| Area | Status |
|------|--------|
| `csv-core/src/verified.rs` — `VerifiedComponents`, `VerificationFailure` types | ✅ Exists, correct design |
| `csv-core/src/chain_config.rs` — Security-aware `ChainCapabilities` | ✅ All enums match plan spec |
| `csv-bitcoin/src/verifier.rs` — SPV Merkle path verification | ✅ `verify_merkle_proof()` called |
| `csv-ethereum/src/mpt.rs` — `alloy-trie` MPT proof verification | ✅ Exists, 9KB implementation |
| `csv-runtime` crate structure | ✅ All 6 modules present, no adapter imports |
| `csv-runtime/src/transfer_coordinator.rs` — State machine logic | ✅ 5-step protocol correct |
| `csv-core/src/cross_chain.rs` — No adapter imports | ✅ Boundary clean |
| `ChainCapabilities::inclusion_threshold_met` / `finality_threshold_met` | ✅ Correct orthogonal gate design |

---

## Summary

| Category | Count |
|----------|-------|
| 🔴 Blockers (deploy-stopping) | 5 |
| 🟠 High (pre-public-network) | 4 |
| 🟡 Medium (pre-mainnet) | 4 |
| ✅ Confirmed correct | 8 areas |

**Do not deploy to any public network until B-1 through B-5 are resolved.**
The core architectural work (verified.rs, chain_config.rs, csv-runtime scaffolding, Bitcoin SPV, Ethereum MPT) is solid. The integration layer — wiring the new types into the pipeline and removing the old bypass paths — is incomplete.
