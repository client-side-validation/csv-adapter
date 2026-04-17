# Unified Chain Integration Guide

This guide provides a standardized pattern for adding new blockchain support to the CSV adapter ecosystem. Following this pattern ensures consistent integration across all modules and makes maintenance easier.

## Overview

The CSV adapter supports multiple modules that need to be updated when adding a new chain:
- `csv-adapter-core` - Core types and protocol definitions
- `csv-adapter-{chain}` - Chain-specific adapter package
- `csv-adapter` - Main meta-crate with unified interface
- `csv-cli` - Command-line interface
- `csv-wallet` - Web wallet application
- `csv-explorer` - Explorer and indexer system

## Integration Pattern

### Step 1: Core Protocol Definition

Add the new chain to the core `Chain` enum in `csv-adapter-core/src/protocol_version.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[non_exhaustive]
pub enum Chain {
    Bitcoin,
    Ethereum,
    Sui,
    Aptos,
    Solana,
    NewChain,  // Add your chain here
}
```

### Step 2: Create Chain Adapter Package

Create a new package following the naming pattern `csv-adapter-{chain}`:

```bash
cargo new --lib csv-adapter-{chain}
```

#### Package Structure Template

```
csv-adapter-{chain}/
  Cargo.toml
  src/
    lib.rs
    adapter.rs
    config.rs
    error.rs
    rpc.rs
    types.rs
    wallet.rs
```

#### Cargo.toml Template

```toml
[package]
name = "csv-adapter-{chain}"
version = "0.2.0"
edition = "2021"
description = "{Chain} adapter for CSV (Client-Side Validation)"
license = "MIT OR Apache-2.0"
authors = ["Amin Razavi, Qwen3"]
repository = "https://github.com/zorvan/csv-adapter"
homepage = "https://github.com/zorvan/csv-adapter"
documentation = "https://docs.rs/csv-adapter-{chain}"
readme = "README.md"
keywords = ["{chain}", "blockchain", "cryptography", "csv-validation"]
categories = ["cryptography::cryptocurrencies"]
publish = true
rust-version = "1.75"

[package.metadata.docs.rs]
all-features = true
rustdoc-args = ["--cfg", "docsrs"]

[dependencies]
csv-adapter-core = { version = "0.2.0", path = "../csv-adapter-core" }
csv-adapter-store = { version = "0.2.0", path = "../csv-adapter-store", optional = true }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
thiserror = "1.0"
hex = "0.4"
sha2 = "0.10"
# Add chain-specific dependencies here

[features]
default = []
rpc = ["csv-adapter-store"]
production = ["rpc"]

[dev-dependencies]
rand = "0.8"
tempfile = "3"
```

### Step 3: Update Workspace Members

Add the new chain to the root `Cargo.toml`:

```toml
[workspace]
members = [
    "csv-adapter-core",
    "csv-adapter-bitcoin",
    "csv-adapter-ethereum",
    "csv-adapter-sui",
    "csv-adapter-aptos",
    "csv-adapter-solana",
    "csv-adapter-{chain}",  # Add here
    "csv-adapter-store",
    "csv-adapter",
    "csv-cli",
    "csv-wallet",
]
```

### Step 4: Update Main CSV Adapter

Add the new chain to `csv-adapter/Cargo.toml`:

```toml
# Chain adapters -- optional, enabled via feature flags
csv-adapter-bitcoin = { version = "0.2.0", path = "../csv-adapter-bitcoin", optional = true }
csv-adapter-ethereum = { version = "0.2.0", path = "../csv-adapter-ethereum", optional = true }
csv-adapter-sui = { version = "0.2.0", path = "../csv-adapter-sui", optional = true }
csv-adapter-aptos = { version = "0.2.0", path = "../csv-adapter-aptos", optional = true }
csv-adapter-solana = { version = "0.2.0", path = "../csv-adapter-solana", optional = true }
csv-adapter-{chain} = { version = "0.2.0", path = "../csv-adapter-{chain}", optional = true }  # Add here
```

Update the features section:

