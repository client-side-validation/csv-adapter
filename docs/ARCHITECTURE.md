# CSV Architecture

> Client-Side Validation for Cross-Chain Rights
> Version: 0.1.0 · Status: Core complete, 4 chains supported

---

## 1. The Core Idea

CSV replaces the traditional blockchain consensus model with **client-side validation**. Instead of every node re-executing every transaction, clients independently verify the full history of a Right before accepting it.

### Why This Matters

| Traditional | CSV |
|-------------|-----|
| Every node validates every transaction | Only the receiving client validates |
| State is on-chain | State is off-chain, anchored on-chain |
| Single-use enforced by consensus | Single-use enforced by the base layer |
| Privacy: all data is public | Privacy: only commitments are public |
| Cross-chain: trusted bridges | Cross-chain: cryptographic proofs |

### The Seal Primitive

Every chain enforces **single-use** through its native mechanism:

| Chain | Mechanism | Guarantee |
|-------|-----------|-----------|
| **Bitcoin** | UTXO spending | Strongest — structural impossibility |
| **Sui** | Object deletion | Strong — object cannot exist after deletion |
| **Aptos** | Resource destruction | Strong — Move VM enforces linear types |
| **Ethereum** | Nullifier registration | Contract-enforced — requires honest contract |

---

## 2. Architecture Overview

```
┌──────────────────────────────────────────────────────────────────┐
│                      Application Layer                           │
│  NFTs · Credentials · Gaming Assets · Supply Chain · DeFi        │
├──────────────────────────────────────────────────────────────────┤
│                    Cross-Chain Protocol Layer                     │
│  ┌─────────────┐  ┌──────────────────┐  ┌─────────────────────┐ │
│  │ LockProvider│→ │TransferVerifier  │→ │   MintProvider      │ │
│  │ (source)    │  │ (proof validation)│  │  (destination)      │ │
│  └─────────────┘  └──────────────────┘  └─────────────────────┘ │
├──────────────────────────────────────────────────────────────────┤
│                       Core Types (csv-adapter-core)              │
│  Right · Commitment · SealRef · AnchorRef · ProofBundle · Hash   │
│  AnchorLayer (trait) · SignatureScheme · CrossChainSealRegistry  │
├──────────────────────────────────────────────────────────────────┤
│                    Chain Adapter Layer                           │
│  ┌──────────┐  ┌──────────┐  ┌─────────┐  ┌─────────┐          │
│  │ Bitcoin  │  │ Ethereum │  │  Sui    │  │ Aptos   │          │
│  │ UTXO     │  │ Nullifier│  │ Object  │  │Resource │          │
│  │ Tapret   │  │ MPT      │  │ Checkpt │  │ Ledger  │          │
│  └──────────┘  └──────────┘  └─────────┘  └─────────┘          │
├──────────────────────────────────────────────────────────────────┤
│                      Transport Layer                             │
│  mempool.space API · Ethereum JSON-RPC · Sui JSON-RPC ·          │
│  Aptos REST API                                                  │
└──────────────────────────────────────────────────────────────────┘
```

---

## 3. The AnchorLayer Trait

Every chain adapter implements the same trait. This is the contract between the core protocol and the chain-specific implementation.

```rust
pub trait AnchorLayer {
    type SealRef;          // What identifies a consumed seal on this chain
    type AnchorRef;        // What identifies a published anchor
    type InclusionProof;   // How to prove a tx was included in a block
    type FinalityProof;    // How to prove the block is final

    /// Publish a commitment to the chain (broadcast a transaction)
    fn publish(&self, commitment: Hash, seal: Self::SealRef) -> Result<Self::AnchorRef>;

    /// Verify that an anchor was included in a block
    fn verify_inclusion(&self, anchor: Self::AnchorRef) -> Result<Self::InclusionProof>;

    /// Verify that the block containing the anchor is finalized
    fn verify_finality(&self, anchor: Self::AnchorRef) -> Result<Self::FinalityProof>;

    /// Mark a seal as consumed (prevents replay)
    fn enforce_seal(&self, seal: Self::SealRef) -> Result<()>;

    /// Create a new seal (e.g., derive a new UTXO reference)
    fn create_seal(&self, value: Option<u64>) -> Result<Self::SealRef>;

    /// Compute the commitment hash for a state transition
    fn hash_commitment(...) -> Hash;

    /// Build a complete proof bundle for cross-chain transfer
    fn build_proof_bundle(&self, anchor: Self::AnchorRef, dag: DAGSegment) -> Result<ProofBundle>;

    /// Handle chain reorganizations
    fn rollback(&self, anchor: Self::AnchorRef) -> Result<()>;

    /// Domain separator for chain-specific isolation
    fn domain_separator(&self) -> [u8; 32];

    /// Signature scheme used by this chain
    fn signature_scheme(&self) -> SignatureScheme;
}
```

---

## 4. Data Flow: Cross-Chain Transfer

### Step-by-Step

