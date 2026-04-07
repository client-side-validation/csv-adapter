//! Aptos adapter error types
//!
//! This module provides a comprehensive error taxonomy for the Aptos adapter,
//! with chain-specific error variants and recovery guidance.

use thiserror::Error;

/// Comprehensive error types for the Aptos adapter.
///
/// Each variant includes context for debugging and recovery guidance.
#[derive(Error, Debug)]
pub enum AptosError {
    /// Error during RPC communication with Aptos node.
    /// Recovery: Retry with backoff, switch to fallback RPC endpoint.
    #[error("RPC error: {0}")]
    RpcError(String),

    /// Seal resource has already been consumed.
    /// Recovery: This is a fatal error for seal replay attempts. Check seal state.
    #[error("Resource already used: {0}")]
    ResourceUsed(String),

    /// State proof verification failed (Merkle proof against accumulator root).
    /// Recovery: Re-fetch proof from different RPC endpoint, check for reorg.
    #[error("State proof verification failed: {0}")]
    StateProofFailed(String),

    /// Event proof verification failed (event emission verification).
    /// Recovery: Re-verify transaction, check event index and data.
    #[error("Event proof verification failed: {0}")]
    EventProofFailed(String),

    /// Checkpoint certification verification failed.
    /// Recovery: Check validator signatures, verify epoch boundaries.
    #[error("Checkpoint certification failed: {0}")]
    CheckpointFailed(String),

    /// Transaction submission or execution failed.
    /// Recovery: Check transaction simulation error, adjust gas parameters.
    #[error("Transaction failed: {0}")]
    TransactionFailed(String),

    /// Error during serialization/deserialization.
    /// Recovery: This is a programming error. Check data format compatibility.
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Timeout while waiting for transaction confirmation.
    /// Recovery: Resubmit transaction with higher gas, check mempool status.
    #[error("Transaction confirmation timeout after {timeout_ms}ms: {tx_hash}")]
    ConfirmationTimeout {
        tx_hash: String,
        timeout_ms: u64,
    },

    /// Chain reorg detected affecting anchor validity.
    /// Recovery: Re-publish commitment at new chain tip.
    #[error("Chain reorg detected at version {version}: anchor may be invalid")]
    ReorgDetected {
        version: u64,
    },

    /// Network mismatch (e.g., mainnet seal on testnet).
    /// Recovery: Ensure network configuration matches chain ID.
    #[error("Network mismatch: expected chain_id {expected}, got {actual}")]
    NetworkMismatch {
        expected: u64,
        actual: u64,
    },

    /// Core adapter error from csv-adapter-core.
    #[error(transparent)]
    CoreError(#[from] csv_adapter_core::AdapterError),
}

impl AptosError {
    /// Returns true if this error is potentially transient and should be retried.
    pub fn is_transient(&self) -> bool {
        match self {
            AptosError::RpcError(_)
            | AptosError::ConfirmationTimeout { .. }
            | AptosError::TransactionFailed(_) => true,
            AptosError::ResourceUsed(_)
            | AptosError::StateProofFailed(_)
            | AptosError::EventProofFailed(_)
            | AptosError::CheckpointFailed(_)
            | AptosError::SerializationError(_)
            | AptosError::ReorgDetected { .. }
            | AptosError::NetworkMismatch { .. }
            | AptosError::CoreError(_) => false,
        }
    }

    /// Construct an error for transaction timeout
    pub fn timeout(tx_hash: &str, timeout_ms: u64) -> Self {
        AptosError::ConfirmationTimeout {
            tx_hash: tx_hash.to_string(),
            timeout_ms,
        }
    }

    /// Construct an error for chain reorg
    pub fn reorg(version: u64) -> Self {
        AptosError::ReorgDetected { version }
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for AptosError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        AptosError::RpcError(err.to_string())
    }
}

impl From<AptosError> for csv_adapter_core::AdapterError {
    fn from(err: AptosError) -> Self {
        match err {
            AptosError::CoreError(e) => e,
            AptosError::RpcError(msg)
            | AptosError::TransactionFailed(msg) => csv_adapter_core::AdapterError::NetworkError(msg),
            AptosError::ResourceUsed(msg) => csv_adapter_core::AdapterError::InvalidSeal(msg),
            AptosError::StateProofFailed(msg)
            | AptosError::EventProofFailed(msg) => csv_adapter_core::AdapterError::InclusionProofFailed(msg),
            AptosError::CheckpointFailed(msg) => csv_adapter_core::AdapterError::NetworkError(msg),
            AptosError::SerializationError(msg) => csv_adapter_core::AdapterError::InvalidSeal(msg),
            AptosError::ConfirmationTimeout { tx_hash, timeout_ms } => {
                csv_adapter_core::AdapterError::NetworkError(
                    format!("Timeout waiting for tx {} after {}ms", tx_hash, timeout_ms)
                )
            }
            AptosError::ReorgDetected { version } => {
                csv_adapter_core::AdapterError::ReorgInvalid(format!("Reorg at version {}", version))
            }
            aptos_err => csv_adapter_core::AdapterError::NetworkError(format!("{}", aptos_err)),
        }
    }
}

pub type AptosResult<T> = Result<T, AptosError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transient_errors() {
        assert!(AptosError::RpcError("connection refused".to_string()).is_transient());
        assert!(AptosError::ConfirmationTimeout { tx_hash: "abc".to_string(), timeout_ms: 30000 }.is_transient());
        assert!(AptosError::TransactionFailed("out of gas".to_string()).is_transient());
    }

    #[test]
    fn test_non_transient_errors() {
        assert!(!AptosError::ResourceUsed("seal consumed".to_string()).is_transient());
        assert!(!AptosError::StateProofFailed("invalid merkle".to_string()).is_transient());
        assert!(!AptosError::ReorgDetected { version: 100 }.is_transient());
    }

    #[test]
    fn test_error_conversion() {
        let aptos_err = AptosError::StateProofFailed("bad proof".to_string());
        let core_err: csv_adapter_core::AdapterError = aptos_err.into();
        assert!(matches!(core_err, csv_adapter_core::AdapterError::InclusionProofFailed(_)));
    }
}
