# CSV Protocol Audit - Open Tasks

**Validated**: May 16, 2026  
**Scope checked**: chain adapters, core verifier, and deployed contract sources referenced by the previous audit.

This file tracks only findings that are still open after validation. Completed or stale items from the previous audit were removed.

## Fixed Or Stale Findings Removed

- Core verifier no longer requires `seal_id == anchor_id` in `validate_domain_separation` or `validate_anchor_reference`.
- Aptos `verify_seal_available` no longer returns `Ok(true)` unconditionally; it fails closed without `rpc` and checks the on-chain resource through `StateProofVerifier`.
- Aptos contract source under `csv-contracts/aptos/contracts/sources/csv_seal.move` contains `lock_sanad`, `mint_sanad`, and `refund_sanad`.
- Bitcoin `BitcoinWallet::sign_transaction` and `verify_signature` no longer return empty/false placeholders.
- Bitcoin mempool RPC uses async `reqwest::Client` and runs HTTP work on a dedicated thread, avoiding nested Tokio runtime panics from `reqwest::blocking`.
- Bitcoin MPC publication no longer uses `OutPoint::null()`; it selects a wallet UTXO.
- Bitcoin P2WPKH sighash now builds `scriptCode` with `HASH160(pubkey)` instead of the first 20 public-key bytes.
- Solana `verify_finality` no longer uses hardcoded slot `1100`; it calls `get_latest_slot`.
- Solana inclusion proof construction no longer emits empty proof bytes.
- Solana `ChainSigner` now signs messages and serialized transactions from a hex-encoded 32-byte secret key.
- Solana `mint_sanad` no longer derives a signing key from the sanad id; it fails closed until typed Anchor minting is wired.
- Solana `lock_sanad` no longer submits a system-program placeholder; it validates inputs and fails closed until typed Anchor wiring is present.
- Solana Anchor account `SIZE` constants include the 8-byte discriminator, and account init sites use the constants directly.
- Sui `lock_sanad` and `mint_sanad` no longer submit zero signatures; they use the configured signing key and fail closed when unavailable.
- Sui `csv_program_id` returns the configured package id instead of `"0xcsvsui"`.
- `ChainBackend::publish_seal` now requires a caller-supplied commitment, and all adapter implementations pass that commitment through instead of synthesizing `b"csv-seal"` placeholders.
- Sui MoveCall construction now fetches the seal object's real version and digest from RPC before signing.
- Sui package source, deployment script, and README now live under `csv-contracts/sui/`.
- Sui inclusion proofs now derive proof bytes from transaction effects instead of a zero vector.
- Bitcoin inclusion proof construction now asks the configured RPC for a Merkle proof from block transaction data; unsupported RPCs fail closed.
- Bitcoin signing no longer inserts `0` as the BIP-143 prevout value; it refuses to sign until prevout amounts are available.
- Bitcoin fee estimation now delegates to the configured RPC/fee API instead of block-height modulo heuristics.
- Bitcoin config-driven adapter creation now requires `custom_settings.xpub`; it no longer creates a throwaway wallet.
- Aptos `lock_sanad` and `mint_sanad` no longer submit untyped custom bytes; they validate inputs and fail closed until signed EntryFunction wiring is present.

## Open Critical

No open critical findings remain in this audit tracker.

## Open High

### HIGH-SOL-01 - Solana Anchor Instruction Wiring Still Incomplete

`csv-solana/src/seal_protocol.rs` still publishes commitments with ad-hoc instruction bytes instead of a complete typed client for the deployed `csv-seal` Anchor program. Sanad lock/mint now fail closed rather than submitting placeholders.

**Required fix**: add typed instruction builders for `create_seal`, `consume_seal`, `lock_sanad`, `mint_sanad`, `refund_sanad`, and `register_nullifier`, using Anchor discriminators and the accounts declared in the on-chain program.

### HIGH-BTC-01 - Bitcoin Transaction Signing Needs Prevout Amount Plumbing

`compute_sighash` now fails closed instead of signing with `0u64`, but `ChainSigner::sign_transaction` still has no API to receive spent UTXO amounts. BIP-143 signatures require the actual prevout amount.

**Required fix**: carry selected UTXO amount into signing and include it in the sighash preimage.

### HIGH-APTOS-01 - Aptos EntryFunction Wiring Still Incomplete

The Move source has the required entry functions, and the Rust backend now fails closed instead of submitting raw bytes. The typed transaction payload/signing path is still missing.

**Required fix**: build typed Aptos entry-function payloads for `lock_sanad`, `mint_sanad`, and `refund_sanad`, signed by the relevant account.

## Open Medium

### MED-APTOS-01 - Aptos Finality Proof Verification Is Shallow

`verify_finality_proof` checks ledger availability and confirmation counts but does not verify a HotStuff certificate or accumulator proof.

### MED-DUP-01 - `backend.rs` And `ops.rs` Still Overlap

The repository still has two parallel abstraction layers: `ChainDriver`/`RpcClient`/`Wallet` and `ChainBackend` plus operation traits.

### MED-DUP-02 - Aptos Address Parsing/Formatting Is Duplicated

`parse_aptos_address` / `format_address` logic still exists in multiple Aptos modules.

### MED-DUP-03 - Domain Separators Can Be Recomputed In Backend Constructors

Some `ChainBackend::new()` constructors still recompute domain separators independently instead of always deriving from `SealProtocol`.

## Low Priority Cleanup

- Consolidate `chain_id()` / `chain_name()` constants across `ChainDriver` and `ChainBackend`.
- Centralize seal point conversion helpers per chain and add roundtrip tests.
- Replace repeated `ChainSanadOps` unsupported-operation bodies with shared defaults where the trait design allows it.
- Decide whether deprecated Aptos `transfer_seal` should abort explicitly or be removed in a module-version bump.
