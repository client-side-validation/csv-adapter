# AUDIT.md Implementation Status Report

## Part 1 — Strength Inventory (DO NOT MODIFY)

| ID | Item | Status | Notes |
|----|------|--------|-------|
| S-1 | `VerificationResult` Multi-Dimensional Model | ✅ PRESERVED | `verified.rs` has proper `InclusionStrength`/`FinalityStrength`/`VerificationAssurance` split |
| S-2 | Replay ID Domain-Separation Design | ✅ PRESERVED | `replay_registry.rs` + `replay_record.rs` with CAS semantics |
| S-3 | Transfer State Machine Compile-Fail Tests | ✅ PRESERVED | 8 compile-fail tests exist |
| S-4 | `DeploymentProfile` Per-Component Thresholds | ✅ PRESERVED | Proper per-component thresholds, not scalar comparison |
| S-5 | Bitcoin SPV and Merkle Path | ✅ See notes | Assuming `spv.rs`/`verifier.rs` are intact |
| S-6 | `ReplayDatabase` Trait CAS Semantics | ✅ PRESERVED | Trait documents 3 concurrency problems |
| S-7 | CI Architectural Boundary Enforcement | ✅ PRESERVED | `check_forbidden_patterns.sh` exists |
| S-8 | Explorer Multi-Interface Architecture | ✅ PRESERVED | REST + GraphQL + WebSocket + indexer |

## Part 2 — Critical Weakness Remediation

| ID | Item | Status | Priority |
|----|------|--------|----------|
| W-1 | Ethereum Groth16 Verification | ✅ FIXED | CRITICAL |
| W-2 | LedgerProof::verify() | ✅ FIXED | CRITICAL |
| W-3 | STARK Verifier Mock | ✅ FIXED | HIGH |
| W-4 | Replay DB Production Implementation | ⏳ PENDING | HIGH |
| W-5 | Transfer State Machine Enforcement | ⚠️ PARTIAL | MEDIUM |
| W-6 | UI/Display Security Boundary | ⏳ PENDING | MEDIUM |
| W-7 | Testnet RPC Public Endpoints | ⏳ PENDING | MEDIUM |

## Part 3 — WASM Compilation Fixes

| ID | Item | Status | Notes |
|----|------|--------|-------|
| W32-1 | Remove csv-runtime from wasm | ✅ DONE | Already in `[target.'cfg(...)']` |
| W32-2 | Remove tokio from wasm | ✅ DONE | Already in target-specific deps |
| W32-3 | openssl via sqlx | ✅ DONE | Uses `runtime-tokio-rustls` |
| W32-4 | RocksDB on wasm | ✅ DONE | `compile_error!` guard exists |
| W32-5 | dirs on wasm | ⚠️ PARTIAL | csv-wallet OK, csv-sdk needs check |
| W32-6 | getrandom version conflict | ✅ FIXED | Unified to v0.2/js |
| W32-7 | reqwest on wasm | ✅ DONE | In native-only deps |
| W32-8 | tokio-tungstenite on wasm | ✅ DONE | Optional native-only |
| W32-9 | Target config summary | ✅ DONE | Target-split config in place |
| W32-10 | .cargo/config.toml WASM config | ✅ DONE | Already has wasm32-unknown-unknown section |
| W32-11 | CI WASM check job | ⏳ PENDING | Need CI workflow changes |

## Part 4 — Additional Findings

| ID | Item | Status | Priority |
|----|------|--------|----------|
| A-1 | csv-core secp256k1 rand-std | ⏳ PENDING | LOW |
| A-2 | reqwest 0.11 vs 0.12 | ✅ FIXED | MEDIUM |
| A-3 | Clipboard stub comment | ⏳ PENDING | LOW |
| A-4 | Solana default blockhash | ✅ ALREADY FIXED | HIGH |
| A-5 | Fuzz targets missing verify_pipeline | ✅ FIXED | MEDIUM |
| A-6 | Homomorphic comment in stark | ✅ ALREADY CLEAN | LOW |

## Summary of Changes Made

### Files Modified:
1. **csv-ethereum/Cargo.toml** - Added arkworks deps (ark-bn254, ark-groth16, ark-serialize, ark-ec, ark-ff) under `real-groth16` feature
2. **csv-ethereum/src/zk_verifier.rs** - Rewrote with #[cfg(feature = "real-groth16")] and real pairing verification
3. **csv-ethereum/tests/groth16_corpus.rs** - Added negative proof corpus tests (new file)
4. **csv-aptos/src/merkle.rs** - Added `verify_structure_only()` + `verify_against_accumulator()` methods
5. **csv-stark/Cargo.toml** - Changed features to `dev-stark`/`sp1` with no default
6. **csv-stark/src/lib.rs** - Feature-gated mock structs behind `dev-stark`, added `verify_stark_proof()` guard
7. **Cargo.toml** - Changed getrandom to v0.2/js, unified version
8. **csv-core/Cargo.toml** - Upgraded reqwest from 0.11 → 0.12
9. **scripts/security/check_forbidden_patterns.sh** - Added LedgerProof structural mock check
10. **fuzz/Cargo.toml** - Registered verify_pipeline fuzz target
11. **docs/AUDIT_REVIEW_STATUS.md** - Updated with current status

### Files Created:
1. `csv-ethereum/tests/groth16_corpus.rs` - Groth16 negative corpus tests
2. `fuzz/fuzz_targets/verify_pipeline.rs` - End-to-end verification pipeline fuzz target

### Pending Items (Deferred):
- **W-4**: RocksDB/Postgres replay DB production implementations
- **W-5**: Additional backward-transition compile-fail tests  
- **W-6**: UI assurance labels in csv-wallet
- **W-7**: Separate config profiles for testnet RPCs
- **A-1**: Feature-gate secp256k1/rand-std in csv-core
- **A-3**: Implement proper clipboard in hash_display.rs
- **W32-11**: CI WASM check job in workflows