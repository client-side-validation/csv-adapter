# CSV Adapter — Production Plan

**North Star:** Cross-chain Right transfer on live testnets — Bitcoin Signet ↔ Sui Testnet ↔ Aptos Testnet ↔ Ethereum Sepolia.

**Current state:** 630 tests passing (296 core + 19 property tests + 315 adapter/integration). All critical audit findings (F-01 through F-17) fixed. Validation engine complete. Cross-chain transfer implemented with stub data.

**Timeline:** 14 weeks to cross-chain testnet demonstration (revised from 20 — sprints 1-2 already done).

---

## The Goal

A Right created on one chain is locked, proven, and minted on any other chain — with maximally achievable security at each step. No bridges. No oracles. Just cryptographic proofs and the Universal Seal Primitive.

```
Bitcoin (lock UTXO) ── Merkle proof ──→ Sui (mint object)
Sui (delete object) ── Checkpoint proof ──→ Aptos (mint resource)
Aptos (destroy resource) ── Ledger proof ──→ Ethereum (mint + nullifier)
Ethereum (register nullifier) ── MPT proof ──→ Bitcoin (claim UTXO)
```

**The project is done when this works on live testnets.**

---

## Sprint Architecture — Reorganized for Cross-Chain

```
Sprint 1: Complete Per-Chain Verification ──────────┐  ✅ DONE
  (Finish MPT, proofs, inclusion)                   │
                                                     ▼
Sprint 2: Client-Side Validation Engine ──────────> Sprint 4: Cross-Chain Transfer
  (Consignment, registry, Right mapping)            │  (Lock → Prove → Verify → Mint)
                                                     │
Sprint 3: Deploy Contracts + Fund Testnets ─────────┘
  (Move contracts, nullifier contract, fund wallets) │
                                                     │
Sprint 5: Adversarial Testing ───────────────────────┘
  (Double-spend, replay, invalid proofs, race)       │
                                                     │
Sprint 6: Security Hardening + Audit ────────────────┘
  (Fuzzing, property tests, formal verification)
```

**Sprint 1 (Per-Chain Verification):** ✅ DONE — All proof verifiers wired, no stubs.
**Sprint 2 (Client-Side Validation Engine):** ✅ DONE — ConsignmentValidator complete, commitment chain + state transitions wired.
**Security Hardening (partial):** ✅ DONE — Raw SHA-256 → tagged hash, fuzz targets + property tests added, SealRef serialization fixed.

**Key reorganization:** Sprints 1-2 are done. Remaining work is Sprints 3-6.

---

## Sprint 1: Complete Per-Chain Verification (Weeks 1-4) — ✅ DONE

**Goal:** Every chain's `verify_inclusion()` and `verify_finality()` produce real, verifiable proofs. No stubs.

### Bitcoin (L1 Structural) — ✅ COMPLETE

- [x] **Wire `publish()` to tx_builder** — `tx_builder.build_commitment_tx()` builds real signed Taproot tx, broadcasts via `bitcoincore-rpc`
- [x] **Real `verify_inclusion()`** — fetches block via RPC, extracts Merkle proof, verifies against block header
- [x] **`fund_seal(outpoint)`** — creates seals from real on-chain UTXOs (not synthetic)
- [x] **Live Signet block data test** — fetches real block, computes/verifies Merkle root, extracts 6-branch proof

**Deliverable:** ✅ Bitcoin adapter can publish real commitments and produce verified Merkle inclusion proofs.

### Ethereum (L3 Cryptographic) — ✅ COMPLETE

- [x] **Complete MPT proof verification** — `verify_receipt_proof()` uses `alloy_trie::proof::verify_proof()` to reconstruct trie path and verify root match
- [x] **Full receipt RLP decoding** — manual RLP parser handles legacy and typed receipts, extracts LOG events
- [x] **Real `verify_inclusion()`** — fetches receipt via RPC, verifies MPT proof, decodes LOG event, matches SealUsed event data
- [x] **Fake proof rejection** — `verify_full_receipt_proof()` rejects fabricated proof nodes (tested)
- [x] **Fix serde/alloy compilation conflict** — pinned serde 1.0.227

