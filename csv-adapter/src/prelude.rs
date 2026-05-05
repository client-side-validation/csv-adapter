//! Prelude module for ergonomic imports.
//!
//! Import everything you need with a single statement:
//!
//! ```
//! use csv_adapter::prelude::*;
//! ```

// Core types
pub use crate::builder::{ClientBuilder, StoreBackend};
pub use crate::client::{CsvClient, NetworkType};
pub use crate::config::{Config, Network, RpcConfig};
pub use crate::cross_chain::{mint_right_on_chain, is_mint_supported, CrossChainError};
pub use crate::errors::CsvError;
pub use crate::events::Event;
#[cfg(feature = "tokio")]
pub use crate::events::EventStream;
pub use crate::proofs::ProofManager;
pub use crate::rights::RightsManager;
pub use crate::transfers::{TransferBuilder, TransferManager};
pub use crate::wallet::Wallet;

// Re-exports from csv-adapter-core
pub use csv_adapter_core::{
    Commitment, Hash, OwnershipProof, ProofBundle, Right, RightId, SealRef,
};

// Agent-friendly types
pub use csv_adapter_core::agent_types::{Chain, ErrorSuggestion, FixAction};

// Unified result type
pub use crate::Result;

// Event types
pub use crate::events::EventRecvError;
