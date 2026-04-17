//! Chain adapter implementations.

use super::chain_adapter::{ChainAdapter, ChainConfig, ChainResult, ChainError};

// Mock adapters for testing
#[cfg(test)]
pub mod mock;

// Real chain adapters (feature-gated)
#[cfg(feature = "bitcoin")]
pub mod bitcoin;

#[cfg(feature = "ethereum")]
pub mod ethereum;

#[cfg(feature = "solana")]
pub mod solana;

#[cfg(feature = "sui")]
pub mod sui;

#[cfg(feature = "aptos")]
pub mod aptos;

// New scalable adapters
pub mod bitcoin_adapter;

// Re-export adapters
#[cfg(feature = "bitcoin")]
pub use bitcoin::BitcoinAdapter;

#[cfg(feature = "ethereum")]
pub use ethereum::EthereumAdapter;

#[cfg(feature = "solana")]
pub use solana::SolanaAdapter;

#[cfg(feature = "sui")]
pub use sui::SuiAdapter;

#[cfg(feature = "aptos")]
pub use aptos::AptosAdapter;

#[cfg(test)]
pub use mock::MockAdapter;

// Re-export new scalable adapters
pub use bitcoin_adapter::BitcoinAdapter as ScalableBitcoinAdapter;
