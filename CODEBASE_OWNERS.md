# Codebase Ownership & Stability

This document maps every crate to a maintenance owner and a stability contract.

## Ownership Table

| Crate                | Owner                     | Stability                                    |
|----------------------|---------------------------|----------------------------------------------|
| csv-adapter-core     | Core Team              | **FROZEN** — no breaking changes without RFC |
| csv-adapter-bitcoin  | Core Team              | **STABLE**                                   |
| csv-adapter-ethereum | Core Team              | **STABLE**                                   |
| csv-adapter-sui      | Core Team              | **STABLE**                                   |
| csv-adapter-aptos    | Core Team              | **STABLE**                                   |
| csv-adapter-solana   | Core Team              | **BETA**                                     |
| csv-adapter          | Core Team              | **STABLE**                                   |
| csv-adapter-store    | Junior dev OK          | **BETA**                                     |
| csv-adapter-keystore | Core Team              | **STABLE**                                   |
| csv-cli              | Junior dev OK          | **BETA**                                     |
| csv-wallet           | Junior dev OK (UI only) | **ALPHA**                                    |
| csv-explorer         | Junior dev OK          | **ALPHA**                                    |

## Rules for Contributors

### Stability Levels

- **FROZEN**: No API changes without extensive review, RFC process, and documentation updates. Only critical bug fixes.
- **STABLE**: API is stable. New features OK, breaking changes require deprecation cycle.
- **BETA**: API may change. New features welcome, breaking changes should be documented.
- **ALPHA**: API is unstable. Experimental work welcome, expect frequent changes.

### Contribution Guidelines

1. **Junior devs** should only touch **ALPHA** or **BETA** crates without senior review.
2. Any change to **STABLE** crates requires core team review.
3. Any change to **csv-adapter-core** (FROZEN) requires:
   - Core team sign-off
   - Documentation update
   - RFC for breaking changes
   - Full test suite pass

### Module Dependency Rules

```text
csv-adapter-core     → no workspace deps (only std + external)
csv-adapter-{chain}  → may depend on csv-adapter-core
csv-adapter          → may depend on csv-adapter-core + csv-adapter-{chain}
csv-adapter-store    → may depend on csv-adapter-core
csv-adapter-keystore → may depend on csv-adapter-core
csv-cli              → may depend on csv-adapter + csv-adapter-store + csv-adapter-keystore
csv-wallet           → may depend on csv-adapter + csv-adapter-store + csv-adapter-keystore
csv-explorer         → may depend on csv-adapter + csv-adapter-store
```

**Important**: `csv-wallet` must NOT directly import `csv-adapter-bitcoin`, `csv-adapter-ethereum`, etc. — all chain operations go through `csv-adapter::ChainFacade`.

## Protocol Invariants (DO NOT VIOLATE)

1. **A SealRef.seal_id must come from a real blockchain transaction.**
   Never construct it from a timestamp, UUID, or random bytes.

2. **A Commitment must be published on-chain before a ProofBundle is built.**
   Never build a ProofBundle without an AnchorRef.

3. **A Right must pass ConsignmentValidator before entering AppState.**
   Never accept a Right without running all 5 validation steps.

4. **Balances are stored as u64 native units (satoshis, lamports, MIST, octas, wei).**
   Never store as f64. Never store as human-readable string.

5. **SealRegistry::check_consumed must run before accepting any incoming transfer.**
   Never accept a transfer without double-spend check.
