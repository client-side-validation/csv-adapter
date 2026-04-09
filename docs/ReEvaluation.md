# Comprehensive CSV Adapter Workspace Audit

## 1. Sprint Status

### Sprint 0: Canonical Model + Cryptographic Foundations (Weeks 1-4)

**Status: ~75% COMPLETE**

#### 0.1 Right Type — COMPLETE

- **File:** `csv-adapter-core/src/right.rs`
- Full struct with `id`, `commitment`, `owner`, `nullifier`, `state_root`, `execution_proof`
- `create()` (as `new()`), `consume()`, `verify()`, `transfer()` all implemented
- Canonical serialization/deserialization roundtrip verified with tests
- 18 unit tests, all passing

#### 0.2 Remove CommitmentV1 — COMPLETE

- **File:** `csv-adapter-core/src/commitment.rs`
- `CommitmentV1` struct fully removed. Only V2 exists (`COMMITMENT_VERSION: u8 = 2`)
- `from_canonical_bytes()` rejects V1: "Unsupported commitment version"
- `#[deprecated]` alias `v1()` points to `simple()` — cosmetic remnant but functionally identical
- No code references old V1 pattern matching

#### 0.3 Tagged Hashing — PARTIALLY COMPLETE

- **File:** `csv-adapter-core/src/tagged_hash.rs`
- BIP-340 style tagged hashing implemented: `tagged_hash(tag, data) = sha256(sha256(tag) || sha256(tag) || data)`
- `csv_tagged_hash()` wrapper with `"urn:lnp-bp:csv:"` prefix implemented
- **USED IN:** MPC tree (`mpc.rs`), DAG (`dag.rs`), commitment hashing (`commitment.rs`) — all 8 fields use tagged hashing
- **NOT USED IN:** Raw `Sha256::new()` still used directly in:
  - `right.rs` (ID computation, nullifier computation) — lines 97, 137
  - `commitment.rs` (seal_id hash computation, empty MPC root) — lines 59, 93, 107
  - `genesis.rs` line 54, `transition.rs` line 62, `schema.rs` line 279, `consignment.rs` line 121
  - `tapret_verify.rs` line 202
  - All Bitcoin proofs (`proofs.rs` lines 30, 36, 247, 251) — double-SHA256 for Merkle, which is Bitcoin-native and correct
  - All Aptos proofs (`proofs.rs` lines 85, 140) — uses domain-prefixed hashing but not csv_tagged_hash
  - Sui proofs (`proofs.rs` lines 43, 96) — raw SHA-256
  - Ethereum proofs (`proofs.rs` line 73) — raw SHA-256

#### 0.4 Swap Custom MPT for alloy-trie — PARTIALLY COMPLETE

- **File:** `csv-adapter-ethereum/src/mpt.rs`
- `alloy-trie` dependency added and used for `HashBuilder`, `compute_state_root()`
- `EMPTY_ROOT_HASH` constant properly used
- **BUT:** `verify_storage_proof()` just checks `!is_empty()` and returns `true` (line 28)
- `verify_receipt_proof()` just checks `!is_empty()` (line 55)
- `verify_full_receipt_proof()` checks non-empty + non-zero root (line 83) — does NOT actually reconstruct the trie and verify the root
- No test vectors against real Ethereum mainnet proofs

#### 0.5 Verify rgb_compat Tapret — PARTIALLY COMPLETE

- **File:** `csv-adapter-core/src/rgb_compat.rs`
- `rgb_compat.rs` has `TapretVerifier` and `RgbCompatibility` types
- Line 279: "In production, this would verify the actual taproot merkle proof"
- Missing: control block verification, Taproot merkle tree leaf position verification, internal key verification (all listed as TODO in plan)
- `CrossChainValidator` is a stub with basic commitment comparison only

---

### Sprint 1: Wire Real RPCs (Weeks 5-12)

**Status: ~70% COMPLETE**

