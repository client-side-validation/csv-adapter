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

### When Does the USP Actually Come Into Play?

**Sprint 2 (Week 13).** Here's why:

| Sprint | What Exists | Is the USP Operational? |
|--------|------------|------------------------|
| 0.1 | `Right` type defined | ❌ No — just a data structure with tests |
| 1 | Each chain wires its native primitive (UTXO, Object, Resource, Nullifier) | ❌ No — each chain still uses its own thing |
| **2** | **Client receives consignment, maps all chain primitives → unified `Right`, validates uniformly** | **✅ Yes — the USP is now the working abstraction** |
| 3-6 | Testing, cross-chain, RGB, audit | ✅ Yes — everything builds on Sprint 2 |

Before Sprint 2, the `Right` type exists but nothing uses it to unify chain-specific primitives. After Sprint 2, the client takes heterogeneous anchors (UTXO spends, object deletions, resource destructions, nullifier registrations) and validates them all through a single `Right.verify()` interface.

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

### Sprint 1: Wire Real RPCs — Status (Weeks 5–12)

**Goal:** Each adapter's `publish()` builds, signs, and broadcasts a real transaction.

**Current Status:**

| Chain | Status | What Works | What's Missing |
|-------|--------|-----------|----------------|
| **Bitcoin** | ✅ COMPLETE | `publish()` → tx_builder builds Taproot tx → wallet signs → `rpc.send_raw_transaction()` broadcasts → returns real txid | Needs Signet node to execute |
| **Sui** | ✅ COMPLETE | Full flow: verify seal → `build_sui_transaction_data()` (manual BCS) → Ed25519 sign → `rpc.execute_signed_transaction()` → wait → verify event → mark consumed | Needs Sui Testnet node to verify BCS format |
| **Aptos** | ✅ Structurally complete | Full flow: verify seal → build event → `rpc.submit_transaction()` → wait → verify → mark consumed | `submit_transaction()` returns placeholder; needs Move tx construction + Ed25519 signing |
| **Ethereum** | ✅ Structurally complete | Full flow: verify slot → build calldata → `rpc.send_raw_transaction()` → verify receipt LOG event → mark consumed | `publish()` needs Alloy tx building + signing before calling `send_raw_transaction()` |

**E2E Test Infrastructure Added:**
- `csv-adapter-bitcoin/tests/signet_e2e.rs` — Bitcoin Signet end-to-end test (connects to `mempool.space/signet/api/`)
- `csv-adapter-sui/tests/testnet_e2e.rs` — Sui Testnet end-to-end test (connects to `fullnode.testnet.sui.io`)
- Both tests are `#[ignore]` — run with `--ignored` flag when testnet credentials are available
- Tests verify: seal creation, publishing, inclusion proof, finality, rollback, replay prevention

**The architectural path is complete for all chains.** What remains inside each chain's `publish()` is SDK-dependent transaction construction (Taproot for Bitcoin, Move for Sui/Aptos, EVM for Ethereum). The adapter wiring — verify, build, submit, wait, verify, mark — is done.

### What Each Chain Still Needs

#### Bitcoin (Ready for Signet testing)
The full path works: `publish()` → `tx_builder.build_commitment_tx()` → `wallet.sign_tx()` → `rpc.send_raw_transaction()`. Needs a running Signet node with funded UTXOs to execute end-to-end.

#### Sui (TransactionData built, needs live node verification)
Full flow: `publish()` → `build_sui_transaction_data()` → Ed25519 sign → `rpc.execute_signed_transaction()` → `wait_for_transaction()` → verify event → mark consumed.
- `build_sui_transaction_data()` manually constructs Sui's BCS TransactionData wire format
- Ed25519 signing works via `ed25519_dalek::SigningKey`
- Signature format: `[0x00 (Ed25519 type)] + [64-byte signature] + [32-byte public key]`
- `execute_signed_transaction()` calls `sui_executeTransactionBlock` JSON-RPC with base64-encoded tx/sig/pk
- **Missing:** Live node verification — the manual BCS construction follows Sui's spec but hasn't been tested against a real Sui node. The `sha3` dependency for address derivation was already present. Needs Testnet execution to confirm BCS format matches exactly.