```
Source Chain (e.g., Bitcoin)           Destination Chain (e.g., Sui)
───────────────────────────           ───────────────────────────────

1. Create Right
   right_id = H(commitment || salt)
   commitment = H(state, rules)

2. Lock Right
   ┌─ Spend UTXO (seal consumed) ─┐
   │   tx = build_commitment_tx()  │
   │   tx includes Tapret output   │
   │   broadcast to mempool        │
   └───────────────────────────────┘
           ↓
3. Generate Proof
   ┌─ Merkle proof (tx in block) ─┐
   │   Checkpoint proof (block    │
   │   certified by validators)   │
   └──────────────────────────────┘
           ↓
4. Transfer Proof ───────────────→
                                    5. Verify Proof
                                    ┌─ Verify Merkle inclusion ─┐
                                    │   Verify checkpoint cert  │
                                    │   Verify seal not spent   │
                                    └───────────────────────────┘
                                           ↓
                                    6. Mint Right
                                    ┌─ Create new Right with     ┐
                                    │   same commitment           │
                                    │   New owner on dest chain   │
                                    └─────────────────────────────┘
```

### What Each Chain Contributes

| Step | Bitcoin | Ethereum | Sui | Aptos |
|------|---------|----------|-----|-------|
| **Seal** | UTXO txid:vout | Nullifier hash | Object ID | Resource address |
| **Lock** | Spend UTXO via Tapret | Call `lockRight()` | Delete RightObject | Destroy RightResource |
| **Proof** | Merkle branch + block header | MPT receipt proof | Checkpoint certification | LedgerInfo + signatures |
| **Finality** | 6 confirmations | 15 confirmations | Certified checkpoint | HotStuff consensus |
| **Mint** | N/A (source only) | `mintRight()` verifies proof | `mint_right()` creates object | `mint_right()` creates resource |

---

## 5. Commitment Chain

A **commitment chain** is a linked sequence of commitments that represents the full history of a Right. Each commitment references the hash of the previous one.

```
Genesis → State A → State B → State C (current)
   ↓         ↓         ↓         ↓
  H(0)     H(A)      H(B)      H(C)
```

Clients validate a Right by:
1. Fetching the full commitment chain from the issuer
2. Verifying each link: `commitment[i].previous_commitment == hash(commitment[i-1])`
3. Verifying the latest commitment is anchored on-chain
4. Verifying the seal was consumed

This means the blockchain only needs to store **anchors** (minimal data), while the full state history is maintained off-chain and validated by clients.

---

## 6. Crate Structure

| Crate | Purpose | Stability |
|-------|---------|-----------|
| **csv-adapter-core** | Core types, traits, validation | 🟢 Stable |
| **csv-adapter-bitcoin** | Bitcoin Signet adapter (UTXO + Tapret) | 🟢 Stable |
| **csv-adapter-ethereum** | Ethereum Sepolia adapter (CSVLock + CSVMint) | 🟢 Stable |
| **csv-adapter-sui** | Sui Testnet adapter (Object model) | 🟢 Stable |
| **csv-adapter-aptos** | Aptos Testnet adapter (Resource model) | 🟡 Maturing |
| **csv-adapter-store** | Persistent seal registry storage | 🟢 Stable |
| **csv-cli** | Command-line interface for all operations | 🟡 Maturing |

### Dependency Graph

```
csv-cli
├── csv-adapter-core (core types + AnchorLayer trait)
├── csv-adapter-bitcoin ─┐
├── csv-adapter-ethereum ├─→ csv-adapter-core
├── csv-adapter-sui ─────┤
├── csv-adapter-aptos ───┤
└── csv-adapter-store ───┘
```

---

## 7. Security Model

### Trust Assumptions

| Component | Trust Model | Why |
|-----------|-------------|-----|
| **Base layer** | Trustless | Bitcoin/Sui/Ethereum/Aptos consensus |
| **Seal consumption** | Trustless | Enforced by base layer rules |
| **Proof generation** | Trustless | Cryptographic verification |
| **Proof verification** | Trustless | On-chain verification at mint time |
| **Data availability** | Semi-trusted | Clients must fetch full state history |
| **RPC endpoints** | Semi-trusted | Multiple endpoints can be verified |

### Attack Vectors and Mitigations

| Attack | Mitigation |
|--------|-----------|
| Double-spend on source chain | Impossible — base layer enforces single-use |
| Double-spend on destination chain | Impossible — contract checks `mintedRights[rightId]` |
| Fraudulent proof | Verified on-chain at mint time (Merkle/checkpoint verification) |
| Chain reorg on source | Rollback mechanism in AnchorLayer trait |
| RPC endpoint lies | Cross-check with multiple endpoints |
| Front-running | Nullifier construction includes user secret |

---

## 8. Performance Characteristics

| Metric | Value | Notes |
|--------|-------|-------|
| **Proof size** | 200-500 bytes | Merkle branch + checkpoint data |
| **Verification gas (ETH)** | ~100k | MPT proof verification |
| **Lock → Mint latency** | ~15-25s (Sui↔Sui) | Without Bitcoin confirmations |
| **Lock → Mint latency (BTC source)** | ~60min | Dominated by Bitcoin 6-conf wait |
| **Throughput** | Unlimited parallel | Each Right is independent |
| **Cost per transfer** | $0.01-0.50 | Gas only, no bridge fees |

---

## 9. Future Work

See [BLUEPRINT.md](./BLUEPRINT.md) for the full development roadmap including:
- Browser Extension Wallet
- DeFi Applications (lending, DEX, insurance)
- Fraud Proofs
- MPC Wallet
- ZK-STARK Support
- React-Based UI
- New chain adapters (Solana, Cosmos, Polkadot)
- RGB Protocol compatibility
- SDK development (TypeScript, Go, Python)
