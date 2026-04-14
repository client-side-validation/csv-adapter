# Documentation Hub

This folder is organized around a small set of canonical documents:

- [Architecture](ARCHITECTURE.md): how the system is structured today
- [Cross-Chain Specification](CROSS_CHAIN_SPEC.md): what the protocol means and verifies
- [Developer Guide](DEVELOPER_GUIDE.md): how to build, test, and extend the repo
- [Implementation Status](CROSS_CHAIN_IMPLEMENTATION.md): what is already implemented
- [Blueprint](BLUEPRINT.md): where the project is headed next

## Start here

If you are new to the repo:

1. Read the [project README](../README.md) for the high-level framing.
2. Read [Architecture](ARCHITECTURE.md) to understand the system boundary.
3. Read [Developer Guide](DEVELOPER_GUIDE.md) to get productive locally.

## By audience

| If you are... | Read this first | Then this |
|---------------|-----------------|-----------|
| New to CSV Adapter | [README](../README.md) | [Architecture](ARCHITECTURE.md) |
| Implementing protocol behavior | [Cross-Chain Specification](CROSS_CHAIN_SPEC.md) | [Implementation Status](CROSS_CHAIN_IMPLEMENTATION.md) |
| Building or modifying code | [Developer Guide](DEVELOPER_GUIDE.md) | [Architecture](ARCHITECTURE.md) |
| Planning roadmap work | [Blueprint](BLUEPRINT.md) | [AluVM Note](ALUVM.md) |
| Working on explorer or wallet indexing | [Explorer and Wallet Indexing](EXPLORER_WALLET_INDEXING.md) | [Implementation Status](CROSS_CHAIN_IMPLEMENTATION.md) |
| Running testnets manually | [E2E Testnet Manual](E2E_TESTNET_MANUAL.md) | [Testnet E2E Report](TESTNET_E2E_REPORT.md) |

## Canonical documents

### System

| Document | Scope |
|----------|-------|
| [Architecture](ARCHITECTURE.md) | Package boundaries, invariants, and chain roles |
| [Cross-Chain Specification](CROSS_CHAIN_SPEC.md) | Transfer semantics, proof bundle structure, and verification rules |
| [Implementation Status](CROSS_CHAIN_IMPLEMENTATION.md) | Current implementation state and known gaps |

### Engineering

| Document | Scope |
|----------|-------|
| [Developer Guide](DEVELOPER_GUIDE.md) | Setup, common commands, crate map, and extension workflow |
| [Explorer and Wallet Indexing](EXPLORER_WALLET_INDEXING.md) | Indexing strategy, wallet bridge model, and API expectations |
| [OpenAPI Reference](csv-api.yaml) | Machine-readable API description |

### Planning

| Document | Scope |
|----------|-------|
| [Blueprint](BLUEPRINT.md) | Product and engineering blueprint |
| [AluVM Note](ALUVM.md) | Experimental design note for future VM integration |

### Operations

| Document | Scope |
|----------|-------|
| [E2E Testnet Manual](E2E_TESTNET_MANUAL.md) | Step-by-step testnet walkthrough |
| [Testnet E2E Report](TESTNET_E2E_REPORT.md) | Captured outcomes, issues, and observations |

## Notes

- This cleanup intentionally removes duplicate planning documents so each topic has one primary home.
- The audit HTML artifact referenced by older docs is not present in this checkout, so it is no longer linked here.
