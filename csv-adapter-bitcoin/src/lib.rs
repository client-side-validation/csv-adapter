//! Bitcoin Adapter for CSV (Client-Side Validation)
//!
//! This adapter implements the AnchorLayer trait for Bitcoin,
//! using UTXOs as single-use seals and Tapret/Opret for commitment publication.

#![warn(missing_docs)]
#![allow(missing_docs)]
#![allow(dead_code)]

pub mod adapter;
pub mod bip341;
pub mod config;
pub mod error;
pub mod proofs;
pub mod proofs_new;
pub mod rpc;
pub mod seal;
pub mod signatures;
pub mod spv;
pub mod tapret;
pub mod tx_builder;
pub mod types;
pub mod wallet;
pub mod testnet_deploy;

#[cfg(feature = "rpc")]
pub mod real_rpc;

pub use adapter::BitcoinAnchorLayer;
pub use config::BitcoinConfig;
pub use types::{BitcoinSealRef, BitcoinAnchorRef, BitcoinInclusionProof, BitcoinFinalityProof};
pub use rpc::BitcoinRpc;
pub use tapret::{TapretCommitment, TapretError, OpretCommitment, mine_tapret_nonce, TAPRET_SCRIPT_SIZE};
pub use tx_builder::{CommitmentTxBuilder, CommitmentData, TxBuilderError};
pub use wallet::{SealWallet, Bip86Path, DerivedTaprootKey, WalletUtxo, WalletError};
pub use spv::SpvVerifier;
pub use bip341::{derive_output_key, TaprootOutput, Bip341Error, generate_test_keypair};

#[cfg(feature = "rpc")]
pub use real_rpc::real_rpc::RealBitcoinRpc;
