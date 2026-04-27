# CSV Adapter — Detailed Recovery & Roadmap Plan

**Date:** April 27, 2026  
**Status:** Codebase functional but structurally strained  
**Goal:** Maintainable foundation → DeFi application in Rust (Dioxus wallet) + TypeScript

---

## Part 0: Where You Are Right Now (Honest Diagnosis)

### What Works

- `csv-adapter-core` — solid protocol primitives (seals, rights, commitments, proofs, cross-chain)
- Chain adapters (bitcoin/ethereum/sui/aptos/solana) — RPC integration working
- `csv-cli` — functional for local operations
- `csv-wallet` — Dioxus app renders, chain API balance queries work
- `csv-explorer` — indexer + REST/GraphQL + Dioxus UI

### What Is Broken or Dangerous

| Problem | Location | Severity |
|---|---|---|
| CLI shells out to `forge`, `sui`, `aptos`, `anchor` binaries | `csv-cli/src/commands/contracts.rs` | 🔴 Critical |
| `csv-wallet` reimplements chain interaction from scratch | `csv-wallet/src/services/chain_api.rs`, `blockchain/service.rs` | 🔴 Critical |
| Private keys stored in plaintext JSON | `~/.csv/data/state.json` | 🔴 Critical |
| `wallet.rs` 1283 lines, `contracts.rs` 1343 lines | `csv-cli/src/commands/` | 🟠 High |
| `blockchain/service.rs` 1199 lines | `csv-wallet/src/services/` | 🟠 High |
| Single-use seals invisible in UI — not shown anywhere meaningful | `csv-wallet/src/pages/seals/` | 🟠 High |
| Dead files committed | `dashboard.rs.tmp`, `nft_page_broken.rs` | 🟡 Medium |
| Wallet has no design system — CSS utility classes, no tokens | `csv-wallet/public/style.css` | 🟡 Medium |
| No BIP-39 / HD wallet | `csv-wallet/src/wallet_core.rs`, `csv-cli/src/state.rs` | 🔴 Critical |
| TypeScript SDK: only spec, no code | `docs/planning/PLAN.md` | 🟡 Medium |

### The Core Confusion

Your `csv-cli` and `csv-wallet` both try to interact with chains **independently** of the adapter crates.  
`contracts.rs` calls `forge deploy`, `sui client publish`, `aptos move publish`, `anchor deploy` as subprocesses.  
`chain_api.rs` in the wallet hard-codes raw JSON-RPC calls that already exist in `csv-adapter-{chain}/src/real_rpc.rs`.

This is why adding a new chain requires changes in 5 places instead of 1.

---

## Part 1: Phase 0 — Dead Code Deletion (Do This First, 1 Day)

No logic changes. Just cleanup. This restores confidence in the codebase.

### Delete These Files Immediately

```
csv-wallet/src/pages/dashboard.rs.tmp          # .tmp = committed mistake
csv-wallet/src/pages/nft_page_broken.rs        # explicitly named broken
csv-wallet/src/assets/valuation.rs             # check if used; likely dead
csv-wallet/src/services/native_signer.rs       # superseded by wallet_core.rs
csv-wallet/src/services/sdk_tx.rs              # empty or stub
csv-wallet/src/services/bitcoin_tx.rs          # duplicates adapter logic
csv-wallet/src/services/solana_tx.rs           # duplicates adapter logic
```

### Verify Before Deleting

```bash
# Check if a file is actually imported anywhere
grep -r "native_signer\|sdk_tx\|bitcoin_tx\|solana_tx\|nft_page_broken\|dashboard.rs.tmp" \
  csv-wallet/src/ --include="*.rs" -l
```

### Clean up csv-adapter-core/src/adapters/

`csv-adapter-core/src/adapters/` contains `aptos_adapter.rs`, `bitcoin_adapter.rs`, etc.  
These are mock/stub implementations that shadow the real crates.  
**Decision required**: if `csv-adapter-core/src/adapters/mock.rs` is used in tests, keep only mock.rs.  
Delete the chain-specific ones — they should live in `csv-adapter-{chain}/`, not here.

```bash
# Check what references them
grep -r "use csv_adapter_core::adapters::" . --include="*.rs"
```

### Remove from Cargo workspace if unused

Check `csv-adapter-store` — `unified.rs` is 650 lines implementing a storage layer  
that `csv-wallet` also duplicates in `csv-wallet/src/storage.rs` (another 300 lines).  
Pick one. The wallet should use `csv-adapter-store`.

