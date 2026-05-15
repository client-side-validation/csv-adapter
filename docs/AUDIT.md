# CSV Protocol — Full Codebase Audit

**Date**: May 2026 | **Scope**: All chain adapters, contracts, core, SDK, CLI  
**Basis**: Full repomix snapshot, all `backend.rs`, `ops.rs`, `seal_protocol.rs`, contracts

---

## Table of Contents

1. [Async Runtime Panic — Bitcoin & Solana](#1-async-runtime-panic--bitcoin--solana)
2. [Contract Audit — Compatibility & Deployment](#2-contract-audit--compatibility--deployment)
3. [Bugs, Stubs & Critical Placeholders](#3-bugs-stubs--critical-placeholders)
4. [Duplicate Implementations](#4-duplicate-implementations)

---

## 1. Async Runtime Panic — Bitcoin & Solana

### Root Cause

`csv-bitcoin/src/mempool_rpc.rs` and the Solana RPC stack use `reqwest::blocking::Client`, which internally creates its own Tokio runtime. When the client is called (or dropped) inside an already-running async runtime (e.g., the CLI's `#[tokio::main]`), Tokio panics with "cannot start a runtime from within a runtime".

Ethereum uses `reqwest::Client` (async). Sui uses async RPC. Bitcoin and Solana do not.

### Evidence

```rust
// csv-bitcoin/src/mempool_rpc.rs
use reqwest::blocking::Client;
use std::thread;

pub struct MempoolSignetRpc {
    client: Client,   // ← blocking client
    ...
}
```

The `BitcoinRpc` trait is entirely synchronous:

```rust
pub trait BitcoinRpc: Send + Sync {
    fn get_block_count(&self) -> Result<u64, ...>;
    fn send_raw_transaction(&self, tx: Vec<u8>) -> Result<[u8;32], ...>;
    // all methods are fn, not async fn
}
```

### Fix Plan

**Step 1 — Make `BitcoinRpc` trait async.**

```rust
#[async_trait]
pub trait BitcoinRpc: Send + Sync {
    async fn get_block_count(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;
    async fn get_block_hash(&self, height: u64) -> Result<[u8;32], ...>;
    async fn is_utxo_unspent(&self, txid: [u8;32], vout: u32) -> Result<bool, ...>;
    async fn send_raw_transaction(&self, tx: Vec<u8>) -> Result<[u8;32], ...>;
    async fn get_tx_confirmations(&self, txid: [u8;32]) -> Result<u64, ...>;
    async fn get_utxos_for_address(&self, address: &str) -> Result<Vec<UtxoInfo>, ...>;
    fn clone_boxed(&self) -> Box<dyn BitcoinRpc + Send + Sync>;
}
```

**Step 2 — Replace `reqwest::blocking::Client` in `MempoolSignetRpc`.**

```rust
use reqwest::Client;  // async

pub struct MempoolSignetRpc {
    client: Client,   // ← async client
    base_url: String,
}
```

Convert all methods to `async fn` and use `.await`. The retry helpers `get_with_retry`, `get_text_with_retry`, `post_text_with_retry` become `async fn` with `tokio::time::sleep` instead of `std::thread::sleep`.

**Step 3 — Update all callers to `.await` the RPC calls.**

Every call site in `csv-bitcoin/src/ops.rs`, `seal_protocol.rs`, and `backend.rs` that calls `self.rpc.*` must add `.await`. Because `BitcoinRpc` trait methods are called from `SealProtocol` (which is synchronous), use `spawn_blocking_async` pattern identical to what Sui already does:

```rust
fn spawn_blocking_async<F, T>(future: F) -> Result<T, BitcoinError>
where
    F: Future<Output = Result<T, BitcoinError>> + Send + 'static,
    T: Send + 'static,
{
    std::thread::spawn(|| {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all().build()?;
        rt.block_on(future)
    }).join().map_err(|_| BitcoinError::Rpc("Thread panicked".into()))
    .and_then(|r| r)
}
```

**Does this violate clean architecture?** No. The architecture already mandates async chain adapters (`ChainDriver`, `ChainBackend`, all use `#[async_trait]`). Making `BitcoinRpc` async is bringing Bitcoin into alignment with the existing pattern, not deviating from it. The `SealProtocol` sync boundary is handled with the `spawn_blocking_async` helper — the same approach Sui and Aptos use for their synchronous `SealProtocol::publish()` impl.

**Solana** has the same problem. The `SolanaRpc` trait should be audited; ensure all RPC implementations use async reqwest or the Solana SDK's async client.

---

## 2. Contract Audit — Compatibility & Deployment

### 2.1 Ethereum (Reference — Mature)

**Status: Deployed ✅ — reference implementation.**

- `CSVLock.sol` + `CSVMint.sol` deployed to Sepolia via `deploy.sh` / Foundry.
- Alloy bindings generated via `build.rs`, consumed in `csv-ethereum/src/ops.rs`.
- `lock_sanad`, `mint_sanad`, `refund_sanad` in `ChainSanadOps` call the Alloy-generated `CsvLockClient` / `CsvMintClient` directly.
- `EthereumConfig` carries `lock_contract_address` and `mint_contract_address`.
- `DEPLOYMENT.md` documents the full flow.

### 2.2 Solana — Compatibility Issues

**Contract status**: Anchor program `csv-seal` has a complete instruction set (`create_seal`, `consume_seal`, `lock_sanad`, `mint_sanad`, `refund_sanad`, `record_sanad_metadata`, `transfer_sanad`, `register_nullifier`). Program ID `CCMF6BvAyTPNJAPtGMVJAR652Hv9VPy9NmVdgC9969dj`.

**Bugs in the contract state (`state.rs`)**:

| Struct | Declared SIZE | Missing |
|--------|--------------|---------|
| `SanadAccount` | `32+32+32+32+32+1+32+32+1+32+1+1+8+1 = 259` | 8-byte discriminator |
| `LockAccount` | `LockRecord::SIZE + 1` | 8-byte discriminator |
| `LockRegistry` | `32+4+4+1 = 41` | 8-byte discriminator |

All Anchor accounts require 8 bytes for the discriminator. Every `SIZE` constant must add `+ 8`. When `#[account(init, space = SanadAccount::SIZE + 8)]` is used in account constraints, the `+ 8` is added again externally — but the constant comment says "8 (discriminator) + ..." and then fails to include it, causing confusion. **Fix**: include `8` in every `SIZE` constant and remove the external `+ 8` at the call site, or document clearly which convention is used.

**Disconnect between Rust and program**:

- `csv-solana/src/seal_protocol.rs` `create_seal` uses the Solana **system program** to transfer lamports and create an account — it does NOT call the Anchor program's `create_seal` instruction. The two are completely different code paths.
- `csv-solana/src/ops.rs` `lock_sanad` builds a raw system-program instruction, not the Anchor program's `lock_sanad` instruction.
- The Anchor program uses `keccak256` for Merkle proof verification (`solana_program::keccak::hashv`), but `csv-core` uses SHA-256/SHA-256d. Cross-chain proof verification will produce different leaf hashes.

**Deployment gap**: `deploy.sh` runs `anchor build` and `anchor deploy` then calls `anchor run initialize` to set up the `LockRegistry`. However, the Rust code never calls `initialize_registry` — it treats the account as always present. A deployment without running `initialize_registry` will cause `lock_sanad` to fail on-chain with account-not-found.

**Action items for Solana**:

1. Fix `SIZE` constants (add 8 bytes each).
2. Make `SolanaSealProtocol::create_seal` call the Anchor program's `create_seal` instruction (build proper Anchor instruction discriminator: `sha256("global:create_seal")[..8]`).
3. Align Merkle hash algorithm: use `keccak256` everywhere, or switch the Anchor program to SHA-256.
4. Add `initialize_registry` call to `deploy.sh` post-deploy step and document in README.
5. Write Rust IDL bindings (via `anchor-gen` or manual) equivalent to Ethereum's Alloy bindings so `ops.rs` can call typed instructions instead of raw system transfers.

### 2.3 Aptos — Significant Gaps

**Contract status**: `csv_seal.move` implements `create_seal`, `consume_seal`, `transfer_seal` (deprecated), and read-only queries. Deployed to testnet per `deploy-output-testnet.txt`.

**Missing cross-chain instructions**: The Move module has no `lock_sanad`, `mint_sanad`, or `refund_sanad` entry functions. The Rust `ChainSanadOps::lock_sanad`, `mint_sanad` implementations in `csv-aptos/src/ops.rs` build raw BCS payloads and call `submit_transaction` — but there is nothing on-chain to receive them. These calls will succeed (the transaction will be accepted) but produce no meaningful state change.

**Event handle address mismatch**: `consume_seal` emits to `borrow_global_mut<AnchorEventHandle>(@csv_seal)`. `initialize_module` stores the handle at `signer::address_of(account)`. If `initialize_module` is called by any account other than `@csv_seal`, the `consume_seal` call will abort with `EMissingData`. The `deploy_testnet.sh` must ensure `initialize_module` is called by the module publisher key.

**Wrong error code in `create_seal`**: `assert!(!exists<Seal>(addr), EAnchorDataExists)` — the error code `EAnchorDataExists` (3) is semantically wrong for a missing/duplicate seal check. Should be a dedicated `ESealAlreadyExists` or reuse `ESealAlreadyConsumed` (1) with a new code.

**`transfer_seal` is deprecated but callable**: The `#[deprecated]` attribute in Move is advisory only; clients can still call it. Add an `assert!(false, EDeprecated)` body guard, or remove the function and bump the module version.

**Compatibility with Ethereum seal model**: The Ethereum model uses a single contract address + storage slot as the seal. The Aptos model uses one resource per address (only one seal per account). For multi-seal workflows, a `Table<u64, Seal>` or vector-based collection is needed — similar to what the code comment calls "collection-based variant."

**Action items for Aptos**:

1. Add `lock_sanad`, `mint_sanad`, `refund_sanad` entry functions to the Move module.
2. Guard `transfer_seal` with `abort EDeprecated` or remove it.
3. Fix `create_seal` error code.
4. Ensure `initialize_module` is idempotent or gated to the module publisher.
5. Wire Rust `ops.rs` lock/mint/refund to the new Move entry functions using `aptos-sdk` typed payloads.

### 2.4 Sui — Mostly Compatible, Needs Wiring

**Contract**: No Move package is present in the repo (`csv-contracts/` has only `aptos/`, `ethereum/`, `solana/`). The `csv-sui/src/deploy.rs` has a `PackageDeployer` that publishes bytecode, and `SuiSealProtocol` assumes a `seal_contract.package_id` in config. The actual Move source for the Sui CSV seal package is missing from the repo.

**Seal model alignment**: Sui's owned-object model is architecturally the closest to Bitcoin's UTXO/single-use-seal model — both enforce single consumption by making a resource un-spendable after use. This is the correct approach. The `SuiSealProtocol::enforce_seal` correctly checks on-chain object existence.

**Transaction building**: `build_and_sign_move_call` in `seal_protocol.rs` manually constructs BCS-encoded `TransactionData` without the `sui-sdk`. The comment acknowledges this is fragile. The implementation uses hardcoded version `1` for the seal object and zeroed digest — this will be rejected by a real Sui node (`ObjectArg::ImmOrOwnedObject` requires the actual object version and digest from the chain).

**Action items for Sui**:

1. Add the Move source package to `csv-contracts/sui/`.
2. Add `deploy.sh` equivalent to Ethereum/Solana.
3. Fix `build_and_sign_move_call`: fetch real object version and digest before building `TransactionData`.
4. Populate `seal_contract.package_id` in `chains/sui.toml`.

### 2.5 Cross-Contract Compatibility Summary

| Feature | Ethereum | Solana | Aptos | Sui |
|---------|----------|--------|-------|-----|
| Seal creation | ✅ contract slot | ⚠️ system acct, not Anchor | ⚠️ one per addr | ⚠️ object, no Move source |
| Seal consumption | ✅ `markSealUsed` event | ✅ `consume_seal` ix | ✅ `consume_seal` | ✅ object deletion |
| Lock sanad | ✅ `CSVLock.lockSanad` | ✅ Anchor `lock_sanad` | 🔴 no Move fn | ⚠️ placeholder sig |
| Mint sanad | ✅ `CSVMint.mintSanad` | ✅ Anchor `mint_sanad` | 🔴 no Move fn | ⚠️ placeholder sig |
| Refund sanad | ✅ `refundSanad` | ✅ Anchor `refund_sanad` | 🔴 no Move fn | 🔴 stub only |
| Merkle hash | keccak256 | keccak256 ✅ | not implemented | SHA-256 |
| Deploy script | ✅ Foundry | ✅ Anchor | ✅ Aptos CLI | 🔴 missing |
| Config field | `lock_contract_address` | program ID in toml | `module_address` | `package_id` |

**Cross-chain proof compatibility**: The `verify_cross_chain_proof` in Solana uses `keccak256`. Ethereum proofs use keccak256 (EVM). Sui uses SHA-256 internally. Aptos uses SHA3-256. The `csv-core` cross-chain proof verifier must use a single agreed hash function or carry the hash algorithm in the proof envelope.

---

## 3. Bugs, Stubs & Critical Placeholders

### 3.1 Bitcoin

**BUG-BTC-01 — `sign_transaction` returns empty bytes**
`csv-bitcoin/src/backend.rs`, `BitcoinWallet::sign_transaction`:

```rust
async fn sign_transaction(&self, _data: &[u8]) -> ChainResult<Vec<u8>> {
    Ok(vec![])  // ← critical: always returns empty signature
}
```

Any code path that calls `wallet.sign_transaction()` will receive an empty vec and silently fail to produce a valid transaction. Fix: implement actual Taproot/SegWit signing via `secp256k1` using the wallet's private key.

**BUG-BTC-02 — `verify_signature` always returns false**

```rust
fn verify_signature(&self, data: &[u8], signature: &[u8]) -> bool {
    // Note: This would need the actual public key from the wallet
    // For now, return false as we need the pubkey to verify
    false
}
```

Fix: store the verifying key alongside the address in `BitcoinWallet`, then use `secp256k1::Secp256k1::verify_ecdsa`.

**BUG-BTC-03 — Simulated Merkle proof in `build_inclusion_proof`**
`csv-bitcoin/src/ops.rs`, `BitcoinChainProofProvider::build_inclusion_proof`:
The proof is constructed by hashing `commitment.as_bytes()` with a fake sibling — not from actual block transaction data. Fix: use `MempoolSignetRpc::extract_merkle_proof` which already correctly fetches block txids and computes the path.

**BUG-BTC-04 — `compute_sighash` uses first 20 pubkey bytes as hash160 placeholder**

```rust
if pubkey.len() >= 20 {
    script_code.extend_from_slice(&pubkey[..20]);
// In real implementation, we'd hash160 the pubkey here
```

This produces an invalid scriptCode for P2WPKH. Fix: compute HASH160 (`SHA256` then `RIPEMD160`) of the pubkey.

**BUG-BTC-05 — `build_mpc_publication_transaction` uses null UTXO**

```rust
input: vec![TxIn {
    previous_output: OutPoint::null(), // ← placeholder
```

This transaction will be rejected by any node. Fix: select a real UTXO from the wallet's UTXO set.

**STUB-BTC-06 — Fee estimation is hardcoded**
`get_fee_estimate_rpc` returns 20 or 5 sat/vbyte based on a simple modulo check on block height. A real implementation should call `estimatesmartfee` or parse mempool fee data from `mempool.space`.

**STUB-BTC-07 — `create_bitcoin_adapter` generates a random wallet**

```rust
let wallet = SealWallet::generate_random(network.to_bitcoin_network());
```

Every call creates a throwaway wallet. Fix: derive from the CLI's master HD seed.

**STUB-BTC-08 — `ChainDriver::create_client` returns error unconditionally**

```rust
async fn create_client(&self, _config: &ChainConfig) -> ChainResult<Box<dyn RpcClient>> {
    Err(ChainError::FeatureNotEnabled("Bitcoin RPC client creation from config requires 'rpc' feature"))
}
```

This blocks the runtime pattern from working without a special-case workaround.

### 3.2 Solana

**BUG-SOL-01 — Hardcoded slot in `verify_finality`**
`csv-solana/src/seal_protocol.rs`:

```rust
let current_slot = 1100u64; // Would fetch from RPC
```

This will never reflect real chain state. Finality checks on Solana devnet (slot > 100k) will always claim the transaction is unfinalized. Fix: call `self.check_rpc()?.get_latest_slot()`.

**BUG-SOL-02 — `sign_transaction` and `sign_message` return `CapabilityUnavailable`**
Both methods in `SolanaBackend` return an error rather than signing. This means the `ChainSigner` trait is effectively broken for Solana. Fix: implement Ed25519 signing using the wallet's keypair stored in `SolanaSealProtocol::wallet`.

**STUB-SOL-03 — Inclusion proof returns empty bytes**

```rust
let proof_bytes = vec![]; // Would fetch and serialize block data
```

`verify_inclusion_proof` then returns `Ok(false)` for any proof where `proof_bytes.is_empty()`. Fix: use `rpc.get_transaction(&sig)` to fetch the confirmed slot, then build the proof from slot data.

**STUB-SOL-04 — `verify_finality_proof` returns `FeatureNotEnabled`**
The verification always fails in non-rpc builds. For testnet, the `rpc` feature should be enabled and the actual slot comparison implemented.

**BUG-SOL-05 — Mint transaction uses a deterministic seed as keypair**

```rust
let copy_len = sanad_bytes.len().min(32);
seed[..copy_len].copy_from_slice(&sanad_bytes[..copy_len]);
// derives keypair from sanad_id seed — not a real signing key
```

The resulting "keypair" is not the owner's actual key. The transaction will be signed by the wrong key and rejected.

### 3.3 Aptos

**BUG-APTOS-01 — `verify_seal_available` always returns `Ok(true)` — CRITICAL**
`csv-aptos/src/seal_protocol.rs`:

```rust
// Check on-chain resource
let exists = Ok(true);  // ← always true!
let exists = exists.map_err(|e: AptosError| ProtocolError::from(e))?;
if !exists {
    return Err(AptosError::StateProofFailed(...));
}
```

The on-chain resource check never queries the RPC. Every seal appears available regardless of its actual on-chain state. This completely defeats replay prevention for Aptos. Fix: replace with the actual `StateProofVerifier::verify_resource_exists_async` call (the scaffolding is already there, just uncommented).

**BUG-APTOS-02 — `send_transaction` in backend uses hardcoded mainnet URL**

```rust
.post("https://fullnode.mainnet.aptoslabs.com/v1/transactions")
```

This is inside the `#[cfg(feature = "rpc")]` block. Testnet transactions are silently sent to mainnet. Fix: use `self.config.network.node_url()`.

**STUB-APTOS-03 — `sign_transaction` and `sign_message` return `CapabilityUnavailable`**
Same pattern as Solana. The `ChainSigner` trait for Aptos is broken.

**STUB-APTOS-04 — `verify_finality_proof` returns `FeatureNotEnabled`**
The proof is never verified. Any finality proof is accepted without cryptographic validation.

**STUB-APTOS-05 — Lock/mint use raw BCS with no on-chain handler**
`ops.rs` `lock_sanad` sends a raw transaction with custom bytes to the Aptos RPC. No Move function parses these bytes. The transaction will succeed (Aptos will accept any valid BCS transaction) but no state change occurs.

### 3.4 Sui

**BUG-SUI-01 — `lock_sanad` uses placeholder zero signature**

```rust
let signature = vec![0u8; 64]; // Placeholder signature
```

A 64-byte zero vector is not a valid Ed25519 signature. The RPC will reject it. Fix: sign `tx_bytes` with the wallet's Ed25519 key.

**BUG-SUI-02 — `mint_sanad` also uses placeholder zero signature**
Same issue. Both cross-chain operations are non-functional.

**BUG-SUI-03 — Object version hardcoded to `1` in `build_and_sign_move_call`**

```rust
tx.extend_from_slice(&1u64.to_le_bytes()); // version 1
tx.extend_from_slice(&[0u8; 32]);           // digest (zeroed for owned objects)
```

A real Sui node will reject `ObjectArg::ImmOrOwnedObject` with an incorrect version or zero digest. Fix: call `rpc.get_object(seal_object_id)` to fetch current version and digest before building the transaction.

**STUB-SUI-04 — `csv_program_id` returns placeholder**

```rust
fn csv_program_id(&self) -> Option<&'static str> {
    Some("0xcsvsui")  // ← not a real package ID
}
```

Fix: return the value from `config.seal_contract.package_id`.

**STUB-SUI-05 — `verify_inclusion` returns zero object_proof**

```rust
Ok(SuiInclusionProof::new(
    vec![0u8; 32], // object_proof would come from tx effects
```

The proof will fail any non-trivial verification.

### 3.5 Core Verifier

**BUG-CORE-01 — `validate_domain_separation` requires `seal_id == anchor_id`**
`csv-core/src/verifier.rs`:

```rust
if bundle.seal_ref.id != bundle.anchor_ref.anchor_id {
    return Err(ProtocolError::Generic("Seal reference mismatch: seal ID and anchor ID must match"));
}
```

For Ethereum the seal ID is `[contract_address(20) + slot(8)]` (28 bytes) and the anchor ID is `tx_hash` (32 bytes) — they can never be equal. For Bitcoin the seal is an `OutPoint` and the anchor is a `txid`. This check is architecturally wrong and will reject all valid proofs from every chain. Fix: remove this equality check; the association between seal and anchor is established by the proof bundle's `transition_dag`, not by byte equality.

**BUG-CORE-02 — `validate_anchor_reference` also checks `anchor_id == seal_id`**
Same incorrect equality constraint appears a second time:

```rust
if bundle.anchor_ref.anchor_id != bundle.seal_ref.id {
    return Err(...);
}
```

Must be removed.

**STUB-CORE-03 — `validate_proof_timestamp` checks only for zero**

```rust
if bundle.anchor_ref.block_height == 0 {
    return Err(ProtocolError::Generic("Invalid proof timestamp: anchor timestamp is 0"));
}
```

This is checking block height, not a timestamp. The comment says "compare against actual current timestamp" but no timestamp exists in `ProofBundle`. Either add a `created_at: u64` field or remove the timestamp validation entirely until it can be implemented properly.

### 3.6 Shared Placeholders Across All Chains

Every chain's `ChainBackend::publish_seal` uses the same dummy commitment:

```rust
commitment_bytes[..8].copy_from_slice(b"csv-seal");
// Rest is zeroes
```

This is called from `publish_seal`, which is the bridge between the generic `ChainBackend` trait and the chain-specific `SealProtocol::publish`. The caller must supply a real commitment hash. Fix: remove the dummy generation; require `commitment` as a parameter to `publish_seal`, or compute it from `seal.id` using the chain's `hash_commitment` method.

---

## 4. Duplicate Implementations

### 4.1 `backend.rs` vs `ops.rs` — Two Parallel Abstraction Layers

Every chain has two files that both claim to be "the chain operations implementation":

- `backend.rs` implements `ChainDriver` → `RpcClient` + `Wallet`
- `ops.rs` implements `ChainBackend` → `ChainQuery + ChainSigner + ChainBroadcaster + ChainDeployer + ChainProofProvider + ChainSanadOps`

The traits overlap almost completely:

| Trait in backend.rs | Equivalent in ops.rs |
|---------------------|---------------------|
| `RpcClient::send_transaction` | `ChainBroadcaster::submit_transaction` |
| `RpcClient::get_transaction` | `ChainQuery::get_transaction` |
| `RpcClient::get_latest_block` | `ChainQuery::get_latest_block_height` |
| `RpcClient::get_balance` | `ChainQuery::get_balance` |
| `RpcClient::is_transaction_confirmed` | `ChainBroadcaster::confirm_transaction` |
| `RpcClient::get_chain_info` | `ChainQuery::get_chain_info` |
| `Wallet::sign_transaction` | `ChainSigner::sign_transaction` |
| `Wallet::verify_signature` | `ChainSigner::verify_signature` |
| `Wallet::generate_address` | `ChainSigner::derive_address` |
| `ChainDriver::create_client` | (entire `ChainBroadcaster`) |
| `ChainDriver::create_wallet` | (entire `ChainSigner`) |

**Recommendation**: Decide on one abstraction layer. `ChainBackend` (ops.rs) is more complete. `ChainDriver` (backend.rs) is used by `chain_registry.rs` in the CLI to dispatch chains. Consider making `ChainDriver::create_client` and `ChainDriver::create_wallet` return the `ChainBackend` directly, and retire the separate `RpcClient` and `Wallet` traits.

### 4.2 `spawn_blocking_async` Defined Twice in Sui

`csv-sui/src/ops.rs` and `csv-sui/src/seal_protocol.rs` both define an identical helper:

```rust
fn spawn_blocking_async<F, T, E>(future: F) -> ChainOpResult<T>  // ops.rs
fn spawn_blocking_async<F, T>(future: F) -> Result<T, SuiError>  // seal_protocol.rs
```

Both spawn a thread, build a `current_thread` runtime, and call `block_on`. Move to `csv-sui/src/runtime.rs` (or reuse the pattern from `csv-sdk/src/runtime.rs`) and import from there.

### 4.3 `parse_address` / `format_address` Triplicated in Aptos

The same hex-decode-with-padding function appears in three places:

- `csv-aptos/src/seal_protocol.rs` — `parse_aptos_address(s: &str)` (module-level `fn`)
- `csv-aptos/src/ops.rs` — `AptosBackend::parse_address(&self, address: &str)` (method)
- `csv-aptos/src/backend.rs` — `parse_aptos_address(s: &str)` (module-level `fn`, identical to seal_protocol.rs)

Same duplication exists for `format_address` (hex encode with `0x` prefix). Fix: move to `csv-aptos/src/types.rs` or `csv-aptos/src/address.rs` and import everywhere.

### 4.4 `verify_seal_available` vs `enforce_seal` — Double-Spend Check Duplicated

Every `SealProtocol` implementation has:

1. `enforce_seal(seal)` — checks registry + on-chain, marks used. Called from `seal_protocol.rs`.
2. `verify_seal_available(seal)` — also checks registry + on-chain. Called from `publish()` before `enforce_seal`.

So for every `publish()` call, both functions query the same local registry and potentially the same RPC endpoint. The two-step check exists because `verify_seal_available` is a read-only pre-flight, while `enforce_seal` atomically marks. This is fine in principle, but the code duplication is large. Consider a single `check_and_reserve_seal` function that returns a `ReservationToken` (RAII guard) and is consumed by `mark_used`.

### 4.5 Domain Separator Computed in Two Places Per Chain

For each chain, the domain separator `[u8; 32]` is computed:

1. In `SealProtocol::from_config()` (the authoritative source)
2. Again in `ChainBackend::new()` (re-derives from scratch using the same algorithm)

`ChainBackend::from_seal_protocol()` correctly retrieves it from the protocol: `seal.get_domain()`. But `ChainBackend::new()` computes it independently — if the algorithm ever diverges, proofs built by `ChainBackend::new()` and verified by `SealProtocol` will fail.

Fix: remove the domain separator computation from `ChainBackend::new()`. Require `from_seal_protocol()` as the only constructor that sets `domain_separator`, forcing the single source of truth.

### 4.6 `chain_id()` / `chain_name()` in Both `ChainDriver` and `ChainBackend`

Every chain implements:

```rust
// In backend.rs (ChainDriver)
fn chain_id(&self) -> &'static str { "bitcoin" }
fn chain_name(&self) -> &'static str { "Bitcoin" }

// Also in ops.rs (ChainBackend)
fn chain_id(&self) -> &'static str { "bitcoin" }
fn chain_name(&self) -> &'static str { "Bitcoin" }
```

If one is updated and the other is not, the two layers will disagree. Fix: define these as associated constants on the `SealProtocol` implementation and let both traits delegate to them.

### 4.7 `create_seal` / `publish_seal` in `ChainBackend` Call Through to `SealProtocol`

In every chain's `ops.rs`:

```rust
fn create_seal(&self, value: Option<u64>) -> ChainOpResult<SealPoint> {
    let chain_seal = self.seal_protocol.create_seal(value)...;
    // convert types
    Ok(SealPoint { id: ..., nonce: ... })
}

fn publish_seal(&self, seal: SealPoint) -> ChainOpResult<CommitAnchor> {
    let chain_seal = /* convert SealPoint back to chain-specific type */;
    let anchor = self.seal_protocol.publish(commitment, chain_seal)...;
    // convert types
    Ok(CommitAnchor { ... })
}
```

These are type-conversion wrappers that add boilerplate without logic. The conversions (e.g., `[u8;32] txid + u32 vout → SealPoint.id[0..36]`) are non-obvious and tested nowhere. A single `SealPointConverter` trait with `to_generic()` / `from_generic()` methods, implemented per chain, would centralize this and allow testing.

### 4.8 `ChainSanadOps` Stubs Copy-Pasted Across All Chains

Every chain's `ops.rs` has nearly identical stub bodies for `create_sanad` and `consume_sanad`:

```rust
async fn create_sanad(...) -> ChainOpResult<SanadOperationResult> {
    // In Ethereum/Bitcoin/Aptos/Sui, creating a sanad involves:
    // 1. Creating a new object/slot/resource/UTXO
    // 2. ...
    Err(ChainOpError::CapabilityUnavailable("..."))
}
```

The comment block is different but the result is the same. If `ChainSanadOps` has a default implementation, the stubs can be removed. If not, move the "not yet implemented" logic to a shared `UnimplementedSanadOps` struct that each chain delegates to until the feature is ready.

---

## Summary Severity Table

| ID | Severity | Area | Description |
|----|----------|------|-------------|
| BUG-BTC-01 | 🔴 Critical | Bitcoin | `sign_transaction` always returns empty bytes |
| BUG-BTC-02 | 🔴 Critical | Bitcoin | `verify_signature` always returns false |
| BUG-APTOS-01 | 🔴 Critical | Aptos | `verify_seal_available` always returns true (replay open) |
| BUG-CORE-01 | 🔴 Critical | Core | Domain check requires `seal_id == anchor_id` (rejects all valid proofs) |
| BUG-CORE-02 | 🔴 Critical | Core | Second copy of same broken check in `validate_anchor_reference` |
| PANIC-01 | 🔴 Critical | Bitcoin/Solana | `reqwest::blocking::Client` panics inside async CLI runtime |
| BUG-SOL-01 | 🔴 Critical | Solana | Hardcoded slot `1100` in finality check |
| BUG-SUI-01 | 🔴 Critical | Sui | `lock_sanad` sends zero-bytes as signature |
| BUG-SUI-02 | 🔴 Critical | Sui | `mint_sanad` sends zero-bytes as signature |
| BUG-SUI-03 | 🔴 High | Sui | Object version hardcoded to 1 in MoveCall |
| BUG-APTOS-02 | 🔴 High | Aptos | `send_transaction` hardcoded to mainnet URL |
| BUG-BTC-03 | 🟠 High | Bitcoin | Simulated Merkle proof (not from block) |
| BUG-BTC-04 | 🟠 High | Bitcoin | Fake hash160 in sighash computation |
| BUG-BTC-05 | 🟠 High | Bitcoin | Null UTXO in MPC publication transaction |
| BUG-SOL-05 | 🟠 High | Solana | Mint uses deterministic seed as signing keypair |
| SOL-CONTRACT-01 | 🟠 High | Solana | SIZE constants missing 8-byte discriminator |
| SOL-CONTRACT-02 | 🟠 High | Solana | Rust does not call Anchor instructions (uses system program) |
| APTOS-CONTRACT-01 | 🟠 High | Aptos | No `lock_sanad` / `mint_sanad` / `refund_sanad` in Move module |
| SUI-CONTRACT-01 | 🟠 High | Sui | Move package source missing from repo |
| STUB-CORE-03 | 🟡 Medium | Core | Timestamp validation checks block_height==0, not real time |
| STUB-BTC-06 | 🟡 Medium | Bitcoin | Fee estimation is hardcoded values |
| STUB-BTC-07 | 🟡 Medium | Bitcoin | Random wallet generated on every adapter creation |
| STUB-BTC-08 | 🟡 Medium | Bitcoin | `ChainDriver::create_client` always returns error |
| STUB-SOL-03 | 🟡 Medium | Solana | Inclusion proof is empty bytes |
| STUB-SOL-04 | 🟡 Medium | Solana | `verify_finality_proof` always returns FeatureNotEnabled |
| STUB-SUI-04 | 🟡 Medium | Sui | `csv_program_id` returns `"0xcsvsui"` placeholder |
| STUB-APTOS-05 | 🟡 Medium | Aptos | Lock/mint send raw bytes with no on-chain handler |
| DUP-01 | 🟡 Medium | All | `backend.rs` vs `ops.rs` — two overlapping abstraction layers |
| DUP-02 | 🟡 Medium | Sui | `spawn_blocking_async` defined twice |
| DUP-03 | 🟡 Medium | Aptos | `parse_address` / `format_address` in three files |
| DUP-04 | 🟡 Medium | All | `verify_seal_available` duplicates `enforce_seal` logic |
| DUP-05 | 🟡 Medium | All | Domain separator computed independently in `new()` and `from_seal_protocol()` |
| DUP-06 | 🟡 Medium | All | `chain_id()`/`chain_name()` in both `ChainDriver` and `ChainBackend` |
| DUP-07 | 🟡 Low | All | Seal type-conversion boilerplate in `create_seal`/`publish_seal` per chain |
| DUP-08 | 🟡 Low | All | `ChainSanadOps` stubs copy-pasted identically across chains |
| APTOS-MOVE-01 | 🟡 Low | Aptos | `transfer_seal` deprecated but still callable; wrong error code in `create_seal` |
| APTOS-MOVE-02 | 🟡 Low | Aptos | Event handle address depends on who calls `initialize_module` |
