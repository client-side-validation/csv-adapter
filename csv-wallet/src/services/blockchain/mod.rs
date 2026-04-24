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
//! - `service` - Main BlockchainService implementation

// Modular components
pub mod types;
pub mod wallet;
pub mod config;
pub mod service;

// Re-exports from modules
pub use types::{
    BlockchainError, TransactionReceipt, TransactionStatus,
    CrossChainStatus, CrossChainProof, ProofData,
    ContractDeployment, ContractType,
    CrossChainTransferResult, ContractDeployments,
};
pub use wallet::{NativeWallet, BrowserWallet, WalletType};
pub use wallet::wallet_connection;
pub use config::BlockchainConfig;

// Re-export main service
pub use service::BlockchainService;