---

## Part 2: Phase 1 — Fix the Chain Interaction Architecture (1–2 Weeks)

This is the most important phase. Everything else depends on it.

### The Problem in Detail

`csv-cli/src/commands/contracts.rs` (lines 97733–99076) does:

```rust
// THIS IS WRONG — calling external binaries is not maintainable
Command::new("forge").arg("build")...
Command::new(&sui_path).arg("client").arg("publish")...
Command::new(&deploy_script).arg("testnet")...
```

`csv-wallet/src/services/chain_api.rs` does:

```rust
// THIS IS ALSO WRONG — duplicating what real_rpc.rs already does
let url = format!("{}/address/{}", config.api_url, address);
self.client.get(&url).send().await?  // raw HTTP, no abstraction
```

### The Correct Architecture

```
csv-adapter-core/        ← AnchorLayer trait, protocol types only
csv-adapter-{chain}/     ← ONE implementation per chain, including:
  src/real_rpc.rs        ← balance, tx submit, address queries
  src/adapter.rs         ← AnchorLayer impl (seal ops)
  src/deploy.rs          ← NEW: contract deployment via RPC (not CLI)
  contracts/             ← contract source (Move/Solidity/Anchor)
csv-adapter/             ← unified CsvClient, re-exports adapters
csv-cli/                 ← calls csv-adapter crate ONLY, no direct chain IO
csv-wallet/              ← calls csv-adapter crate ONLY, no direct chain IO
```

### Step 2.1: Create `deploy.rs` in Each Adapter

Replace `contracts.rs` subprocess calls with Rust crate calls.

**`csv-adapter-ethereum/src/deploy.rs`** — use `ethers-rs` or `alloy` to deploy:

```rust
pub async fn deploy_csv_lock(
    rpc_url: &str,
    private_key: &str,
    bytecode: &[u8],
) -> Result<H160, EthereumError> {
    // Use ethers::ContractFactory directly
    // No forge subprocess needed
}
```

**`csv-adapter-sui/src/deploy.rs`** — use `sui-sdk`:

```rust
pub async fn publish_csv_package(
    rpc_url: &str,
    compiled_modules: Vec<Vec<u8>>,   // pre-compiled Move bytecode
    signer: &SuiKeyPair,
) -> Result<ObjectID, SuiError> {
    // Use sui_sdk::SuiClientBuilder directly
    // No `sui client publish` subprocess
}
```

**`csv-adapter-aptos/src/deploy.rs`** — use `aptos-sdk`:

```rust
pub async fn publish_csv_module(
    rpc_url: &str,
    module_bytes: Vec<Vec<u8>>,
    signer: &LocalAccount,
) -> Result<HashValue, AptosError> {
    // Use aptos_sdk::rest_client directly
}
```

**`csv-adapter-solana/src/deploy.rs`** — use `anchor-client` or raw `solana-client`:

```rust
pub async fn deploy_csv_program(
    rpc_url: &str,
    program_keypair: &Keypair,
    program_data: &[u8],
) -> Result<Pubkey, SolanaError> {
    // Use solana_client::rpc_client directly
    // No anchor deploy subprocess
}
```

### Step 2.2: Pre-compile Contracts at Build Time

Move contracts should be compiled to bytecode once and embedded.  
Use `build.rs` in each adapter crate.

**`csv-adapter-sui/build.rs`** — extend the existing one:

```rust
// In build.rs: invoke sui Move compiler as build dependency
// Embed compiled bytecode as &[u8] constant
// Never require `sui` CLI at runtime
fn compile_move_packages() {
    // use move_compiler crate directly
    // output to OUT_DIR
}
```

For Ethereum, Solidity compilation can use `solc-rust` or pre-compiled bytecode stored as hex.

**Result:** `csv-cli contracts deploy --chain ethereum` works with zero external tools installed.

### Step 2.3: Refactor `csv-cli/src/commands/contracts.rs`

Before (~1343 lines):

```
contracts.rs
  fn deploy_ethereum() → runs forge, parses stdout
  fn deploy_sui()      → runs sui CLI, parses stdout  
  fn deploy_aptos()    → runs aptos CLI, parses stdout
  fn deploy_solana()   → runs anchor CLI, parses stdout
```

After (target ~200 lines):

```
contracts.rs
  fn deploy_ethereum() → calls csv_adapter_ethereum::deploy::deploy_csv_lock()
  fn deploy_sui()      → calls csv_adapter_sui::deploy::publish_csv_package()
  fn deploy_aptos()    → calls csv_adapter_aptos::deploy::publish_csv_module()
  fn deploy_solana()   → calls csv_adapter_solana::deploy::deploy_csv_program()
```