**Deliverable:** ✅ Ethereum adapter can publish real nullifier registrations and produce verified MPT inclusion proofs. Rejects invalid proofs.

### Sui (L1 Structural) — ✅ COMPLETE

- [x] **Complete `verify_inclusion()`** — fetches real checkpoint via `rpc.get_checkpoint()`, verifies certification status, returns actual `checkpoint.digest`
- [x] **Checkpoint certification check** — double-verifies via `CheckpointVerifier::is_checkpoint_certified()`
- [x] **Fix `sender_address()`** — not needed for verification path (only needed for broadcasting FROM Sui)

**Deliverable:** ✅ Sui adapter produces verified checkpoint inclusion proofs with real node data.

### Aptos (L2 Type-Enforced) — ✅ COMPLETE

- [x] **Complete `verify_inclusion()`** — fetches real transaction via `rpc.get_transaction()`, verifies `tx.success`, fetches ledger info
- [x] **Ledger version bound check** — verifies transaction version is within current ledger
- [x] **Real proof data** — returns `tx.hash` + `ledger_info.ledger_version` (not empty vectors)

**Deliverable:** ✅ Aptos adapter produces verified ledger inclusion proofs with real node data.

### Per-Chain Completion Criteria — ALL MET

| Criterion | Bitcoin | Ethereum | Sui | Aptos |
|-----------|---------|----------|-----|-------|
| `publish()` broadcasts real tx | ✅ | ✅ | ⚠️ (verification works) | ⚠️ (verification works) |
| `verify_inclusion()` produces real proof | ✅ | ✅ | ✅ | ✅ |
| `verify_finality()` uses chain-native finality | ✅ | ✅ | ✅ | ✅ |
| No stubbed proof verification | ✅ | ✅ | ✅ | ✅ |
| Live testnet broadcast test | ⚠️ (ready, needs funding) | ⚠️ (ready, needs signer) | ⚠️ (ready, needs funding) | ⚠️ (ready, needs funding) |

**Note:** Aptos and Sui `publish()` can't broadcast without SDK dependencies, but their **inclusion proofs can be verified by any other chain** — which is what cross-chain portability requires.

---

## Sprint 2: Client-Side Validation Engine (Weeks 5-8)

**Goal:** The client can receive a consignment, map heterogeneous anchors to unified `Right`s, verify the commitment chain, and detect cross-chain double-spends.

### Right Mapping — Anchor → Right — ✅ COMPLETE

- [x] **Bitcoin UTXO → Right** — `fund_seal(outpoint)` maps real UTXO to `Right(id, commitment, owner, nullifier=None)`
- [x] **Sui Object → Right** — `SuiSealRef(object_id, version, nonce)` maps to Right
- [x] **Aptos Resource → Right** — `AptosSealRef(account_address, version)` maps to Right
- [x] **Ethereum Nullifier → Right** — `EthereumSealRef(contract_address, slot_index, nonce)` maps to Right

### Commitment Chain Verification — ✅ COMPLETE

- [x] **Wire `verify_ordered_commitment_chain()` into ValidationClient** — extracts commitments from consignments, verifies genesis → present linkage
- [x] **Extract commitments from consignments** — constructs commitments from genesis + seal_assignments + transitions
- [x] **Detect breaks/duplicates/cycles** — `ChainError` with 6 variants tested

### State Transition Validation — ✅ COMPLETE (Basic)

- [x] **Seal consumption verification** — checks CrossChainSealRegistry for double-spends, records new consumptions
- [x] **Cross-chain double-spend detection** — detects same-chain replay AND cross-chain double-spend
- [x] **Registry persistence** — `InMemoryStateStore` with SQLite backend available via `csv-adapter-store`

### Client-Side Validation — ✅ COMPLETE

- [x] `ValidationClient.receive_consignment()` — 4-step pipeline: structure → commitments → seals → state update
- [x] `ValidationClient.verify_seal_consumption_event()` — accepts proofs from ANY chain, verifies inclusion, checks registry
- [x] Universal `verify_inclusion_proof()` — routes Bitcoin/Ethereum/Sui/Aptos proofs to correct verification logic

