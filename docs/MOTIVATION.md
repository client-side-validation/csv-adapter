# Why CSV Exists

## The Problem: Cross-Chain is Broken

Current "bridges" require you to trust a third party—validators, multi-sigs, or optimistic challenge games. When bridges fail (and they do), billions of dollars are lost. The fundamental issue: **bridges transfer assets through trust, not through proof**.

## The CSV Philosophy

**CSV is not a bridge. It is a verification model.**

The core insight: blockchains already enforce single-use guarantees. Bitcoin's UTXOs can only be spent once. Sui's objects can only be deleted once. The problem isn't the chains—it's how we prove these facts across chains.

### Three Principles

1. **Chain enforces single-use** — We don't replicate consensus. We rely on each chain's native mechanics.

2. **Client verifies proof** — The receiving side validates cryptographic evidence, not bridge attestations.

3. **Sanad stays portable** — The "Sanad" (the asset representation) lives off-chain in the client, making it chain-agnostic until anchoring.

## How It Works

```
Sanad (off-chain) → Seal (on-chain) → Consumed (proof) → Re-anchored (new chain)
```

A "Seal" is a cryptographic proof of single-use commitment. Once a seal is consumed on the source chain, it generates a proof bundle that can be verified anywhere—no bridge required.

## Why This Matters

| Bridge Approach | CSV Approach |
|---------------|--------------|
| Trust validators | Trust cryptography |
| 2/3 multi-sig failure = loss | Proof invalid = rejection |
| Centralized control | Client-side verification |
| Expensive (pay bridge fees) | Cheap (pay only chain fees) |
| 30 min+ finality | As fast as source chain finality |

## Real-World Impact

- **96-97% cost savings** vs traditional bridges
- **No central point of failure**
- **Works with any chain** that can express single-use commitments
- **Privacy-preserving** — proofs reveal only what's necessary

## The Long-Term Vision

CSV enables a world where assets flow between chains as easily as packets flow between networks. No trusted intermediaries. Just cryptographic proof, client verification, and chain-native guarantees.

This is the foundation for **client-side validated finance**—where users hold their own proofs, verify their own state, and only interact with chains when absolutely necessary.