### Step 2.4: Remove `csv-wallet/src/services/chain_api.rs`

`ChainApi::get_bitcoin_balance()` calls `mempool.space/api/address/{addr}`.  
This already exists in `csv-adapter-bitcoin/src/real_rpc.rs`.

Replace with a thin wrapper calling the adapter:

```
csv-wallet/src/services/
  chain_api.rs   → DELETE
  mod.rs         → expose balance queries via csv-adapter feature flag
```

Add `real-rpc` feature to adapter crates if not already gated:

```toml
# csv-wallet/Cargo.toml
csv-adapter-bitcoin = { path = "../csv-adapter-bitcoin", features = ["real-rpc"] }
csv-adapter-ethereum = { path = "../csv-adapter-ethereum", features = ["real-rpc"] }
```

Then in wallet hooks:

```rust
// csv-wallet/src/hooks/use_balance.rs
// BEFORE: calls chain_api.rs (duplicate HTTP)
// AFTER: calls adapter directly
use csv_adapter_bitcoin::real_rpc::BitcoinRpc;
let balance = BitcoinRpc::new(&rpc_url).get_balance(&address).await?;
```

### Step 2.5: Refactor `csv-wallet/src/services/blockchain/service.rs`

At 1199 lines, `BlockchainService` is a god object. Split:

```
csv-wallet/src/services/blockchain/
  service.rs      → thin orchestrator, max 200 lines
  signer.rs       → NEW: sign tx per chain (extracted from service.rs)
  submitter.rs    → NEW: submit signed tx per chain
  estimator.rs    → NEW: gas/fee estimation per chain
  types.rs        → existing, keep
  config.rs       → existing, keep
  wallet.rs       → existing BrowserWallet, keep
```

Each extracted module should be under 150 lines.

---

## Part 3: Phase 2 — BIP-39 + Encrypted Key Storage (1 Week)

### Current State

- `csv-cli/src/state.rs` (line 83632): `state.json` stores private keys as plaintext hex
- `csv-wallet/src/wallet_core.rs`: `ChainAccount.private_key: String` — plaintext in memory
- No BIP-39 mnemonic generation anywhere

### Step 3.1: Add `csv-adapter-keystore` Crate

Create new crate: `csv-adapter-keystore/`

```
csv-adapter-keystore/
  src/
    lib.rs
    bip39.rs        ← mnemonic gen/restore (use `bip39` crate)
    bip44.rs        ← HD derivation per chain (use `tiny-bip32`)
    keystore.rs     ← AES-256-GCM encrypted file format
    memory.rs       ← zeroize-on-drop key type
  Cargo.toml
```

Key types:

```rust
// csv-adapter-keystore/src/memory.rs
use zeroize::ZeroizeOnDrop;

#[derive(ZeroizeOnDrop)]
pub struct SecretKey(pub(crate) [u8; 32]);
// Never implements Display, Clone, or Serialize
// Only exposes signing operations
```

BIP-44 paths per chain:

```rust
// csv-adapter-keystore/src/bip44.rs
pub fn derivation_path(chain: Chain, account: u32) -> DerivationPath {
    match chain {
        Chain::Bitcoin  => "m/86'/0'/0'/0/0".parse(), // Taproot BIP-86
        Chain::Ethereum => "m/44'/60'/0'/0/0".parse(),
        Chain::Sui      => "m/44'/784'/0'/0/0".parse(),
        Chain::Aptos    => "m/44'/637'/0'/0/0".parse(),
        Chain::Solana   => "m/44'/501'/0'/0/0".parse(),
    }
}
```

Keystore file format (ETH-compatible):

```rust
// csv-adapter-keystore/src/keystore.rs
pub struct KeystoreFile {
    pub version: u8,          // 3
    pub id: Uuid,
    pub crypto: KdfParams,    // scrypt or argon2id
    pub ciphertext: Vec<u8>,  // AES-256-GCM
    pub mac: [u8; 32],
}
```

### Step 3.2: Migrate `csv-cli/src/state.rs`

Current state.json structure stores plaintext keys.  
Migration path:

1. On first run after upgrade: detect old state.json with plaintext keys
2. Prompt for passphrase
3. Encrypt all keys into `~/.csv/keystore/{uuid}.json`
4. Replace state.json key fields with `keystore_ref: String` (UUID)
5. Delete raw key fields from state.json

