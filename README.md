# CSV Adapter — Client-Side Validation via Universal Seal Primitive

[![Build](https://img.shields.io/badge/build-passing-brightgreen)]()
[![Tests](https://img.shields.io/badge/tests-535%20passing-brightgreen)]()
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)]()

**CSV Adapter** is a **client-side validation system** built on the **Universal Seal Primitive (USP)**. It lets you create, transfer, and consume Rights on any chain — with clients doing the validation, not the blockchain.

> We are not building a bridge. We are building a validation system where each chain enforces single-use at its strongest available guarantee, and clients verify everything else.

**Status: Architectural prototype.** The trait design is sound, the code compiles, and 535 tests pass. But **no adapter has been tested against a live network.** See [Reality Check](#reality-check).

---

## Table of Contents

- [Reality Check](#reality-check)
- [The USP Framework](#the-usp-framework)
- [Enforcement Layers](#enforcement-layers)
- [Architecture](#architecture)
- [Quick Start](#quick-start)
- [Philosophy](#philosophy)
- [Design Decisions](#design-decisions)
- [Network Support](#network-support)
- [Project Structure](#project-structure)
- [Test Results](#test-results)
- [What Remains](#what-remains)
- [Key Dependencies](#key-dependencies)
- [License](#license)

---

## Reality Check

**Honest assessment:**

| Claim | Status |
|-------|--------|
| "Publish Rights to Bitcoin" | ❌ `publish()` returns fake txids. Wiring incomplete. |
| "Publish Rights to Sui" | ❌ Direct HTTP client exists, not wired to adapter. |
| "Publish Rights to Aptos" | ❌ REST API client exists, not wired to adapter. |
| "Publish Rights to Ethereum" | ❌ Alloy declared, not integrated. |
| "Cross-chain Right transfer" | ❌ Not implemented. |
| "RGB compatible" | ⚠️ Re-implementation exists, unverified against reference. |
| "Move contracts deployed" | ❌ `.move` files exist, never compiled or deployed. |
| "Tests pass" | ⚠️ 100% use mock RPCs. Zero live network tests. |

**This is a well-structured Rust skeleton with the right abstractions. It is not a deployable system.**

---

## The USP Framework

The **Universal Seal Primitive** defines a canonical `Right` type that every chain enforces at its strongest available guarantee:

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

**Core Invariant:** A Right can be exercised at most once under the strongest available guarantee of the host chain.

### Client-Side Validation

The chain does NOT validate state transitions. It only:
1. Records the commitment (anchor)
2. Enforces single-use of the Right (via UTXO spend, object deletion, resource destruction, or nullifier registration)

**Clients do everything else:**
1. Fetch the full state history for a contract
2. Verify the commitment chain from genesis to present
3. Check that no Right was consumed more than once
4. Accept or reject the consignment based on local validation

This is not a bridge. It's a validation system where the blockchain provides the minimum guarantee (single-use enforcement) and clients verify the rest.

### Architecture Gap: What Supports CSV vs What's Missing

| CSV Requirement | Current State | Gap |
|----------------|---------------|-----|
| Chain enforces single-use | ✅ Per-chain adapter | Wiring incomplete |
| Client fetches state history | ❌ Not implemented | Sprint 2 |
| Client verifies commitment chain | ❌ Not implemented | Sprint 2 |
| Client checks no double-consumption | ⚠️ SealRegistry exists (in-memory only) | No persistence, no cross-chain |
| Client accepts/rejects consignment | ⚠️ Consignment struct exists | Validation flow not built |
| Full state history storage | ❌ Not implemented | Sprint 2 |

**The skeleton is there. The validation machinery is not.** The `AnchorLayer` trait and `Consignment` type are the right abstractions. But the engine that fetches history, verifies commitments, detects conflicts, and accepts/rejects consignments does not exist yet. That's Sprint 2.

### The Degradation Rule

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

---

## Enforcement Layers

| Level | Name | Guarantee | Chains | Our Adapter | Nullifier? |
|-------|------|-----------|--------|-------------|------------|
| **L1** | Structural | Native single-use | Bitcoin, Sui | `BitcoinAnchorLayer`, `SuiAnchorLayer` | ❌ No |
| **L2** | Type-Enforced | Language-level scarcity | Aptos | `AptosAnchorLayer` | ❌ No |
| **L3** | Cryptographic | Nullifier-based | Ethereum | `EthereumAnchorLayer` | ✅ Yes |

### Bitcoin (L1 — Reference)
UTXO spending = Right consumption. Chain enforces single-use structurally. No nullifier needed.

### Sui (L1 — Structurally Aligned)
Object deletion = Right consumption. Object versioning prevents double-spend natively. No nullifier needed.

### Aptos (L2 — Programmable)
Resource destruction = Right consumption. Move VM enforces non-duplication at language level. Account-scoped (not independent like UTXOs).

### Ethereum (L3 — Simulation)
Nullifier registration = Right consumption. Smart contract tracks `nullifiers[id] = true`. Security depends on contract correctness, not structural guarantees.

---

## Architecture

```
┌──────────────────────────────────────────┐
│          csv-adapter-core                 │
│  Right type + AnchorLayer trait          │
│  Degradation model (L1 → L2 → L3)        │
└──────────────────────────────────────────┘
    L1│        L3│        L1│        L2│
    ┌─┴──┐    ┌─┴───┐   ┌─┴───┐   ┌─┴───┐
    │BTC │    │ ETH │   │ Sui │   │Aptos│
    │(0.30)   │(Alloy)   │HTTP │   │HTTP │
    └────┘    └─────┘   └─────┘   └─────┘
```

Each adapter implements the `AnchorLayer` trait with the Right lifecycle appropriate to its enforcement layer. L1 adapters have no nullifier logic. L3 adapters require nullifier registry contracts.

---

## Quick Start

```bash
git clone https://github.com/your-org/csv-adapter.git
cd csv-adapter
cargo build --workspace
cargo test --workspace
```

```rust
use csv_adapter_bitcoin::BitcoinAnchorLayer;
use csv_adapter_core::{Hash, AnchorLayer};

// Returns fake txid — no live network connection
let adapter = BitcoinAnchorLayer::signet()?;
let right = adapter.create_seal(Some(100_000))?;
let anchor = adapter.publish(Hash::new([0xAB; 32]), right)?;
```

---

## Philosophy

Client-Side Validation flips the blockchain paradigm: validation is pushed to the edges. Only contract participants verify state transitions. The blockchain provides commitment anchoring and single-use enforcement — not global validation.

**The USP insight:** different chains enforce single-use at different levels. Bitcoin does it structurally (UTXOs). Sui does it structurally (Objects). Aptos does it via type system (Move resources). Ethereum does it cryptographically (nullifier contracts). Rather than pretending these are equivalent, we model the degradation explicitly and let each chain enforce at its strongest available guarantee.

Full specification: [docs/Blueprint.md](docs/Blueprint.md)

---

## Design Decisions

### 1. Degradation Over Simulation
We don't pretend Ethereum has structural single-use. We model it honestly as L3 Cryptographic with a nullifier registry. This is weaker than L1 Structural, and the documentation says so.

### 2. Canonical Right Type
The `Right` struct is chain-agnostic. Each adapter maps its native primitive (UTXO, Object, Resource, Nullifier) to this type at the boundary.

### 3. Official Blockchain Libraries
`rust-bitcoin`, `alloy`, `ed25519-dalek` — we don't re-implement cryptography. Sui/Aptos use direct HTTP calls (SDKs not yet integrated).

### 4. No Mocks That Lie
`StubBitcoinRpc` explicitly refuses to broadcast transactions with a clear error. No fabricated txids. No silent failures.

### 5. Signature Scheme Per Chain
Bitcoin/Ethereum: Secp256k1. Sui/Aptos: Ed25519. The adapter declares its scheme so verification selects the right algorithm.

### 6. Rollback as Right Un-consumption
When a chain reorg invalidates an anchor, the adapter clears the consumption marker so the Right can be re-exercised.

---

## Network Support

| Chain | Networks | Enforcement Layer | Default |
|-------|----------|-------------------|---------|
| **Bitcoin** | Mainnet, Testnet3, Signet, Regtest | L1 Structural | Signet |
| **Ethereum** | Mainnet, Sepolia, Holesky, Dev | L3 Cryptographic | Sepolia |
| **Sui** | Mainnet, Testnet, Devnet, Local | L1 Structural | Testnet |
| **Aptos** | Mainnet, Testnet, Devnet | L2 Type-Enforced | Testnet |

Configurations exist for all networks. **None tested against live nodes.**

---

## Project Structure

```
csv-adapter/
├── csv-adapter-core/          # Right type, AnchorLayer trait, degradation model
├── csv-adapter-bitcoin/       # L1 Structural: UTXO seals, Tapret anchoring
├── csv-adapter-ethereum/      # L3 Cryptographic: nullifier registry, MPT proofs
├── csv-adapter-sui/           # L1 Structural: object seals, checkpoint finality
├── csv-adapter-aptos/         # L2 Type-Enforced: resource seals, HotStuff finality
├── csv-adapter-store/         # SQLite persistence
└── docs/
    ├── Blueprint.md           # Universal Seal Primitive specification
    └── PRODUCTION_PLAN.md     # 30-week plan with degradation-based priorities
```

---

## Test Results

```
535 tests passing across all crates

  csv-adapter-core:        230
  csv-adapter-bitcoin:      82
  csv-adapter-ethereum:     60
  csv-adapter-sui:          48
  csv-adapter-aptos:        10
  csv-adapter-store:         3
  Integration tests:         2
```

**Important: 100% of tests use mock/stub RPCs.** No test has connected to a live node.

Run all tests:
```bash
cargo test --workspace
```

---

## What Remains

**30 weeks to production.** Full plan in [docs/PRODUCTION_PLAN.md](docs/PRODUCTION_PLAN.md).

| Sprint | Duration | Goal |
|--------|----------|------|
| 0. Canonical Model + Crypto | 4 weeks | `Right` type, tagged hashing, no V1, alloy-trie |
| 1. Wire RPCs (L1→L2→L3) | 8 weeks | Bitcoin/Sui (L1), Aptos (L2), Ethereum (L3) |
| 2. Client-Side Validation | 4 weeks | RGB-mode validation across all enforcement layers |
| 3. End-to-End Testing | 4 weeks | Live testnets, concrete use-case through ALL layers |
| 4. Cross-Chain Portability | 4 weeks | Right transfer between chains with proof |
| 5. RGB Verification | 3 weeks | Compare against reference, cross-validate |
| 6. Security Hardening | 3 weeks | Fuzzing, property tests, third-party audit |

---

## Key Dependencies

| Chain | Library | Version | Purpose | Status |
|-------|---------|---------|---------|--------|
| Bitcoin | `bitcoin` | 0.30 | Block/tx parsing, Merkle trees, Taproot | ✅ Used |
| Bitcoin | `bitcoincore-rpc` | 0.17 | Node RPC | ⚠️ Declared, not wired |
| Ethereum | `alloy` | 0.9 | Transaction building, signing | ⚠️ Declared, not wired |
| Sui/Aptos | `ed25519-dalek` | 2.0 | Ed25519 signature verification | ✅ Used |
| All | `rusqlite` | 0.30 | SQLite persistence | ✅ Used |

---

## License

MIT or Apache-2.0 — choose the license that best fits your use case.
