# Implementation Guides

This directory contains implementation guides for each supported blockchain in the CSV adapter ecosystem.

## 📋 Available Guides

### 🟡 [Bitcoin](./bitcoin/)
- **[Address Types](./bitcoin/BITCOIN_ADDRESS_TYPES.md)** - Bitcoin address formats and validation
- **[Integration Guide](./bitcoin/README.md)** - Complete Bitcoin implementation guide

### 🔵 [Ethereum](./ethereum/)
- **[Testnet Manual](./ethereum/E2E_TESTNET_MANUAL.md)** - Ethereum testnet setup and deployment
- **[Integration Guide](./ethereum/README.md)** - Complete Ethereum implementation guide

### 🟢 [Aptos](./aptos/)
- **[ALUVM Integration](./aptos/ALUVM.md)** - Aptos ALUVM implementation details
- **[Integration Guide](./aptos/README.md)** - Complete Aptos implementation guide

### 🟣 [Sui](./sui/)
- **[Integration Guide](./sui/README.md)** - Complete Sui implementation guide

### 🟣 [Solana](./solana/)
- **[Integration Guide](./solana/README.md)** - Complete Solana implementation guide
- **[Configuration](./solana/CONFIG.md)** - Solana adapter configuration
- **[RPC Client](./solana/RPC.md)** - Solana RPC client details

## 🚀 Getting Started

1. **Choose Your Chain**: Select the appropriate guide from above
2. **Follow Integration Guide**: Each chain has a comprehensive README with step-by-step instructions
3. **Configure Settings**: Set up network, RPC endpoints, and program IDs
4. **Test Integration**: Use provided test commands and examples

## 🔧 Common Patterns

All adapters follow the same architectural patterns:

### Core Components

- **Anchor Layer**: Implements `AnchorLayer` trait for chain-specific operations
- **RPC Client**: Handles blockchain communication with fallback support
- **Configuration**: Network settings and chain-specific parameters
- **Error Handling**: Comprehensive error types and recovery mechanisms

### Standard Operations

```rust
// Create a client
let client = CsvClient::builder()
    .with_chain(Chain::Bitcoin)
    .with_bitcoin_config(bitcoin_config)
    .build()?;

// Create a right
let right = client.create_right()
    .chain(Chain::Bitcoin)
    .owner("address")
    .commitment("hash")
    .await?;

// Consume a seal
let seal = client.consume_seal()
    .chain(Chain::Bitcoin)
    .seal_ref("utxo_ref")
    .right_id("right_id")
    .await?;
```

## 📚 Additional Resources

- **[Architecture](../architecture/ARCHITECTURE.md)** - System architecture overview
- **[Cross-Chain](../cross-chain/CROSS_CHAIN_IMPLEMENTATION.md)** - Cross-chain transfer details
- **[Security](../security/EXPLORER_WALLET_INDEXING.md)** - Security considerations
- **[API Reference](../api/csv-api.yaml)** - Complete API specification

---

*Last updated: April 2026*