```rust
// csv-cli/src/state.rs  
// ADD migration fn
pub fn migrate_to_keystore(state: &mut State, passphrase: &str) -> Result<()> {
    for account in &mut state.accounts {
        if account.private_key_hex.is_some() {
            let ks = KeystoreFile::encrypt(
                account.private_key_hex.as_ref().unwrap(),
                passphrase,
            )?;
            ks.save_to(keystore_path(&account.id))?;
            account.keystore_ref = Some(account.id.clone());
            account.private_key_hex = None; // zeroize
        }
    }
    Ok(())
}
```

### Step 3.3: Migrate `csv-wallet/src/wallet_core.rs`

Replace `ChainAccount.private_key: String` with `ChainAccount.keystore_id: String`.  
The wallet **never holds a private key in memory longer than the signing operation**.

```rust
// csv-wallet/src/wallet_core.rs
pub struct ChainAccount {
    pub id: String,
    pub chain: Chain,
    pub name: String,
    pub keystore_id: String,    // ← replaces private_key: String
    pub address: String,
    #[serde(default, skip_serializing)]
    pub balance: f64,
}
```

Signing flow:

```
User action → unlock keystore (passphrase or device PIN)
           → derive SecretKey (zeroize-on-drop)  
           → sign tx in memory
           → SecretKey dropped immediately
           → submit signed tx
```

For browser wallet (Dioxus/WASM), use the Web Crypto API via `web_sys` for AES-GCM,  
avoiding storing keys in localStorage (current approach) or sessionStorage.

---

## Part 4: Phase 3 — Make CSV Primitives First-Class in UI (1–2 Weeks)

This is the philosophical heart of the project. **Single-use seals and client-side validation are the product.**  
Currently they are buried or invisible. This phase makes them visible, understandable, and delightful.

### Current State of Seal/Proof UI

`csv-wallet/src/pages/seals/list.rs` — exists but shows raw hex IDs  
`csv-wallet/src/pages/seals/verify.rs` — exists, functional but no visual feedback  
`csv-wallet/src/pages/proofs/` — exists but no clear visual state machine  

### Step 4.1: Seal Lifecycle Visualizer Component

Create `csv-wallet/src/components/seal_lifecycle.rs`:

A component that shows a seal's current state with a visual state machine:

```
[OPEN] ──lock──► [LOCKED: waiting N confirmations] ──proven──► [CONSUMED: proof portable]
  │                                                                      │
  └─ chain badge (Bitcoin/Sui/etc)                         └─ cross-chain proof bundle
```

States to display clearly:

- `SealState::Open` — green dot, "Available to spend"
- `SealState::Locked { confirmations, required }` — orange, progress bar
- `SealState::Consumed { tx_hash, proof_bundle }` — checkmark, "Proof ready"
- `SealState::Spent` — grey, "Single-use exhausted"

The component should make it viscerally obvious: **this seal can only be used once**.

### Step 4.2: Proof Portability Panel

Create `csv-wallet/src/components/proof_inspector.rs`:

When a user has a proof bundle, show:

```
╔═══════════════════════════════════╗
║  PROOF BUNDLE — Portable          ║
║  Right: abc...123                 ║
║  Source: Bitcoin Signet           ║
║  Type:   SPV Merkle + Tapret      ║
║  Size:   212 bytes                ║
║  ┌─────────────────────────────┐  ║
║  │ ✓ Seal consumed at block    │  ║
║  │   825,142                   │  ║
║  │ ✓ Merkle inclusion proven   │  ║
║  │ ✓ Taproot commitment valid  │  ║
║  └─────────────────────────────┘  ║
║  [Verify Locally] [Export] [Send] ║
╚═══════════════════════════════════╝
```

`csv-wallet/src/pages/proofs/verify.rs` already exists — extend it with this visual.

### Step 4.3: CSV Concept Onboarding Flow

Create `csv-wallet/src/pages/onboarding/mod.rs` (new):

Three-screen flow shown once on first run:

**Screen 1: "What is a Right?"**  
> A Right is a cryptographic claim you own something.  
> Unlike tokens, it can't be copied. It lives on exactly one chain at a time.

**Screen 2: "What is a Single-Use Seal?"**  
> The seal is the lock. When you transfer a Right, the seal is consumed —  
> destroyed on the source chain. Only one valid proof can ever exist.

**Screen 3: "Client-Side Validation"**  
> You verify proofs yourself. No bridge. No oracle. No trust.  
> The math runs on your machine.