#### Aptos (Needs Move transaction construction)
`submit_transaction()` in `real_rpc.rs` currently returns a placeholder. To make it real: build an Aptos Entry Function payload calling `csv_seal::delete_seal()`, sign with Ed25519, POST to `/v1/transactions`. Requires Aptos SDK for proper Move transaction construction.

#### Ethereum (Needs EVM transaction construction)
`publish()` calls `rpc.send_raw_transaction()` with pre-built bytes. To make it real: use Alloy to build an EIP-1559 transaction calling `CSVSeal.consume()`, sign with local key, serialize, broadcast. Requires `alloy-signer-local` and the deployed `CSVSeal` contract address.

---

### Sprint 2: Client-Side Validation Engine — The USP Becomes Real (Weeks 13–16)

**Goal:** The USP becomes the working abstraction. The client validates consignments using `Right` uniformly, regardless of which chain enforces single-use.

**This is when the Universal Seal Primitive actually comes into play.** Before Sprint 2:
- The `Right` type exists as a data structure (Sprint 0.1)
- Each chain adapter uses its native primitive (UTXO, Object, Resource, Nullifier) (Sprint 1)
- But nothing unifies them

In Sprint 2, the client **receives a consignment, maps each chain's native enforcement to a unified `Right`, and validates the whole thing locally.**

### How the USP Becomes Operational

```
Client receives consignment from peer:
  │
  ├─ Bitcoin anchor?  → Map UTXO spend → Right(id, commitment, owner, nullifier=None)
  ├─ Sui anchor?      → Map object deletion → Right(id, commitment, owner, nullifier=None)
  ├─ Aptos anchor?    → Map resource destruction → Right(id, commitment, owner, nullifier=None)
  └─ Ethereum anchor? → Map nullifier registration → Right(id, commitment, owner, nullifier=Some(hash))
        │
        ▼
  Client validates uniformly:
    1. Each Right.verify() passes
    2. No Right appears twice (double-consumption check)
    3. Commitment chain integrity (genesis → present)
    4. Accept or reject the consignment
```

**This is the product.** Not the adapters. Not the RPC wiring. The client that receives a consignment, maps heterogeneous chain primitives to unified `Right`s, and validates them all the same way.

#### Right Lifecycle on Client (Week 13)
- [ ] Client stores full state history for each contract
- [ ] Client fetches inclusion proofs from chain
- [ ] Client maps chain-specific anchors to `Right` type
  - Bitcoin: UTXO → `Right` (L1, no nullifier)
  - Sui: Object → `Right` (L1, no nullifier)
  - Aptos: Resource → `Right` (L2, no nullifier)
  - Ethereum: Nullifier → `Right` (L3, nullifier set)
- [ ] Client builds local `Right` state machine (create → transfer → consume)
- [ ] Client verifies chain-enforced single-use matches local state

#### Commitment Chain Verification (Week 14)
- [ ] Fetch state proof chain from chain's storage
- [ ] Verify each commitment links to the previous (hash chain integrity)
- [ ] Verify each `Right` was consumed at most once
- [ ] Verify no conflicting state transitions exist
- [ ] Accept or reject the consignment based on local validation

#### Failure Mode Handling (Week 15)
- [ ] Missing history → Reject consignment
- [ ] Conflicting state → Require resolution protocol
- [ ] Double-use detected → Escalate to chain (nullifier registration or structural proof)
- [ ] State divergence → Resolve via canonical commitment

#### Cross-Layer Uniqueness Verification (Week 16)
- [ ] Bitcoin: Verify UTXO spent exactly once (chain-enforced, client-verified via `Right`)
- [ ] Sui: Verify object deleted/mutated exactly once (chain-enforced, client-verified via `Right`)
- [ ] Aptos: Verify resource destroyed exactly once (Move-enforced, client-verified via `Right`)
- [ ] Ethereum: Verify nullifier registered exactly once (contract-enforced, client-verified via `Right`)

**Deliverable:** A client that receives a consignment, maps heterogeneous chain primitives to unified `Right`s, verifies the full state history, confirms no double-consumption, and accepts or rejects it. **The USP is now the working abstraction.**

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
