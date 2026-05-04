# CSV Adapter — Comprehensive Engineering Plan

**Date:** May 2026  
**Status:** Codebase at production-candidate, protocol correctness gaps, wallet in design debt  
**Objective:** Scalable, maintainable, real-world DeFi application maximizing CSV + Single-Use Seal advantages

---

## Table of Contents

1. [Honest Assessment — Where You Are](#1-honest-assessment)
2. [Critical Bugs — Fix Today](#2-critical-bugs)
3. [Phase 0 — Foundation Stability](#3-phase-0-foundation-stability)
4. [Phase 1 — Protocol Correctness (The Core Differentiator)](#4-phase-1-protocol-correctness)
5. [Phase 2 — Codebase Control & Maintainability](#5-phase-2-codebase-control)
6. [Phase 3 — Wallet Design System](#6-phase-3-wallet-design-system)
7. [Phase 4 — CSV/Single-Use-Seal Competitive Advantages](#7-phase-4-competitive-advantages)
8. [Phase 5 — ZK Proofs (Ordered Correctly)](#8-phase-5-zk-proofs)
9. [Phase 6 — Ecosystem Expansion](#9-phase-6-ecosystem)
10. [What NOT to Build Yet](#10-what-not-to-build-yet)
11. [File Ownership Map](#11-file-ownership-map)

---

## 1. Honest Assessment

### What works well

- `csv-adapter-core` protocol center is architecturally sound
- `AnchorLayer` → `FullChainAdapter` trait hierarchy is clean
- Chain-graded trust model (UTXO vs Object vs Nullifier) is a genuine insight
- `validator.rs` 5-step pipeline is correct in design
- `commitment_chain.rs` hash-linkage is correct
- `mpc.rs` MPC tree structure is correct
- Explorer stack (indexer + GraphQL + WebSocket) is ambitious and correct

### Where drift has set in

| Location | Problem |
|----------|---------|
| `csv-wallet/src/seals/manager.rs:130` | `create_seal` generates fake timestamp IDs — NOT real chain seals |
| `csv-wallet/src/wallet_core.rs` | `balance: f64` — floating-point for financial values |
| `csv-wallet/src/context/state.rs` | `PartialEq` misses `rights`, `seals`, `proofs`, `transfers` — causes silent UI staleness |
| `csv-wallet/src/main.rs` | 300ms `sleep` waiting for Tailwind CDN Play — development hack in production |
| `csv-wallet/src/` | Two wallet state systems: `context/wallet.rs` (WalletContext) vs `core/wallet.rs` (WalletMetadata) — no clear owner |
| `csv-explorer/ui/src/pages/wallet.rs:494` | `seal_ref: format!("seal_{}", &right_id[..16])` — fake seal refs in explorer UI |
| Docs everywhere | `experimental` features (ZK, VM, MPC) described as if integrated — they are trait stubs |

### Premature optimization assessment

Your instinct list: *chains, ZK proofs, AluVM, MPC, integrations, more scalability*

| Idea | Verdict | Reason |
|------|---------|--------|
| More chain adapters | **Wait** | 5 chains not yet fully wired to wallet |
| ZK proofs (SP1/Risc0) | **Phase 5** — not premature, but requires Phase 1 first | Seal correctness must precede proof correctness |
| AluVM execution | **Phase 5** | Trait exists, real integration needs stable state machine |
| MPC tree | **Phase 4** | Already correct types, needs wire-up to anchoring |
| More cross-chain bridges | **Wait** | Current cross-chain is not fully tested on testnet |
| TypeScript SDK | **Phase 6** | High value, needs stable Rust API surface first |
| AI agent (MCP server) | **Phase 6** | High value, needs stable CLI/API first |

---

## 2. Critical Bugs — Fix Today

### BUG-01: Fake Seal IDs (Protocol Correctness — SEVERITY: CRITICAL)

**File:** `csv-wallet/src/seals/manager.rs:130`

```rust
// CURRENT — WRONG: This is a made-up ID with no chain anchoring
let seal_id = format!("seal_{}_{}", chain, chrono::Utc::now().timestamp_millis());
```

A single-use seal MUST be a chain-native reference:

- **Bitcoin**: `OutPoint { txid, vout }` — a specific UTXO
- **Ethereum**: `(contract_address, storage_slot)` or nullifier hash
- **Sui**: `ObjectId` — a real Sui object
- **Aptos**: `(resource_address, key)` — a real Move resource
- **Solana**: `Pubkey` of the seal account

**Fix:** `SealManager::create_seal` must call the chain adapter to create a real on-chain seal, then store the returned chain-native identifier.

```rust
// CORRECT architecture:
pub async fn create_seal(
    &self,
    chain: Chain,
    adapter: &dyn FullChainAdapter,
    value: Option<u64>,
) -> Result<SealRecord, String> {
    // Delegate to the real chain adapter — this publishes the seal on-chain
    let seal_ref = adapter.create_seal(value).await
        .map_err(|e| e.to_string())?;
    
    let record = SealRecord {
        id: hex::encode(&seal_ref.seal_id),  // Real chain-native ID
        chain,
        seal_ref: Some(seal_ref),             // Store the full SealRef
        status: SealStatus::Unconsumed,
        // ...
    };
    self.store.save_seal(&record)?;
    Ok(record)
}
```

This also means `SealRecord` needs a `seal_ref: Option<SealRef>` field to carry the real protocol type.

### BUG-02: Float Balance (Financial Correctness — SEVERITY: HIGH)

**File:** `csv-wallet/src/wallet_core.rs:23`

```rust
// CURRENT — WRONG: f64 loses precision for large satoshi/lamport values
pub balance: f64,
```

**Fix:** Store in chain-native units as `u64`, convert to display string at render time.

```rust
// In wallet_core.rs
pub balance_raw: u64,      // Satoshis / Lamports / Octas / MIST / Wei

// In display layer (components)
fn format_balance(raw: u64, chain: Chain) -> String {
    match chain {
        Chain::Bitcoin  => format!("{:.8} BTC", raw as f64 / 1e8),
        Chain::Ethereum => format!("{:.6} ETH", raw as f64 / 1e18),
        Chain::Solana   => format!("{:.6} SOL", raw as f64 / 1e9),
        Chain::Sui      => format!("{:.9} SUI", raw as f64 / 1e9),
        Chain::Aptos    => format!("{:.8} APT", raw as f64 / 1e8),
        _ => format!("{} units", raw),
    }
}
```

### BUG-03: Silent UI Staleness (Dioxus Reactivity — SEVERITY: HIGH)

**File:** `csv-wallet/src/context/state.rs:43`

```rust
// CURRENT — WRONG: rights, seals, proofs, transfers changes are invisible
impl PartialEq for AppState {
    fn eq(&self, other: &Self) -> bool {
        self.selected_chain == other.selected_chain
            && self.selected_network == other.selected_network
            && self.wallet.total_accounts() == other.wallet.total_accounts()
            && self.nfts.len() == other.nfts.len()
            && self.nft_collections.len() == other.nft_collections.len()
        // rights.len(), seals.len(), proofs.len(), transfers.len() NOT CHECKED
    }
}
```

**Fix:**

```rust
impl PartialEq for AppState {
    fn eq(&self, other: &Self) -> bool {
        self.selected_chain == other.selected_chain
            && self.selected_network == other.selected_network
            && self.wallet.total_accounts() == other.wallet.total_accounts()
            && self.rights.len() == other.rights.len()
            && self.seals.len() == other.seals.len()
            && self.proofs.len() == other.proofs.len()
            && self.transfers.len() == other.transfers.len()
            && self.contracts.len() == other.contracts.len()
            && self.transactions.len() == other.transactions.len()
            && self.nfts.len() == other.nfts.len()
            && self.notification.is_some() == other.notification.is_some()
    }
}
```

### BUG-04: Fake seal_ref in Explorer UI

**File:** `csv-explorer/ui/src/pages/` (line 52494 in repomix)

```rust
// CURRENT — WRONG:
seal_ref: format!("seal_{}", &right_id.read()[..16.min(right_id.read().len())]),
```

This creates a display string that looks like a seal ref but contains a truncated right ID. Users will trust this as a real seal reference. **Fix:** Use the actual `SealRef.seal_id` bytes serialized to hex, or display "unknown" honestly.

### BUG-05: Tailwind CDN Play in Production

**File:** `csv-wallet/src/main.rs:28`

```rust
// CURRENT — WRONG: Production code waiting for a CDN to scan classes
sleep(Duration::from_millis(300)).await;
ready.set(true);
```

**Fix:** Switch to Tailwind CLI with `--watch` for dev and pre-built CSS output for production. Add to `Dioxus.toml` the `tailwind_input` path. See Phase 3 for full wallet CSS architecture.

---

## 3. Phase 0 — Foundation Stability

**Goal:** Stop accumulating debt. Establish hard gates.  
**Timeline:** 1–2 weeks  
**Owner:** You personally, before delegating anything

### 0.1 Establish Code Health Gates

Create `.github/workflows/health.yml` that fails CI on:

```yaml
- cargo clippy -- -D warnings -D clippy::float_arithmetic  # catches f64 balance misuse
- cargo test --workspace
- cargo doc --no-deps --document-private-items  # doc coverage
```

Add a `CONTRIBUTING.md` with the rule: **no PR merges without CI green**.

### 0.2 Fix the 5 Critical Bugs Above

Apply BUG-01 through BUG-05 fixes in a single PR titled "Protocol and UI correctness fixes". This PR must not add features.

### 0.3 Resolve Dual Wallet State System

Current situation: two systems coexist with unclear ownership.

| Module | Purpose |
|--------|---------|
| `csv-wallet/src/context/wallet.rs` | `WalletContext` — Dioxus context provider with localStorage persistence |
| `csv-wallet/src/core/wallet.rs` | `WalletMetadata`, `BitcoinNetwork` — HD wallet structures |
| `csv-wallet/src/wallet_core.rs` | `ChainAccount`, `WalletData` — account management |

**Decision:** Consolidate to:

```
csv-wallet/src/
  wallet/
    mod.rs           <- re-exports everything
    account.rs       <- ChainAccount (moved from wallet_core.rs)
    data.rs          <- WalletData (moved from wallet_core.rs)  
    hd.rs            <- HD/BIP-44 derivation (moved from core/wallet.rs)
    metadata.rs      <- WalletMetadata (moved from core/wallet.rs)
    context.rs       <- WalletContext (moved from context/wallet.rs)
    storage.rs       <- LocalStorageManager (moved from storage.rs)
```

Delete `csv-wallet/src/wallet_core.rs`, `csv-wallet/src/core/wallet.rs`, `csv-wallet/src/context/wallet.rs` after migration. Update all imports. This is a pure refactor — no behavior change.

### 0.4 Create a `CODEBASE_OWNERS.md`

Map every crate to a maintenance owner and a stability contract:

```markdown
| Crate | Owner | Stability |
|-------|-------|-----------|
| csv-adapter-core | You | FROZEN — no breaking changes without RFC |
| csv-adapter-bitcoin | You | STABLE |
| csv-adapter-ethereum | You | STABLE |
| csv-adapter-sui | You | STABLE |
| csv-adapter-aptos | You | STABLE |
| csv-adapter-solana | You | BETA |
| csv-adapter | You | STABLE |
| csv-adapter-store | Junior dev OK | BETA |
| csv-adapter-keystore | You | STABLE |
| csv-cli | Junior dev OK | BETA |
| csv-wallet | Junior dev OK (UI only) | ALPHA |
| csv-explorer | Junior dev OK | ALPHA |
```

**Rule for junior devs:** Only touch `ALPHA`/`BETA` crates. Any change to `STABLE` crates requires your review. Any change to `csv-adapter-core` requires your sign-off and a doc update.

---

## 4. Phase 1 — Protocol Correctness

**Goal:** Make the CSV/seal semantics actually correct end-to-end.  
**Timeline:** 2–4 weeks  
**Why first:** Everything else (ZK, MPC, cross-chain, UI) builds on top of this. Getting proofs right on wrong seals gives you nothing.

### 1.1 Wire Real Chain Seals to SealManager

The flow must be:

```
User: "Create a seal on Sui"
  → csv-wallet/src/seals/manager.rs::create_seal(chain, adapter)
  → csv-adapter-sui/src/seal.rs::create_seal_object()
  → Sui RPC: create Sui Object → returns ObjectId
  → SealRef { seal_id: object_id_bytes, nonce: None }
  → SealRecord { id: hex(object_id), seal_ref: Some(SealRef), ... }
  → stored in csv-wallet localStorage
```

**Files to change:**

- `csv-wallet/src/seals/manager.rs` — add `adapter` parameter to `create_seal`
- `csv-wallet/src/seals/store.rs` — add `seal_ref` field to storage schema
- `csv-wallet/src/services/seal_service.rs` — wire adapter resolution
- `csv-adapter-sui/src/seal.rs` — verify `create_seal_object` exists and returns `SealRef`
- `csv-adapter-bitcoin/src/seal.rs` — verify creates real UTXO-based seal
- `csv-adapter-ethereum/src/seal.rs` — verify calls `CSVLock.sol` and returns storage slot

### 1.2 Wire Validator to the UI

`csv-adapter-core/src/validator.rs` contains a correct 5-step validation pipeline. It is never called from the wallet. When a user receives a proof bundle or a consignment, the validator must run before the right is accepted into state.

**Current flow (broken):**

```
User receives transfer → UI updates state optimistically → no validation
```

**Correct flow:**

```
User receives consignment
  → csv-wallet/src/pages/validate/consignment.rs (already exists!)
  → csv-adapter-core::validator::ConsignmentValidator::validate()
  → All 5 steps pass → accept into AppState
  → Any step fails → reject, show user which step failed and why
```

**Files to change:**

- `csv-wallet/src/pages/validate/consignment.rs` — connect to `ConsignmentValidator`
- `csv-wallet/src/pages/validate/proof.rs` — connect to `ProofVerifier`
- `csv-wallet/src/pages/validate/seal.rs` — connect to `SealRegistry::check_consumed`
- `csv-wallet/src/context/wallet.rs` — add `accept_consignment()` method that runs validator first

### 1.3 Make Commitment Chain Walkable in UI

`csv-adapter-core/src/commitment_chain.rs` correctly walks the hash chain. Surface this.

**Add to `csv-wallet/src/pages/rights/journey.rs`** (already exists):

- Show each commitment in the chain as a timeline node
- Show the genesis commitment separately highlighted
- Show `previous_commitment` hash linking to prior node
- Show chain (Bitcoin/Ethereum/etc.) for each anchor

This is the **primary UI proof** that CSV works differently from bridges. The user can SEE their entire provenance chain. No trusted party needed.

### 1.4 Testnet Integration Tests

**File:** `.github/workflows/ci.yml` — add a job:

```yaml
testnet-integration:
  runs-on: ubuntu-latest
  if: github.ref == 'refs/heads/main'
  steps:
    - run: cargo test --test integration_bitcoin -- --ignored  
    - run: cargo test --test integration_ethereum -- --ignored
    - run: cargo test --test integration_sui -- --ignored
```

Create `csv-adapter-bitcoin/tests/integration_bitcoin.rs` with:

- Create seal on signet
- Publish commitment
- Verify inclusion
- Verify finality
- Build proof bundle
- Verify proof bundle

These tests already have groundwork in `csv-adapter-bitcoin/examples/signet_real_tx_demo.rs`. Promote to proper integration tests.

---

## 5. Phase 2 — Codebase Control & Maintainability

**Goal:** Junior devs can work without breaking protocol invariants.  
**Timeline:** 3–4 weeks (run parallel to Phase 1)

### 2.1 Error Handling Unification

Current state: each crate defines its own error types with no consistent mapping.

**Problem files:**

- `csv-adapter-core/src/error.rs` — `AdapterError`
- `csv-adapter-bitcoin/src/error.rs` — `BitcoinError`
- `csv-adapter-ethereum/src/error.rs` — `EthereumError`
- `csv-adapter-sui/src/error.rs` — `SuiError`
- `csv-adapter-aptos/src/error.rs` — `AptosError`
- `csv-adapter-solana/src/error.rs` — `SolanaError`
- `csv-adapter/src/errors.rs` — `CsvError`
- `csv-wallet/src/` — `String` errors everywhere

**Fix:** All chain errors must implement `Into<AdapterError>`. Add `From<BitcoinError> for AdapterError`, etc. The wallet uses only `CsvError` which wraps `AdapterError`. No `String` errors in service calls.

In `csv-adapter-core/src/error.rs`, add:

```rust
#[derive(Debug, thiserror::Error)]
pub enum AdapterError {
    // ... existing variants ...
    
    /// Seal was never created on-chain (caught fake seal IDs)
    #[error("Seal {0} has no on-chain anchor — was it created via a real chain adapter?")]
    SealNotAnchored(String),
}
```

### 2.2 Module Dependency Rules

Enforce with a lint tool (cargo-deny or custom):

```
csv-adapter-core     → no workspace deps (only std + external)
csv-adapter-{chain}  → may depend on csv-adapter-core
csv-adapter          → may depend on csv-adapter-core + csv-adapter-{chain}
csv-adapter-store    → may depend on csv-adapter-core
csv-adapter-keystore → may depend on csv-adapter-core
csv-cli              → may depend on csv-adapter + csv-adapter-store + csv-adapter-keystore
csv-wallet           → may depend on csv-adapter + csv-adapter-store + csv-adapter-keystore
csv-explorer         → may depend on csv-adapter + csv-adapter-store
```

**Violations to fix:**

- `csv-wallet` must NOT directly import `csv-adapter-bitcoin`, `csv-adapter-ethereum`, etc. — all chain operations go through `csv-adapter::ChainFacade`
- Check current `csv-wallet/Cargo.toml` for direct chain adapter deps

### 2.3 Public API Surface Audit

`csv-adapter-core/src/lib.rs` has three tiers: Stable, Beta, Experimental. Enforce this in CI:

```rust
// Add to csv-adapter-core/build.rs:
// Check that no Experimental item appears in Stable re-exports
```

Add `#[doc(hidden)]` to all experimental items. Gate them harder:

```rust
#[cfg(all(feature = "experimental", not(doc)))]
pub mod vm;
```

### 2.4 State Machine for Cross-Chain Transfers

**File:** `csv-adapter-core/src/cross_chain.rs`

Cross-chain transfers have implicit state (Lock → WaitFinality → ProveInclusion → MintDestination) but this is not modeled as an explicit state machine. Junior devs are adding code that skips steps.

**Add to `csv-adapter-core/src/cross_chain.rs`:**

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TransferState {
    /// Seal locked on source chain, tx submitted
    Locked { source_tx: String, lock_height: u64 },
    /// Waiting for finality on source chain
    AwaitingFinality { confirmations_needed: u32, confirmations_have: u32 },
    /// Finality reached, building proof bundle
    BuildingProof,
    /// Proof bundle ready, transmitting to destination
    ProofReady { bundle: ProofBundle },
    /// Minting on destination chain
    Minting { dest_tx: Option<String> },
    /// Transfer complete
    Complete { dest_tx: String, dest_seal: SealRef },
    /// Transfer failed, reason recorded
    Failed { reason: String, recoverable: bool },
}
```

The wallet's `csv-wallet/src/pages/cross_chain/` pages should drive this machine, not maintain their own ad-hoc status.

### 2.5 Junior Dev Guardrails

Create `csv-adapter-core/src/PROTOCOL_INVARIANTS.md`:

```markdown
# Protocol Invariants — DO NOT VIOLATE

1. A SealRef.seal_id must come from a real blockchain transaction.
   Never construct it from a timestamp, UUID, or random bytes.

2. A Commitment must be published on-chain before a ProofBundle is built.
   Never build a ProofBundle without an AnchorRef.

3. A Right must pass ConsignmentValidator before entering AppState.
   Never accept a Right without running all 5 validation steps.

4. balances are stored as u64 native units (satoshis, lamports, MIST, octas, wei).
   Never store as f64. Never store as human-readable string.

5. SealRegistry::check_consumed must run before accepting any incoming transfer.
   Never accept a transfer without double-spend check.
```

---

## 6. Phase 3 — Wallet Design System

**Goal:** Intentional, maintainable UI with a clear user model.  
**Timeline:** 2–3 weeks  
**The real question:** Who is this wallet for?

### 3.1 Define the User Model First

Three distinct user personas require three distinct modes. Make this an explicit app decision:

**Mode A — Developer Mode**  
Target: People building on CSV protocol. Want raw data, proof inspector, seal IDs, commitment hashes, transition DAGs.  
Current pages that serve this: `seals/`, `proofs/`, `validate/`, `contracts/`, `rights/journey.rs`

**Mode B — User Mode**  
Target: End users holding tokenized rights or NFTs. Want simple: "What do I own? Send it. Receive it."  
Current pages that serve this: `wallet_page.rs`, `rights/list.rs`, `rights/transfer.rs`

**Mode C — Validator Mode**  
Target: Counterparties verifying a proof bundle they received. Want: paste proof, get yes/no with reason chain.  
Current pages that serve this: `validate/`, `proofs/verify.rs`, `proofs/verify_cross_chain.rs`

**Decision to make:** Is the wallet primarily Mode A (developer tool) or Mode B (end user)? The current routes try to be all three. Mode A and Mode C can live in a collapsible "Advanced" section. Mode B should be the default experience.

### 3.2 Remove Tailwind CDN Play

**File:** `csv-wallet/src/main.rs`

```toml
# Dioxus.toml — add:
[web.watcher]
watch_path = ["src", "public"]

[application]
tailwind_input = "public/tailwind.css"
tailwind_output = "assets/tailwind.css"
```

```
# public/tailwind.css:
@tailwind base;
@tailwind components;
@tailwind utilities;
```

Run `npx tailwindcss -i public/tailwind.css -o assets/tailwind.css --watch` in dev. CI generates the CSS before build. The 300ms hack in `main.rs` is deleted entirely.

Remove from `main.rs`:

```rust
// DELETE THIS:
use_effect(move || {
    use gloo_timers::future::sleep;
    sleep(Duration::from_millis(300)).await;
    ready.set(true);
});
```

### 3.3 Design Token Consolidation

`csv-wallet/src/components/design_tokens.rs` already defines good CSS variables. The problem is Tailwind classes bypass these tokens.

**Rule going forward:** All custom color/spacing decisions use CSS variables from `design_tokens.rs`. Tailwind utility classes allowed for layout/flex/grid. Disallow direct Tailwind color classes (`bg-blue-500` → use `var(--color-primary-500)` instead).

**Add to design_tokens.rs** — Seal-specific semantic tokens (this is your competitive advantage visually):

```css
/* Single-Use Seal States — core UI vocabulary */
--seal-unconsumed: #22c55e;     /* Green — seal is valid, unspent */
--seal-consumed: #6b7280;       /* Gray — seal is spent, right transferred */
--seal-double-spent: #ef4444;   /* Red — protocol violation detected */
--seal-pending: #f59e0b;        /* Amber — awaiting chain finality */

/* Proof verification states */
--proof-valid: #22c55e;
--proof-invalid: #ef4444;
--proof-pending: #f59e0b;
--proof-unverified: #6b7280;

/* Commitment chain visualization */
--chain-genesis: #8b5cf6;       /* Purple — genesis commitment */
--chain-node: #3b82f6;          /* Blue — intermediate commitment */
--chain-latest: #22c55e;        /* Green — latest valid commitment */
--chain-link: #374151;          /* Dark — hash chain link lines */
```

### 3.4 Navigation Architecture

Current sidebar has flat navigation across all 3 user modes. **Restructure:**

```
csv-wallet/src/layout.rs (updated)

Sidebar:
  [Mode Switcher: User | Developer | Validator]
  
  User Mode:
    Portfolio          /wallet
    My Rights          /rights
    Send/Receive       /rights/transfer
    Cross-Chain        /cross-chain
    History            /transactions
  
  Developer Mode (collapsed by default):
    Seals              /seals
    Proofs             /proofs  
    Contracts          /contracts
    Validate           /validate
    Settings           /settings
  
  Validator Mode:
    Verify Proof       /proofs/verify
    Verify Transfer    /proofs/verify-cross-chain
    Check Seal         /seals/verify
```

This requires changes to:

- `csv-wallet/src/layout.rs` — mode switcher
- `csv-wallet/src/components/sidebar.rs` — conditional navigation
- `csv-wallet/src/routes.rs` — no structural change needed

### 3.5 Component Inventory — What to Keep, What to Fix

| Component | Location | Status | Action |
|-----------|----------|--------|--------|
| `Header` | `components/header.rs` | Good | Keep |
| `Sidebar` | `components/sidebar.rs` | Needs mode switcher | Fix |
| `Card` | `components/card.rs` | Good | Keep |
| `SealStatus` | `components/seal_status.rs` | Good, extend with `--seal-*` tokens | Fix |
| `SealVisualizer` | `components/seal_visualizer.rs` | Good concept | Keep, connect to real data |
| `ProofInspector` | `components/proof_inspector.rs` | Good, connect to validator | Fix |
| `ChainDisplay` | `components/chain_display.rs` | Good | Keep |
| `HashDisplay` | `components/hash_display.rs` | Good | Keep |
| `Onboarding` | `components/onboarding.rs` | Needs real seal creation flow | Fix |
| `Dropdown` | `components/dropdown.rs` | Functional | Keep |

---

## 7. Phase 4 — CSV/Single-Use-Seal Competitive Advantages

**Goal:** Make your unique protocol properties visible and defensible.  
**Timeline:** 4–6 weeks  
**This is your moat.** Traditional bridges hold funds. You don't. Traditional NFT bridges require trust. You don't. Make this VISIBLE.

### 4.1 The Proof Bundle as First-Class UI Object

Currently the `ProofBundle` is created and stored but never shown to users as a standalone artifact they can share, export, or verify independently.

**Add:** `csv-wallet/src/pages/proofs/bundle.rs` (new page)

A proof bundle page that shows:

- Source chain anchor (tx hash, block height, finality status)
- Inclusion proof (Merkle branch, root)
- Finality proof (confirmations, checkpoint, etc.)
- Seal reference consumed (real chain-native ID from BUG-01 fix)
- Commitment hash
- Right ID before and after
- **Export as file** (portable proof, shareable with any counterparty)
- **QR code** of the proof bundle hash

The route: `/proofs/:id/bundle`

This directly exploits the CSV advantage: the receiver can verify this bundle OFFLINE, without trusting the source chain's RPC. A traditional bridge gives you a receipt; you give them a proof.

### 4.2 Commitment Chain Visualizer (Provenance Timeline)

**File:** `csv-wallet/src/pages/rights/journey.rs` (exists, needs wiring)

`commitment_chain.rs` walks the full history. Show it:

```
Genesis Seal (Bitcoin Signet, Block 234,891)
  │ Commitment: 0xa3f8...
  │ Owner: bc1q...
  ▼
Transfer to Ethereum (Block 19,234,100)
  │ Commitment: 0x7c2d...
  │ Old Seal consumed, New Seal opened  
  ▼
Transfer to Sui (Checkpoint 8,234,901)
  │ Commitment: 0x1e9a...
  │ Current Owner: 0x7f3c...
  ● Current State
```

Wire `journey.rs` to `commitment_chain.rs::CommitmentChainWalker`. Pull real data from `state_store.rs`. No mocked data.

**This visualization is your primary competitive story.** Show it on the right detail page and in marketing materials.

### 4.3 Double-Spend Detection Surface

`csv-adapter-core/src/seal_registry.rs` has `CrossChainSealRegistry`. When a double-spend is detected, show it explicitly in the UI.

**Add to `csv-wallet/src/components/seal_status.rs`:**

```rust
SealStatus::DoubleSpent => rsx! {
    div { class: "seal-double-spent",
        // Red, prominent warning
        // Show both transactions that claimed the seal
        // Link to block explorer for both
        // Explain what this means (protocol-level protection)
    }
}
```

**This is a feature, not an error.** A traditional bridge would hide this or silently fail. You surface it and explain it. This is proof that your system detected fraud.

### 4.4 Offline Verification Mode

**Add:** `csv-wallet/src/pages/validate/offline.rs` (new page)

Allow users to paste or upload a `ProofBundle` JSON and verify it completely offline (no RPC calls needed). The verification is:

1. Parse `ProofBundle`
2. `proof_verify.rs::ProofVerifier::verify_bundle(bundle)` — pure cryptographic check
3. Show each step passing/failing
4. Never make an RPC call

**Why this matters:** This is the CSV competitive advantage over bridges. "Your counterparty doesn't need to trust any server. They can verify your right with this file alone."

### 4.5 MPC Tree Wire-Up

**File:** `csv-adapter-core/src/mpc.rs` — types are correct

The MPC tree allows multiple protocols to share one on-chain transaction output. This reduces costs and is a scalability primitive.

Wire it to Bitcoin adapter's commitment publishing:

**File:** `csv-adapter-bitcoin/src/tapret.rs` — add MPC tree support

```rust
// When publishing a commitment for Bitcoin:
// 1. Collect all pending commitments for this Bitcoin block window
// 2. Build MpcTree with one leaf per commitment  
// 3. Publish single tapret with MPC root
// 4. Store MpcProof per commitment for inclusion verification
```

**File:** `csv-adapter-bitcoin/src/proofs.rs` — `build_inclusion_proof` includes the MPC branch

This is not premature optimization. It's infrastructure. The MPC tree directly reduces on-chain costs for users with multiple rights on Bitcoin.

#### MPC Integration Status

##### Current State: Infrastructure Ready

  The MPC (Multi-Protocol Commitment) batching infrastructure is in place and ready for integration. This feature enables multiple CSV commitments to share a single on-chain Bitcoin transaction, reducing costs by 90%+ at scale.

  **Implemented:**

- `csv-adapter-bitcoin/src/mpc_batch.rs` - Core batching infrastructure
- `MpcBatcher` - Queue and batch management
- `MpcTreeExt` - Merkle branch generation for proofs
- `PendingCommitment` - Commitment queuing structure
- Error handling (`MpcError`) in Bitcoin adapter

  **Pending Integration:**
  To complete the MPC integration with `BitcoinAnchorLayer::publish()`, the following steps are required:

  1. **Batcher Instance in BitcoinAnchorLayer**
    - Add `mpc_batcher: MpcBatcher` field to `BitcoinAnchorLayer`
    - Initialize with configurable batch thresholds

  2. **Configuration for Batch Thresholds**
    - `batch_size` - Maximum commitments per batch (default: 10)
    - `min_batch_size` - Minimum before auto-batch (default: 2)
    - `max_wait_seconds` - Timeout for forcing batch (default: 300)

  3. **Timer/Scheduler for Batch Publication**
    - Periodic check for batch readiness
    - Timeout-based forced publication
    - Background task integration

  4. **Integration with Commitment Flow**
    - Modify `publish()` to queue commitments when batching enabled
    - Batch publication path: build MPC tree → publish root → distribute proofs
    - Single-commitment fallback when batching disabled

  **Important:** This is a runtime configuration choice, not a missing feature. Single-commitment publishing works correctly today. Batching is a cost optimization that can be enabled when operational requirements warrant it.

### 4.6 Single-Use Seal UX Vocabulary

Create a consistent vocabulary throughout the wallet:

| Protocol concept | Current UI language | Better language |
|------------------|--------------------|----|
| SealRef consumed | "Transfer complete" | "Seal spent — proof generated" |
| ProofBundle ready | "Proof ready" | "Portable proof ready — share with counterparty" |
| Validator passes | (not shown) | "5/5 validation steps passed" |
| Double-spend detected | (crashes) | "Protocol protection triggered: duplicate claim detected" |
| Offline verification | (not available) | "Verify this proof offline — no server needed" |

This vocabulary teaches users the value of CSV vs bridges without explaining it technically.

---

## 8. Phase 5 — ZK Proofs (Ordered Correctly)

**Timeline:** 8–12 weeks (start ONLY after Phase 1 and Phase 4 are stable)

ZK proofs are NOT premature optimization here — they are a natural extension of the CSV trust model. The current proof bundles require the verifier to trust RPC data. ZK proofs eliminate that requirement entirely.

### 5.1 Correct Ordering

```
Phase 1 complete: Real seals, real commitments, real proof bundles
Phase 4 complete: MPC tree wired, offline verification working
         ↓
Phase 5 start: Replace RPC-trust assumptions with ZK proofs
```

Do NOT start Phase 5 while seals are still fake timestamp strings.

### 5.2 Bitcoin SPV ZK (SP1 or Risc0)

**File:** `csv-adapter-core/src/zk_proof.rs` — types exist, need backend

Target: A `ZkSealProof` that proves a Bitcoin UTXO was spent in block X without requiring the verifier to query any Bitcoin RPC.

**Implementation path:**

1. Choose SP1 (easier developer experience) over Risc0 for initial implementation
2. Write guest program: `csv-adapter-bitcoin/src/sp1_guest/spv.rs`
   - Input: raw Bitcoin transaction + Merkle branch + block header
   - Output: public inputs = `{ seal_id: OutPoint, block_hash: Hash, commitment: Hash }`
3. Wire to `zk_proof.rs::ZkProver::prove_seal`
4. Store `ZkSealProof` alongside `ProofBundle` in `csv-adapter-store`
5. Add `verify_zk_seal` to `csv-wallet/src/pages/proofs/verify.rs`

**This eliminates the RPC trust assumption for Bitcoin**, which is the most important chain for CSV semantics.

### 5.3 Ethereum Groth16 (for EVM compatibility)

SP1 proofs are large (~1MB). For Ethereum, use Groth16 which produces 256-byte proofs verifiable in a Solidity contract.

**Target:** Update `csv-adapter-ethereum/contracts/src/CSVLock.sol` to accept a Groth16 proof as an alternative to SPV-style verification.

### 5.4 AluVM Integration

**File:** `csv-adapter-core/src/vm/aluvm.rs` — stub exists

AluVM provides deterministic execution for contract validation scripts. This enables schema-enforced state transition rules that run client-side, verified by any counterparty.

Wire `AluVmAdapter::execute` to the validator pipeline:

- Schema validation step in `validator.rs` currently does structural checks only
- With AluVM: schema can contain bytecode that enforces custom invariants
- Example: "This right can only be transferred to addresses starting with specific prefix"

Start with `PassthroughVM` for all existing rights (backward compatible), migrate to `AluVmAdapter` for new rights with bytecode schemas.

---

## 9. Phase 6 — Ecosystem Expansion

**Timeline:** After Phase 5, or parallel to Phase 5 for TypeScript SDK

### 6.1 TypeScript SDK (High Priority)

`typescript-sdk/` is mentioned in docs but not present in the repomix. This is a major gap — most DeFi integrators use TypeScript.

**Structure:**

```
typescript-sdk/
  src/
    client.ts          <- CsvClient class (mirrors csv-adapter/src/client.rs)
    seal.ts            <- SealRef types
    right.ts           <- Right types  
    proof.ts           <- ProofBundle types, offline verification
    chains/
      bitcoin.ts
      ethereum.ts
      sui.ts
      aptos.ts
      solana.ts
    verify.ts          <- Offline proof verification (WASM or pure TS)
  tests/
  package.json
```

**Key:** The TypeScript client should be able to verify a `ProofBundle` received from a Rust peer. This requires either:

- WASM compilation of `csv-adapter-core::proof_verify` — best approach
- Pure TypeScript re-implementation of the verification — fallback

### 6.2 MCP Server (AI Agent Integration)

`csv-mcp-server` enables AI agents (Claude, GPT, etc.) to operate CSV rights workflows.

**High-value actions for MCP:**

- `create_seal(chain, value)` — agent creates a seal
- `transfer_right(right_id, destination)` — agent transfers a right
- `verify_proof(bundle_json)` — agent verifies a proof bundle
- `get_rights(address)` — agent lists rights for an address
- `monitor_transfer(transfer_id)` — agent watches transfer status

Build on top of `csv-cli` commands — the MCP server is a thin wrapper that exposes CLI operations as MCP tools.

### 6.3 Explorer Polish

The explorer stack is comprehensive. Polish priorities:

1. `csv-explorer/indexer/src/chain_indexer.rs` — ensure all 5 chain indexers run reliably
2. `csv-explorer/storage/src/schema.sql` — add index on `seal_id` for fast lookup
3. `csv-explorer/api/src/graphql/` — add `sealByChainNativeId` query (cross-chain seal lookup)
4. `csv-explorer/ui/` — replace fake seal_ref strings (BUG-04)

---

## 10. What NOT to Build Yet

These are the premature optimizations to avoid until Phases 1-4 are complete:

| Idea | Why to wait |
|------|-------------|
| More chain adapters (Cosmos, Polkadot, etc.) | 5 chains not all wired correctly yet |
| Layer-2 support (Lightning, Arbitrum) | Base-layer protocol not fully stable |
| Complex DeFi primitives (AMM, lending) | Rights protocol must work first |
| Mobile app | WASM wallet not optimized yet |
| Enterprise features (multi-sig, governance) | Protocol not mature enough |
| RGB protocol full compatibility | Experimental, large scope |
| Additional VM backends (WASM VM) | AluVM not integrated yet |
| Batched cross-chain transfers | Single transfers not fully tested |

The MPC tree (Phase 4.5) is NOT in this list — it's infrastructure that reduces costs and is already partially implemented.

---

## 11. File Ownership Map

### Files Only You Should Touch (Protocol Core)

```
csv-adapter-core/src/seal.rs                ← single-use semantics
csv-adapter-core/src/commitment.rs          ← hash linkage
csv-adapter-core/src/commitment_chain.rs    ← chain verification
csv-adapter-core/src/validator.rs           ← 5-step pipeline
csv-adapter-core/src/proof_verify.rs        ← cryptographic gatekeeper
csv-adapter-core/src/seal_registry.rs       ← double-spend prevention
csv-adapter-core/src/cross_chain.rs         ← lock-and-prove protocol
csv-adapter-core/src/traits.rs              ← AnchorLayer trait contract
csv-adapter-core/src/protocol_version.rs    ← canonical chain IDs/constants
csv-adapter-bitcoin/src/tapret.rs           ← Tapret commitment (Bitcoin-specific)
csv-adapter-bitcoin/src/spv.rs              ← SPV verification
csv-adapter-bitcoin/src/bip341.rs           ← BIP-341 Taproot
```

### Files Junior Devs Can Work On (UI Layer)

```
csv-wallet/src/pages/**                     ← UI pages (wire to services)
csv-wallet/src/components/**                ← UI components
csv-wallet/src/hooks/**                     ← Dioxus hooks
csv-explorer/ui/src/**                      ← Explorer frontend
csv-cli/src/commands/**                     ← CLI command implementations
```

### Files Requiring Careful Review (Integration Layer)

```
csv-wallet/src/services/**                  ← Must use ChainFacade, not raw adapters
csv-wallet/src/seals/**                     ← Must produce real SealRefs after BUG-01 fix
csv-explorer/indexer/src/**                 ← Must handle chain reorgs correctly
csv-adapter/src/facade.rs                   ← Unified facade — changes affect all consumers
csv-adapter/src/client.rs                   ← Main client API — semver-sensitive
```

### New Files to Create (Priority Order)

```
1. csv-wallet/src/pages/proofs/bundle.rs         ← Proof bundle viewer (Phase 4.1)
2. csv-wallet/src/pages/validate/offline.rs      ← Offline verification (Phase 4.4)  
3. csv-adapter-bitcoin/src/mpc_publish.rs        ← MPC tree publishing (Phase 4.5)
4. csv-adapter-core/src/PROTOCOL_INVARIANTS.md   ← Junior dev guardrails (Phase 2.5)
5. CODEBASE_OWNERS.md                            ← Ownership map (Phase 0.4)
6. typescript-sdk/src/client.ts                  ← TS SDK entry (Phase 6.1)
7. csv-adapter-bitcoin/tests/integration.rs      ← Signet integration tests (Phase 1.4)
```

---

## Priority Execution Order

```
Week 1-2:   BUG-01 through BUG-05 + Phase 0 (Stability)
Week 3-4:   Phase 1.1 (Real seals) + Phase 1.2 (Validator wired)
Week 5-6:   Phase 2 (Error unification, state machine, guardrails)
            Phase 3 (Wallet design system, Tailwind fix, navigation)
Week 7-8:   Phase 1.3 (Commitment chain UI) + Phase 4.1 (Proof bundle page)
Week 9-10:  Phase 4.2 (Provenance timeline) + Phase 4.3 (Double-spend surface)
Week 11-12: Phase 4.4 (Offline verify) + Phase 4.5 (MPC tree)
Week 13+:   Phase 5 (ZK proofs) — only after Phase 4 is stable
Week 17+:   Phase 6 (TypeScript SDK, MCP server)
```

---

## The Competitive Advantage Summary

Traditional bridges and NFT solutions:

- Hold funds in custodial contracts
- Require trusting bridge operators
- Cannot prove history without RPC
- Double-spends require chain reorganization to detect

Your system with CSV + Single-Use Seals:

- **No custody** — rights are off-chain state, seals are chain-enforced one-shot locks
- **No trusted bridge** — proof bundles are self-verifying
- **Offline verification** — anyone with the bundle can verify without any server
- **Cryptographic double-spend prevention** — the seal registry plus chain-native single-use enforce it mathematically
- **Cross-chain provenance** — the commitment chain is a tamper-evident audit log no bridge provides

Make these properties **visible** in the wallet. Users who understand them will choose your system over bridges. Users who don't understand them will still benefit from them. That's the goal.
