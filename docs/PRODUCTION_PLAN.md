# CSV Adapter — Production Plan

**Current state:** Architectural prototype. Compiles clean. 535 unit tests pass (all mock). Cannot publish a real transaction to any blockchain. The client-side validation engine does not exist yet.

**Goal:** Production-ready client-side validation system where the Universal Seal Primitive enforces single-use at each chain's strongest guarantee, and clients verify the rest.

**Timeline:** 24–30 weeks from today.

---

## What This Is Building

A **client-side validation system** where:

1. **The chain does the minimum:** records commitments, enforces single-use of Rights
2. **Clients do everything else:** fetch state history, verify commitment chains, detect double-consumption, accept/reject consignments

The Universal Seal Primitive is the abstraction that makes this work across chains. The degradation model (L1→L2→L3) determines *how* each chain enforces single-use. But the **product** is the validation engine that runs on the client.

### Current Architecture vs What's Needed

| CSV Component | Exists? | Status | Sprint |
|---------------|---------|--------|--------|
| `Right` type (canonical primitive) | ✅ Yes | Created with full tests | Sprint 0.1 |
| Client-side validation engine | 📋 Scaffolded | `client.rs` + `validator.rs` placeholders | Sprint 2 |
| `AnchorLayer` trait (chain interface) | ✅ Yes | Sound abstraction | Sprint 1 |
| Per-chain RPC wiring | ⚠️ Partial | Stub/HTTP clients exist, not wired | Sprint 1 |
| Consignment struct | ✅ Yes | Exists but validation not built | Sprint 2 |
| State history storage | ❌ No | Client stores full history | Sprint 2 |
| Commitment chain verification | ❌ No | Verify genesis → present | Sprint 2 |
| Cross-chain conflict detection | ❌ No | Check no double-consumption | Sprint 2 |
| Nullifier registry (Ethereum L3) | ❌ No | Smart contract needed | Sprint 1 |
| Cross-chain Right transfer | ❌ No | Lock-and-prove mechanism | Sprint 4 |
| RGB compatibility verification | ⚠️ Partial | Unverified re-implementation | Sprint 5 |

**The skeleton is there. The `Right` type is built. The validation machinery is scaffolded but not implemented.**

---

## The Blueprint: Universal Seal Primitive (USP)

> We are not implementing "single-use seals."
> We are implementing **a portable right system whose uniqueness and consumption guarantees degrade gracefully across heterogeneous chains.**

### Core Invariant (Non-Negotiable)

> **A Right can be exercised at most once under the strongest available guarantee of the host chain.**

### The Canonical Primitive

```rust
Right {
  id: Hash,               // Unique identifier
  commitment: Hash,       // Encodes state + rules
  owner: OwnershipProof,  // Signature / capability / object ownership
  nullifier: Option<Hash>,// One-time consumption marker (L3+)
  state_root: Option<Hash>,
  execution_proof: Option<Proof>,
}
```

### The Degradation Model (The Heart of the System)

| Level | Name | Guarantee Type | Chains | Our Adapter |
|-------|------|---------------|--------|-------------|
| **L1** | Structural | Native single-use | Bitcoin, Sui | `BitcoinAnchorLayer`, `SuiAnchorLayer` |
| **L2** | Type-Enforced | Language-level scarcity | Aptos | `AptosAnchorLayer` |
| **L3** | Cryptographic | Nullifier-based | Ethereum | `EthereumAnchorLayer` |
| **L4** | Optimistic | Fraud/challenge | Rollups | *(future)* |
| **L5** | Social/Economic | Reputation/slashing | Off-chain | *(future)* |

### The Degradation Rule (Determines What Each Adapter Does)

```
IF native single-use exists (L1):
    DO NOT introduce nullifier
    → Bitcoin: spend UTXO, Sui: consume object

ELSE IF non-duplicable resource exists (L2):
    USE resource lifecycle
    → Aptos: destroy Move resource

ELSE:
    REQUIRE nullifier tracking (L3)
    → Ethereum: mapping(bytes32 => bool) public nullifiers
```

### What This Means for Implementation Priority