This lives in `csv-wallet/src/pages/onboarding/` and is shown if `state.onboarded == false`.

### Step 4.4: Seal consumption in cross-chain transfer UI

`csv-wallet/src/pages/cross_chain/transfer.rs` — extend with step-by-step visualization:

```
Step 1: [🔒] Locking seal on Bitcoin Signet
         Transaction: txid abc...
         Waiting for 2/6 confirmations ████░░░░░░

Step 2: [📋] Generating proof bundle  
         Merkle path: 7 hops
         Tapret commitment: verified

Step 3: [✈️] Submitting to Sui Testnet
         Gas estimate: 5,000,000 MIST

Step 4: [✓] Seal consumed. Right transferred.
         Proof portable to any chain.
```

This is NOT a new feature — just making the existing multi-step process visible.

### Step 4.5: Right Detail Page Enhancement

`csv-wallet/src/pages/rights/show.rs` — add:

- Commitment hash display with "What's this?" tooltip explaining client-side validation
- Commitment chain history (all previous states)
- "Verify this Right" button that runs local proof verification
- QR code for sharing Right ID for cross-chain reception

---

## Part 5: Phase 4 — Split the Large Files (1 Week)

Target: no file over 400 lines. Most files under 200 lines.

### `csv-cli/src/commands/wallet.rs` (1283 lines → split to 4 files)

```
csv-cli/src/commands/wallet/
  mod.rs          ← command definitions + dispatch, ~100 lines
  accounts.rs     ← create, list, show, delete accounts, ~250 lines
  keys.rs         ← import, export, migrate, sign, ~250 lines
  recovery.rs     ← mnemonic gen/restore, backup, ~200 lines
  balances.rs     ← balance queries, faucet, ~150 lines
```

Dispatch in `mod.rs`:

```rust
pub fn execute(action: WalletAction, ...) -> Result<()> {
    match action {
        WalletAction::Create { .. }  => accounts::cmd_create(..),
        WalletAction::Import { .. }  => keys::cmd_import(..),
        WalletAction::Mnemonic { .. }=> recovery::cmd_mnemonic(..),
        WalletAction::Balance { .. } => balances::cmd_balance(..),
        // ...
    }
}
```

### `csv-cli/src/commands/contracts.rs` (1343 lines → split)

```
csv-cli/src/commands/contracts/
  mod.rs          ← command defs + dispatch, ~80 lines
  deploy.rs       ← calls adapter deploy fns (Phase 1), ~150 lines
  verify.rs       ← on-chain contract verification, ~100 lines
  status.rs       ← deployed contract status, ~80 lines
```

### `csv-wallet/src/services/blockchain/service.rs` (1199 lines → split)

Already described in Phase 1:

```
service.rs      → ~200 lines (orchestrator)
signer.rs       → ~200 lines (sign per chain)
submitter.rs    → ~150 lines (submit + retry)
estimator.rs    → ~100 lines (gas estimation)
```

### `csv-adapter-store/src/unified.rs` (650 lines → split)

```
csv-adapter-store/src/
  unified.rs      → remaining shared types, ~100 lines
  rights_store.rs → Right CRUD, ~150 lines
  seals_store.rs  → Seal registry CRUD, ~150 lines
  transfers_store.rs → Transfer records, ~150 lines
  proofs_store.rs → Proof bundle storage, ~100 lines
```

---

## Part 6: Phase 5 — Wallet Design System (Parallel to Phases 1–4)

**Note:** You said you're not a designer. This is an opinionated prescription, not a discussion.

### The Design Direction: "Cryptographic Minimalism"

The wallet handles cryptographic primitives. The design should feel like a terminal that grew into a UI — not a consumer app. Think: **monospace font for addresses, stark color for seal states, dense information, zero decorative chrome.**

Reference aesthetic: Vercel CLI → became Vercel dashboard. Functional, typographic, dark.

### Step 6.1: Design Tokens (add to `csv-wallet/public/style.css`)

Extend the existing `style.css` with a proper token layer at the top:

