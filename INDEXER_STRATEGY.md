# CSV Explorer Indexing Strategy

## Problem

Indexing all transactions across chains would create **~5 TB/year** of data. CSV protocol data is <0.1% of chain activity.

## Solution: Selective Indexing

Index **only CSV-related data**: Rights, Seals, Transfers, Contracts, Priority Addresses.

**Result: ~1 GB/year** (99.9% reduction)

## Per-Chain Indexing (Implemented)

| Chain | What Gets Indexed | Filter Method | Proof Types | Status |
|-------|------------------|---------------|-------------|--------|
| **Bitcoin** | OP_RETURN Tapret (65 bytes) or Opret (64 bytes) commitments with `CSV-` protocol ID prefix, CSV address UTXO spends, Priority address txs | Address + OP_RETURN payload parsing | `Merkle` inclusion, `ConfirmationDepth` finality, `HashBased` commitment | ✅ Complete |
| **Ethereum** | `SealUsed(bytes32,bytes32)` events from CSV contracts, `CrossChainLock`/`RightMinted` cross-chain events | Event signature topic filtering | `MerklePatricia` inclusion, `FinalizedBlock` finality, `KZG` commitment | ✅ Complete |
| **Sui** | `{package_id}::csv_seal::AnchorEvent` events, CrossChain/bridge events | Event type name (`csv_seal` + `AnchorEvent`) | `ObjectProof` inclusion, `Checkpoint` finality, `HashBased` commitment | ✅ Complete |
| **Aptos** | `{module_address}::csv_seal::AnchorEvent` events, CrossChain/bridge_transfer events | Event type name (`csv_seal` + `AnchorEvent`) | `Accumulator` inclusion, `Checkpoint` finality, `HashBased` commitment | ✅ Complete |
| **Solana** | Transaction log messages containing `csv_right`/`RightCreated`/`csv_seal`/`SealConsumed` | Log message string matching | `AccountState` inclusion, `SlotBased` finality, `HashBased` commitment | ✅ Complete |

## Priority Address Indexing

Wallets register addresses with priority levels (High/Normal/Low). Indexers:
1. Add addresses to watch lists
2. Scan historical data for registered addresses
3. Monitor new blocks for address activity
4. Index at configurable intervals (High: 10s, Normal: 1m, Low: 5m)

## Advanced Commitment & Proof Support

The indexer now supports multiple commitment schemes and proof types:

**Commitment Schemes**: `HashBased`, `Pedersen`, `KZG`, `Bulletproofs`, `Multilinear`, `FRI`, `Custom`

**Inclusion Proofs**: `Merkle` (BTC), `MerklePatricia` (ETH), `ObjectProof` (SUI), `Accumulator` (APT), `AccountState` (SOL)

**Finality Proofs**: `ConfirmationDepth` (BTC), `FinalizedBlock` (ETH), `Checkpoint` (SUI/APT), `SlotBased` (SOL)

Each indexed record includes commitment scheme, proof type, and metadata for analytics and filtering.

## Configuration

```toml
[chains.bitcoin]
enabled = true
network = "testnet"
rpc_url = "https://mempool.space/testnet/api"
csv_addresses = ["tb1p...", "tb1q..."]

[indexer.wallet_bridge]
high_priority_interval_ms = 10_000
normal_priority_interval_ms = 60_000
low_priority_interval_ms = 300_000
```

## Database Schema

- `enhanced_rights` - Rights with commitment scheme & proof metadata
- `enhanced_seals` - Seals with proof types
- `enhanced_inclusion_proofs` - Detailed proof records
- `enhanced_transfers` - Transfers with cross-chain proof data
- `proof_statistics` - Aggregated statistics

Indexed by: commitment scheme, proof type, owner, chain, verification status.

## Principle

**Index only what matters for CSV. Ignore everything else.**