| Chain | Status | Details |
|-------|--------|---------|
| **Bitcoin** | Structurally complete, needs funded Signet wallet | `real_rpc.rs` wraps `bitcoincore-rpc`. `send_raw_transaction()` works. `publish_commitment()` returns placeholder txid with TODO. Taproot tx building in `tx_builder.rs` is complete but not wired into `publish()` path. |
| **Sui** | Structurally complete, needs live node + deployed contract | `real_rpc.rs` implements full JSON-RPC: `sui_getObject`, `sui_getTransactionBlock`, `sui_executeTransactionBlock`. BCS TransactionData builder in adapter. `sender_address()` returns error (placeholder). Ed25519 signing ready. |
| **Aptos** | Structurally complete, needs live node + deployed contract | `real_rpc.rs` implements full REST API: accounts, resources, transactions, events. `submit_transaction()` returns placeholder hash. `submit_signed_transaction()` is real. `publish_module()` returns placeholder. `verify_checkpoint()` always returns true. |
| **Ethereum** | Most complete of all chains | `real_rpc.rs` uses Alloy for full EIP-1559 tx signing and broadcasting. `publish()` function builds calldata, gets nonce, signs, broadcasts. Requires `with_signer()` to be called. All JSON-RPC methods implemented. |

---

### Sprint 2: Client-Side Validation Engine (Weeks 13-16)

**Status: ~30% COMPLETE**

- `Right` type exists (Sprint 0) — **COMPLETE**
- `ValidationClient` in `csv-adapter-core/src/client.rs` — **STUBBED**
  - `verify_commitment_chain()` returns `Err(ChainError::EmptyChain)` always (line 177)
  - `verify_rights_and_seals()` iterates `seal_assignments` but does NOT map chain anchors to unified `Right` types
  - No anchor-to-Right mapping exists for any chain
  - `update_local_state()` creates synthetic records with zeroed `block_height` and `tx_hash`
- `ConsignmentValidator` in `csv-adapter-core/src/validator.rs` — **PARTIAL**
  - Structural validation: calls `consignment.validate_structure()` — real
  - Commitment chain validation: "placeholder" (line 148) — always passes
  - Seal consumption: uses `CrossChainSealRegistry` — real
  - State transition validation: "Simplified for now" (line 217) — always passes
- State history storage: `InMemoryStateStore` only, no persistent store
- Commitment chain verification: `verify_ordered_commitment_chain()` exists but is not wired into the client

---

### Sprint 3: End-to-End Testing (Weeks 17-20)

**Status: ~15% COMPLETE**

See Section 5 for full analysis.

---

### Sprint 4: Cross-Chain Right Portability (Weeks 21-24)

**Status: ~20% COMPLETE**

- `CrossChainSealRegistry` exists and detects cross-chain double-spends
- `CrossChainValidator` exists but only compares commitment hashes
- No lock-and-prove mechanism
- No cross-chain transfer format specified
- Nullifier scope: undecided (noted as "Open Design Edge #1" in plan)
- Settlement strategy: undecided (noted as "Open Design Edge #3")

---

### Sprint 5: RGB Verification (Weeks 25-27)

**Status: ~10% COMPLETE**

- `rgb_compat.rs` has stub validator with placeholder implementations
- No comparison against actual RGB reference implementation
- No interoperability testing

---

### Sprint 6: Security Hardening (Weeks 28-30)

**Status: ~5% COMPLETE**

- No fuzzing
- No property-based testing
- No external audit
- Basic replay prevention via seal registries

---

## 2. All Stubs, TODOs, Placeholders

### TODO / FIXME / unimplemented! (4 instances)

| File | Line | Text | Severity |
|------|------|------|----------|
| `csv-adapter-ethereum/src/rpc.rs` | 53 | `unimplemented!("as_any() must be implemented by concrete types")` | Low — fallback for non-downcastable types |
| `csv-adapter-sui/src/rpc.rs` | 71 | `unimplemented!("as_any() must be implemented by concrete types")` | Low — same pattern |
| `csv-adapter-aptos/src/rpc.rs` | 88 | `unimplemented!("as_any() must be implemented by concrete types")` | Low — same pattern |
| `csv-adapter-bitcoin/src/real_rpc.rs` | 189-190 | `// TODO: Integrate with tx_builder to create proper Taproot tx` / returns placeholder txid | **HIGH** — Bitcoin publish() cannot create real Taproot tx |

### Simulated/Placeholder Transaction IDs

