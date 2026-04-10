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
pub mod testnet_deploy;
pub mod tx_builder;
pub mod types;
pub mod wallet;

#[cfg(feature = "rpc")]
pub mod real_rpc;

pub use adapter::BitcoinAnchorLayer;
pub use bip341::{derive_output_key, generate_test_keypair, Bip341Error, TaprootOutput};
pub use config::{BitcoinConfig, Network};
pub use rpc::BitcoinRpc;
pub use spv::SpvVerifier;
pub use tapret::{
    mine_tapret_nonce, OpretCommitment, TapretCommitment, TapretError, TAPRET_SCRIPT_SIZE,
};
pub use tx_builder::{CommitmentData, CommitmentTxBuilder, TxBuilderError};
pub use types::{BitcoinAnchorRef, BitcoinFinalityProof, BitcoinInclusionProof, BitcoinSealRef};
pub use wallet::{Bip86Path, DerivedTaprootKey, SealWallet, WalletError, WalletUtxo};

#[cfg(feature = "rpc")]
pub use real_rpc::real_rpc::{RealBitcoinRpc, TxInfo};