| Chain | Layer | What "Seal" Means | Nullifier Needed? | Priority |
|-------|-------|-------------------|-------------------|----------|
| **Bitcoin** | L1 Structural | Spend UTXO | ❌ No | **1st** (Reference) |
| **Sui** | L1 Structural | Delete/Mutate Object | ❌ No | **2nd** (Reference-aligned) |
| **Aptos** | L2 Type-Enforced | Destroy Resource | ❌ No (language enforces) | **3rd** |
| **Ethereum** | L3 Cryptographic | Register Nullifier | ✅ Yes (smart contract) | **4th** (Hardest) |

**Key insight from Blueprint:** Ethereum/Solana are *verification layers, not state layers.* We're not simulating seals on Ethereum — we're building a nullifier registry that provides cryptographic (but not structural) single-use enforcement.

---

## Implementation Priority: Why This Order

**This is not a sequential list. It's a priority order based on the degradation model.**

Forget branding or VM differences. The property we're hunting is:

> A Right that can be exercised at most once under the strongest available guarantee of the host chain.

### Bitcoin (Priority 1 — L1 Structural, Reference Implementation)

Bitcoin UTXOs provide:
- Unique identity (txid + output index)
- Single ownership (UTXO set)
- Atomic consumption (spent = gone)
- Global consensus on state (Nakamoto consensus)
- Parallelizable (independent UTXO set)
- **No nullifier needed** — the chain enforces single-use structurally

**This is the reference.** Everything else degrades from this.

### Sui (Priority 2 — L1 Structural, Reference-Aligned)

Sui objects behave nearly identically to Bitcoin UTXOs:

| Property | Bitcoin UTXO | Sui Object |
|----------|-------------|------------|
| Unique identity | ✔ (txid:vout) | ✔ (object ID) |
| Owned | ✔ | ✔ |
| Consumed | ✔ (spent) | ✔ (deleted/mutated) |
| Parallelizable | ✔ | ✔ |
| Version tracking | Implicit | Explicit (object versions) |
| **Nullifier needed?** | ❌ No | ❌ No |

Sui is not "compatible" with CSV — it's **structurally aligned**. Right = Object. Consumption = object deletion. No nullifier required.

### Aptos (Priority 3 — L2 Type-Enforced)

Aptos resources are:
- Non-copyable, non-duplicable (enforced at language level)
- Must be moved or destroyed (Move semantics)
- But tied to accounts, not independent objects
- No native notion of independent ownership graph like UTXOs

Aptos gives you **programmable Rights via resource lifecycle, not structural Rights**. Right = Resource. Consumption = resource destruction. No nullifier needed — the Move VM enforces non-duplication.

### Ethereum (Priority 4 — L3 Cryptographic)

Ethereum has:
- Global mutable state
- No natural scarcity units
- No native single-use enforcement
- Smart contracts that can track consumption via storage mappings

Ethereum requires a **nullifier registry contract**. This is the hardest chain because **we're building the property we need on top of a system that doesn't have it structurally.** Right = Nullifier entry. Consumption = `nullifiers[id] = true`. Security depends on contract correctness (cryptographic guarantee, not structural).

---

## Sprint 0: Canonical Model + Cryptographic Foundations (Weeks 1–4)

**Why first:** Building real network integration on top of broken commitment encoding AND without the canonical Right type is wasted effort. The Blueprint defines what we're actually building.

### 0.1 Implement the `Right` Type in Core
- [ ] Define `Right` struct in `csv-adapter-core/src/right.rs`
  - `id: Hash` — unique identifier
  - `commitment: Hash` — encodes state + rules
  - `owner: OwnershipProof` — signature / capability / object ownership
  - `nullifier: Option<Hash>` — one-time consumption marker (L3+)
  - `state_root: Option<Hash>` — off-chain state commitment
  - `execution_proof: Option<Proof>` — optional ZK/fraud proof
- [ ] Implement `Right::create()` — generates commitment and ID
- [ ] Implement `Right::consume()` — marks nullifier (L3) or triggers destruction (L1/L2)
- [ ] Implement `Right::verify()` — client-side validation flow
- [ ] Write unit tests for all lifecycle operations

**Files:** `csv-adapter-core/src/right.rs` (new), `csv-adapter-core/src/lib.rs`

### 0.2 Remove CommitmentV1 Entirely
- [ ] Delete `CommitmentV1` struct and all enum variants
- [ ] Remove all V1 test functions from `commitment.rs`
- [ ] Remove V1 parsing path from `from_canonical_bytes()`
- [ ] Update all `Commitment::v1()` call sites in adapters to use `Right`-based flow
- [ ] Verify no code references `V1`, `CommitmentV1`, or `Commitment::V2` pattern matching