```css
:root {
  /* Protocol colors — each maps to a concept, not a chain */
  --color-seal-open:     #22c55e;   /* green — available */
  --color-seal-locked:   #f59e0b;   /* amber — in-flight */
  --color-seal-consumed: #94a3b8;   /* slate — spent */
  --color-seal-error:    #ef4444;   /* red — failed */

  /* Chain accent colors */
  --color-bitcoin:  #f7931a;
  --color-ethereum: #627eea;
  --color-sui:      #4da2ff;
  --color-aptos:    #2dd8a3;
  --color-solana:   #9945ff;

  /* Typography */
  --font-mono: 'JetBrains Mono', 'Fira Code', monospace;  /* for addresses, hashes */
  --font-body: 'IBM Plex Sans', system-ui, sans-serif;    /* for text */

  /* Surfaces */
  --surface-0: #09090b;  /* background */
  --surface-1: #18181b;  /* card */
  --surface-2: #27272a;  /* input, hover */
  --surface-3: #3f3f46;  /* border */

  /* Spacing scale */
  --space-1: 4px;
  --space-2: 8px;
  --space-3: 12px;
  --space-4: 16px;
  --space-6: 24px;
  --space-8: 32px;
}
```

### Step 6.2: Hash/Address Display Component

Create `csv-wallet/src/components/hash_display.rs`:

```rust
// Shows a 32-byte hash as: abc1...f93d [copy]
// Uses --font-mono, truncates middle, full on hover
#[component]
pub fn HashDisplay(hash: String, label: Option<String>) -> Element { ... }
```

This one component should replace every place the wallet shows raw hex.  
Currently addresses and hashes are shown as raw strings everywhere — this is the #1 UX fix.

### Step 6.3: Chain Badge Component

`csv-wallet/src/components/chain_badge.rs` already exists — ensure it uses `--color-{chain}` tokens.

### Step 6.4: Proof Visual Component

Described in Phase 3 Step 4.2. Use `--color-seal-*` tokens throughout.

### Step 6.5: Sidebar / Navigation Redesign

`csv-wallet/src/components/sidebar.rs` — current sidebar has standard nav links.  
Reorganize around CSV concepts, not generic wallet concepts:

```
── RIGHTS         (was: Assets/NFTs)
── SEALS          (was: buried)
── PROOFS         (was: buried under Validate)
── TRANSFERS      (cross-chain)
── VALIDATE       (keep)
── WALLET         (keys, accounts)
── SETTINGS
```

The user should immediately understand this is a Rights wallet, not a token wallet.

---

## Part 7: Phase 6 — DeFi Application (After Phases 1–4 Complete)

Per `docs/planning/PLAN.md` Section 10, start with **Cross-Chain DEX (HTLS)** — most elegant demonstration of the CSV primitive.

### Target: `csv-app-dex/` (New Crate)

```
csv-app-dex/
  src/
    lib.rs
    htls.rs          ← Hash Time-Locked Swap protocol
    order.rs         ← Order types (limit, market)
    matching.rs      ← Off-chain order matching
    settlement.rs    ← On-chain settlement via CSV rights
    proofs.rs        ← Cross-chain swap proof bundles
  contracts/
    ethereum/        ← Solidity HTL contract
    sui/             ← Move HTL module
    aptos/           ← Move HTL module
  Cargo.toml
```

### HTLS Protocol Implementation

Atomic cross-chain swap:

```rust
// csv-app-dex/src/htls.rs

pub struct HashTimeLockSwap {
    pub swap_id: Hash,
    pub party_a: SwapParty,      // offers Right on chain X
    pub party_b: SwapParty,      // offers Right on chain Y
    pub preimage_hash: Hash,     // sha256(secret)
    pub timeout: Duration,       // if expired, both refund
}

pub struct SwapParty {
    pub chain: Chain,
    pub right_id: RightId,
    pub lock_address: String,    // CSV contract address on chain
    pub owner: String,
}

// Step 1: A locks Right on chain X with preimage_hash
pub async fn party_a_lock(swap: &HashTimeLockSwap, signer: &SecretKey) 
    -> Result<LockProof>;

// Step 2: B locks Right on chain Y with same preimage_hash
pub async fn party_b_lock(swap: &HashTimeLockSwap, signer: &SecretKey) 
    -> Result<LockProof>;

// Step 3: A reveals preimage on chain Y → claims B's Right
// B sees preimage on chain Y → claims A's Right on chain X
pub async fn claim(
    lock_proof: &LockProof, 
    preimage: &[u8; 32],
    signer: &SecretKey,
) -> Result<ClaimProof>;
```

### DEX UI in `csv-wallet`

Add to `csv-wallet/src/pages/`:

```
dex/
  mod.rs
  swap.rs          ← swap form (Right A on chain X ↔ Right B on chain Y)
  orderbook.rs     ← order book display
  history.rs       ← swap history
```

