# CSV Protocol — Production Audit & Readiness

**Auditor**: Independent Review | **Date**: May 2026 | **Scope**: Full repo (repomix snapshot)
**Target scenario**: CLI wallet create → CSV wallet create → fund → Sanad create → multi-hop cross-chain transfer → explore

---

## AUDIT VERDICT SUMMARY

All previously identified "Potemkin Village" verification patterns have been resolved. The codebase now uses real cryptographic verification throughout the proof pipeline, with all 10 canonical validation steps implemented. Three P0 items remain: Ethereum contract deployment, Explorer deployment, and real testnet RPC wiring.

---

## 2. Priority Matrix

| Level | Task | Owner |
| :--- | :--- | :--- |
| **P0** | Deploy `CSVLock.sol` + `CSVMint.sol` to Sepolia, fill deployment manifest | Core |
| **P0** | Deploy Explorer to public URL with testnet indexer | DevOps |
| **P0** | Wire real testnet RPC endpoints for E2E certification | Backend |
| **P1** | Add compile_fail tests: `locked_to_minting.rs`, `observed_to_completed.rs` | QA |
| **P1** | Enforce pinned block hash in proof construction | Core |
| **P1** | Add RPC metrics (`rpc_disagreement_total`, `provider_failure_total`) | Observability |
| **P2** | Explorer WebSocket push for transfer status | Fullstack |
| **P2** | Wire events into proof_pipeline for correlation | Core |

| Area | Status | Blockers |
|---|---|---|
| Contract Deployment| 🔴 Broken | Contracts not deployed to testnet |
| Explorer — Transactions | ⚠️ Schema ready | Indexer not live |
| Masterplan alignment | ⚠️ Partially stale | 1 must-ship still open (Ethereum deployment) |

---

## PART 1 — SECURITY

*All security items from the May 2026 audit have been resolved.*

---

## PART 2 — ARCHITECTURE GAPS

### ARCH-01 🔴 BLOCKING — Ethereum Contract Deployment

**What's needed**:
In CSV-CONTRACTS Scripts (for all chains)
So both csv-cli and csv-wallet should be able to get contract addresses for each chain to create and send Sanads.

* Compile and deploy `CSVLock.sol` + `CSVMint.sol` to Sepolia using foundry
* Send deployed addresses in `chains/ethereum.toml` under `[testnet]`
* Send deployed address into `lock_contract_address` field on `EthereumBackend`

---

### ARCH-05 ⚠️ HIGH — Explorer Has Schema but No Running Indexer for Demo

**Directory**: `csv-explorer/`

The Explorer has complete SQL schema, REST API, GraphQL, and UI. But for demo step 5 ("csv-explorer list all transactions with links to source chains"), the indexer must be running against actual testnet nodes. Currently:

* `config.testnet.toml` has placeholder RPC endpoints
* Block explorer links (`blockstream.info`, `suiexplorer.com`, etc.) require real tx hashes from real chains
* The `wallet_bridge.rs` priority indexing works but needs the wallet to register addresses via the bridge API

**What's needed**:

1. Deploy explorer with testnet config to a public URL (as masterplan requires before Stage 1)
2. Wire testnet RPC endpoints in `config.testnet.toml`
3. Add WebSocket push for transfer status updates (wired in schema but the ws handler at `csv-explorer/api/src/websocket.rs` needs the subscription feed)

---

### ARCH-06 ⚠️ HIGH — Transfer Record Chain Explorer Links Missing

**File**: `csv-explorer/shared/src/types.rs` → `TransferRecord`

`lock_tx` and `mint_tx` fields exist but are not populated with block explorer URLs in the REST response. The UI `transfers.rs` page shows tx hashes but doesn't build clickable links.

**What's needed**: In `csv-explorer/api/src/rest/handlers.rs` → `get_transfer()`, append block explorer URL based on chain: `format!("https://blockstream.info/testnet/tx/{}", lock_tx)` for Bitcoin, etc. These are the "links to source chains" required by demo step 5.

---

## PART 3 — CI / TESTING

---

### TEST-03 ⚠️ HIGH — Nextest Timeout Too Aggressive for Cryptographic Tests

**File**: `.config/nextest.toml`

```toml
slow-timeout = { period = "6s", terminate-after = "1" }
```

Tests running longer than 6 seconds are killed. Argon2id key derivation, Merkle proof generation, and RPC-backed tests regularly exceed this. The result is false test failures that hide real bugs.

**Fix**: Raise to `period = "30s", terminate-after = "3"` for the default profile. Add a `[profile.crypto]` profile with 120s for key derivation and ZK-related tests.

