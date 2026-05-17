# CSV Protocol — Production Readiness Audit

**Scope:** Full codebase audit covering production readiness, cross-chain transfer verifiability, multi-chain compatibility, contract maturity, deployability, stub/placeholder detection, and architectural clarity.

**Reference applications:** `csv-cli` and `csv-wallet` are treated as the canonical integration examples; all findings trace back through `csv-sdk` to `csv-core`.

**Chains in scope:** Bitcoin, Ethereum, Solana, Aptos, Sui (all currently present in the repo). Bitcoin and Ethereum are explicitly added to the expanded requirement.

---

## Table of Contents

1. [Executive Summary](#1-executive-summary)
2. [Architecture Assessment — Parallel Abstractions & Redundancy](#2-architecture-assessment)
3. [Stub & Placeholder Inventory — CRITICAL](#3-stub--placeholder-inventory)
4. [Cross-Chain Transfer Verifiability](#4-cross-chain-transfer-verifiability)
5. [Chain-by-Chain Compatibility Matrix](#5-chain-by-chain-compatibility-matrix)
6. [Smart Contract Maturity & Deployability](#6-smart-contract-maturity--deployability)
7. [Explorer Indexer Completeness](#7-explorer-indexer-completeness)
8. [Cryptography & Signature Correctness](#8-cryptography--signature-correctness)
9. [csv-cli Production Gaps](#9-csv-cli-production-gaps)
10. [csv-wallet Production Gaps](#10-csv-wallet-production-gaps)
11. [Prioritised Fix Plan](#11-prioritised-fix-plan)
12. [Per-Chain Verification & Compatibility Expansion Guide](#12-per-chain-verification--compatibility-expansion-guide)

---

## 1. Executive Summary

The codebase demonstrates a well-designed protocol core (`csv-core`) with sound cryptographic types, a clear lock-prove-verify-mint cross-chain state machine, and correct chain-specific verifier implementations for all five chains. The most recent audit pass visibly hardened many areas. However, **several high-severity production blockers remain**, the most dangerous of which are not visible to tests because the stub functions return `Ok(())` or `Ok(vec![])` — passing all test gates while silently doing nothing on a real chain.

**Severity Legend**

- 🔴 **CRITICAL** — Will silently corrupt state or allow fraud on mainnet. Block release.
- 🟠 **HIGH** — Wrong results, broken features, or security weaknesses under realistic conditions.
- 🟡 **MEDIUM** — Functional gaps, degraded UX, or missing cross-chain parity.
- 🟢 **LOW** — Code quality, maintainability, or future-proofing concerns.

| Severity | Count |
|----------|-------|
| 🔴 CRITICAL | 9 |
| 🟠 HIGH | 14 |
| 🟡 MEDIUM | 11 |
| 🟢 LOW | 7 |

---

## 2. Architecture Assessment

### 2.1 Parallel `ChainId` Definitions — Resolved, with a Caveat

**Status:** Largely fixed. `csv-store/src/state/core.rs` re-exports `csv_core::ChainId` (`pub use csv_core::ChainId`), meaning there is only one definition at the type level.

**Remaining problem 🟡:** `csv-wallet/src/chains/*.rs` still imports `csv_store::state::ChainId` while `csv-core` and `csv-sdk` use `csv_core::protocol_version::ChainId`. These are the same underlying type due to the re-export, but the *import path* diverges across the tree, making grep-based auditing confusing and making it easy for future contributors to inadvertently break the equivalence. Standardise all imports to `use csv_core::ChainId` via a single re-export point in `csv_sdk::prelude`.

### 2.2 Parallel Service Layers in `csv-wallet`

**Status 🟠:** The wallet has three overlapping service layers that all purport to handle blockchain operations:

- `csv-wallet/src/services/blockchain.rs` — wraps `csv_sdk::CsvClient`, but `transfer_sanad_local` and `execute_cross_chain_transfer` return hardcoded placeholder `TransferResult` structs with `"pending"` hash strings.
- `csv-wallet/src/services/chain_api.rs` — delegates to `ChainRuntime` from `csv-sdk`. This is the correct path.
- `csv-wallet/src/chains/*.rs` — thin format helpers, acceptable.

**Fix:** Remove or tombstone `blockchain.rs`'s transfer functions. Route all wallet transfer calls through `chain_api.rs` → `csv_sdk::transfers()` which is already wired to real chain adapters.

### 2.3 `ChainRuntime` vs. Direct Adapter Access

**Status 🟢:** The architecture correctly mandates that `csv-cli` and `csv-wallet` never import chain-specific crates directly — they go through `csv-sdk::CsvClient`. This is well-enforced in `csv-cli/src/commands/cross_chain/transfer.rs`. The wallet's `blockchain.rs` violates this by instantiating its own `CsvClient` inside `BlockchainService::new()` with an `InMemory` store backend, meaning production transfers from the wallet bypass any configured persistent store.

### 2.4 `csv-stark` Experimental Flag

**Status 🟢:** `csv-stark` is gated behind `experimental` feature and its `Cargo.toml` documents the mock status. The `MockStarkProver` and `MockStarkVerifier` are clearly named. This is acceptable, provided the `experimental` flag is never enabled in production builds. Recommend a CI step that fails if `experimental` is in any production build profile.

---

## 3. Stub & Placeholder Inventory

This is the most critical section. The following are real production code paths (not tests) that silently return empty or success results.

### 3.1 🔴 `ParallelVerifyService::verify_single_seal` and `verify_single_proof` — No-ops

**File:** `csv-wallet/src/services/parallel_verify.rs`

```rust
// verify_single_seal
async fn verify_single_seal(&self, _seal: &SealRecord) -> Result<(), SealError> {
    // Placeholder: In production, implement actual seal verification
    Ok(())
}

// verify_single_proof (same pattern)
```

**Impact:** The wallet's "Parallel Verification" feature, exposed as a UI affordance to users, verifies nothing. Every seal and proof bundle will report as valid regardless of chain state. A double-spent seal will appear valid. This is a silent fraud vector.

**Fix:** Implement using the `ChainVerifier` trait already present on each chain adapter. The algorithm is:

1. Resolve the chain from `seal.chain`.
2. Call `ChainVerifier::verify_seal_registry(seal_id_hash)` on the appropriate adapter obtained from `ChainRuntime`.
3. Call `csv_core::verifier::verify_proof()` for proof bundles, passing the seal registry closure that calls step 2.

Use `tokio::task::JoinSet` (native) or `futures::future::join_all` (WASM) to achieve the claimed parallelism.

### 3.2 🔴 `BlockchainService::transfer_sanad_local` and `execute_cross_chain_transfer` — Fake Results

**File:** `csv-wallet/src/services/blockchain.rs`

```rust
// Returns placeholder with "pending" hashes — never submits to chain
Ok(TransferResult {
    transfer_id: format!("local-{}-{}-to-{}", chain, sanad_id, to),
    lock_tx_hash: "pending".to_string(),
    mint_tx_hash: "pending".to_string(),
    ...
})
```

**Impact:** Users initiate transfers from the wallet UI, the UI reports success, but nothing is submitted on-chain. Funds are not moved. Users will never know unless they check a block explorer.

**Fix:** Delete `BlockchainService::transfer_sanad_local` and `execute_cross_chain_transfer`. Wire the wallet's transfer UI pages directly to `csv_sdk::CsvClient::transfers().cross_chain(...)`, which already has real chain dispatch.

### 3.3 🔴 `mint_sanad_on_chain` Missing Bitcoin and Aptos — Silent `ChainNotSupported`

**File:** `csv-sdk/src/cross_chain.rs`

```rust
pub async fn mint_sanad_on_chain(...) {
    match chain.as_str() {
        "sui" => { /* real implementation */ }
        "solana" => { /* real implementation */ }
        _ => Err(CrossChainError::ChainNotSupported(...))  // catches "bitcoin", "ethereum", "aptos"
    }
}
```

**Impact:** Any cross-chain transfer *to* Bitcoin, Ethereum, or Aptos will fail at the mint step with `ChainNotSupported`. Since Bitcoin and Ethereum are the two most expected destination chains, this effectively makes the cross-chain feature non-functional for most users.

**Fix for Bitcoin (destination):** Bitcoin cannot natively "mint" a Sanad in the Move/Solana sense. The correct pattern is a **tapret** (taproot-embedded commitment) output sent to the destination owner's address. Use the existing `csv-bitcoin::tapret` module:

1. Construct a Taproot output embedding the Sanad commitment via `TapretProof`.
2. Sign and broadcast using `BitcoinRpc::broadcast_transaction`.
3. Return the TXID as `mint_tx_hash`.

**Fix for Ethereum (destination):** Call `CsvMintClient::mint_sanad(...)` — the binding already exists in `csv-ethereum/src/bindings/csv_mint.rs`. Encode the cross-chain proof as the `proof` and `proofRoot` parameters.

**Fix for Aptos (destination):** Call the `csv_seal::mint_sanad` Move function via `AptosRpc::submit_transaction` with the `EntryFunction` builder already present in `csv-aptos/src/entry_function.rs`.

### 3.4 🔴 `EthereumBackend::validate_transaction` — Signature Check Skipped

**File:** `csv-ethereum/src/ops.rs`

```rust
// TODO: Fix signature validation once Alloy API is stable
// For now, skip signature validation as the API has changed
```

**Impact:** Ethereum transactions are accepted into the protocol without signature verification. A malformed or spoofed transaction can pass validation.

**Fix:** Use `alloy_consensus::TxEnvelope::recover_signer()` from the `alloy-consensus` crate (which is already in the dependency tree via `alloy`). The call is:

```rust
let signer = tx_envelope.recover_signer()?;
```

This recovers the signing address from the ECDSA signature without an unstable API.

### 3.5 🔴 `csv-core/src/zk_proof.rs` — Placeholder Verifier Keys

**File:** `csv-core/src/zk_proof.rs`, function `default_verifier_registry()`

```rust
/// This is a convenience function that registers placeholder verifiers.
/// In production, these would be loaded from on-chain verifier contracts.
```

**Impact:** ZK proof verification uses placeholder (zero-length) verifier keys. Any ZK proof will either trivially pass or fail, depending on whether the backend checks the key, not whether the proof is mathematically valid.

**Fix:** Remove `default_verifier_registry()` entirely from non-test code. Verifier keys must be loaded at runtime from a versioned, chain-anchored registry (the `ZkVerifierRegistry` pattern is correct; just forbid the placeholder path). Add a CI check: `rg "default_verifier_registry\(\)" --glob "!*test*" --glob "!*spec*"` must return zero results.

### 3.6 🔴 `BitcoinSpvProver::prove_seal_consumption` — Mock SP1 Proof in Production Path

**File:** `csv-bitcoin/src/zk_prover.rs`

```rust
if self.sp1_available && self.prover_key.is_some() {
    // In production with SP1 available:
    // ... For now, we generate a structured placeholder ...
    proof_data.extend_from_slice(b"SP1_BTC_SPV_");
    proof_data.extend_from_slice(witness.hash().as_bytes());
    // This is NOT a real ZK proof.
}
```

**Impact:** Even when `sp1_available == true`, the prover emits a fake proof prepended with `"SP1_BTC_SPV_"`. Any verifier that trusts this proof is accepting non-cryptographic data as a ZK proof.

**Fix:** Integrate the actual SP1 proving pipeline. The guest program (`csv-bitcoin/src/sp1_guest/spv.rs`) is already written. The missing step is calling `sp1_sdk::ProverClient::prove(elf, stdin)`. Use `SP1_PROVER_KEY` env var for the proving key. Gate with `#[cfg(feature = "sp1")]` and return a hard error (not a fake proof) when SP1 is unavailable in production builds.

### 3.7 🟠 `update_manifest.rs` — Hardcoded `"TODO"` in Deployment Manifest

**File:** `csv-contracts/ethereum/scripts/update_manifest.rs`

```rust
lock_obj.insert("bytecode_hash", json!("TODO: Compute deployed bytecode hash"));
mint_obj.insert("bytecode_hash", json!("TODO: Compute deployed bytecode hash"));
constructor_args.insert("verifier", json!("TODO: Set verifier address"));
```

**Impact:** The deployment manifest shipped to production contains literal `"TODO"` strings. Any tooling that validates bytecode hashes will mismatch. The verifier address is not set.

**Fix:** Use `sha3::Keccak256::digest(bytecode_bytes)` to compute the hash. The verifier address must be resolved from `foundry.toml` or a deployment config file. This script must fail loudly if the addresses are missing.

### 3.8 🟠 `csv-contracts/solana/build.rs` — Empty Placeholder Bytecode

**File:** `csv-contracts/solana/build.rs`

```rust
// Program ID mismatch — runtime deployment will need actual bytecode.
println!("cargo:warning=Using empty placeholder - runtime deployment will need actual bytecode.");
String::new() // Empty placeholder - callers must handle missing bytecode
```

**Impact:** Any code that reads the compiled Solana program bytecode from this build artifact gets an empty string. Deployment scripts depending on this artifact will silently deploy nothing.

**Fix:** Perform a real `anchor build` during the build step (gated on `ANCHOR_BUILD=1` env var), or commit the compiled `.so` file and embed it with `include_bytes!`. Add a `panic!` fallback instead of `String::new()` so the failure is loud.

### 3.9 🟠 `NftGallery` and `NftCollections` — Hardcoded Empty State in UI

**File:** `csv-wallet/src/pages/nft_page.rs`

```rust
let _nfts: Vec<NftRecord> = Vec::new(); // TODO: Implement NFT fetching
let _collections: Vec<NftCollection> = Vec::new(); // TODO: Implement NFT collection fetching
```

**Impact:** The NFT Gallery page is reachable from the wallet navigation and always shows an empty state. This is not a safety issue but produces a broken user experience.

**Fix:** Either implement NFT fetching via chain-specific token APIs (Alchemy `alchemy_getNFTs`, Solana `getProgramAccounts`, Sui `suix_getOwnedObjects`) or remove the NFT routes from navigation until implemented. Do not ship a permanently empty page.

### 3.10 🟠 All Explorer `get_latest_synced_block()` Return `Ok(0)`

**Files:** All indexers in `csv-explorer/indexer/src/`

```rust
async fn get_latest_synced_block(&self) -> ChainResult<u64> {
    Ok(0)
}
```

**Impact:** The explorer's sync coordinator uses this function to determine the resume point after a restart. Always returning `0` means the indexer restarts from genesis on every process restart, causing full re-indexing and massive RPC load.

**Fix:** Store the last synced block in the `csv-explorer/storage` PostgreSQL database. Read it via `SELECT MAX(block_number) FROM seals WHERE chain = $1` on startup, or maintain a dedicated `sync_state` table. Use the `csv-explorer/storage/src/repositories/sync.rs` module that already exists for this purpose.

---

## 4. Cross-Chain Transfer Verifiability

### 4.1 State Machine Correctness

The `TransferState` enum in `csv-core/src/cross_chain.rs` and the companion `TransferStatus` in `csv-core/src/protocol_version.rs` correctly model the full lifecycle: `Locked → AwaitingFinality → BuildingProof → ProofReady → Minting → Complete`. The `CrossChainTransfer::execute()` orchestrator enforces the sequence in code. **This is sound.**

### 4.2 `StandardTransferVerifier` — Correctness

The verifier performs all required checks in order:

1. Proof-chain compatibility (`inclusion_proof.matches_chain`).
2. Hash algorithm consistency.
3. Double-spend check against the in-memory `CrossChainRegistry`.
4. Ownership signature verification.
5. Finality depth check.

**Gap 🟡:** The ownership verification calls `verify_ownership`, which dispatches to `Signature::verify(scheme)`. However, the `scheme` passed is derived from `proof.lock_event.owner.scheme`, which is an `Option`. If `None`, the code falls through to the default scheme. A malicious actor can omit the scheme field to force the verifier to use the protocol default (`MlDsa65`), which — without the `pq` feature — returns an error rather than verifying. This causes legitimate transfers to be rejected when the `pq` feature is not compiled in, rather than using the chain's native scheme. **Always derive the expected scheme from `CrossChainHashAlgorithm::for_chain(&source_chain)`, not from the proof payload.**

### 4.3 Finality Depth — Chain-Specific Values

| Chain | Configured Depth | Correct Industry Value |
|-------|-----------------|----------------------|
| Bitcoin | 6 (`MIN_REQUIRED_CONFIRMATIONS`) | ✅ 6 |
| Ethereum | 12 (hardcoded in `EthereumVerifier`) | ✅ 12 post-merge |
| Solana | 32 slots (from `chains/solana.toml`) | ✅ ~32 slots ≈ confirmed |
| Aptos | 1 (HotStuff, correct) | ✅ |
| Sui | 1 (Narwhal/Bullshark, correct) | ✅ |

**Gap 🟡:** Ethereum's `verify_finality` hardcodes `12` inline in the method body. This should be loaded from `EthereumConfig::finality_depth` (which is populated from `chains/ethereum.toml`). If the config value changes, the in-code value won't update.

### 4.4 `CrossChainRegistry` — In-Memory Only, Not Durable

**File:** `csv-core/src/cross_chain.rs`

The `CrossChainRegistry` uses a `HashMap` in memory. The `PersistentTransferRegistry` in `csv-sdk/src/cross_chain.rs` wraps SQLite. The problem is that `CrossChainTransfer::execute()` takes the *core* `CrossChainRegistry` by value, not the SDK's persistent one. The CLI correctly calls `PersistentTransferRegistry::load_into_registry()` before execution and `save_from_registry()` after. **The wallet does not perform this load/save cycle** — transfers recorded in wallet sessions are lost on restart, making the in-wallet double-spend protection non-durable.

**Fix 🟠:** The wallet must use `PersistentTransferRegistry` backed by SQLite (or `csv-store`'s encrypted storage) and perform load/save around every transfer execution.

### 4.5 Inclusion Proof Verification — Chain Analysis

| Chain | Proof Type | Implementation |
|-------|-----------|----------------|
| Bitcoin | `BitcoinMerkleProof` — double-SHA256 Merkle branch | `BitcoinVerifier::verify_inclusion` parses a binary format (`"CSV_BTC_PROOF"` prefix) and validates a SHA256d checksum. **Does not implement a full SPV Merkle branch verification** — only checks a self-computed checksum. |
| Ethereum | `EthereumMPTProof` — MPT storage proof | `EthereumVerifier::verify_inclusion` calls `verify_storage_proof` via `csv-ethereum/src/mpt.rs`. Correct path. |
| Sui | `SuiCheckpointProof` — checkpoint Merkle path | `SuiVerifier::verify_inclusion` walks the Merkle path correctly using SHA256 with domain separation `"SUI::CHECKPOINT::LEAF"`. |
| Aptos | `AptosLedgerProof` — accumulator Merkle path | `AptosVerifier::verify_inclusion` walks the Merkle path using SHA256 with `"APTOS::ACCUMULATOR::LEAF"`. |
| Solana | `SolanaSlotProof` — slot-based | `SolanaVerifier::verify_inclusion` calls `verify_inclusion_proof` from `csv-solana/src/proofs.rs`. |

**Critical gap 🔴 (Bitcoin):** The Bitcoin inclusion proof implementation (`BitcoinVerifier::verify_inclusion`) validates an internal checksum but does not verify the actual SPV Merkle branch against the block header's `merkle_root` field. A crafted proof with valid checksums but an invalid Merkle path will pass.

**Required algorithm (Bitcoin SPV Merkle verification):**

1. Deserialise the 80-byte block header; extract `merkle_root` at bytes 36–68.
2. Starting from `txid`, iteratively SHA256d-hash with each `merkle_branch` node (left/right determined by the tx index bit at each level).
3. Compare the final computed root with the header's `merkle_root`.
4. Verify the block header's `proof_of_work` (hash ≤ target) using `bitcoin::BlockHeader::validate_pow`.
5. Confirm `confirmations` using the current chain tip from `BitcoinRpc::get_block_count()`.

Use the `bitcoin` crate's `merkle_tree::verify_merkle_proof` function for step 2.

---

## 5. Chain-by-Chain Compatibility Matrix

### 5.1 Bitcoin

| Feature | Status | Notes |
|---------|--------|-------|
| RPC node connection | ✅ | `csv-bitcoin/src/node.rs` + mempool.space fallback |
| UTXO seal (tapret) | ✅ | `tapret.rs`, `bip341.rs` |
| SPV inclusion proof generation | ⚠️ | `csv-bitcoin/src/proofs.rs` generates proof; see §4.5 for verification gap |
| SPV inclusion proof verification | 🔴 | Only checksum, not real Merkle verification |
| Finality proof | ✅ | Confirmation count via RPC |
| Seal registry (UTXO spent check) | ✅ | `is_utxo_unspent` via RPC |
| Signature scheme | ✅ | Secp256k1 ECDSA |
| Cross-chain as source | ✅ | Lock via tapret output |
| Cross-chain as destination | 🔴 | Missing from `mint_sanad_on_chain` dispatch |
| Taproot address format (wallet) | 🟠 | `"bc1q" + hex(pubkey[..20])` is P2WPKH encoding, not Taproot. Taproot uses bech32m (`bc1p` prefix) and encodes the tweaked internal key. |
| Explorer indexer | ✅ | Fixed two-step Mempool.space API |

### 5.2 Ethereum

| Feature | Status | Notes |
|---------|--------|-------|
| RPC node connection | ✅ | Alloy-based `EthereumNode` |
| MPT storage proof | ✅ | `csv-ethereum/src/mpt.rs` |
| MPT verification | ✅ | `verify_storage_proof` |
| Seal registry (CSVLock) | ✅ | `verify_seal_registry` queries storage proof |
| Signature scheme | ⚠️ | Secp256k1 OK; `validate_transaction` skips sig check |
| Contract deployment | ✅ | Foundry-delegated (correct) |
| Cross-chain as source | ✅ | `CSVLock::lockSanad` |
| Cross-chain as destination | 🔴 | Missing from `mint_sanad_on_chain` dispatch |
| Explorer indexer | ✅ | keccak256 event signatures fixed |
| EIP-1559 fee support | 🟡 | `gas_price` fields are legacy; no `maxFeePerGas`/`maxPriorityFeePerGas` |

### 5.3 Solana

| Feature | Status | Notes |
|---------|--------|-------|
| RPC connection | ✅ | `csv-solana/src/node.rs` |
| Seal (PDA account) | ✅ | Anchor program `csv-seal` |
| Slot inclusion proof | ✅ | `verify_inclusion_proof` |
| Seal registry | ✅ | Account closed = consumed |
| Signature scheme | ✅ | Ed25519 |
| Cross-chain as source | ✅ | Program `lock_sanad` instruction |
| Cross-chain as destination | ✅ | `mint_sanad_from_hex_key` |
| Explorer indexer | ✅ | `getSlot` fix applied |
| Anchor IDL sync | 🟡 | `instructions.rs` is empty — all instructions defined in `lib.rs`. Fine for now but breaks modular IDL generation. |
| Deprecated `build_solana_transaction` | 🟡 | Marked `DEPRECATED` in `csv-sdk`; callers must migrate to `build_solana_transaction_with_blockhash`. |

### 5.4 Aptos

| Feature | Status | Notes |
|---------|--------|-------|
| RPC connection | ✅ | `csv-aptos/src/node.rs` |
| Accumulator proof | ✅ | Sibling hash walk |
| Seal registry | ✅ | Resource existence check |
| Signature scheme | ✅ | Ed25519 |
| Cross-chain as source | ✅ | `csv_seal::lock_sanad` Move entry function |
| Cross-chain as destination | 🔴 | Missing from `mint_sanad_on_chain` dispatch |
| Explorer indexer | ✅ | AptosEvent parsed from Move events |
| Finality | ✅ | HotStuff instant, confirmed by `finality_data` presence |

### 5.5 Sui

| Feature | Status | Notes |
|---------|--------|-------|
| RPC connection | ✅ | `csv-sui/src/node.rs` |
| Checkpoint proof | ✅ | Merkle path walk |
| Object seal | ✅ | Sui object ownership model |
| Seal registry | ✅ | Object deleted = consumed |
| Signature scheme | ✅ | Ed25519 |
| Cross-chain as source | ✅ | Move `lock_sanad` |
| Cross-chain as destination | ✅ | `csv_sui::mint::mint_sanad` |
| Explorer indexer | ✅ | SUI indexer functional |
| `seal_contract package_id` | 🟡 | Falls back to placeholder `"0x0"` if `package_id` is `None` in config. Should be required, not optional. |

---

## 6. Smart Contract Maturity & Deployability

### 6.1 Ethereum — CSVLock.sol

**Overall: 🟠 HIGH risk — two unguarded functions**

| Item | Status |
|------|--------|
| Nullifier double-spend prevention | ✅ `usedSeals` mapping |
| Cross-chain lock event emission | ✅ `CrossChainLock` event |
| Time-locked refund | ✅ `REFUND_TIMEOUT = 24 hours` |
| Re-entrancy protection | ✅ State written before external call in `refundSanad` |
| `setLockContract` access control | 🔴 `external` with no `onlyOwner` or `onlyAdmin` modifier — **any address can replace the lock contract address** |
| `registerNullifier` access control | 🔴 `external` with no guard — **any address can register nullifiers**, blocking legitimate seals |
| ERC-165 interface support | 🟡 None; consider adding for composability |
| Events indexed properly | ✅ Key fields are `indexed` |
| Gas optimisations | ✅ Batch SLOADs noted in comments |

**Required fix for `setLockContract`:**

```solidity
address public owner;
constructor(address _mintContract) {
    owner = msg.sender;
    mintContract = _mintContract;
}
modifier onlyOwner() {
    require(msg.sender == owner, "CSVLock: not owner");
    _;
}
function setLockContract(address _lockContract) external onlyOwner { ... }
```

**Required fix for `registerNullifier`:** Either restrict to `onlyOwner`/trusted relayer, or remove it if `markSealUsed` covers the same purpose (they overlap).

### 6.2 Ethereum — CSVMint.sol

**Overall: 🟠 HIGH risk — `trusted_verifier` not enforced on all paths**

| Item | Status |
|------|--------|
| Double-mint prevention | ✅ `mintedSanads` mapping |
| Nullifier registration | ✅ |
| On-chain Merkle proof verification | ✅ `_verifyMerkleProof` with leaf position |
| `trusted_verifier` enforcement | ⚠️ `mintSanad` checks `msg.sender == trustedVerifier` but `batchMintSanads` does not — review |
| Leaf hash construction | ✅ `keccak256(sanadId ‖ commitment ‖ sourceChain)` |
| `_hashPair` ordering | 🟡 Uses sort-based canonical ordering — ensure this matches off-chain proof construction exactly |

### 6.3 Solana — csv-seal Anchor Program

**Overall: 🟡 MEDIUM — functional but missing events**

| Item | Status |
|------|--------|
| State accounts | ✅ `SealRegistry`, `SealAccount`, `LockAccount` in `state.rs` |
| Instructions: `initialize`, `lock_sanad`, `mint_sanad`, `refund_sanad` | ✅ Defined in `lib.rs` |
| `instructions.rs` | 🟡 Empty file — all logic in `lib.rs`. This prevents Anchor IDL from correctly generating per-instruction documentation. |
| Events | ✅ Defined in `events.rs` |
| Errors | ✅ Defined in `errors.rs` |
| Tests | ⚠️ `csv_seal.ts` only tests `initialize` — `lock_sanad`, `mint_sanad`, `refund_sanad` not tested |
| Deployment script | ✅ `deploy.sh` exists |
| Program ID in `Anchor.toml` | 🟡 Must match the compiled program's derived address — verify after each build |

### 6.4 Aptos — csv_seal.move

**Overall: ✅ GOOD — well-structured Move module**

| Item | Status |
|------|--------|
| Resource model (Move semantics) | ✅ Correct use of `move_from`, `move_to` |
| Access control | ✅ `signer` enforced |
| Registry initialisation guard | ✅ `assert!(!exists<Registry>(addr))` |
| Refund path | ✅ With timeout |
| Testnet deployment output | ✅ `deploy-output-testnet.txt` present |
| Contract address in `chains/aptos.toml` | 🟡 Still `0x1234...` placeholder — update with real deployed address |

### 6.5 Sui — csv_seal.move

**Overall: ✅ GOOD**

| Item | Status |
|------|--------|
| Object model | ✅ Correct use of Sui objects |
| Registry as shared object | ✅ |
| `Published.toml` | ✅ Present with real package ID |
| Duplicate source file | 🟡 `csv-contracts/sui/contracts/csv_seal.move` AND `csv-contracts/sui/sources/csv_seal.move` AND `csv-contracts/sui/contracts/sources/csv_seal.move` — three copies. Determine canonical location, delete duplicates. |

---

## 7. Explorer Indexer Completeness

### 7.1 Sync State Persistence — All Chains

Every chain indexer returns `Ok(0)` from `get_latest_synced_block()`. This causes full re-indexing on restart.

**Fix (all chains):** In `csv-explorer/storage/src/repositories/sync.rs`, add:

```sql
CREATE TABLE IF NOT EXISTS sync_state (
    chain TEXT PRIMARY KEY,
    last_synced_block BIGINT NOT NULL DEFAULT 0,
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
```

Load with `SELECT last_synced_block FROM sync_state WHERE chain = $1`. Upsert after each block.

### 7.2 Transfer Indexing — Bitcoin, Ethereum, Aptos Missing

```
async fn index_transfers(&self, _block: u64) -> ChainResult<Vec<TransferRecord>> {
    Ok(Vec::new())  // Bitcoin, Ethereum, Aptos
}
```

**Fix for each chain:**

- **Bitcoin:** Parse OP_RETURN outputs matching `CSV_XFER_` magic prefix (same detection as sanad parsing). Extract `sanad_id`, `source_chain`, `destination_chain` from the script.
- **Ethereum:** Filter logs for `CrossChainLock` event (`sig_cross_chain_lock()`) and `SanadMinted` event. Both event signatures are already computed in `ethereum.rs`.
- **Aptos:** Filter Move events for type `{module_address}::csv_seal::CrossChainLockEvent` and `MintEvent`.

### 7.3 Contract Indexing — All Chains Return Single Static Entry

```
async fn index_contracts(&self, _block: u64) -> ChainResult<Vec<CsvContract>> {
    Ok(vec![CsvContract { /* static program_id */ }])
}
```

This is acceptable as a bootstrap, but the contract list should be loaded from the `contracts` database table seeded by the deployment manifest, not hardcoded per block.

---

## 8. Cryptography & Signature Correctness

### 8.1 ML-DSA-65 (Post-Quantum) — Feature-Gated but Unverified in CI

The `pq` feature flag enables real `pqcrypto_dilithium::dilithium3` operations. Without the flag, `verify_ml_dsa65` returns a hard error. The flag is not in the default feature set.

**Gap 🟠:** `SignatureScheme::default()` returns `MlDsa65`, meaning any code that creates a `SignatureScheme` without specifying one uses a scheme that will fail verification unless `pq` is compiled in. This creates a silent version mismatch: a proof signed with `MlDsa65` on a `pq`-enabled build cannot be verified on a non-`pq` build (CLI, wallet browser build).

**Fix:** Add `pq` to the default feature set for `csv-core`, or change the default scheme back to `Secp256k1` and expose `MlDsa65` as an opt-in. Document clearly in `PROTOCOL_INVARIANTS.md` which scheme is expected for each proof type.

### 8.2 Bitcoin Taproot Address Encoding (csv-wallet)

**File:** `csv-wallet/src/chains/bitcoin.rs`

```rust
pub fn format_address(pubkey_bytes: &[u8]) -> String {
    // Simplified Taproot address format
    format!("bc1q{}", hex::encode(&pubkey_bytes[..20]))
}
```

This is P2WPKH (SegWit v0) encoded with a hex suffix instead of bech32, which is **not a valid Bitcoin address** in any format. Taproot addresses use bech32m encoding with a `bc1p` prefix.

**Fix:** Use the `bitcoin` crate:

```rust
use bitcoin::key::TweakedPublicKey;
use bitcoin::address::Address;
use bitcoin::Network;

pub fn format_address(pubkey_bytes: &[u8], network: Network) -> String {
    let internal_key = XOnlyPublicKey::from_slice(&pubkey_bytes[..32])
        .expect("valid 32-byte x-only key");
    // Taproot key-path spend: tweak with empty script
    let tweaked = TweakedPublicKey::dangerous_assume_tweaked(internal_key);
    Address::p2tr_tweaked(tweaked, network).to_string()
}
```

### 8.3 Secp256k1 Verification — Message Must Be Pre-Hashed

**File:** `csv-core/src/signature.rs`

`verify_secp256k1` enforces `message.len() == 32`, correctly requiring a pre-hashed message. All callers pass `bundle.transition_dag.root_commitment.as_bytes()` which is a `Hash` (32 bytes). **Correct.**

### 8.4 Ed25519 Signature Length Constraint

`verify_ed25519` enforces `signature.len() == 64` and `public_key.len() == 32`. The Solana, Sui, and Aptos signature parsers all use the `[pk_len (4 bytes LE)][public_key][signature]` wire format, which preserves these constraints. **Correct.**

### 8.5 Cross-Chain Hash Algorithm Dispatch

`CrossChainHashAlgorithm::for_chain()` correctly maps:

- `"bitcoin"` → `DoubleSha256`
- `"ethereum"` / `"solana"` → `Keccak256`
- `"aptos"` → `Sha3_256`
- `"sui"` → `Sha256`

**Gap 🟡:** Solana's native hashing is SHA256 (not Keccak256). The Solana inclusion proof in `SolanaVerifier::verify_inclusion` calls the external `verify_inclusion_proof` from `csv-solana::proofs`. If that function uses SHA256 internally but the cross-chain hash algorithm claims Keccak256, the `attested_root_hash` computed for the transfer proof will mismatch. Audit `csv-solana/src/proofs.rs` and align with `for_chain("solana")`.

---

## 9. csv-cli Production Gaps

### 9.1 Cross-Chain Transfer — Correct Architecture

`csv-cli/src/commands/cross_chain/transfer.rs` correctly uses `csv_sdk::CsvClient::transfers().cross_chain(...)`. The state recording after transfer uses `state.add_transfer(...)`. **This path is correct.**

### 9.2 Proof Generation (`cmd_proofs`) — Verify Chain Backend is Real

`csv-cli/src/commands/proofs.rs` calls through the SDK. Ensure the `AdapterBuilder` in the CLI config wires real RPC clients, not the `InMemory` store backend used by `BlockchainService` in the wallet.

### 9.3 `cmd_validate` — Offline Verification Gap

**File:** `csv-cli/src/commands/validate.rs`

The validate command calls `verify_proof()` from `csv-core::verifier`. The `seal_registry` callback passed is: `|_| false` (always reports seal as unused). This means the CLI's offline validation **never detects double-spends**. It only checks cryptographic correctness, not seal uniqueness.

**Fix:** The CLI should query the appropriate `ChainVerifier::verify_seal_registry()` for the relevant chain (deduced from `bundle.anchor_ref`) before reporting a seal as valid. Wire through `CsvClient::chain_runtime()`.

### 9.4 Balance Query — Real Implementation

`csv-cli/src/commands/wallet/balance.rs` delegates to `ChainApi::get_balance()` which calls `ChainRuntime::get_balance()`. **This is real.** Verify the runtime actually calls `ChainQuery::get_balance` on the registered adapter.

### 9.5 Wallet Export / QR Code

**File:** `csv-wallet/src/pages/validate/offline.rs` (line ~80069)

```rust
// QR code placeholder (simplified - static pattern)
// TODO: Implement actual export
```

The QR code in the offline verification page renders a static placeholder pattern. Use the `qrcode` crate (`qrcode = "0.13"`) to generate a real QR encoding of the proof bundle hash or consignment URL.

---

## 10. csv-wallet Production Gaps

### 10.1 Wallet Storage Backend — Always InMemory

**File:** `csv-wallet/src/services/blockchain.rs`

```rust
CsvClientBuilder::new()
    .with_store_backend(csv_sdk::builder::StoreBackend::InMemory)
    .build()
```

The wallet's internal `BlockchainService` uses an `InMemory` store. All wallet state written through this path is lost on reload. The wallet also has `csv-wallet/src/core/storage.rs` and `csv-wallet/src/wallet/storage.rs` for encrypted local persistence — these must be the backing store for the SDK client in the wallet.

**Fix:** Initialise `CsvClient` with `StoreBackend::Encrypted { path, passphrase }` using the wallet's existing `EncryptedStorage` infrastructure.

### 10.2 Seal Consumption Page — Registry Not Consulted

**File:** `csv-wallet/src/pages/seals/consume.rs`

The consume page does not check whether the seal is already consumed on-chain before submitting the transaction. A user could attempt to consume an already-spent UTXO (Bitcoin) or a closed PDA (Solana), wasting fee money and getting a confusing error.

**Fix:** Before showing the "Consume" button as active, call `ChainVerifier::verify_seal_registry(seal_id)` and display a warning if the seal is already consumed.

### 10.3 `use_wallet_connection` Hook — MetaMask Always Fails

**File:** `csv-wallet/src/hooks/use_wallet_connection.rs` and `csv-wallet/src/services/blockchain.rs`

```rust
pub async fn connect_metamask() -> Result<NativeWallet, String> {
    Err("MetaMask not available".to_string())
}
```

MetaMask connection always returns an error. For the browser WASM target, this must use `wasm_bindgen_futures` and call the `window.ethereum.request({ method: 'eth_requestAccounts' })` JS API via `web_sys`.

### 10.4 Proof Export / Share — Three Unimplemented TODOs

**File:** `csv-wallet/src/pages/validate/offline.rs`

```rust
// TODO: Implement proof export functionality
// TODO: Implement share functionality
// TODO: Implement save to wallet functionality
```

**Fix:** Use the `csv-core/src/consignment.rs` `Consignment::to_bytes()` serialiser and store in `csv-wallet/src/wallet/storage.rs`. For sharing, encode as base64 and open a `data:` URI or use the Web Share API via `wasm_bindgen`.

---

## 11. Prioritised Fix Plan

### Phase 1 — Critical Safety (Do Before Any Mainnet Activity)

| ID | Fix | Owner Hint |
|----|-----|-----------|
| P1-1 | `ParallelVerifyService::verify_single_seal/proof` — implement real verification via `ChainVerifier` | `csv-wallet/src/services/parallel_verify.rs` |
| P1-2 | `BlockchainService` transfer functions — delete fake implementations, route to `CsvClient::transfers()` | `csv-wallet/src/services/blockchain.rs` |
| P1-3 | `mint_sanad_on_chain` — add Bitcoin (tapret), Ethereum (`CsvMintClient`), and Aptos (`EntryFunction`) arms | `csv-sdk/src/cross_chain.rs` |
| P1-4 | Bitcoin SPV Merkle branch verification — replace checksum with `bitcoin::merkle_tree::verify_merkle_proof` | `csv-bitcoin/src/verifier.rs` |
| P1-5 | `CSVLock::setLockContract` and `registerNullifier` — add `onlyOwner` modifier | `csv-contracts/ethereum/contracts/src/CSVLock.sol` |
| P1-6 | `EthereumBackend::validate_transaction` — implement signature recovery via `alloy_consensus::TxEnvelope::recover_signer` | `csv-ethereum/src/ops.rs` |
| P1-7 | Remove `default_verifier_registry()` from non-test code; enforce runtime key loading | `csv-core/src/zk_proof.rs` |
| P1-8 | `BitcoinSpvProver` — return hard error (not fake proof) when SP1 unavailable in production build | `csv-bitcoin/src/zk_prover.rs` |
| P1-9 | `cmd_validate` (CLI) — query real seal registry, not always-false closure | `csv-cli/src/commands/validate.rs` |

### Phase 2 — High Priority Functional Gaps

| ID | Fix | Owner Hint |
|----|-----|-----------|
| P2-1 | `get_latest_synced_block` — persist to `sync_state` table for all chains | `csv-explorer/indexer/src/*.rs` |
| P2-2 | `index_transfers` — implement for Bitcoin, Ethereum, Aptos | `csv-explorer/indexer/src/` |
| P2-3 | `PersistentTransferRegistry` — use in wallet, not in-memory registry | `csv-wallet/src/core/wallet.rs` |
| P2-4 | Bitcoin Taproot address encoding — use `bitcoin::Address::p2tr_tweaked` with bech32m | `csv-wallet/src/chains/bitcoin.rs` |
| P2-5 | ML-DSA-65 default scheme mismatch — align default with `pq` feature availability | `csv-core/src/signature.rs` |
| P2-6 | Solana hash algorithm in cross-chain — verify `for_chain("solana")` = SHA256, not Keccak256 | `csv-core/src/cross_chain.rs`, `csv-solana/src/proofs.rs` |
| P2-7 | CSVMint.sol `batchMintSanads` — verify `trustedVerifier` check is applied | `csv-contracts/ethereum/contracts/src/CSVMint.sol` |
| P2-8 | `update_manifest.rs` — compute real bytecode hash, set verifier address | `csv-contracts/ethereum/scripts/update_manifest.rs` |
| P2-9 | Solana `build.rs` — emit hard error instead of empty bytecode | `csv-contracts/solana/build.rs` |
| P2-10 | MetaMask connect — implement via `web_sys::window().ethereum()` | `csv-wallet/src/services/blockchain.rs` |
| P2-11 | Wallet storage backend — use `EncryptedStorage`, not `InMemory` | `csv-wallet/src/services/blockchain.rs` |
| P2-12 | Ethereum `verify_finality` hardcoded `12` — load from `EthereumConfig` | `csv-ethereum/src/verifier.rs` |
| P2-13 | Ownership scheme derivation — use chain-canonical scheme, not proof-payload field | `csv-core/src/cross_chain.rs` |
| P2-14 | Seal consumption UI guard — check `verify_seal_registry` before enabling consume button | `csv-wallet/src/pages/seals/consume.rs` |

### Phase 3 — Medium / Quality

| ID | Fix |
|----|-----|
| P3-1 | Standardise all `ChainId` imports to `use csv_core::ChainId` |
| P3-2 | Deduplicate Sui Move contract files (three copies) |
| P3-3 | Aptos `chains/aptos.toml` — replace placeholder contract address with real deployed address |
| P3-4 | NFT Gallery — implement or remove from navigation |
| P3-5 | QR code in offline verify — use `qrcode` crate |
| P3-6 | Proof export / share in wallet — implement using `Consignment::to_bytes()` |
| P3-7 | Solana `instructions.rs` — move instruction handlers from `lib.rs` for clean IDL generation |
| P3-8 | EIP-1559 fee fields — add `maxFeePerGas`/`maxPriorityFeePerGas` to Ethereum transaction builder |
| P3-9 | Sui `seal_contract package_id` — make required, not optional |
| P3-10 | Add CI gate: `experimental` feature must never appear in production build profiles |
| P3-11 | Add CI gate: `default_verifier_registry()` calls in non-test code fail the build |

---

## 12. Per-Chain Verification & Compatibility Expansion Guide

This section provides junior-developer-friendly guidance for each chain.

### 12.1 Bitcoin — Adding Real SPV Verification

**Crate:** `bitcoin = "0.31"` (already present)

**Data structure:** Use `bitcoin::merkle_tree::PartialMerkleTree` which implements RFC-compliant SPV Merkle proof encoding.

**Algorithm:**

```rust
use bitcoin::{BlockHeader, Txid, merkle_tree::PartialMerkleTree};

pub fn verify_bitcoin_spv(
    txid: &[u8; 32],
    merkle_branch: &[[u8; 32]],
    block_header_bytes: &[u8; 80],
    expected_confirmations: u64,
    current_height: u64,
    lock_height: u64,
) -> bool {
    // 1. Deserialise block header
    let header: BlockHeader = bitcoin::consensus::deserialize(block_header_bytes)
        .expect("80-byte block header");
    
    // 2. Check proof of work
    if !header.validate_pow(header.target()).is_ok() { return false; }
    
    // 3. Verify txid is in merkle tree
    let txid = Txid::from_slice(txid).unwrap();
    // Build PartialMerkleTree from branch and compute root
    // bitcoin crate provides verify() on PartialMerkleTree
    let computed_root = compute_merkle_root(txid, merkle_branch);
    if computed_root != header.merkle_root { return false; }
    
    // 4. Check confirmations
    current_height.saturating_sub(lock_height) >= expected_confirmations
}

fn compute_merkle_root(txid: Txid, branch: &[[u8; 32]]) -> bitcoin::TxMerkleNode {
    let mut current = txid.to_byte_array();
    for sibling in branch {
        // Double-SHA256 with canonical ordering (smaller hash is left child)
        let (left, right) = if current <= *sibling {
            (current, *sibling)
        } else {
            (*sibling, current)
        };
        let mut hasher = sha2::Sha256::new();
        hasher.update(left);
        hasher.update(right);
        let first = hasher.finalize();
        let mut hasher2 = sha2::Sha256::new();
        hasher2.update(first);
        current = hasher2.finalize().into();
    }
    bitcoin::TxMerkleNode::from_byte_array(current)
}
```

### 12.2 Ethereum — EIP-1559 Transaction Builder

**Crate:** `alloy = "0.3"` (already present)

Replace `gas_price` with:

```rust
use alloy_rpc_types::TransactionRequest;

let tx = TransactionRequest::default()
    .max_fee_per_gas(base_fee * 2 + priority_fee)
    .max_priority_fee_per_gas(priority_fee)
    .gas_limit(estimated_gas);
```

Get `base_fee` from `eth_getBlockByNumber("pending", false).baseFeePerGas`.

### 12.3 Solana — Correct Blockhash Flow

```rust
use solana_sdk::{hash::Hash, transaction::Transaction};

// Always fetch fresh blockhash before building transaction
let blockhash: Hash = rpc.get_latest_blockhash()?;
let tx = Transaction::new_with_payer(&instructions, Some(&payer.pubkey()));
tx.sign(&[&payer], blockhash);
```

Never use the deprecated `build_solana_transaction` which takes a placeholder blockhash.

### 12.4 Aptos — Entry Function Pattern

```rust
use csv_aptos::entry_function::EntryFunctionBuilder;

let entry_fn = EntryFunctionBuilder::new(module_address, "csv_seal", "mint_sanad")
    .arg_bytes32(sanad_id)
    .arg_bytes32(commitment)
    .arg_string(source_chain)
    .arg_bytes(source_seal_point)
    .arg_bytes(proof_bytes)
    .build();

rpc.submit_transaction(&signer, entry_fn).await?;
```

The builder is already present in `csv-aptos/src/entry_function.rs`.

### 12.5 Sui — Move Call Pattern

```rust
use csv_sui::ops::SuiBackend;

let result = backend.call_move_function(
    package_id,
    "csv_seal",
    "mint_sanad",
    vec![/* type args */],
    vec![
        SuiCallArg::Pure(bcs::to_bytes(&sanad_id)?),
        SuiCallArg::Pure(bcs::to_bytes(&commitment)?),
        SuiCallArg::Object(registry_object_id),
    ],
    gas_budget,
).await?;
```

---

## Appendix A — Forbidden Patterns CI Checklist

Add these checks to `.github/workflows/production-guarantee.yml`:

```yaml
- name: No placeholder verifier keys in production
  run: |
    rg "default_verifier_registry\(\)" --glob "!*test*" --glob "!*spec*" && exit 1 || true

- name: No SP1 mock proofs in production
  run: |
    rg "SP1_BTC_SPV_" --glob "!*test*" && exit 1 || true

- name: No pending/placeholder transaction hashes
  run: |
    rg '"pending"' csv-wallet/src csv-cli/src --glob "!*test*" && exit 1 || true

- name: No always-false seal registry closures
  run: |
    rg "\|_\| false" csv-core/src csv-sdk/src csv-cli/src --glob "!*test*" && exit 1 || true

- name: No experimental feature in production profiles
  run: |
    if grep -r "experimental" Cargo.toml --include="*.toml" | grep -v "#"; then exit 1; fi
```

---

## Appendix B — Test Coverage Gaps

The compile-fail tests in `csv-core/tests/compile_fail/` are excellent and must be kept. The following integration test scenarios are **missing**:

1. **Double-spend across sessions** — Start a transfer, kill the process, restart, attempt the same transfer. Should fail due to `PersistentTransferRegistry`.
2. **Bitcoin SPV against real signet block** — The examples in `csv-bitcoin/examples/signet_*.rs` exist but should be promoted to integration tests in CI using a signet node.
3. **Cross-chain transfer end-to-end** — Lock on Ethereum Sepolia, verify on Solana devnet (or vice versa). The `scripts/test-cross-chain.sh` script exists but is not in the CI pipeline.
4. **Reorg recovery** — `csv-core/src/reorg/` exists but no integration test exercises the reconciliation path with a real chain.
5. **Solana Anchor instruction coverage** — `csv_seal.ts` only tests `initialize`. `lock_sanad`, `mint_sanad`, and `refund_sanad` must have test cases before mainnet.
