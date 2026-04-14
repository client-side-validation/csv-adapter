# Explorer and Wallet Indexing

Related docs: [Documentation Hub](INDEX.md), [Architecture](ARCHITECTURE.md), [Implementation Status](CROSS_CHAIN_IMPLEMENTATION.md)

## Purpose

This document combines the previously split indexing strategy and wallet-integration notes into one canonical design reference.

## Problem

A generic multi-chain explorer can index everything, but a wallet needs something narrower and faster:

- data relevant to the user's addresses
- predictable freshness for recently touched assets
- a consistent API for rights, transfers, seals, and proofs

The goal is to let the explorer serve broad observability while still supporting wallet-grade responsiveness.

## Recommended model

Use two complementary indexing modes:

| Mode | Purpose |
|------|---------|
| Global indexing | Build a network-wide picture for explorer and analytics use cases |
| Priority address indexing | Pull forward wallet-relevant addresses, contracts, and transfers |

This gives the system a scalable default while still serving interactive wallet workflows.

## Core components

### Explorer stack

The repo already contains the main explorer layers in `csv-explorer`:

- `indexer`
- `api`
- `storage`
- `ui`
- `shared`

Those should remain the system of record for indexed chain data and higher-level query surfaces.

### Wallet-facing bridge

Wallet integrations should not bypass the explorer's data model ad hoc. Instead they should rely on a stable bridge that can:

- register addresses of interest
- query wallet-scoped rights and transfer history
- subscribe to updates where available
- surface priority status and sync progress

## Data contract

At minimum, wallet-facing indexing should expose:

- tracked addresses by chain
- rights associated with those addresses
- transfer history and current status
- recent proof-related activity
- sync freshness or lag indicators

## Design principles

### 1. Wallets should query intent, not raw chain internals

A wallet should ask for "rights for this address" or "recent transfers involving this wallet", not reconstruct those views from low-level chain events itself.

### 2. Priority indexing should be explicit

Address prioritization should be a first-class feature, not an accidental cache side effect.

### 3. Multi-chain support needs one vocabulary

The explorer and wallet should share terms for:

- rights
- seals
- anchors
- transfers
- proofs
- sync status

Without that shared vocabulary, cross-chain behavior becomes hard to explain and harder to debug.

## Implementation direction

The most useful near-term path is:

1. keep the explorer as the indexing backbone
2. standardize wallet registration and wallet-scoped query endpoints
3. make priority-address indexing visible in storage and API behavior
4. expose sync state so the wallet can explain stale data instead of guessing

## Why this doc exists

The repo previously had two separate documents describing largely the same area from different angles. Keeping one canonical doc here makes it easier to update the explorer and wallet stories together.