The DEX page should show the HTLS protocol steps visually — exactly like the cross-chain transfer UI in Phase 3.

---

## Part 8: Phase 7 — TypeScript SDK (Parallel to Phase 6)

Per `docs/planning/PLAN.md` Section 7, this is labeled Q1 2026 priority.

### Create `csv-sdk-ts/` at workspace root

```
csv-sdk-ts/
  src/
    index.ts
    client.ts        ← CsvClient class
    rights.ts        ← Right lifecycle
    seals.ts         ← Seal operations (new — make seals first-class)
    transfers.ts     ← Cross-chain transfers
    proofs.ts        ← Proof generation + verification (WASM calls)
    wallet.ts        ← Key management (uses csv-adapter-keystore via WASM)
    chains/
      bitcoin.ts
      ethereum.ts
      sui.ts
      aptos.ts
      solana.ts
    errors.ts        ← Typed error classes
    types.ts
  wasm/              ← WASM bindings generated from csv-adapter-core
  test/
  package.json
  tsconfig.json
```

### WASM Bridge Strategy

Use `wasm-bindgen` to expose `csv-adapter-core` to TypeScript:

```rust
// csv-adapter-core/src/wasm.rs (new, feature-gated)
#[cfg(feature = "wasm")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "wasm")]
#[wasm_bindgen]
pub struct WasmCsvClient { inner: CsvClient }

#[cfg(feature = "wasm")]
#[wasm_bindgen]
impl WasmCsvClient {
    pub fn verify_proof(&self, proof_bytes: &[u8]) -> bool { ... }
    pub fn create_commitment(&self, data: &[u8]) -> Vec<u8> { ... }
}
```

Chain-specific RPC calls (balance, submit tx) stay in TypeScript — they're async HTTP  
and don't benefit from WASM. Only cryptography (hash, sign, verify) goes through WASM.

---

## Part 9: Priority Matrix — What to Do First

The PLAN.md and Temporary.md have 30+ items. Here is the true priority, based on code state:

| Priority | Task | Why First |
|---|---|---|
| **P0** | Phase 0: Delete dead files | Zero risk, instant sanity |
| **P0** | Phase 1: Fix `contracts.rs` subprocess calls | Blocks any chain deploy workflow |
| **P0** | Phase 2: BIP-39 + encrypted keys | Security hole — must close |
| **P1** | Phase 1: Remove `chain_api.rs` duplication | Causes divergence bugs |
| **P1** | Phase 4: Split large files | Makes junior dev contribution possible |
| **P1** | Phase 3: Seal/proof visibility in UI | This IS the product |
| **P2** | Phase 5: Design system tokens | Fast, high impact |
| **P2** | Phase 6: Cross-Chain DEX (HTLS) | First real DeFi app |
| **P3** | Phase 7: TypeScript SDK | Needs stable Rust API first |
| **P3** | `csv-adapter-keystore` crate publish | After BIP-39 implementation |

**Do NOT start** DeFi app, TypeScript SDK, ZK proofs, or browser extension  
until Phases 0–2 are done. They will inherit the architecture bugs.

---

## Part 10: Files to Touch in Priority Order

### Week 1 (Phase 0 + start Phase 1)

```
DELETE:  csv-wallet/src/pages/dashboard.rs.tmp
DELETE:  csv-wallet/src/pages/nft_page_broken.rs
DELETE:  csv-wallet/src/services/native_signer.rs
DELETE:  csv-wallet/src/services/bitcoin_tx.rs
DELETE:  csv-wallet/src/services/solana_tx.rs
CREATE:  csv-adapter-ethereum/src/deploy.rs
CREATE:  csv-adapter-sui/src/deploy.rs
CREATE:  csv-adapter-aptos/src/deploy.rs
CREATE:  csv-adapter-solana/src/deploy.rs
MODIFY:  csv-cli/src/commands/contracts.rs → call adapter deploy fns
```

### Week 2 (Phase 1 continued + Phase 2 start)

```
DELETE:  csv-wallet/src/services/chain_api.rs
MODIFY:  csv-wallet/src/hooks/use_balance.rs → use adapter real_rpc
SPLIT:   csv-wallet/src/services/blockchain/service.rs → 4 files
CREATE:  csv-adapter-keystore/ (new crate)
MODIFY:  Cargo.toml → add csv-adapter-keystore to workspace
```

### Week 3 (Phase 2 continued + Phase 3 start)

