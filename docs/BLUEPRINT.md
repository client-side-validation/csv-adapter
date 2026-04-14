# Blueprint

Related docs: [Architecture](ARCHITECTURE.md), [Implementation Status](CROSS_CHAIN_IMPLEMENTATION.md), [Developer Guide](DEVELOPER_GUIDE.md), [Explorer and Wallet Indexing](EXPLORER_WALLET_INDEXING.md), [AluVM Note](ALUVM.md)

## Purpose

This blueprint is the forward-looking document for CSV Adapter. It describes where the project should invest next and how the repository should evolve from a strong protocol core into a cohesive developer platform.

## Current baseline

The codebase already contains:

- a mature protocol center in `csv-adapter-core`
- four chain adapters
- a unified Rust client
- a CLI
- a wallet, explorer, TypeScript SDK, local-dev tooling, and MCP server

The roadmap should build on that reality instead of re-describing already-shipped structure as if it were hypothetical.

## Product direction

CSV Adapter should become the default developer stack for portable, proof-verified rights across chains.

That means optimizing for three outcomes:

1. clear protocol trust boundaries
2. fast developer onboarding
3. reusable tooling across CLI, SDK, wallet, explorer, and agents

## Strategic principles

### 1. Protocol first

The core abstractions in `csv-adapter-core` remain the source of truth. New capabilities should preserve:

- single-use enforcement at the chain layer
- proof portability at the protocol layer
- verification at the client layer

### 2. One concept, many surfaces

The same protocol should be available through:

- Rust APIs
- TypeScript APIs
- CLI workflows
- wallet UX
- explorer APIs
- machine-readable agent tools

### 3. Canonical documentation

Every important topic should have one obvious home. The repo should avoid multiple planning files that drift apart.

## Priority workstreams

### Workstream A: protocol hardening

Focus:

- tighten proof verification boundaries
- improve replay and registry guarantees
- keep experimental modules clearly labeled
- strengthen integration coverage around real chain conditions

### Workstream B: developer platform

Focus:

- mature `csv-adapter` as the ergonomic Rust surface
- continue the TypeScript SDK story
- keep CLI flows aligned with library APIs
- make local development faster and more reproducible

### Workstream C: wallet and explorer coherence

Focus:

- align explorer indexing with wallet needs
- standardize wallet-to-indexer contracts
- improve visibility into rights, transfers, proofs, and seal history

### Workstream D: agent and automation support

Focus:

- keep `csv-mcp-server` and machine-readable API surfaces current
- expose structured statuses and actionable errors
- make agent workflows reuse the same business logic as CLI and SDK flows

## Near-term roadmap

### Near term

- keep architecture and protocol docs in lockstep with the code
- strengthen implementation-status tracking without turning it into a changelog
- reduce friction across CLI, SDK, wallet, and explorer workflows
- make local development and testnet testing easier to reproduce

### Medium term

- deepen explorer and wallet integration
- improve developer-facing diagnostics and observability
- mature agent-facing APIs and status reporting
- clarify feature maturity across experimental modules

### Longer term

- broader chain support where the seal model remains honest
- advanced proof compression and privacy work
- stronger programmable validation and VM strategy
- richer application-layer examples and starter kits

## Blueprint for documentation and DX

The previous documentation set mixed roadmap, specification, implementation notes, and agent planning across overlapping files. Going forward:

- [Architecture](ARCHITECTURE.md) explains what exists
- [Cross-Chain Specification](CROSS_CHAIN_SPEC.md) explains what the protocol means
- [Developer Guide](DEVELOPER_GUIDE.md) explains how to work on it
- this document explains what should happen next

That split is part of the product strategy, not just a docs cleanup.

## Success metrics

Track progress with a small set of signals:

| Metric | Why it matters |
|--------|----------------|
| Time to first successful local workflow | Measures onboarding friction |
| Number of surfaces sharing the same protocol contract | Measures architectural reuse |
| Documentation drift incidents | Measures source-of-truth discipline |
| End-to-end reproducibility across chains | Measures operational maturity |
| Agent and automation success rate | Measures machine-usable interfaces |

## Design notes for future work

Some topics remain explicitly exploratory and should stay framed that way:

- AluVM integration
- RGB compatibility expansion
- advanced MPC wallet patterns
- zero-knowledge proof compression
- broader chain coverage beyond the current set

Those should be explored through design notes and targeted implementation plans, not blended into current-state docs.