| File | Pattern | Context |
|------|---------|---------|
| `csv-adapter-bitcoin/src/adapter.rs:318,333` | `b"sim-commit"` | Mock-mode publish returns deterministic fake txid |
| `csv-adapter-ethereum/src/adapter.rs:216` | `b"sim-tx-"` | Mock-mode publish returns fake tx hash |
| `csv-adapter-aptos/src/real_rpc.rs:247` | `b"aptos-submit"` | `submit_transaction()` returns fabricated hash |
| `csv-adapter-aptos/src/real_rpc.rs:361` | `b"aptos-module"` | `publish_module()` returns fabricated hash |

### "In production" / "For now" Comments (66 instances — key ones)

**Critical (affects correctness):**

- `csv-adapter-ethereum/src/proofs.rs:48-49`: "In production: fully verify MPT proof. For now, check proof has data"
- `csv-adapter-aptos/src/proofs.rs:168-181`: "In production: verify the Merkle proof against the accumulator root. For now, accept any valid proof with data"
- `csv-adapter-aptos/src/merkle.rs:420-421`: "In production, verify the proof against the ledger root. For now, just check that the root is non-zero"
- `csv-adapter-bitcoin/src/real_rpc.rs:182-190`: "In production, this would build and sign a proper Taproot commitment transaction. For now, it returns a placeholder txid"
- `csv-adapter-sui/src/real_rpc.rs:199`: "For now, return a placeholder" (sender_address)
- `csv-adapter-aptos/src/real_rpc.rs:375-376`: "In production, verify checkpoint signatures. For now, assume valid"

**Structural (affects production readiness):**

- `csv-adapter-sui/src/adapter.rs:455`: "For production: use sui-sdk's transaction builder"
- `csv-adapter-aptos/src/adapter.rs:252-253`: "In production, use aptos-sdk's BCS serialization. For now, use the JSON payload hash as the message to sign"
- `csv-adapter-core/src/client.rs:159`: "For now, we use anchors as a proxy for commitments"
- `csv-adapter-core/src/client.rs:177-178`: "For now, we'll create a synthetic commitment chain from the consignment. In production, commitments would be extracted from transitions"
- `csv-adapter-core/src/validator.rs:143-147`: "In production, extract commitments from consignment and verify chain. For now, record that this step ran" — passes true
- `csv-adapter-core/src/validator.rs:211-217`: "In production, verify state transitions... Simplified for now" — passes true

---

## 3. Partially Implemented Components

### AnchorLayer Method-by-Method Analysis

#### Bitcoin (`csv-adapter-bitcoin/src/adapter.rs`)

| Method | Status | Details |
|--------|--------|---------|
| `publish()` | PARTIAL | Checks UTXO unspent. With RPC: builds Taproot tx via `tx_builder`, signs, broadcasts via `send_raw_transaction`. Without RPC: returns `"sim-commit"` txid. Tx builder is NOT wired into the main `publish()` flow for real tx construction. |
| `verify_inclusion()` | PARTIAL | With RPC: fetches real block, extracts Merkle proof. Without RPC: returns empty proof with anchor data. |
| `verify_finality()` | REAL | Correctly computes confirmations vs `finality_depth`. |
| `enforce_seal()` | REAL | Marks seal used in registry, checks replay. |
| `create_seal()` | REAL | Derives from HD wallet deterministically. |
| `hash_commitment()` | REAL | Uses `Commitment::simple()` with domain separator. |
| `build_proof_bundle()` | REAL | Builds `ProofBundle` from inclusion + finality proofs. |
| `rollback()` | PARTIAL | Clears seal from registry if reorg detected. Does not actually unspend UTXO. |
| `domain_separator()` | REAL | Returns `"CSV-BTC-"` + network magic. |
| `signature_scheme()` | REAL | Returns `Secp256k1`. |

#### Sui (`csv-adapter-sui/src/adapter.rs`)

