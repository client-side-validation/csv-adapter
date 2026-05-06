//! Cross-chain operations for CSV rights.
//!
//! This module provides functionality for minting rights on destination chains
//! as part of cross-chain transfers.

use crate::CsvError;
use csv_adapter_core::{Chain, Hash};

/// Result type for cross-chain operations.
pub type CrossChainResult<T> = Result<T, CrossChainError>;

/// Error type for cross-chain operations.
#[derive(Debug, thiserror::Error)]
pub enum CrossChainError {
    /// The requested chain is not supported.
    #[error("Chain not supported: {0}")]
    ChainNotSupported(String),

    /// RPC operation failed.
    #[error("RPC error: {0}")]
    RpcError(String),

    /// Invalid configuration.
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Missing feature for the operation.
    #[error("Feature not enabled: {0}")]
    FeatureNotEnabled(String),

    /// Underlying adapter error.
    #[error("Adapter error: {0}")]
    AdapterError(String),
}

impl From<CrossChainError> for CsvError {
    fn from(e: CrossChainError) -> Self {
        CsvError::Generic(format!("Cross-chain error: {}", e))
    }
}

/// Mint a right on the destination chain as part of a cross-chain transfer.
///
/// # Arguments
///
/// * `chain` - Destination chain to mint on.
/// * `rpc_url` - RPC endpoint URL for the destination chain.
/// * `contract` - Contract/package address on the destination chain.
/// * `private_key` - Private key for signing (hex-encoded, with or without 0x prefix).
/// * `right_id` - Unique identifier of the right being minted.
/// * `commitment` - Commitment hash for the right.
/// * `source_chain` - Identifier of the source chain.
/// * `source_seal_ref` - Reference to the seal on the source chain.
///
/// # Returns
///
/// The transaction hash/digest of the mint transaction.
///
/// # Errors
///
/// Returns `CrossChainError` if:
/// - The chain is not supported
/// - The RPC call fails
/// - The transaction cannot be built or submitted
pub async fn mint_right_on_chain(
    chain: Chain,
    rpc_url: &str,
    contract: &str,
    private_key: &str,
    right_id: Hash,
    commitment: Hash,
    source_chain: u8,
    source_seal_ref: Hash,
) -> CrossChainResult<String> {
    match chain {
        #[cfg(all(feature = "sui", feature = "rpc"))]
        Chain::Sui => {
            use csv_adapter_sui::mint::mint_right;
            
            mint_right(
                rpc_url,
                contract,
                private_key,
                right_id,
                commitment,
                source_chain,
                source_seal_ref,
            )
            .await
            .map_err(|e| CrossChainError::AdapterError(format!("{:?}", e)))
        }
        
        #[cfg(not(all(feature = "sui", feature = "rpc")))]
        Chain::Sui => {
            // Suppress unused variable warnings when feature is not enabled
            let _ = (rpc_url, contract, private_key, right_id, commitment, source_chain, source_seal_ref);
            Err(CrossChainError::FeatureNotEnabled(
                "Sui cross-chain mint requires 'sui' and 'rpc' features.".to_string()
            ))
        }
        
        #[cfg(feature = "solana")]
        Chain::Solana => {
            use csv_adapter_solana::mint::mint_right_from_hex_key;
            // Solana requires state_root parameter - use zero hash as default
            let state_root = Hash::new([0u8; 32]);
            
            mint_right_from_hex_key(
                rpc_url,
                contract,
                private_key,
                right_id,
                commitment,
                state_root,
                source_chain,
                source_seal_ref,
            )
            .map_err(|e| CrossChainError::AdapterError(format!("{:?}", e)))
        }
        
        #[cfg(not(feature = "solana"))]
        Chain::Solana => {
            // Suppress unused variable warnings when feature is not enabled
            let _ = (rpc_url, contract, private_key, right_id, commitment, source_chain, source_seal_ref);
            Err(CrossChainError::FeatureNotEnabled(
                "Solana cross-chain mint requires 'solana' feature.".to_string()
            ))
        }
        
        _ => {
            // Suppress unused variable warnings for unsupported chains
            let _ = (rpc_url, contract, private_key, right_id, commitment, source_chain, source_seal_ref);
            Err(CrossChainError::ChainNotSupported(format!(
                "Cross-chain mint not available for {:?}",
                chain
            )))
        }
    }
}

/// Check if cross-chain mint is supported for a given chain.
pub fn is_mint_supported(chain: Chain) -> bool {
    match chain {
        #[cfg(all(feature = "sui", feature = "rpc"))]
        Chain::Sui => true,
        #[cfg(feature = "solana")]
        Chain::Solana => true,
        _ => false,
    }
}
