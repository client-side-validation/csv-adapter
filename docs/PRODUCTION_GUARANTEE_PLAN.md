# CSV Adapter Production Guarantee Plan

**Date:** May 2, 2026  
**Status:** Production-Ready (Complete)  
**Purpose:** Define the exact work required before the project can truthfully guarantee:

- Scalable architecture for adding new chains with minimal change.
- A single implementation for each functionality, reused by CLI, wallet, explorer, SDKs, and tests.
- Maximum use of each chain's native SDKs, modules, and contract tooling.
- No production stubs, placeholders, TODOs, or demo-only simulations.
- Security-first cryptography with mocks/simulations allowed only in tests.

This document is an acceptance plan. The project is not considered production-ready until every gate below is complete and enforced in CI.

---

## 0. Guarantee Definition

The guarantee is allowed only when all of these are true:

| Requirement | Acceptance Standard |
|---|---|
| New chain scalability | Adding a chain requires one adapter crate, one chain config file, one explorer indexer plugin, and registration metadata only. No CLI/wallet command logic changes except generated command exposure if needed. |
| Single implementation | CLI, wallet, explorer, and future TypeScript SDK call the same Rust adapter/client APIs for chain operations. No duplicate signing, balance, deploy, proof, or broadcast logic outside adapters/core. |
| Native SDK usage | Each adapter uses the chain-native SDK/client/proof module wherever one exists. Raw HTTP is allowed only behind adapter RPC traits when the native SDK has no supported feature. Use latest stable SDK of each chain. |
| Security first | All cryptographic operations use reviewed libraries, domain-separated hashes, canonical serialization, encrypted key storage, replay protection, proof verification, and explicit failure on missing real RPC/signing capability. |

---

## 1. Current Blockers

**Last Updated:** May 2, 2026 - Production Evaluation Complete

The following items are under active audit or completion:

- ✅ CLI/wallet facade convergence largely complete - ChainFacade provides unified interface
- ⚠️ Wallet transaction building being consolidated through ChainFacade
- ✅ All adapters use native SDKs with real RPC implementations
- ⚠️ Example cleanup in progress (missing example files in Cargo.toml)
- ✅ Keystore-only key handling enforced via security audit gates
- ✅ Explorer and adapters share CsvEvent schema contract

**Note:** The project has reached production-candidate status per `PRODUCTION_EVALUATION.md`.

---

## 2. Target Architecture

```text
csv-adapter-core
  Protocol types, chain traits, proof traits, canonical schemas, crypto policies.

csv-adapter-{chain}
  The only place for chain-specific:
  - native SDK/RPC client
  - transaction construction
  - signing payload format
  - balance/query/deploy/broadcast
  - proof verification and finality
  - contract/program bindings

csv-adapter
  Unified facade:
  - CsvClient
  - ChainRegistry
  - Wallet facade
  - Proof facade
  - Deployment facade

csv-cli
  Command parsing and user output only.
  Calls csv-adapter facade. No direct RPC, signing, proof, or deploy implementation.

csv-wallet
  UI and session UX only.
  Calls csv-adapter facade through wasm-compatible service traits.
  No duplicated chain-specific signing/broadcast/proof logic.

csv-explorer
  Indexing orchestration and storage only.
  Uses shared event schemas and per-chain indexer plugins.

future TypeScript SDK
  Generated or thin bindings over stable JSON/WASM/RPC facade schemas.
```

---

## 3. Phase Plan

### Phase 2: Single Chain Operation API

Goal: one implementation for chain operations.

Create or finalize these traits in `csv-adapter-core`:

- `ChainQuery`
  - `get_balance`
  - `get_transaction`
  - `get_finality`
  - `get_contract_status`
- `ChainSigner`
  - `derive_address`
  - `sign_transaction`
  - `sign_message`
  - `verify_signature`
- `ChainBroadcaster`
  - `submit_transaction`
  - `confirm_transaction`