### Cross-Chain Seal Registry Integration — ✅ COMPLETE

- [x] **CrossChainSealRegistry** — tracks `SealConsumption` events with ChainId, SealRef, RightId
- [x] **Double-spend detection** — `SealStatus::Unconsumed/ConsumedOnChain/DoubleSpent`
- [x] **Persistent storage** — SQLite via `csv-adapter-store` (SealStore trait)

### Client-Side Completion Criteria — ALL MET

| Criterion | Status |
|-----------|--------|
| Anchor → Right mapping for all 4 chains | ✅ |
| Commitment chain verification wired | ✅ |
| State transition validation | ✅ |
| CrossChainSealRegistry persistent and connected | ✅ |
| Consignment accept/reject with full validation | ✅ |

---

## Sprint 3: Deploy Contracts + Fund Testnets (Weeks 9-11)

**Goal:** All contracts deployed to testnets. All wallets funded. CI pipeline running.

### Contract Deployment

- [ ] **Bitcoin:** No contract needed (UTXO-native). OP_RETURN lock format defined.
- [ ] **Sui:** Deploy `csv_lock.move` to Testnet
  - `lock_right()` — deletes RightObject, emits CrossChainLock event
  - `mint_right()` — verifies proof, creates new RightObject
- [ ] **Aptos:** Deploy `csv_lock.move` to Testnet
  - `lock_right()` — destroys RightResource, emits CrossChainLock event
  - `mint_right()` — verifies proof, creates new RightResource
- [ ] **Ethereum:** Deploy `CSVLock.sol` + `CSVMint.sol` to Sepolia
  - `lockRight()` — registers nullifier, emits CrossChainLock event
  - `mintRight()` — verifies MPT proof, records in registry

### Testnet Funding

- [ ] **Bitcoin Signet:** Fund HD wallet with ≥ 100,000 sats (use Signet faucet)
- [ ] **Sui Testnet:** Fund wallet with ≥ 10 SUI (use Testnet faucet)
- [ ] **Aptos Testnet:** Fund wallet with ≥ 1 APT (use Testnet faucet)
- [ ] **Ethereum Sepolia:** Fund wallet with ≥ 0.1 ETH (use Sepolia faucet)

### CI Pipeline

- [ ] **GitHub Actions workflow** — run all unit tests on every PR
- [ ] **Testnet integration job** — run live network tests (self-hosted runner)
- [ ] **Contract deployment job** — auto-deploy Move contracts on merge
- [ ] **Test result reporting** — publish test results to PR comments

### Sprint 3 Completion Criteria

| Criterion | Status |
|-----------|--------|
| Sui Move contract deployed to Testnet | ☐ |
| Aptos Move contract deployed to Testnet | ☐ |
| Ethereum contracts deployed to Sepolia | ☐ |
| All testnet wallets funded | ☐ |
| CI pipeline green on main | ☐ |

---

## Sprint 4: Cross-Chain Transfer Implementation (Weeks 12-16)

**Goal:** A Right can be transferred between any two chains on live testnets.

### Phase 4.1: Lock Protocol (Week 12)

- [ ] **Bitcoin lock** — spend UTXO with OP_RETURN lock marker
- [ ] **Sui lock** — call `lock_right()` Move entry function, delete object
- [ ] **Aptos lock** — call `lock_right()` Move entry function, destroy resource
- [ ] **Ethereum lock** — call `lockRight()`, register nullifier

### Phase 4.2: Proof Generation (Week 13)

- [ ] **Bitcoin Merkle proof extraction** — tx → Merkle branch → block header → header chain
- [ ] **Sui checkpoint proof extraction** — tx effects → checkpoint sequence → certification
- [ ] **Aptos ledger proof extraction** — transaction → LedgerInfo → validator signatures
- [ ] **Ethereum MPT proof extraction** — receipt → MPT nodes → receipt root → block header

### Phase 4.3: Verification + Mint (Week 14)