---

## PART 4 — MASTERPLAN VALIDATION

Cross-checking each "Must-Ship Before Demo" item against actual code:

| Item | Masterplan Says | Code Reality | Gap |
|---|---|---|---|
| ETH contract deployment | 3–5 days | **Not started** | `CapabilityUnavailable` in `deploy_lock_contract()` |
| Explorer deployment | 1 day | **Not started** | Indexer not live |
| Block explorer links | 0.5 day | **Not started** | Links not populated |


---

## PART 5 — WIRING CHECKLIST FOR DEMO SCENARIO

The exact 5-step scenario: CLI create wallet → CSV wallet create → fund → Sanad create → multi-hop transfer → explorer view.

### Step 1: `csv wallet generate` (CLI)

Keystore dir permissions `0700` (SEC-05) — implemented in native_keystore.rs SecurityPolicy.

### Step 2: CSV Wallet — Create New Wallet

Passphrase minimum entropy (SEC-06) — enforced via SecurityPolicy min_passphrase_length (12 chars).

### Step 3: Fund Both Wallets

Balance must reject silent-zero on RPC failure (chain_api.rs has the error type, verify it propagates).

### Step 4: Create Sanad and Transfer Multi-Hop

`csv cross-chain transfer` calls runtime.lock_sanad() on source chain (ARCH-03 complete — all 5 steps wired).
* [ ] Ethereum must be deployable if ETH chain involved (ARCH-01 — not yet deployed)


### Step 5: Explorer — List Transactions with Chain Links

* [ ] Indexer must be running against testnet (ARCH-05)
* [ ] Block explorer links must be populated (ARCH-06)
* [ ] WebSocket push for live status updates needs wiring
* [ ] Explorer must be deployed at public URL

---

## PART 6 — GUARDRAILS AND ENGINEERING RULES

These must be enforced before any code reaches `main`:

### Cryptographic Guardrails

* **No `Ok(true)` in verification paths.** Every verification path must return `Err` if data is unavailable, not a passing result.
* **No mock signatures in production code.** The production guarantee CI (once paths are fixed) must catch `fake_sig`, `mock_proof`, `[0u8; 64]` patterns in non-test code.
* **Seal registry check is mandatory.** `verify_proof()` must always call the seal registry callback. Never skip it with `//todo`.
* **Empty proof bundles are rejected.** Zero-length `inclusion_proof` bytes must fail, not pass.

### Key Management Guardrails

* **No raw private key bytes in logs.** Add a lint rule: `grep -r "private_key\|secret_key\|signing_key" --include="*.rs" | grep "println\|info!\|debug!"` must return zero results.
* **Zeroize on drop.** Any type holding a private key must implement `Zeroize` + `ZeroizeOnDrop`. Verify with `#[derive(ZeroizeOnDrop)]`.
* **No key material in state files.** `unified_storage.json` must never contain fields with names matching `key|secret|mnemonic|seed|private`.

### Chain Integration Guardrails

* **Testnet by default.** Any new chain integration must default to testnet in config. Mainnet requires explicit `--network mainnet` flag.
* **RPC endpoints via env vars only.** No hardcoded endpoints in any config file checked into the repo.
* **Finality depth is never zero.** `min_confirmations = 0` must fail at config parse time.

### State Machine Guardrails

* **Transfer status is append-only.** A transfer record cannot go from `Completed` back to `Pending`. Add validation in `UnifiedStateManager::update_transfer()`.
* **Sanad status is monotonic.** `Active → Transferred → Consumed`. Reverse transitions must panic/error.
* **Double-consume fails loudly.** Attempting to consume an already-consumed Sanad must return a specific error, not silently succeed or update the record.

### CI Guardrails (after fixing TEST-01)

* **Production guarantee gates must pass on every PR.** No merges to `main` with failing gates.
* **`cargo audit` on every push.** Already in CI — keep it.
* **`cargo clippy -- -D warnings` blocks merge.** Already in CI — keep it.
* **No `unwrap()` in production paths.** Add clippy lint `#![deny(clippy::unwrap_used)]` to `csv-core/src/lib.rs`, `csv-keys/src/lib.rs`.

---

## PART 7 — PRIORITY WORK ORDER

To reach the 5-step demo scenario as fast as possible:

**Week 1 (Ethereum deployment)**

1. Deploy CSVLock.sol to Sepolia via Foundry (3-5 days)
2. Deploy CSVMint.sol to Sepolia via Foundry (1 day)
3. Fill deployment manifest with real addresses (1 hour)
4. Configure lock_contract_address / mint_contract_address in EthereumBackend (1 hour)

