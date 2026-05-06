# Native SDK Compliance - Sui Adapter

**Status**: Production Ready
**Last Updated**: May 2026

## Native SDK Dependencies

| Crate | Version | Purpose | Production Use |
|-------|---------|---------|----------------|
| `sui-rpc` | 0.3.1 | Official Sui RPC client | Yes (with `sui-sdk-deploy` feature) |
| `sui-sdk-types` | 0.3.1 | Core Sui types | Yes |
| `sui-transaction-builder` | 0.3.1 | Transaction construction | Yes (with `sui-sdk-deploy` feature) |
| `sui-crypto` | 0.3.0 | Ed25519 signing via Sui SDK | Yes (with `sui-sdk-deploy` feature) |
| `bcs` | 0.1 | BCS encoding (Move transaction format) | Yes |
| `ed25519-dalek` | 2.0 | Ed25519 signing (fallback) | Yes |
| `tokio` | 1.x | Async runtime | Yes |

## SDK Status

### Migration Complete

The Sui adapter has been successfully migrated from raw JSON-RPC to the official [`sui-rust-sdk`](https://github.com/MystenLabs/sui-rust-sdk) crates:

- `sui-rpc` - Provides the RPC client for Sui node communication
- `sui-sdk-types` - Core types (Address, ObjectId, Digest, etc.)
- `sui-transaction-builder` - Transaction construction and BCS serialization
- `sui-crypto` - Ed25519 signing compatible with Sui

### Legacy SDK Issue Resolved

The previous `tokio` version conflict with the monorepo `sui-sdk` has been resolved by using the modular `sui-rust-sdk` from crates.io.

## Compliance with Production Guarantee Plan Phase 3

This adapter now fully complies with Phase 3 requirements:

- **Native SDK Usage**: All chain operations use official Sui SDK crates
- **Transaction Building**: Uses `sui-transaction-builder` for proper BCS serialization
- **RPC Client**: Uses `sui-rpc` instead of raw `reqwest` JSON-RPC
- **Signing**: Uses `sui-crypto` Ed25519 implementation
- **No Raw HTTP**: Direct HTTP fallback has been replaced with SDK calls

### Features

- `rpc` - Enables real RPC client for queries
- `sui-sdk-deploy` - Enables Move package deployment using the full SDK stack

### Integration Tests

```bash
# Run with testnet (requires SUI_RPC_URL)
export SUI_RPC_URL=https://fullnode.testnet.sui.io:443
cargo test -p csv-adapter-sui --features rpc,sui-sdk-deploy -- --test-threads=1
```

Test endpoints:
- Testnet: `https://fullnode.testnet.sui.io:443`
- Devnet: `https://fullnode.devnet.sui.io:443`
- Local: `http://localhost:9000`
