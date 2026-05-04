//! Wallet module - Consolidated wallet management.
//!
//! This module combines:
//! - HD wallet generation and metadata (from core/wallet.rs)
//! - Chain account management (from wallet_core.rs)
//! - UI context with persistence (from context/wallet.rs)
//! - Storage management (from storage.rs)

pub mod account;
pub mod context;
pub mod data;
pub mod hd;
pub mod metadata;
pub mod storage;

// Re-export commonly used types for convenience
pub use account::ChainAccount;
pub use context::WalletContext;
pub use data::WalletData;
pub use hd::{ExtendedWallet, BitcoinNetwork};
pub use metadata::WalletMetadata;
pub use storage::WalletStorage;