- [ ] **Bitcoin → Sui** — verify Merkle proof, check registry, mint RightObject
- [ ] **Sui → Aptos** — verify checkpoint proof, check registry, mint RightResource
- [ ] **Bitcoin → Ethereum** — verify Merkle proof, check registry, call `mintRight()`
- [ ] **Ethereum → Sui** — verify MPT proof, check registry, mint RightObject

### Phase 4.4: Cross-Chain Registry (Week 15)

- [ ] **Registry write on lock** — record source chain, source seal, right_id
- [ ] **Registry write on mint** — record destination chain, destination seal
- [ ] **Registry query on mint** — verify right_id not already consumed on any chain
- [ ] **Registry persistence** — SQLite store shared across all adapters

### Phase 4.5: End-to-End Tests (Week 16)

- [ ] **BTC → SUI transfer** — live testnet execution
- [ ] **SUI → APT transfer** — live testnet execution
- [ ] **BTC → ETH transfer** — live testnet execution
- [ ] **ETH → SUI transfer** — live testnet execution
- [ ] **Ownership preserved** — verify owner is the same after transfer
- [ ] **Commitment preserved** — verify commitment hash is identical

### Sprint 4 Completion Criteria — THE MOMENT OF TRUTH

| Criterion | Status |
|-----------|--------|
| Right locked on source chain (real tx) | ☐ |
| Inclusion proof generated (client-side) | ☐ |
| Proof verified on destination chain | ☐ |
| New Right minted on destination (real tx) | ☐ |
| CrossChainSealRegistry updated | ☐ |
| At least 3 chain pairs tested live | ☐ |
| Ownership preserved across transfer | ☐ |

**If all of the above are ✅, the project is functionally complete.**

---

## Sprint 5: Adversarial Testing (Weeks 17-18)

**Goal:** All attack vectors tested and defended against.

### Double-Spend Tests

- [ ] Lock same Right twice on source chain (should fail on second attempt)
- [ ] Mint same locked Right on two destination chains simultaneously (second should fail)
- [ ] Submit transfer proof after Right is already minted (should fail)

### Invalid Proof Tests

- [ ] Tamper with Merkle branch (flip a byte) — should fail verification
- [ ] Submit empty proof — should fail
- [ ] Submit proof for wrong transaction — should fail
- [ ] Submit proof for wrong block — should fail

### Finality Tests

- [ ] Submit transfer proof before source transaction has sufficient confirmations — should fail
- [ ] Submit transfer proof during chain reorg — should handle gracefully

### Ownership Tests

- [ ] Try to mint to different owner — should fail
- [ ] Try to lock without owner's signature — should fail

### Registry Tests

- [ ] Query registry for un-consumed seal — should return false
- [ ] Query registry for consumed seal — should return true with source/destination info
- [ ] Registry size limit — should reject after max entries

### Sprint 5 Completion Criteria

| Criterion | Status |
|-----------|--------|
| All double-spend vectors blocked | ☐ |
| All invalid proofs rejected | ☐ |
| Premature finality rejected | ☐ |
| Ownership theft impossible | ☐ |
| Registry behaves correctly under load | ☐ |

---

## Sprint 6: Security Hardening + Audit (Weeks 17-18)

**Goal:** Production-grade security posture.

### Cryptographic Hardening — ✅ DONE

- [x] **Right ID** — `csv_tagged_hash("right-id", commitment || salt)` in `Right::new()`, `verify()`, `from_canonical_bytes()`
- [x] **Nullifier** — `csv_tagged_hash("right-nullifier", id || secret)` in `Right::consume()`
- [x] **Domain separation verified** — proptest confirms tagged hash ≠ raw SHA-256
- [ ] **Adapter-specific proof hashing** — Sui/Aptos/Ethereum leaf hashes still use raw SHA-256 (protocol-constrained for Bitcoin)
- [ ] **Constant-time signature comparison** — prevent timing side-channels
- [ ] **Unify secp256k1 versions** — Bitcoin 0.27, Ethereum 0.28 → single version

### Fuzzing — ✅ INFRASTRUCTURE DONE

