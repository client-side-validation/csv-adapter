# Cross-Chain Specification

Related docs: [Architecture](ARCHITECTURE.md), [Implementation Status](CROSS_CHAIN_IMPLEMENTATION.md), [Developer Guide](DEVELOPER_GUIDE.md)

## Purpose

This document defines the protocol meaning of a cross-chain transfer in CSV Adapter. It focuses on semantics and verification rules, not on crate-by-crate implementation details.

## The governing rule

CSV is not a bridge protocol. It is a proof portability protocol.

- the source chain enforces single-use
- the sender or adapter produces evidence of that enforcement
- the receiver verifies the evidence
- the right is accepted because the evidence is valid

The asset that crosses domains is the proof bundle, not a trusted attestation from an intermediary.

## Protocol objects

| Object | Meaning |
|--------|---------|
| `Right` | Portable client-side claim or state object |
| `Seal` | Chain-specific primitive that can be consumed at most once |
| `Commitment` | Hash of a state transition bound to a seal and domain separator |
| `Anchor` | Published chain reference tied to a commitment |
| `ProofBundle` | Inclusion evidence, finality evidence, and transition context |

## Single-chain transfer semantics

Within one chain, the protocol is:

1. A right is associated with an active seal.
2. The current owner consumes that seal through a valid chain action.
3. The chain records enough data to prove the action happened.
4. The receiver verifies the proof and updates local state.

The chain guarantees single-use. The receiver guarantees acceptance only after verification.

## Cross-chain transfer semantics

Cross-chain portability adds a destination verification context, but the core idea stays the same:

1. Consume the source-chain seal.
2. Produce inclusion and finality evidence from the source chain.
3. Bind the evidence to the expected commitment and state transition.
4. Verify that the seal has not already been replayed elsewhere.
5. Accept or mint the destination-side representation according to the destination model.

The destination does not need to trust the source chain's RPC endpoint. It only needs enough verified proof material to reproduce the decision.

## Proof bundle requirements

A valid proof bundle must answer four questions:

1. Which chain enforced single-use?
2. Which seal was consumed?
3. Where is the on-chain evidence of that consumption?
4. Why is that evidence final enough to trust?

Conceptually, the portable bundle includes:

- source chain identity
- consumed `SealRef`
- `AnchorRef` or equivalent transaction reference
- `InclusionProof`
- `FinalityProof`
- commitment and transition context

The exact binary representation differs by chain adapter, but the verification contract is the same.

## Verification pipeline

Any verifier, local or destination-side, should apply the same logical sequence:

1. Decode the proof bundle.
2. Confirm the bundle matches the expected chain and right identifiers.
3. Verify inclusion against authenticated chain data.
4. Verify finality according to chain-specific rules.
5. Confirm the consumed seal and commitment agree with the expected transition.
6. Check replay and double-spend guards.
7. Accept the transition only if all prior checks succeed.

## Chain-specific models

### Bitcoin

- Seal model: UTXO spend
- Inclusion evidence: transaction membership in a block via Merkle branch
- Finality model: confirmation depth
- Security character: strongest structural single-use guarantee in the current set

### Sui

- Seal model: object deletion or object-state transition
- Inclusion evidence: transaction and effects in checkpoint contents
- Finality model: certified checkpoint
- Security character: structural object lifecycle with validator-certified finality

### Aptos

- Seal model: resource destruction or controlled resource movement
- Inclusion evidence: transaction and ledger proof material
- Finality model: ledger finality from the chain's consensus output
- Security character: strong type-enforced linearity

### Ethereum

- Seal model: nullifier registration through contract state
- Inclusion evidence: receipt and log path data
- Finality model: confirmation threshold
- Security character: contract-mediated single-use rather than structural single-use

## Invariants

The protocol relies on a few invariants that should remain true across implementations:

| Invariant | Why it matters |
|-----------|----------------|
| A seal is accepted as consumed at most once | Prevents replay and double-spend |
| A commitment is domain-separated by chain context | Prevents cross-chain confusion and replay |
| Inclusion evidence must bind to authenticated chain history | Prevents forged transaction claims |
| Finality rules must be chain-specific | Prevents false equivalence across chains |
| The receiver decides acceptance after verification | Preserves client-side validation model |

## What the spec does not claim

This spec does not assume:

- that every chain offers the same security properties
- that every transfer requires destination-chain minting
- that an RPC response is itself trustworthy
- that bridge-style messaging is part of the protocol

## Relationship to implementation

The implementation lives across:

- `csv-adapter-core` for shared proof and validation logic
- `csv-adapter-*` crates for chain-specific proof generation
- `csv-cli` for orchestration and end-user flows

For shipped status and current gaps, see [Implementation Status](CROSS_CHAIN_IMPLEMENTATION.md).