```toml
# Chain features
bitcoin = ["dep:csv-adapter-bitcoin", "dep:bitcoin", "dep:bip32"]
ethereum = ["dep:csv-adapter-ethereum"]
sui = ["dep:csv-adapter-sui"]
aptos = ["dep:csv-adapter-aptos"]
solana = ["dep:csv-adapter-solana"]
{chain} = ["dep:csv-adapter-{chain}"]  # Add here
all-chains = ["bitcoin", "ethereum", "sui", "aptos", "solana", "{chain}"]  # Add here
```

Update the builder in `csv-adapter/src/builder.rs`:

```rust
/// Enable all supported chains (requires `all-chains` feature).
pub fn with_all_chains(self) -> Self {
    self.with_chain(Chain::Bitcoin)
        .with_chain(Chain::Ethereum)
        .with_chain(Chain::Sui)
        .with_chain(Chain::Aptos)
        .with_chain(Chain::Solana)
        .with_chain(Chain::{Chain})  # Add here
}
```

Add feature validation:

```rust
fn check_chain_feature(chain: Chain) -> Result<(), CsvError> {
    match chain {
        // ... existing chains
        Chain::{Chain} => {
            #[cfg(not(feature = "{chain}"))]
            return Err(CsvError::BuilderError(
                "{Chain} adapter requires the '{chain}' feature flag".to_string(),
            ));
            #[cfg(feature = "{chain}")]
            Ok(())
        }
        _ => Ok(()),
    }
}
```

### Step 5: Update CLI Package

Add to `csv-cli/Cargo.toml`:

```toml
csv-adapter-bitcoin = { version = "0.2.0", path = "../csv-adapter-bitcoin", features = ["signet-rest"] }
csv-adapter-ethereum = { version = "0.2.0", path = "../csv-adapter-ethereum" }
csv-adapter-sui = { version = "0.2.0", path = "../csv-adapter-sui" }
csv-adapter-aptos = { version = "0.2.0", path = "../csv-adapter-aptos" }
csv-adapter-solana = { version = "0.2.0", path = "../csv-adapter-solana" }
csv-adapter-{chain} = { version = "0.2.0", path = "../csv-adapter-{chain}" }  # Add here
```

Update RPC features:

```toml
[features]
default = []
rpc = [
    "csv-adapter-bitcoin/rpc",
    "csv-adapter-ethereum/rpc",
    "csv-adapter-sui/rpc",
    "csv-adapter-aptos/rpc",
    "csv-adapter-solana/rpc",
    "csv-adapter-{chain}/rpc",  # Add here
]
```

### Step 6: Update Wallet Package

Add to `csv-wallet/Cargo.toml`:

```toml
# Core CSV Adapter types
csv-adapter-core = { path = "../csv-adapter-core" }
csv-adapter-solana = { path = "../csv-adapter-solana", optional = true }
csv-adapter-{chain} = { path = "../csv-adapter-{chain}", optional = true }  # Add here
```

Add features:

```toml
[features]
default = []
solana = ["dep:csv-adapter-solana"]
{chain} = ["dep:csv-adapter-{chain}"]  # Add here
```

### Step 7: Update Explorer Package

Add to `csv-explorer/Cargo.toml` workspace dependencies:

```toml
csv-adapter-core = { path = "../csv-adapter-core" }
csv-adapter-solana = { path = "../csv-adapter-solana" }
csv-adapter-{chain} = { path = "../csv-adapter-{chain}" }  # Add here
```

Update indexer features in `csv-explorer/indexer/Cargo.toml`:

```toml
[features]
default = []
bitcoin = []
ethereum = []
sui = []
aptos = []
solana = []
{chain} = []  # Add here
all-chains = ["bitcoin", "ethereum", "sui", "aptos", "solana", "{chain}"]  # Add here
```

### Step 8: Create Chain Configuration

Create `chains/{chain}.toml`:

```toml
# {Chain} Chain Configuration

chain_id = "{chain}"
chain_name = "{Chain}"
default_network = "mainnet"
rpc_endpoints = [
    "https://rpc.{chain}.example.com"
]
program_id = "CsvProgram{Chain}11111111111111111111111111111"
block_explorer_urls = [
    "https://explorer.{chain}.example.com"
]

[custom_settings]
supports_nfts = true
supports_smart_contracts = true
account_model = "Account"  # or "UTXO", "Object"
confirmation_blocks = 6
max_batch_size = 100
supported_networks = ["mainnet", "testnet"]

# Chain-specific settings
[custom_settings.{chain}]
# Add chain-specific configuration here
```