**Files:** `csv-adapter-core/src/commitment.rs`, all adapters' `hash_commitment()` methods

### 0.3 Tagged Hashing (Domain Separation)
- [ ] Implement tagged hash: `sha256(sha256(tag) || sha256(tag) || data)`
- [ ] Replace all raw `Sha256::new().update(data)` in `mpc.rs` with tagged hashes
- [ ] Replace all raw SHA-256 in `commitment.rs` with tagged hashes
- [ ] Replace all raw SHA-256 in `dag.rs` with tagged hashes
- [ ] Use tag prefix: `"urn:lnp-bp:csv:"` for all commitment-related hashes
- [ ] Verify MPC tree hashing matches RGB's tagged hash scheme

**Files:** `csv-adapter-core/src/mpc.rs`, `commitment.rs`, `dag.rs`

### 0.4 Swap Custom MPT for alloy-trie (Ethereum)
- [ ] Remove custom MPT implementation in `mpt.rs` (~1,022 lines)
- [ ] Add `alloy-trie` dependency to `csv-adapter-ethereum/Cargo.toml`
- [ ] Implement MPT verification using `alloy_trie::verify_proof()`
- [ ] Test against real Ethereum mainnet proof vectors (not mocked data)

**Files:** `csv-adapter-ethereum/src/mpt.rs`, `proofs.rs`, `Cargo.toml`

### 0.5 Verify rgb_compat Tapret Against LNP/BP Standard #6
- [ ] Obtain LNP/BP standard #6 (Tapret specification)
- [ ] Compare Tapret verification byte-for-byte
- [ ] Add control block verification (currently missing)
- [ ] Add Taproot merkle tree leaf position verification (currently missing)
- [ ] Add internal key verification (currently missing)

**Files:** `csv-adapter-core/src/rgb_compat.rs`

**Deliverable:** Canonical `Right` type defined. Cryptographic foundations are correct. No V1 remnants, all hashing uses domain separation, Ethereum uses EF-tested trie, Tapret verification is structurally sound.

---

## Sprint 1: Wire Real RPCs — Degradation Order (Weeks 5–12)

**Goal:** Each adapter implements the Right lifecycle at its appropriate enforcement layer.

**Order follows the degradation model:** L1 Structural (Bitcoin, Sui) → L2 Type-Enforced (Aptos) → L3 Cryptographic (Ethereum).

### Bitcoin — Weeks 5-6 (L1 Structural, Reference Implementation)
- [ ] Wire `RealBitcoinRpc` to `BitcoinAnchorLayer.publish()`
- [ ] Build Taproot commitment transaction via `CommitmentTxBuilder`
- [ ] Sign with Schnorr (BIP-341) via `SealWallet`
- [ ] Broadcast via `bitcoincore-rpc`
- [ ] Return real txid and block height in `BitcoinAnchorRef`
- [ ] Wire `extract_merkle_proof_from_block()` to `verify_inclusion()`
- [ ] Test against Signet node (local or public)
- [ ] **Verify behavior matches RGB exactly** — this is the L1 reference
- [ ] **Confirm: no nullifier logic needed** (chain enforces structurally)

**Files:** `csv-adapter-bitcoin/src/adapter.rs`, `real_rpc.rs`

### Sui — Weeks 7-8 (L1 Structural, Reference-Aligned)
- [ ] Build MoveCall transaction for `csv_seal::consume_seal()` (object consumption)
- [ ] Sign transaction (Ed25519 via `ed25519-dalek`)
- [ ] Submit via Sui JSON-RPC (`sui_executeTransactionBlock`)
- [ ] Wait for checkpoint finality
- [ ] Parse events to verify `AnchorEvent`
- [ ] **Key: Sui object consumption ≈ Bitcoin UTXO spending**
  - Object deletion = UTXO spending
  - Object versioning prevents double-spend natively
  - **No nullifier needed** — the chain enforces structurally
- [ ] Test against Sui Testnet

**Files:** `csv-adapter-sui/src/adapter.rs`, `real_rpc.rs`