**Week 2 (Explorer + testing)**

5. Deploy explorer to public testnet URL (1 day)
6. Populate block explorer links in REST response (0.5 day)
7. Add WebSocket push for live status updates (1 day)
8. Add compile_fail tests: `locked_to_minting.rs`, `observed_to_completed.rs` (0.5 day)
9. Tune nextest timeouts (1 hour)

---

## PART 8 — GLOBAL REPOSITORY GUARDRAILS

### Forbidden Runtime Patterns

The following patterns are forbidden in production runtime code:

```rust
unwrap()
expect()
new_unchecked()
unsafe
Sha256::digest
Keccak256::digest
blake3::hash
```

Allowed only in:

```text
/tests
/fuzz
/benches
```

---

### CI Enforcement Targets

Created:

```text
/scripts/security/check_forbidden_patterns.sh
```

CI MUST fail on:

| Pattern | Scope |
|---|---|
| TODO | protocol modules |
| FIXME | protocol modules |
| unwrap() | runtime code |
| expect() | runtime code |
| unsafe | outside approved modules |
| raw hashing | outside crypto module |
| mock proofs | production code |
| manual ABI encoding | EVM adapters |

---

### Approved Unsafe Modules

Unsafe MAY exist only in:

```text
csv-crypto/
csv-zk/
```

All unsafe blocks MUST include:

```rust
// SAFETY:
```

Without exception.



---

### Phase 3 — Finality + Reorg Safety

**Status: Structure Done — Reorg Handling Complete**

| Item | Status | Action |
|---|---|---|
| Pinned block hash in proof construction | Missing | Enforce: no `latest_block` in proof building |
| Compromised mint path observability | Missing | Add event emission |

**Priority: Enforce pinned block hash; add event emission for compromised mint path.**

---

### Phase 4 — RPC Trust Hardening

**Status: Quorum Client Done — Integrated**

| Item | Status | Action |
|---|---|---|
| `csv-observability/src/metrics/` (RPC metrics) | Partial | Add: `rpc_disagreement_total`, `provider_failure_total`, `provider_timeout_total` |

**Priority: Add RPC metrics; wire quorum to remaining chain adapters.**

---

### Phase 5 — ABI + Contract Safety

**Status: Generated Bindings Used — Migration Complete**

| Item | Status | Action |
|---|---|---|
| `sanad_contract.rs` manual encoding | Still exists | Deprecate after ops.rs migration |
| `seal_contract.rs` manual encoding | Still exists | Deprecate after migration |
| `cargo xtask verify-bindings` | Missing | Create xtask |
| `deployments/deployment-manifest.json` | Exists (TODOs) | Fill in real values, add signature |
| Immutable contract deployment | Not enforced | Add deployment verification |

**Priority: Create `cargo xtask verify-bindings`; fill deployment manifest.**

---

### Phase 6 — Wallet Security

**Status: csv-keys Solid — Wallet Integration Complete**

| Item | Status | Action |
|---|---|---|
| `localStorage` mnemonic persistence | Not found | Verify none exists (good) |
| Hardware wallet path | Missing | Implement |


---

### Phase 7 — Property Testing + Fuzzing

**Status: Infrastructure Ready**

| Item | Status | Action |
|---|---|---|
| `csv-core/tests/compile_fail/` (3 tests) | Partial | Add 3 more missing tests |
| CI fuzz corpus execution | Missing | Add to CI workflows |

**Priority: Add fuzz execution to CI.**

---

### Phase 8 — Observability + Forensics

**Status: Events Schema Done — Minimal Metrics**

| Item | Status | Action |
|---|---|---|
| `csv-core/src/events.rs` (15 event types) | Done | |
| Required events defined | All defined | Wire into proof_pipeline |
| Correlation fields (transfer_id, operation_id, etc.) | Partial | Add to CsvEvent struct |
| Structured JSON output | Missing | Add JSON formatter to event indexer |
| RPC metrics (partial) | Exists | Add missing metrics |
| Tracing/logging module | Missing | Add structured logging |

**Priority: Wire events into proof_pipeline and reorg detection; add correlation fields.**

---

### Phase 9 — E2E Certification

**Status: Real Cryptographic E2E — Needs Real Chain Wiring**

| Item | Status | Action |
|---|---|---|
| Real Bitcoin Signet to Ethereum Sepolia flow | Missing | Wire real RPCs |
| Failure injection tests | Missing | Add: RPC disagreement, reorg, duplicate proof, node restart |
| Offline verification demo | Partial | Complete wiring |

**Priority: Replace simulated proofs with real testnet RPC-backed certification flow.**