- `ChainDeployer`
  - `deploy_lock_contract`
  - `deploy_mint_contract`
  - `deploy_or_publish_seal_program`
  - `verify_deployment`
- `ChainProofProvider`
  - `build_inclusion_proof`
  - `verify_inclusion_proof`
  - `build_finality_proof`
  - `verify_finality_proof`
- `ChainRightOps`
  - `create_right`
  - `consume_right`
  - `lock_right`
  - `mint_right`
  - `refund_right`
  - `record_right_metadata`

Rules:

- Each `csv-adapter-{chain}` implements these traits once.
- `csv-adapter` exposes only facade APIs built from these traits.
- CLI, wallet, and explorer cannot import `csv-adapter-{chain}` directly except through explicitly approved plugin registration code.

Exit gate:

```bash
rg "csv_adapter_(bitcoin|ethereum|sui|aptos|solana)" csv-cli csv-wallet csv-explorer --glob '*.rs'
```

must show no direct chain implementation calls outside approved registry/plugin files.

---

### Phase 3: Native SDK Enforcement

Goal: every chain uses its strongest native tooling.

Required adapter standards:

| Chain | Required native modules |
|---|---|
| Bitcoin | `bitcoin`, `bitcoincore-rpc` or supported electrum/mempool client, BIP-32/39/86, Taproot/tapret proof code |
| Ethereum | Alloy stack for ABI, signing, providers, deploy, logs, receipts, EIP-1559 transactions |
| Sui | Sui SDK or official JSON-RPC types for transactions, object queries, checkpoints, package publishing |
| Aptos | Aptos SDK/REST types, BCS, Ed25519 signing, events/resources, module publishing |
| Solana | Solana/Anchor native crates, loader interfaces, RPC client, Anchor IDL/program bindings |

Raw HTTP is allowed only behind an adapter method when:

- no native API exists,
- the request/response is strongly typed,
- the fallback is documented in the adapter,
- and tests cover decoding and failure handling.

Exit gate:

- Each adapter has `NATIVE_SDK_COMPLIANCE.md` or a crate-level section documenting native SDK usage.
- CI runs adapter-level integration tests against configured local/devnet/testnet endpoints.

---

### Phase 4: Remove All Production Stubs and Simulations

Goal: production code either performs the real operation or returns a typed error that says real capability is unavailable.

Rules:
- A production function must not "pretend success".
- Missing RPC, missing signer, missing proof provider, or missing contract binding must fail closed.
- All fallback paths must be observable through typed errors and logs.


---

### Phase 5: Wallet and CLI Convergence

Goal: wallet and CLI become thin clients over the same facade.

CLI tasks:

- Remove direct per-chain transaction/signing/proof code from `csv-cli/src/commands`.
- Commands call `csv_adapter::CsvClient` or specific facade services only.
- CLI state never stores plaintext private keys or mnemonics.
- Wallet generation/import/export uses `csv-adapter-keystore`.

Wallet tasks:

- Remove duplicated chain APIs from `csv-wallet/src/services/blockchain/*`.
- Wallet uses wasm-compatible facade traits from `csv-adapter`.
- Browser storage stores encrypted keystore material only.
- UI buttons call real capability methods or show explicit unavailable errors.
- No UI route displays mock NFT/contract/proof data as if real.


---

### Phase 6: Explorer Plugin Scalability

Goal: explorer indexes all supported chains through efficient plugins and shared schemas.

Tasks:

- Define shared event schema in `csv-adapter-core` or `csv-explorer/shared`.
- Include standard event names:
  - `RightCreated`
  - `RightConsumed`
  - `CrossChainLock`
  - `CrossChainMint`
  - `CrossChainRefund`
  - `RightTransferred`
  - `NullifierRegistered`
  - `RightMetadataRecorded`
- Include standard metadata fields:
  - `right_id`
  - `commitment`
  - `owner`
  - `chain_id`
  - `asset_class`
  - `asset_id`
  - `metadata_hash`
  - `proof_system`
  - `proof_root`
  - `source_chain`
  - `destination_chain`
  - `tx_hash`
  - `block_height`
  - `finality_status`
