//! Page components - modular structure.
//!
//! Organized into feature modules:
//! - `accounts` - Dashboard and account management
//! - `rights` - Rights management (list, create, show, transfer, consume)
//! - `proofs` - Proof generation and verification
//! - `cross_chain` - Cross-chain transfers
//! - `contracts` - Contract deployment and management
//! - `seals` - Seal creation and verification
//! - `tests` - Test scenarios
//! - `validate` - Validation utilities
//! - `transactions` - Transaction history
//! - `settings` - Application settings
//! - `common` - Shared UI helpers
//!
//! Note: During the migration from old_pages.rs, some modules re-export
//! components from old_pages. These will be fully migrated incrementally.

// Common UI helpers (fully migrated)
pub mod common;

// NFT and Wallet pages (already separate files)
pub mod nft_page;
pub mod wallet_page;

// Feature modules (re-exporting from old_pages during migration)
pub mod accounts;
pub mod contracts;
pub mod cross_chain;
pub mod proofs;
pub mod rights;
pub mod seals;
pub mod settings;
pub mod tests;
pub mod transactions;
pub mod validate;

// Re-exports from nft_page and wallet_page (standalone files)
pub use nft_page::{NftCollections, NftDetail, NftGallery};
pub use wallet_page::WalletPage;

// Re-exports from accounts module
pub use accounts::{AccountTransactions, Dashboard};

// Re-exports from rights module (already migrated)
pub use rights::{ConsumeRight, CreateRight, RightJourney, Rights, ShowRight, TransferRight};

// Re-exports from proofs module
pub use proofs::{GenerateProof, Proofs, VerifyCrossChainProof, VerifyProof};

// Re-exports from cross_chain module
pub use cross_chain::{CrossChain, CrossChainRetry, CrossChainStatus, CrossChainTransfer};

// Re-exports from contracts module
pub use contracts::{AddContract, ContractStatus, Contracts, DeployContract};

// Re-exports from seals module
pub use seals::{ConsumeSeal, CreateSeal, Seals, VerifySeal};

// Re-exports from tests module
pub use tests::{RunScenario, RunTests, Test};

// Re-exports from validate module
pub use validate::{
    Validate, ValidateCommitmentChain, ValidateConsignment, ValidateProof, ValidateSeal,
};

// Re-exports from transactions module
pub use transactions::{TransactionDetail, Transactions};

// Re-exports from settings module
pub use settings::Settings;

// Common UI helpers - re-export everything from common module for convenience

// Migration complete: old_pages.rs has been removed
