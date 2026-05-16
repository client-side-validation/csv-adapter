# Architecture Cleanup: Duplicate Abstraction Layers

## MED-DUP-01: backend.rs And ops.rs Still Overlap

### Current State

The repository has two parallel abstraction layers:

1. **ChainDriver/RpcClient/Wallet** (backend.rs)
   - Implements `ChainDriver` from `csv-core::driver`
   - Provides unified driver interface for cross-chain operations
   - Includes `RpcClient` and `Wallet` trait implementations
   - Used by higher-level chain adapter code

2. **ChainBackend + Operation Traits** (ops.rs)
   - Implements `ChainBackend` from `csv-core::backend`
   - Provides chain-specific backend implementations
   - Includes operation traits: ChainQuery, ChainSigner, ChainBroadcaster, ChainDeployer, ChainProofProvider, ChainSanadOps
   - Used by core verification and cross-chain logic

### Overlap Analysis

Both layers provide RPC and wallet functionality:

- `backend.rs::RpcClient` vs `ops.rs::ChainQuery` - both query chain state
- `backend.rs::Wallet` vs `ops.rs::ChainSigner` - both handle signing
- Both have transaction broadcasting capabilities

### Recommended Approach

#### Option 1: Consolidate to ChainBackend (Preferred)

- Keep `ChainBackend` as the single source of truth
- Implement `ChainDriver` as a thin wrapper around `ChainBackend`
- Deprecate direct `RpcClient`/`Wallet` usage
- Migration path:
  1. Add adapter methods to `ChainBackend` to expose `RpcClient`/`Wallet` functionality
  2. Update `ChainDriver` implementations to delegate to `ChainBackend`
  3. Gradually migrate callers to use `ChainBackend` directly
  4. Remove deprecated `RpcClient`/`Wallet` implementations

#### Option 2: Keep Both Layers with Clear Separation

- Keep both layers but clarify their purpose:
  - `ChainDriver`: High-level cross-chain orchestration
  - `ChainBackend`: Low-level chain-specific operations
- Add documentation to prevent confusion
- Ensure no code duplication between layers

### Implementation Status

**High-Priority Tasks (Completed):**

- ✅ Solana Anchor typed instruction builders
- ✅ Bitcoin prevout amount plumbing
- ✅ Aptos EntryFunction typed payloads
- ✅ Aptos address parsing/formatting consolidation
- ✅ Domain separator recomputation fix

**Medium-Priority Tasks:**

- ⏸️ Duplicate abstraction layer cleanup (requires architectural review)

### Next Steps

1. Review with team to choose consolidation approach
2. If Option 1 chosen: Implement adapter pattern in ChainBackend
3. If Option 2 chosen: Add clear documentation and separation of concerns
4. Update AUDIT.md to reflect chosen approach