```
MODIFY:  csv-wallet/src/wallet_core.rs → remove private_key: String
MODIFY:  csv-cli/src/state.rs → add keystore migration
CREATE:  csv-wallet/src/components/seal_lifecycle.rs
CREATE:  csv-wallet/src/components/proof_inspector.rs
CREATE:  csv-wallet/src/components/hash_display.rs
MODIFY:  csv-wallet/src/pages/seals/list.rs → use SealLifecycle component
```

### Week 4 (Phase 4 + Phase 5)

```
SPLIT:   csv-cli/src/commands/wallet.rs → wallet/ directory
SPLIT:   csv-cli/src/commands/contracts.rs → contracts/ directory
SPLIT:   csv-adapter-store/src/unified.rs → 5 focused files
MODIFY:  csv-wallet/public/style.css → add design token block
CREATE:  csv-wallet/src/components/chain_badge.rs (update)
MODIFY:  csv-wallet/src/components/sidebar.rs → CSV-concept nav
```

---

## Part 11: Architectural Rules Going Forward

These prevent the current situation from recurring.

### Rule 1: No `std::process::Command` for chain interaction

CLI tools interact with chains exclusively through `csv-adapter-{chain}` crates.  
If a chain operation isn't in an adapter crate, it doesn't exist.

### Rule 2: No raw HTTP to chain RPCs in `csv-wallet`

All RPC calls go through the adapter's `real_rpc.rs` module.  
The wallet makes zero direct HTTP calls to chain endpoints.

### Rule 3: Private keys never touch `String` or `Vec<u8>` in production paths

All key material uses `SecretKey(pub(crate) [u8; 32])` with `zeroize::ZeroizeOnDrop`.

### Rule 4: New chain = new `csv-adapter-{chain}` crate only

Adding Cosmos/Polkadot does not touch `csv-cli`, `csv-wallet`, or `csv-adapter-core`.  
Only `csv-adapter/Cargo.toml` gains a new optional dependency.

### Rule 5: UI components must not contain business logic

Dioxus components: display only. All state lives in context (`csv-wallet/src/context/`).  
All chain operations live in services (`csv-wallet/src/services/`).  
Services call adapter crates. Never the other way.

### Rule 6: File size limit — 400 lines

Any file over 400 lines must be split before merging. No exceptions.

---

## Appendix: Answering Your Specific Questions

### "Where am I in the plan?"

Honest answer: You are at **Phase 0 of the real work**.  
The BLUEPRINT.md says "COMPLETED" for most workstreams, but that reflects prototype-level completion.  
Production-level work on security (BIP-39), chain interaction (no subprocess), and code organization has not started.

The Temporary.md items (DeFi apps, TypeScript SDK, ZK proofs) are all **Q1 aspirations** that cannot start until the foundation issues are resolved.

Current completion by category:

- Protocol core (`csv-adapter-core`): **85%** — solid, some experimental modules
- Chain adapters (RPC integration): **70%** — working but not unified with wallet
- CLI (functional): **60%** — works but shells out, needs split
- Wallet (functional): **50%** — works but security gaps, design debt
- DeFi application: **0%** — not started
- TypeScript SDK: **0%** — spec only

### "What about Single-Use Seals in the UI?"

Currently seals appear in `pages/seals/list.rs` as a list of hex IDs.  
Proofs appear in `pages/proofs/` as raw verification results.

The concept of **single-use** is never surfaced visually. There is no indicator that says  
"this seal can only be consumed once" or "this proof proves exactly one consumption happened."

Phase 3 in this plan (Steps 4.1–4.5) addresses this entirely. The seal lifecycle visualizer  
component is the single highest-impact UX change you can make without adding features.

### "Is the architecture scalable for plug-and-play chains?"

The architecture **intends** to be plug-and-play via `AnchorLayer` trait.  
It is **not currently** because `csv-cli/src/commands/contracts.rs` has chain-specific subprocess logic,  
and `csv-wallet/src/services/blockchain/service.rs` has chain-specific match arms for HTTP calls.

After Phase 1, adding Cosmos as a chain means:

1. Create `csv-adapter-cosmos/` with `AnchorLayer` impl
2. Add `csv-adapter-cosmos = { optional = true }` to `csv-adapter/Cargo.toml`
3. Zero changes to `csv-cli` or `csv-wallet`

That is the actual plug-and-play goal. It requires Phase 1 first.

---

*This document should replace `docs/planning/BLUEPRINT.md` and `docs/planning/Temporary.md`.*  
*Temporary.md aspirations are still valid — they belong in Phase 6+ after foundation work.*