| Method | Status | Details |
|--------|--------|---------|
| `publish()` | PARTIAL | With RPC: builds BCS TransactionData manually, signs with Ed25519, submits via `execute_signed_transaction`, verifies event. Without RPC: marks seal and returns fake anchor. |
| `verify_inclusion()` | STUB | Returns proof with checkpoint hash derived from `anchor.checkpoint` but no real verification. Comment says "In production: get tx, verify effects, verify event, build object proof." |
| `verify_finality()` | REAL | Calls `CheckpointVerifier::is_checkpoint_certified()` against RPC. |
| `enforce_seal()` | REAL | Checks registry, marks used. |
| `create_seal()` | REAL | Uses timestamp-based nonce + SHA-256. |
| `hash_commitment()` | REAL | Uses `Commitment::simple()` with domain separator. |
| `build_proof_bundle()` | REAL | Constructs `ProofBundle` from inclusion + finality. |
| `rollback()` | PARTIAL | Clears seal registry but doesn't reverse on-chain object state. |
| `domain_separator()` | REAL | Returns `"CSV-SUI-"` + chain_id. |
| `signature_scheme()` | REAL | Returns `Ed25519`. |

#### Aptos (`csv-adapter-aptos/src/adapter.rs`)

| Method | Status | Details |
|--------|--------|---------|
| `publish()` | PARTIAL | With RPC: builds Entry Function JSON, signs with Ed25519, submits, waits, verifies event. Without RPC: marks seal and returns fake anchor. |
| `verify_inclusion()` | STUB | Returns empty proof. Comment says "In production: get tx by version, verify success, verify event, build Merkle proof." |
| `verify_finality()` | REAL | Calls `CheckpointVerifier::is_version_finalized()`. |
| `enforce_seal()` | REAL | Checks registry, marks used. |
| `create_seal()` | REAL | SHA-256 of `"aptos-seal"`. |
| `hash_commitment()` | REAL | Uses `Commitment::simple()`. |
| `build_proof_bundle()` | REAL | Constructs `ProofBundle`. |
| `rollback()` | PARTIAL | Clears registry but doesn't reverse on-chain resource state. |
| `domain_separator()` | REAL | Returns `"CSV-APTOS-"` + chain_id byte. |
| `signature_scheme()` | REAL | Returns `Ed25519`. |

#### Ethereum (`csv-adapter-ethereum/src/adapter.rs`)

| Method | Status | Details |
|--------|--------|---------|
| `publish()` | MOST COMPLETE | With RPC + `RealEthereumRpc`: calls `publish()` which builds EIP-1559 tx, signs with Alloy, broadcasts, verifies receipt LOG event. Without RPC: returns `"sim-tx-"` hash. |
| `verify_inclusion()` | PARTIAL | With RPC: fetches block hash, state root, receipt. But `receipt_rlp` in proof is empty. Without RPC: returns anchor data as proof. |
| `verify_finality()` | REAL | Uses `FinalityChecker` with RPC for finalized block check. |
| `enforce_seal()` | REAL | Marks seal in registry. |
| `create_seal()` | REAL | Derives from CSVSeal contract address + slot. |
| `hash_commitment()` | REAL | Uses `Commitment::simple()`. |
| `build_proof_bundle()` | REAL | Constructs `ProofBundle` with signatures extracted from DAG. |
| `rollback()` | REAL | Clears seal if anchor block < current block. |
| `domain_separator()` | REAL | Returns `"CSV-ETH-"` + chain_id. |
| `signature_scheme()` | REAL | Returns `Secp256k1`. |

---

### Proof Verification — Real vs Partial

| Component | File | Status | Details |
|-----------|------|--------|---------|
| Bitcoin Merkle proof | `proofs.rs`, `spv.rs` | REAL | Full double-SHA256 Merkle proof verification. Tested against real block data. |
| Bitcoin SPV proof | `spv.rs` | REAL | Verifies merkle proof + confirmation depth. |
| Ethereum MPT storage proof | `mpt.rs` | STUB | `verify_storage_proof()` checks `!is_empty()` and returns `true` |
| Ethereum MPT receipt proof | `mpt.rs` | PARTIAL | `verify_receipt_proof()` checks non-empty nodes but does NOT reconstruct trie |
| Ethereum full receipt proof | `mpt.rs` | PARTIAL | Checks non-empty + non-zero root, does NOT verify actual MPT path |
| Ethereum LOG event decoding | `proofs.rs` | REAL | Full RLP decoding, topic matching, `seal_id`/commitment comparison |
| Aptos state proof | `proofs.rs` | STUB | "For now, accept any valid proof with data" — checks `state_proof.len() >= 32` |
| Aptos event proof | `proofs.rs` | STUB | Checks `event_proof.len() >= 32`, verifies data match only |
| Aptos event-in-tx | `proofs.rs` | REAL | Fetches tx via RPC, checks success, searches events for matching data |
| Sui object proof | `proofs.rs` | REAL | Checks object exists via RPC |
| Sui event-in-tx | `proofs.rs` | REAL | Fetches tx, checks success, compares event data |