### Step 9: Implementation Requirements

#### Core Adapter Implementation

Implement the `AnchorLayer` trait in your adapter:

```rust
use csv_adapter_core::traits::AnchorLayer;

pub struct {Chain}Adapter {
    // Adapter implementation
}

impl AnchorLayer for {Chain}Adapter {
    // Implement required methods
    fn create_seal(&self, ...) -> Result<SealRef, AdapterError> { ... }
    fn publish_anchor(&self, ...) -> Result<AnchorRef, AdapterError> { ... }
    // ... other methods
}
```

#### RPC Client Implementation

Implement chain-specific RPC client:

```rust
pub struct {Chain}RpcClient {
    // RPC client implementation
}

impl {Chain}RpcClient {
    pub async fn send_transaction(&self, tx: &[u8]) -> Result<String, ChainError> { ... }
    pub async fn get_transaction(&self, hash: &str) -> Result<serde_json::Value, ChainError> { ... }
    // ... other RPC methods
}
```

#### Wallet Implementation

Implement chain-specific wallet:

```rust
pub struct {Chain}Wallet {
    // Wallet implementation
}

impl {Chain}Wallet {
    pub fn from_private_key(&self, private_key: &str) -> ChainResult<()> { ... }
    pub async fn sign_transaction(&self, data: &[u8]) -> ChainResult<Vec<u8>> { ... }
    // ... other wallet methods
}
```

### Step 10: Testing

Create comprehensive tests for your chain adapter:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_{chain}_adapter_basic() {
        // Test basic adapter functionality
    }
    
    #[tokio::test]
    async fn test_{chain}_rpc_client() {
        // Test RPC client
    }
    
    #[tokio::test]
    async fn test_{chain}_wallet() {
        // Test wallet functionality
    }
}
```

### Step 11: Documentation

Create documentation in `docs/implementation/{chain}/`:

- `README.md` - Overview and getting started
- `CONFIG.md` - Configuration guide
- `RPC.md` - RPC client documentation
- `EXAMPLES.md` - Usage examples

## Validation Checklist

Before submitting your new chain integration, verify:

- [ ] Core `Chain` enum updated
- [ ] Adapter package created with correct structure
- [ ] Workspace members updated
- [ ] Main csv-adapter updated
- [ ] CLI package updated
- [ ] Wallet package updated
- [ ] Explorer package updated
- [ ] Chain configuration created
- [ ] All feature flags work correctly
- [ ] Builder pattern works
- [ ] RPC client implemented
- [ ] Wallet implemented
- [ ] Tests written and passing
- - Documentation created

## Automation Script

To streamline this process, you can create a shell script that automates the boilerplate:

```bash
#!/bin/bash
# add-chain.sh <chain-name> <Chain-Name>

CHAIN=$1
CHAIN_NAME=$2

# Create adapter package
cargo new --lib csv-adapter-$CHAIN

# Update workspace members
sed -i 's/"csv-adapter-solana",/"csv-adapter-solana",\n    "csv-adapter-'$CHAIN'",/' Cargo.toml

# ... continue with other automated updates
```

## Benefits of Unified Pattern

1. **Consistency**: All chains follow the same integration pattern
2. **Maintainability**: Easy to update across all modules
3. **Testing**: Standardized testing approach
4. **Documentation**: Consistent documentation structure
5. **Automation**: Boilerplate can be automated
6. **Scalability**: Easy to add new chains in the future

## Examples

See existing implementations for reference:
- `csv-adapter-bitcoin` - UTXO-based chains
- `csv-adapter-ethereum` - Account-based EVM chains
- `csv-adapter-solana` - Account-based Solana chains
- `csv-adapter-sui` - Object-based chains
- `csv-adapter-aptos` - Resource-based chains

Each demonstrates different blockchain paradigms while following the same integration pattern.