### Aptos — Weeks 9-10 (L2 Type-Enforced)
- [ ] Build Move entry function for `csv_seal::delete_seal()` (resource destruction)
- [ ] Sign transaction (Ed25519)
- [ ] Submit via Aptos REST API (`/v1/transactions`)
- [ ] Wait for version confirmation
- [ ] Parse events to verify `AnchorEvent`
- [ ] **Key nuance: Move VM enforces non-duplication**
  - Resource destruction = Right consumption
  - Language-level guarantee prevents copying
  - **No nullifier needed** — Move type system enforces scarcity
  - But resources are account-scoped (not independent like UTXOs/objects)
- [ ] Test against Aptos Testnet

**Files:** `csv-adapter-aptos/src/adapter.rs`, `real_rpc.rs`

### Ethereum — Weeks 11-12 (L3 Cryptographic, Nullifier-Based)
- [ ] Deploy `CSVSeal` nullifier registry contract to Sepolia
  - `mapping(bytes32 => bool) public nullifiers`
  - `function consume(bytes32 rightId, bytes32 commitment) external`
- [ ] Integrate Alloy for transaction building + signing
- [ ] Build EIP-1559 transaction that calls `consume()`
- [ ] **Key: Ethereum requires nullifier tracking**
  - No structural single-use guarantee
  - Contract storage provides cryptographic guarantee
  - `nullifier = H(right_id || owner_secret)` — deterministic, unique
  - Security depends on contract correctness (social guarantee)
- [ ] Sign with local key or `alloy-signer-local`
- [ ] Broadcast via Alloy provider
- [ ] Parse receipt logs to verify consumption event
- [ ] Test against Sepolia public RPC

**Files:** `csv-adapter-ethereum/src/adapter.rs`, `real_rpc.rs`, `seal_contract.rs`

**Deliverable:** All four adapters implement Right lifecycle at their appropriate enforcement layer. Bitcoin and Sui working without nullifiers (L1). Aptos working via Move resource destruction (L2). Ethereum working via nullifier registry (L3).

---

### Sprint 2: Client-Side Validation Engine (Weeks 13–16)

**Goal:** The core product. Client-side validation flow works for all enforcement layers (Blueprint §6).

**This is what the product actually does.** Everything before this sprint builds infrastructure. This sprint builds the thing users run.

#### Right Lifecycle on Client (Week 13)
- [ ] Client stores full state history for each contract
- [ ] Client fetches inclusion proofs from chain
- [ ] Client builds local Right state machine (create → transfer → consume)
- [ ] Client verifies chain-enforced single-use matches local state

#### Commitment Chain Verification (Week 14)
- [ ] Fetch state proof chain from chain's storage
- [ ] Verify each commitment links to the previous (hash chain integrity)
- [ ] Verify each Right was consumed at most once
- [ ] Verify no conflicting state transitions exist
- [ ] Accept or reject the consignment based on local validation

#### Failure Mode Handling (Week 15)
- [ ] Missing history → Reject consignment
- [ ] Conflicting state → Require resolution protocol
- [ ] Double-use detected → Escalate to chain (nullifier registration or structural proof)
- [ ] State divergence → Resolve via canonical commitment

#### Cross-Layer Uniqueness Verification (Week 16)
- [ ] Bitcoin: Verify UTXO spent exactly once (chain-enforced, client-verified)
- [ ] Sui: Verify object deleted/mutated exactly once (chain-enforced, client-verified)
- [ ] Aptos: Verify resource destroyed exactly once (Move-enforced, client-verified)
- [ ] Ethereum: Verify nullifier registered exactly once (contract-enforced, client-verified)

**Deliverable:** A client that can receive a consignment, verify the full state history, confirm no double-consumption, and accept or reject it. This is the product.

---

### Sprint 3: End-to-End Testing (Weeks 17–20)

