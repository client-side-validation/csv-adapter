# Solana Integration Guide

This guide covers the implementation of CSV (Client-Side Validation) adapter for the Solana blockchain.

## Overview

The Solana adapter provides CSV protocol support for Solana's account-based architecture using program accounts as seals and program instructions for commitment publishing.

## Architecture

### Components

- **csv-adapter-solana**: Core adapter package implementing `AnchorLayer` trait
- **Solana Indexer**: Blockchain data indexing for CSV-related transactions
- **NFT Support**: Multi-chain NFT management in wallet

### Key Features

- Account-based seals using Solana program accounts
- SPL token support for NFT tracking
- Program instruction detection for CSV operations
- Cross-chain transfer support
- RPC client with fallback support

## Implementation Details

### Sealing Mechanism

Solana uses program accounts as single-use seals:

1. **Account Creation**: Create a new program account for each right
2. **Seal Consumption**: Close the account to consume the seal
3. **Proof Generation**: Generate Merkle proofs for account state changes

### Commitment Schemes

- **Hash-based**: Default SHA-256 commitments
- **Program-based**: Using Solana program instructions
- **Account State**: Leveraging account data hashes

### Configuration

```toml
[csv-adapter-solana]
network = "devnet"  # devnet, testnet, mainnet, local
rpc_url = "https://api.devnet.solana.com"
csv_program_id = "CsvProgram11111111111111111111111111111111111"
timeout_seconds = 30
max_retries = 3
```

## Usage

### Basic Setup

```rust
use csv_adapter_solana::{SolanaAnchorLayer, SolanaConfig};
use csv_adapter_core::{CsvClient, Chain};

let config = SolanaConfig::for_network(Network::Devnet)
    .with_csv_program_id("YourProgramIdHere");

let client = CsvClient::builder()
    .with_chain(Chain::Solana)
    .with_solana_config(config)
    .build()?;
```

### Creating a Right

```rust
let right = client
    .create_right()
    .chain(Chain::Solana)
    .owner("owner_address")
    .commitment("commitment_hash")
    .await?;
```

### Consuming a Seal

```rust
let seal = client
    .consume_seal()
    .chain(Chain::Solana)
    .seal_ref("seal_account_address")
    .right_id("right_id")
    .await?;
```

## Development

### Local Development

```bash
# Start local Solana validator
solana-test-validator

# Run indexer with local network
cargo run --bin indexer --network local
```

### Testing

```bash
# Run tests
cargo test -p csv-adapter-solana

# Run with specific features
cargo test -p csv-adapter-solana --features rpc
```

## Security Considerations

- Always validate program IDs before use
- Use commitment levels for transaction confirmation
- Implement proper error handling for network failures
- Secure private keys with proper encryption

## Troubleshooting

### Common Issues

1. **RPC Connection Failed**
   - Check network connectivity
   - Verify RPC URL is correct
   - Ensure validator is running

2. **Transaction Not Found**
   - Verify commitment level
   - Check transaction signature
   - Confirm network matches

3. **Account Not Found**
   - Verify account address format
   - Check if account was closed
   - Confirm correct cluster

### Debug Commands

```bash
# Enable debug logging
RUST_LOG=debug

# Check Solana cluster status
solana cluster-version

# Get account info
solana account <ADDRESS>
```

## Integration with Wallet

The Solana adapter integrates seamlessly with the CSV wallet:

- NFT gallery displays Solana NFTs
- Cross-chain transfers support Solana as source/destination
- Account management for Solana keypairs
- Transaction history and status tracking

## Performance

### Optimization Tips

1. **Batch Operations**: Group multiple operations in single transactions
2. **Connection Pooling**: Reuse RPC connections
3. **Caching**: Cache account states and program data
4. **Async Processing**: Use concurrent operations where possible

### Monitoring

Monitor key metrics:

- Transaction success rate
- RPC response times
- Account state changes
- Error rates by type

---

*Last updated: April 2026*