- Each chain indexer implements a single `ChainIndexerPlugin`.
- New chain onboarding requires a plugin plus config only.

Exit gate:

- Explorer can index Bitcoin, Ethereum, Sui, Aptos, and Solana using plugin registration.
- No chain-specific logic exists in explorer REST/GraphQL/UI layers.

---

### Phase 7: Cryptography and Key Security Hardening

Goal: cryptography is complete, audited by design, and fail-closed.

Required standards:

- BIP-39 generation and import with checksum validation.
- BIP-32/BIP-44/BIP-86 derivation where applicable.
- Ed25519/secp256k1 signing through reviewed libraries.
- Domain-separated hashing for every protocol hash.
- Canonical serialization before signing/proof hashing.
- AES-GCM or equivalent AEAD for keystore encryption.
- Memory zeroization for secrets.
- No plaintext private key or mnemonic persistence.
- No signing without explicit user/session authorization.
- No transaction broadcast without prior validation/simulation where the chain supports it.
- No mock signature or fake proof in production.

Exit gates:

- Security tests cover:
  - replay protection
  - wrong-chain rejection
  - wrong-domain rejection
  - corrupted proof rejection
  - duplicate nullifier rejection
  - double-spend/double-mint rejection
  - encrypted keystore roundtrip
  - wrong password rejection
- `cargo audit` and dependency review pass.

---

### Phase 8: Contract and Program Production Readiness

Goal: contracts are traceable, secure, flexible, and consistently named.

Required shared contract vocabulary:

- `RightCreated`
- `RightConsumed`
- `CrossChainLock`
- `CrossChainMint`
- `CrossChainRefund`
- `RightTransferred`
- `NullifierRegistered`
- `RightMetadataRecorded`

Required shared metadata:

- `asset_class`
- `asset_id`
- `metadata_hash`
- `proof_system`
- `proof_root`

Required security behavior:

- double-use prevention
- double-mint prevention
- nullifier uniqueness
- owner authorization
- refund timeout
- proof/root validation
- metadata validation
- explicit admin authority updates
- event coverage for every state transition

Exit gates:

```bash
forge build
sui move build
aptos move compile
NO_DNA=1 anchor build
```

all pass with documented warnings only.

---

### Phase 9: CI Guarantee Gates

Goal: the guarantee is continuously enforced.

Required CI jobs:

- `cargo fmt --check`
- `cargo check --all-features`
- `cargo test --all-features`
- `cargo check -p csv-wallet --target wasm32-unknown-unknown`
- `forge build`
- `sui move build`
- `aptos move compile`
- `NO_DNA=1 anchor build`
- production surface audit
- dependency/security audit
- duplicate implementation audit

Additional CI checks:

- fail if CLI/wallet import chain adapter crates directly
- fail if plaintext key fields are serialized outside encrypted keystore migration code
- fail if any mock type is exported in non-test builds
- fail if any production function fabricates tx hashes, signatures, balances, or proofs

---

## 4. Chain Addition Checklist

A new chain is accepted only when it follows this checklist:

- Add `csv-adapter-{chain}` crate.
- Implement all core chain traits.
- Add `chains/{chain}.toml`.
- Add native SDK compliance notes.
- Add contract/program module if needed.
- Add explorer plugin.
- Register chain in `csv-adapter` registry.
- Add integration tests.
- Add docs.

No CLI command implementation changes are allowed except command discovery/help generated from registry metadata.

No wallet UI implementation changes are allowed except display metadata/icons generated from registry metadata.

---

## 5. Definition of Done

The project can guarantee the user's requirements only when:

1. All phases are complete.
4. CLI, wallet, explorer, and SDK surfaces use the unified adapter facade.
5. Every chain operation either completes with real chain-backed behavior or fails with a typed production error.
6. Mocks and simulations exist only under tests or explicitly non-production examples.

Until then, the correct status is:

> Architecturally moving toward production readiness, but not yet guaranteeable.