### Signature Verification

| Chain | File | Status | Details |
|-------|------|--------|---------|
| Bitcoin | `signatures.rs` | REAL | secp256k1 ECDSA verify via `secp256k1` crate. Verifies sig recovers correct pubkey. |
| Ethereum | `signatures.rs` | REAL | secp256k1 ECDSA verify via `secp256k1` crate. Same approach. |
| Sui | `signatures.rs` | REAL | Ed25519 verify via `ed25519_dalek`. Verifies sig against pubkey + message. |
| Aptos | `signatures.rs` | REAL | Ed25519 verify via `ed25519_dalek`. Same approach. |
| Core proof_verify | `proof_verify.rs` | REAL | Routes to correct scheme, verifies seal replay, checks signatures + confirmations. |

**All signature verification is properly implemented** — no stubs. They correctly verify cryptographic signatures.

### State Machines

| Component | Status | Details |
|-----------|--------|---------|
| Right lifecycle (create/transfer/consume) | COMPLETE | `right.rs` — full state machine with nullifier tracking |
| Seal lifecycle | COMPLETE | Per-chain seal registries track create → enforce → consumed |
| Commitment chain | PARTIAL | `commitment_chain.rs` has `verify_ordered_commitment_chain()` but it is NOT wired into the validation pipeline |
| Consignment validation | PARTIAL | `validator.rs` — structural check real, commitment chain stub, state transition stub |
| Client-side validation engine | STUB | `client.rs` — `verify_commitment_chain()` always returns `EmptyChain`, no anchor-to-Right mapping |

---

## 4. Cryptographic Hardening Gaps

### Tagged Hashing

**What's correct:**

- `tagged_hash()` and `csv_tagged_hash()` properly implement BIP-340 style: `sha256(sha256(tag) || sha256(tag) || data)`
- Tag prefix: `"urn:lnp-bp:csv:"`
- Used in: commitment hashing (all 8 fields), MPC tree (leaf + internal nodes), DAG node hashing

**What's NOT using tagged hashing (potential collision vectors):**

- `Right::new()` ID computation: raw `Sha256::new()` — should use tagged hash
- `Right::consume()` nullifier: raw `Sha256::new()` — should use tagged hash with domain
- `Commitment::simple()` seal_id: raw `Sha256::new()` — should use tagged hash
- `Commitment::simple()` empty MPC root: hardcoded hash of `"csv-empty-mpc-root"` via raw SHA-256
- Genesis, transition, schema, consignment hashes: all use raw `Sha256::new()`
- All adapter-specific proof hashing (Aptos event_hash, state leaf_hash): use domain-prefixed raw SHA-256, not csv_tagged_hash
- Bitcoin double-SHA256: this is Bitcoin-native and correct (not a gap)

### Commitment Encoding

- Canonical bytes: correct fixed-length format (version(1) + 7x32 fields = 225 bytes)
- Only V2 supported — V1 fully removed
- Deserialization checks version and minimum length
- **Issue:** The `#[deprecated]` `v1()` function alias still exists (line 130 of `commitment.rs`) — cosmetic only, points to `simple()`

### Signature Schemes

- **Secp256k1 (Bitcoin/Ethereum):** Properly verified via `secp256k1` crate. No gaps.
- **Ed25519 (Sui/Aptos):** Properly verified via `ed25519_dalek`. No gaps.
- **Issue:** Bitcoin adapter has `secp256k1 = "0.27"` but Ethereum has `secp256k1 = "0.28"` — minor version mismatch (different crates, same API)

### MPT Verification (Ethereum)

