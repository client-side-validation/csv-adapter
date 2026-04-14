# CSV Explorer Indexing Strategy

## Problem

Indexing all transactions across chains would create **~5 TB/year** of data. CSV protocol data is <0.1% of chain activity.

## Solution: Selective Indexing

Index **only CSV-related data**: Rights, Seals, Transfers, Contracts, Priority Addresses.

**Result: ~1 GB/year** (99.9% reduction)

## Per-Chain Indexing

| Chain | What Gets Indexed | Filter Method | Proof Types |
|-------|------------------|---------------|-------------|
| **Bitcoin** | OP_RETURN commitments, CSV address UTXO spends, Priority address txs | Address filtering | Merkle inclusion, ConfirmationDepth finality |
| **Ethereum** | `RightCreated`, `SealConsumed`, `CrossChainTransfer` events from CSV contracts | Event signature filtering | MerklePatricia inclusion, FinalizedBlock finality |
| **Sui** | `RightCreated`, `SealCreated/Consumed` Move events from CSV packages | Event type name filtering | ObjectProof inclusion, Checkpoint finality |
| **Aptos** | `RightCreated`, `SealCreated/Consumed` Move events from CSV modules | Event type name filtering | Accumulator inclusion, Checkpoint finality |

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
