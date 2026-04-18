//! Chain adapter implementations.

pub use super::chain_adapter::{ChainAdapter, ChainAdapterExt, ChainResult, ChainError, RpcClient, Wallet};
pub use super::chain_config::{ChainConfig, ChainCapabilities, AccountModel};

// Mock adapters for testing
#[cfg(test)]
pub mod mock;

// New scalable adapters
pub mod bitcoin_adapter;

#[cfg(test)]
pub use mock::MockAdapter;

// Re-export new scalable adapters
pub use bitcoin_adapter::BitcoinAdapter as ScalableBitcoinAdapter;