- Custom MPT removed (Sprint 0.4 deliverable)
- `alloy-trie` used for `HashBuilder` and `compute_state_root()`
- **CRITICAL GAP:** `verify_storage_proof()` returns `true` if proof has data (line 28 of `mpt.rs`)
- **CRITICAL GAP:** `verify_receipt_proof()` only checks `!node.is_empty()` (line 55)
- **CRITICAL GAP:** `verify_full_receipt_proof()` checks non-empty + non-zero root but does NOT reconstruct the trie path
- The comment at `mpt.rs:16-17` says: "For L3 security, we trust the node's proof verification" — this is a trust assumption, not trustless verification

### Merkle Proof Verification

- **Bitcoin:** FULLY REAL. Double-SHA256 Merkle tree with proper branch verification. Tested against real Signet block data.
- **Aptos:** STUBBED. `StateProofVerifier::verify()` just checks `state_proof.len() >= 32`. Comment: "In production: verify the Merkle proof against the accumulator root"
- **Aptos merkle.rs:** `verify_merkle_proof()` checks `root != [0; 32]` only

### Nullifier Computation

- `Right::consume()`: `nullifier = sha256(id || secret)` — deterministic, uses raw SHA-256
- Blueprint spec says: `nullifier = H(right_id || secret || context)` — missing the `context` field
- No domain separation on nullifier hash
- Ethereum nullifier registry: `nullifiers` mapping in contract, verified in receipt LOG event

### Domain Separators

All chains have proper domain separators:

| Chain | Domain | Construction |
|-------|--------|-------------|
| Bitcoin | `CSV-BTC-` + magic bytes (8 bytes) | `domain[..8] = b"CSV-BTC-"`, `domain[8..12] = network_magic` |
| Ethereum | `CSV-ETH-` + chain_id LE (8 bytes) | `domain[..8] = b"CSV-ETH-"`, `domain[8..16] = chain_id` |
| Sui | `CSV-SUI-` + chain_id string (up to 24 bytes) | `domain[..8] = b"CSV-SUI-"`, `domain[8..] = chain_id` |
| Aptos | `CSV-APTOS-` + chain_id (1 byte) | `domain[..10] = b"CSV-APTOS-"`, `domain[10] = chain_id` |

All incorporated into `Commitment` via `commitment-domain` tagged hash field.

---

## 5. End-to-End Test Gaps

### Ignored Tests (9 total)

| File | Line | Test | Reason |
|------|------|------|--------|
| `csv-adapter-bitcoin/src/testnet_deploy.rs` | 108 | `test_testnet_deployment` | Requires live network access |
| `csv-adapter-bitcoin/tests/signet_real_tx.rs` | 14 | `test_signet_e2e_publish_and_verify` | Requires funded Signet wallet |
| `csv-adapter-bitcoin/tests/signet_real_tx.rs` | 195 | `test_signet_real_merkle_proof_verification` | Requires internet |
| `csv-adapter-bitcoin/tests/signet_real_tx.rs` | 252 | `test_signet_block_data_from_live_node` | Requires internet |
| `csv-adapter-bitcoin/tests/signet_integration.rs` | 10 | `test_live_signet_merkle_proof` | Requires internet access |
| `csv-adapter-bitcoin/tests/signet_e2e.rs` | 27 | `test_full_signet_e2e_lifecycle` | Requires funded Signet |
| `csv-adapter-bitcoin/tests/signet_e2e.rs` | 94 | `test_signet_merkle_proof_with_real_block` | Requires internet |
| `csv-adapter-sui/tests/testnet_e2e.rs` | 26 | `test_sui_testnet_e2e_publish_and_verify` | Needs funded wallet + deployed contract |
| `csv-adapter-sui/tests/testnet_e2e.rs` | 123 | `test_sui_testnet_real_block_data` | Requires internet |

### Passing E2E Tests That Use Mocks

- `test_signet_real_block_data` — **PASSES** but fetches from mempool.space/signet API (read-only), does not publish
- `test_sui_testnet_real_block_data` — **PASSES** but fetches from fullnode.testnet.sui.io (read-only), does not publish

### Live Network Tests That Don't Exist

