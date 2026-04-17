# Architecture

Related docs: [Documentation Hub](INDEX.md), [Cross-Chain Specification](CROSS_CHAIN_SPEC.md), [Developer Guide](DEVELOPER_GUIDE.md), [Blueprint](BLUEPRINT.md)

## System summary

CSV Adapter is a client-side validation system for transferable rights. The codebase is built around one central architectural decision:

- the chain enforces single-use
- the client verifies history and proof validity
- the protocol keeps the right portable by keeping it off-chain

The repository implements that model through a Rust core, per-chain adapters, CLI orchestration, and a growing set of ecosystem tools.

## Architectural layers

```text
Applications and tools
  csv-cli | csv-wallet | csv-explorer | typescript-sdk | csv-mcp-server

Unified API surface
  csv-adapter

Protocol core
  csv-adapter-core
    Right | Commitment | ProofBundle | AnchorLayer | validator | cross_chain

Chain adapters
  csv-adapter-bitcoin
  csv-adapter-ethereum
  csv-adapter-sui
  csv-adapter-aptos

Storage and supporting infrastructure
  csv-adapter-store | csv-local-dev | external RPC providers
```

## The core protocol boundary

The clearest boundary in the code is the `AnchorLayer` trait in `csv-adapter-core/src/traits.rs`. It defines the lifecycle every adapter must provide:

- create a seal
- publish a commitment
- produce inclusion evidence
- produce finality evidence
- enforce seal consumption
- build a proof bundle
- handle rollbacks and chain-specific replay isolation

That trait is the key reason the multi-chain design stays coherent. The adapters differ in proof formats and transport details, but they conform to one protocol contract.

## Core data model

The main stable protocol surface is re-exported from `csv-adapter-core/src/lib.rs`:

| Type | Role |
|------|------|
| `Right` | Portable client-side state object |
| `Commitment` | Hash-linked state transition anchor |
| `SealRef` | Chain-specific single-use reference |
| `AnchorRef` | Reference to published anchor data |
| `InclusionProof` | Evidence that an anchor is in chain history |
| `FinalityProof` | Evidence that the anchor is sufficiently final |
| `ProofBundle` | Portable verification package |
| `AnchorLayer` | Adapter interface for all supported chains |

The core crate also contains broader protocol machinery such as commitment chains, consignments, DAG segments, validators, registry logic, and experimental modules for VM, MPC, and RGB compatibility.

## Chain model

CSV intentionally does not flatten every chain into the same trust profile. Instead it records what each chain is actually good at.

| Chain | Seal model | Verification artifacts |
|-------|------------|------------------------|
| Bitcoin | UTXO spend | transaction data, Merkle branch, block confirmations |
| Sui | Object deletion or mutation | transaction effects, checkpoint contents, certification |
| Aptos | Move resource destruction | transaction proof, ledger info, event stream |
| Ethereum | Nullifier registration | logs, receipts, MPT-style evidence, confirmations |

That graded-enforcement model is one of the strongest architectural choices in the repo. It avoids pretending all chains offer the same primitive.

## Transfer flow

At a high level, a cross-chain transfer looks like this:

1. Create or identify the current right and its active seal.
2. Consume the seal on the source chain.
3. Collect inclusion and finality data from the source chain.
4. Build a portable proof bundle.
5. Verify the proof bundle on the receiving side.
6. Re-anchor or mint a destination-side representation when the destination model requires it.

The CLI expresses this through source-side lock providers, a universal verification step, and destination-side mint providers. The protocol meaning is described in more detail in [Cross-Chain Specification](CROSS_CHAIN_SPEC.md).

## Repository structure

### Rust workspace

The root Cargo workspace currently includes:

- `csv-adapter-core`
- `csv-adapter-bitcoin`
- `csv-adapter-ethereum`
- `csv-adapter-sui`
- `csv-adapter-aptos`
- `csv-adapter-store`
- `csv-adapter`
- `csv-cli`
- `csv-wallet`

### Adjacent packages

The repo also contains adjacent packages that are not part of the root Rust workspace but are important to the product story:

- `typescript-sdk`
- `csv-mcp-server`
- `csv-local-dev`
- `csv-explorer`
- `csv-tutorial`
- `csv-vscode`
- `create-csv-app`

One of the previous documentation problems was that top-level docs described only the Rust workspace and underrepresented these adjacent packages.

## Security and trust model

The security posture follows directly from the architecture:

| Concern | Primary control |
|---------|-----------------|
| Seal replay | Base-layer single-use semantics plus registry checks |
| Fraudulent inclusion claims | Cryptographic proof verification |
| Weak finality assumptions | Chain-specific finality rules |
| Reorg handling | Adapter rollback hooks |
| Data availability | Client-side storage and proof transport discipline |
| RPC dishonesty | Verification against returned proof data, ideally across multiple providers |

The system is strongest when the proof bundle is treated as the portable artifact of record, not the RPC response that helped build it.

## Architectural assessment

### Strengths

- Strong protocol center in `csv-adapter-core`
- Clean adapter boundary via `AnchorLayer`
- Clear chain-specific modeling instead of false abstraction
- Broad ecosystem surface already present in the repo

### Current pressure points

- Documentation had drifted away from the real package layout
- Some docs mixed implemented behavior with future-state ideas
- Operational tooling and ecosystem packages need consistent top-level framing

## What this means for contributors

When you change the system, the primary places to keep aligned are:

1. `csv-adapter-core` API surface and invariants
2. the per-chain adapter that implements the invariant
3. `csv-cli` or higher-level clients that expose the behavior
4. the canonical docs in this folder

If a change is architectural, update this document first or alongside the code.
