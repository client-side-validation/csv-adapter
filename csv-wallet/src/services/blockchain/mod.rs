//! Real blockchain service for web wallet.
//! Provides contract deployment, cross-chain transfers, and proof generation.
//!
//! Uses native signing with imported private keys - no browser wallet required.
//!
//! # Module Structure
//!
//! - `types` - Error types, transaction receipts, proof data structures
//! - `wallet` - NativeWallet and BrowserWallet abstractions
//! - `config` - BlockchainConfig for RPC endpoints
//! - `service` - Main BlockchainService orchestrator
//! - `signer` - Transaction signing per chain
//! - `submitter` - Transaction submission/broadcasting
//! - `estimator` - Gas/fee estimation per chain

// Modular components
pub mod types;
pub mod wallet;
pub mod config;
pub mod service;
pub mod signer;
pub mod submitter;
pub mod estimator;

// Re-exports from modules
pub use types::{
    BlockchainError,
    ContractDeployment, ContractType,
};
pub use wallet::{NativeWallet, BrowserWallet, WalletType};
pub use wallet::wallet_connection;
pub use config::BlockchainConfig;
#[allow(unused_imports)]
pub use signer::TransactionSigner;
#[allow(unused_imports)]
pub use submitter::TransactionSubmitter;
#[allow(unused_imports)]
pub use estimator::{FeeEstimator, FeePriority};

// Re-export main service
pub use service::BlockchainService;