**Goal:** Full lifecycle tested across all enforcement layers. Concrete use-case runs through ALL layers (Blueprint Test #1).

#### Test Matrix

| Test | Bitcoin Signet | Ethereum Sepolia | Sui Testnet | Aptos Testnet |
|------|---------------|------------------|-------------|---------------|
| Connect to RPC | [ ] | [ ] | [ ] | [ ] |
| Query chain state | [ ] | [ ] | [ ] | [ ] |
| Create seal | [ ] | [ ] | [ ] | [ ] |
| Publish commitment | [ ] | [ ] | [ ] | [ ] |
| Verify inclusion | [ ] | [ ] | [ ] | [ ] |
| Verify finality | [ ] | [ ] | [ ] | [ ] |
| Seal replay prevention | [ ] | [ ] | [ ] | [ ] |
| Rollback handling | [ ] | [ ] | [ ] | [ ] |
| Network failure handling | [ ] | [ ] | [ ] | [ ] |

#### Infrastructure
- [ ] Set up CI with testnet access (GitHub Actions or self-hosted runner)
- [ ] Create test fixtures (pre-funded testnet wallets)
- [ ] Add retry logic with exponential backoff for transient RPC failures
- [ ] Add timeout configuration per chain
- [ ] Write failure-mode tests (node down, reorg, insufficient funds)

**Deliverable:** All 36 test matrix items passing on live testnets. CI pipeline green.

---

### Sprint 4: Cross-Chain Right Portability (Weeks 21–24)

**Goal:** Move a Right between chains with verifiable proof. Concrete cross-chain flow tested (Blueprint Test #3).

#### Design (Week 21)
- [ ] Specify cross-chain Right transfer format
- [ ] Define nullifier scope (global? per contract? per application?)
  - **Decision needed** — Blueprint §12 Open Design Edge #1
- [ ] Define settlement strategy (immediate finality vs optimistic delay)
  - **Decision needed** — Blueprint §12 Open Design Edge #3
- [ ] Specify proof format for cross-chain verification

#### Implementation (Weeks 22–23)
- [ ] Implement lock-and-prove on source chain (L1/L2)
- [ ] Implement nullifier-based mint on destination chain (L3)
- [ ] Implement cross-chain proof verification
- [ ] Add `CrossChainValidator` with real verification logic

#### Testing (Week 24)
- [ ] Test Bitcoin → Ethereum Right transfer (L1 → L3)
- [ ] Test Sui → Aptos Right transfer (L1 → L2)
- [ ] Test double-spend prevention across chains
- [ ] Test adversarial scenario: double-spend under latency (Blueprint Test #2)

**Deliverable:** Working cross-chain Right transfers between at least 2 chain pairs with proof verification.

---

### Sprint 5: RGB Verification (Weeks 25–27)

**Goal:** Verify CSV Adapter is truly compatible with RGB protocol — not just a re-implementation.

#### Comparison (Week 25)
- [ ] Obtain RGB reference implementation
- [ ] Map `Right` type to RGB consignment fields
- [ ] Identify all divergences (field names, serialization, validation rules)
- [ ] Document compatibility matrix

#### Alignment (Week 26)
- [ ] Fix any format divergences
- [ ] Verify Tapret structure matches RGB + BIP-341 exactly
- [ ] Verify OP_RETURN fallback matches RGB specification
- [ ] Verify schema validation rules match RGB

#### Interop Testing (Week 27)
- [ ] Create a CSV Right and validate it with RGB tools
- [ ] Create an RGB consignment and validate it with CSV tools
- [ ] Test state transfer between CSV and RGB implementations
- [ ] Document interoperability guarantees

**Deliverable:** Verified compatibility matrix. At least one successful cross-validation between CSV Rights and RGB consignments.

---

### Sprint 6: Security Hardening (Weeks 28–30)

**Goal:** Production-grade security posture.

#### Code Review (Week 28)
- [ ] Internal audit of all critical paths
- [ ] Review signature verification logic
- [ ] Review Right consumption logic for race conditions
- [ ] Review proof verification for edge cases
- [ ] Review nullifier storage strategies for privacy leaks

#### Testing (Week 28–29)
- [ ] Fuzz test all parsing functions (proptest or afl)
- [ ] Fuzz test proof verification
- [ ] Fuzz test signature verification
- [ ] Property-based testing of Right lifecycle
- [ ] Property-based testing of nullifier registry

#### External Audit (Week 30)
- [ ] Engage third-party security auditor
- [ ] Provide scope: all adapters, core types, proof verification, nullifier handling
- [ ] Fix all critical/high findings
- [ ] Publish audit report

**Deliverable:** Audit report published. All critical findings resolved.

---

## Dependency Graph

```
Sprint 0: Canonical Model + Crypto ──────────────────────┐
                                                         ▼
Sprint 1: Wire RPCs (L1→L2→L3) ───────────────────────> Sprint 3: E2E Testing
                                                         │
Sprint 2: Client-Side Validation ────────────────────────┘
                                                         │
Sprint 4: Cross-Chain Portability ───────────────────────┘
                                                         │
Sprint 5: RGB Verification ──────────────────────────────┤ (parallel)
                                                         │
Sprint 6: Security Hardening ────────────────────────────┘
```

Sprint 0 is a prerequisite for everything — no point wiring RPCs on broken crypto or without the `Right` type.
Sprint 1 and 2 can start in parallel (RPC wiring and client validation are independent).
Sprint 3 depends on both 1 and 2. Sprints 4, 5, and 6 start once Sprint 3 is complete.

---

## Risks and Mitigations

| Risk | Impact | Likelihood | Mitigation |
|------|--------|------------|------------|
| SDK API changes | Medium | High | Pin versions. Abstract behind trait. |
| Testnet faucet limits | Low | High | Run local nodes as fallback. |
| Move contract bugs | High | Medium | Formal verification of Move code. Test extensively before deployment. |
| Cross-chain design flaws | Critical | High | Security review before implementation. Start with 2 chains. |
| Audit finds critical issues | High | Medium | Budget 2 extra weeks for fixes. Start audit early. |
| RGB reference incompatible | Medium | Low | Early comparison (Week 15) to detect issues before deep integration. |

---

## Milestones

| Week | Milestone | Success Criteria |
|------|-----------|-----------------|
| 4 | Canonical `Right` type + Crypto correct | Right struct defined, no V1, tagged hashing, alloy-trie |
| 6 | Bitcoin RPC wired (L1) | `publish()` returns real txids, matches RGB behavior, no nullifier |
| 8 | Sui RPC wired (L1) | Object consumption works, structural alignment verified |
| 10 | Aptos RPC wired (L2) | Resource destruction works, Move non-duplication verified |
| 12 | Ethereum RPC wired (L3) | Nullifier registry works on Sepolia, contract-enforced single-use |
| 16 | Client-side validation works | Validation flow passes for all 4 enforcement layers |
| 20 | E2E tests green | All 36 test matrix items passing, concrete use-case runs through ALL layers |
| 24 | Cross-chain working | Right transfer between 2 chain pairs, adversarial test passes |
| 27 | RGB verified | Compatibility matrix published, cross-validation successful |
| 30 | Audited | Third-party audit published, all critical findings fixed |

---

## Resource Requirements

| Resource | Quantity | Duration |
|----------|----------|----------|
| Rust developers | 2–3 | 30 weeks |
| Move developer | 1 | Weeks 9–12 |
| Security auditor | External firm | Week 30 |
| Testnet infrastructure | Self-hosted nodes (optional) | Weeks 17–20 |
| Audit budget | $30k–$80k | Week 30 |

---

## Go/No-Go Criteria for Production

All of the following must be true:

- [ ] Canonical `Right` type implemented with all fields (id, commitment, owner, nullifier, state_root, execution_proof)
- [ ] All 4 adapters implement Right lifecycle at appropriate enforcement layer (L1/L2/L3)
- [ ] Bitcoin and Sui work WITHOUT nullifiers (structural enforcement)
- [ ] Aptos works via Move resource destruction (type-enforced)
- [ ] Ethereum works via nullifier registry (cryptographic enforcement)
- [ ] Client-side validation flow passes for all enforcement layers
- [ ] All 36 E2E test matrix items passing
- [ ] Concrete use-case runs through ALL layers (Blueprint Test #1)
- [ ] Adversarial scenario passes: double-spend under latency (Blueprint Test #2)
- [ ] Cross-chain Right transfer between at least 2 chain pairs (Blueprint Test #3)
- [ ] RGB compatibility verified with cross-validation successful
- [ ] Third-party security audit completed with no unresolved critical findings
- [ ] CI pipeline green on every commit
- [ ] All dependencies pinned to specific versions
- [ ] Operations runbook written (incident response, monitoring, alerting)
- [ ] Nullifier scope decision documented (Blueprint §12 Edge #1)
- [ ] Privacy level decision documented (Blueprint §12 Edge #2)
- [ ] Settlement strategy decision documented (Blueprint §12 Edge #3)

---

*Created: April 10, 2026*
*Updated: April 10, 2026 — Aligned with Blueprint (Universal Seal Primitive)*
*Updated: April 10, 2026 — Added degradation model (L1→L2→L3)*
*Next review: After Sprint 0 (Week 4)*