1. **Bitcoin:** No test that actually broadcasts a transaction to Signet and confirms it
2. **Sui:** No test that executes a signed transaction on Sui Testnet
3. **Aptos:** No test that submits a transaction to Aptos Testnet
4. **Ethereum:** No test that sends a real transaction to Sepolia and verifies the LOG event
5. **Cross-chain:** No test of any cross-chain Right transfer
6. **Reorg handling:** No test of rollback under actual chain reorganization
7. **RPC failure:** No test of adapter behavior when node goes offline mid-operation

### Gap Between "Unit Tests Pass" and "Works on Testnet"

The gap is **enormous**. Here is the honest breakdown:

| Layer | Unit Tests | Testnet Ready | Gap |
|-------|-----------|---------------|-----|
| Right type | 100% | N/A (data type) | None |
| Commitment encoding | 100% | Needs integration | Minor |
| Signature verification | 100% | Needs live key pairs | Minor |
| Bitcoin Merkle proofs | 100% | Needs live broadcast | Medium |
| Bitcoin publish | Mock only | Tx builder not wired | **Large** |
| Sui publish | Mock only | BCS format unverified against live node | **Large** |
| Aptos publish | Mock only | JSON BCS serialization unverified | **Large** |
| Ethereum publish | Real path exists | Needs signer + contract deployed | Medium |
| MPT verification | Struct only | Doesn't reconstruct trie | **Large** |
| Aptos Merkle proofs | Stub only | Not implemented | **Large** |
| Client-side validation | Stub only | Not implemented | **Critical** |
| Cross-chain transfers | Not started | Not started | **Critical** |

### CI Pipeline Status

- `.github/` directory is **EMPTY** — no CI pipeline configured at all
- No GitHub Actions workflows
- No testnet runner configuration
- No automated deployment

---

## 6. Cross-Chain Gaps

### Can a Right Move Between Chains?

**No.** There is no implementation of cross-chain Right transfer.

**What exists:**

- `CrossChainSealRegistry` (`seal_registry.rs`) — detects if the SAME seal reference is used on multiple chains (double-spend detection)
- `CrossChainValidator` (`rgb_compat.rs`) — compares commitment hashes across chain-specific anchors

**What does NOT exist:**

- Lock-and-prove mechanism on source chain
- Nullifier-based mint on destination chain
- Cross-chain proof verification
- Right portability format specification
- Settlement strategy (immediate vs optimistic)

### Cross-Chain Seal Registry Wiring

- The `CrossChainSealRegistry` is wired into `ValidationClient` and `ConsignmentValidator`
- It tracks `SealConsumption` events with `ChainId`, `SealRef`, `RightId`, `block_height`, `tx_hash`
- Detects both same-chain replay and cross-chain double-spend
- **But:** Each chain adapter has its OWN per-chain `SealRegistry` (Bitcoin, Sui, Aptos, Ethereum each have a local one). These are NOT connected to the `CrossChainSealRegistry`. A seal consumed on Bitcoin is tracked in Bitcoin's local registry, but the `CrossChainSealRegistry` in the client is a separate instance. There is no synchronization mechanism.

### Nullifier Scope

**Undecided.** Blueprint Section 12 lists this as "Open Design Edge #1":

- Global? Per contract? Per application?
- No decision has been made
- Current implementation: per-Right (nullifier is a field on `Right`)

---

## 7. Dependency Issues

### Version Conflicts

| Dependency | Core | Bitcoin | Ethereum | Sui | Aptos | Issue |
|-----------|------|---------|----------|-----|-------|-------|
| `secp256k1` | 0.27 | 0.27 | **0.28** | - | - | Version mismatch between Bitcoin and Ethereum |
| `reqwest` | - | 0.11 (dev) | **0.12** | 0.11 | 0.11 | Ethereum uses 0.12, others use 0.11 |
| `serde` | 1.0 (workspace: 1.0.227) | 1.0 | 1.0 | 1.0 | 1.0 | Workspace pins 1.0.227, crates use "1.0" (flexible) |

### Compilation Error

**CRITICAL:** `cargo check --all-features` **FAILS** with:

```
error[E0433]: failed to resolve: could not find `__private` in `serde`
  --> alloy-consensus (lib)
```

This is an `alloy-consensus` incompatibility with the current serde version. The `rpc` feature for Ethereum cannot compile with `--all-features`.

### Unused Dependencies