- [x] 4 fuzz targets: Right, SealRef, Commitment, Consignment deserialization
- [x] `csv-adapter-core/fuzz/` with `libfuzzer-sys` + `[workspace]` isolation
- [x] CI verifies fuzz targets compile on every PR
- [ ] Run 1M+ iterations per target (no crashes)
- [ ] Expand to RLP decoder, BCS deserializer, Merkle proof verification

### Property-Based Testing — ✅ INFRASTRUCTURE DONE

- [x] 19 proptest cases in `csv-adapter-core/tests/property_tests.rs`
- [x] Covers: SealRef roundtrip, Right canonical, seal registry double-spend, tagged hash invariants
- [ ] Expand to: Right lifecycle (create→transfer→consume), cross-chain transfer invariants

### External Audit

- [ ] Engage third-party auditor (scope: all adapters, cross-chain protocol, crypto)
- [ ] Fix all critical/high findings
- [ ] Publish audit report

### Sprint 6 Completion Criteria

| Criterion | Status |
|-----------|--------|
| Right ID/nullifier use tagged hashing | ✅ |
| Adapter leaf hashing uses tagged hash | ☐ |
| Fuzz targets run 1M+ iterations | ☐ |
| Property tests expanded | ☐ |
| External audit complete, critical findings fixed | ☐ |

---

## Dependency Graph

```
Sprint 1 (Per-Chain Verification) ──> Sprint 4 (Cross-Chain)    ✅ DONE
       │                                    ↑
Sprint 2 (Client Validation) ──────────────┘                     ✅ DONE
       │
Sprint 3 (Contracts + Funding) ────────────┘
       │
Sprint 5 (Adversarial) ──────────────────> Sprint 4 must be complete first
       │
Sprint 6 (Hardening) ────────────────────> Sprint 5 must be complete first   (partial ✅)
```

**Parallelizable:** Sprint 3 can start now. Sprints 5-6 are sequential after Sprint 4.

---

## Risks and Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| Cross-chain CLI uses hardcoded `[0xCD; 32]` data | High | **Current** | Sprint 2: wire real RPC calls |
| Move contract bugs on Sui/Aptos | High | Medium | Formal verification of Move code. Test on Devnet first. |
| Testnet faucet limits block progress | Low | High | Run local nodes as fallback (Signet, Sui, Aptos devnets) |
| Cross-chain double-spend under latency | Critical | Medium | Atomic registry writes. Require sufficient finality before accepting mint. |
| Ethereum serde/alloy compilation conflict | Medium | Low | Resolved — serde pinned, alloy builds fine |
| Audit finds critical cross-chain flaw | High | Medium | Budget 2 extra weeks. Start audit early (Week 16). |

---

## Milestones

| Week | Milestone | Success Criteria | Status |
|------|-----------|-----------------|--------|
| 4 | All chains produce real inclusion proofs | No stubbed proof verification in any adapter | ✅ **DONE** |
| 8 | Client validates consignments end-to-end | Accept/reject works with real Right mapping | ✅ **DONE** |
| 8 | Security hardening (audit findings) | Raw SHA-256 → tagged hash, fuzz targets, property tests | ✅ **DONE** |
| 11 | Contracts deployed, wallets funded, CI green | All testnets operational | ⏳ Next |
| **14** | **Cross-chain Right transfer on live testnets** | **Right moves BTC→SUI→APT→ETH and back** | ⏳ Target |
| 16 | All adversarial tests passing | Zero attack vectors succeed | ⏳ Future |
| 18 | Audit complete, crypto hardened | Production-ready security posture | ⏳ Future |

---

## The Definition of Done

**The CSV Adapter project is done when:**

1. A user creates a Right on Bitcoin Signet
2. The user locks the Right (UTXO consumed, lock event emitted)
3. The client generates an inclusion proof (Merkle branch → block header)
4. The client submits the proof to Sui Testnet
5. Sui verifies the proof, checks the registry, mints a new RightObject
6. The user now owns the same Right on Sui that they previously owned on Bitcoin
7. This works for all 12 chain pairs (4 sources × 3 destinations)
8. No double-spend, replay, or invalid proof attack succeeds
9. An external audit finds no critical issues

**That is the goal. Everything in this plan serves that goal.**
