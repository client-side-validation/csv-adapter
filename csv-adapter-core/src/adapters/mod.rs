//! Chain adapter implementations.
//!
//! This module provides the core adapter traits and configuration types.
//! The actual chain-specific implementations are in their respective crates:
//! - `csv-adapter-bitcoin` - Bitcoin implementation
//! - `csv-adapter-solana` - Solana implementation
//! - `csv-adapter-aptos` - Aptos implementation
//! - `csv-adapter-sui` - Sui implementation
//! - `csv-adapter-ethereum` - Ethereum implementation

pub use super::chain_adapter::{
    ChainAdapter, ChainAdapterExt, ChainError, ChainResult, RpcClient, Wallet,
};
pub use super::chain_config::{AccountModel, ChainCapabilities, ChainConfig};

// Test adapters for testing
#[cfg(test)]
pub mod test;

#[cfg(test)]
pub use test::MockAdapter;