- `bitcoin = "0.30"` in `csv-adapter-core` (optional, only used when `tapret` feature enabled)
- `alloy-contract = "0.5"` in Ethereum — declared but not directly imported anywhere
- `aptos-sdk = "0.4"` in Aptos — declared as optional but `real_rpc.rs` uses raw reqwest instead
- `tiny-keccak` in Ethereum — not found in any source file
- `sha3` in Ethereum — not found in any source file

### Missing Critical Dependencies

- No `k256` crate for Ethereum-style address recovery (current code uses `secp256k1` which works but is less idiomatic for Ethereum)
- No `rlp` crate in Ethereum for proper receipt RLP decoding (manual RLP parser used instead)
- No persistent storage for `CrossChainSealRegistry` (only `InMemoryStateStore` exists)

---

## 8. Code Quality

### Compilation Warnings

`cargo check --all-features` produces **22 warnings** in `csv-adapter-core`:

**Unused imports (7):**

- `ConsignmentError` in `rgb_compat.rs`
- `TweakedPublicKey`, `UntweakedPublicKey` in `tapret_verify.rs`
- `Digest` in `mpc.rs`
- `InclusionProof`, `FinalityProof` in `proof_verify.rs`
- `serde_json` in `consignment.rs`
- `Anchor` in `consignment.rs`
- `MpcProof` in `mpc.rs`

**Unused variables (6):**

- `anchor`, `commitments` in `client.rs` (filter_map that returns None)
- `transition` in `client.rs`
- `consignment` (2x) in `rgb_compat.rs`
- `assignment` in `rgb_compat.rs`

**Unnecessary mut (2):**

- `errors` in `rgb_compat.rs` (2 instances)

### Dead Code

Files with `#![allow(dead_code)]`:

- `csv-adapter-bitcoin/src/adapter.rs`
- `csv-adapter-ethereum/src/adapter.rs`
- `csv-adapter-sui/src/adapter.rs`
- `csv-adapter-aptos/src/adapter.rs`

This silences warnings for entire adapter modules, meaning significant amounts of code are untested/unused in the current build.

### Test Coverage

**Total: ~594 tests pass** across 23 test suites. 4 tests are ignored.

**Covered well:**

- Right lifecycle (18 tests)
- Commitment encoding (16 tests)
- MPC tree (10+ tests)
- DAG (24+ tests)
- Seal registries (per-chain + cross-chain)
- Signature verification (per chain)
- Bitcoin Merkle proofs
- Sui checkpoint verification
- Aptos checkpoint verification
- Ethereum MPT (basic tests, no real proof vectors)

**NOT COVERED (gaps):**

- Client-side validation engine (tests exist but test stub behavior)
- Commitment chain verification (stub)
- State transition validation (stub)
- Real transaction broadcasting (all ignored)
- Cross-chain Right transfers (no implementation)
- Reorg/rollback under real conditions
- Network failure handling
- Fuzzing of any parsing function
- Property-based testing

---

## Summary: The Honest Truth

This codebase is a **well-architected skeleton with real cryptographic foundations but incomplete operational machinery**. The analogy is: the engine block is cast, the cylinders are bored, the crankshaft is balanced — but the fuel injectors aren't connected and the transmission is in neutral.

**What's genuinely good:**

- Right type is complete and well-tested
- Commitment V2 encoding is correct with tagged hashing for critical paths
- Domain separators are properly set per chain
- Signature verification is real (secp256k1 and Ed25519)
- Seal registries prevent replay (per-chain) and detect cross-chain double-spends
- Bitcoin Merkle proof verification is real and tested against live block data
- Ethereum Alloy integration for tx signing/broadcasting is structurally complete

**What's not real yet:**

1. **Bitcoin publish()** — tx_builder not wired, returns placeholder
2. **Ethereum MPT verification** — accepts any non-empty proof
3. **Aptos state/event proof verification** — accepts any non-empty proof
4. **Client-side validation engine** — commitment chain extraction returns EmptyChain, no anchor-to-Right mapping
5. **Cross-chain Right transfers** — not started
6. **CI pipeline** — does not exist
7. **Live network tests** — 9 ignored, 0 passing for actual tx broadcast
8. **Compilation with --all-features** — fails on alloy-consensus serde conflict
