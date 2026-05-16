//! Solana Adapter for CSV (Client-Side Validation)
//!
//! This adapter implements the SealProtocol trait for Solana,
//! using program accounts as single-use seals and program instructions for commitment publication.

#![warn(missing_docs)]
#![allow(missing_docs)]
#![allow(dead_code)]

pub mod backend;
pub mod config;
pub mod error;
pub mod mint;
pub mod ops;
pub mod program;
pub mod proofs;
pub mod rpc;
pub mod seal;
pub mod seal_protocol;
pub mod sync_coordinator;
pub mod types;
pub mod verifier;
pub mod wallet;

pub use backend::{SolanaRpcClient, SolanaWallet, create_solana_adapter};
pub use config::{Network, SolanaConfig};
pub use error::{SolanaError, SolanaResult};
pub use mint::mint_sanad_from_hex_key;
pub use rpc::SolanaRpc;
pub use seal_protocol::SolanaSealProtocol;
pub use types::{SolanaCommitAnchor, SolanaFinalityProof, SolanaInclusionProof, SolanaSealPoint};
pub use wallet::{ProgramWallet, WalletError};

// Ops exports
pub use ops::SolanaBackend;

#[cfg(feature = "rpc")]
pub mod node;

#[cfg(feature = "rpc")]
pub use node::real_rpc_impl::SolanaNode;
