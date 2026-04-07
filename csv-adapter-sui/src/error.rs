//! Sui adapter error types
//!
//! This module provides a comprehensive error taxonomy for the Sui adapter,
//! with chain-specific error variants and recovery guidance.

use thiserror::Error;

/// Comprehensive error types for the Sui adapter.
///
/// Each variant includes context for debugging and recovery guidance.
#[derive(Error, Debug)]
pub enum SuiError {
    /// Error during RPC communication with Sui node.
    /// Recovery: Retry with backoff, switch to fallback RPC endpoint.
    #[error("RPC error: {0}")]
    RpcError(String),

    /// Seal object has already been consumed.
    /// Recovery: This is a fatal error for seal replay attempts. Check object state.
    #[error("Object already used: {0}")]
    ObjectUsed(String),

    /// State proof verification failed (object existence/ownership).
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
    #[error("Transaction confirmation timeout after {timeout_ms}ms: {tx_digest}")]
    ConfirmationTimeout {
        tx_digest: String,
        timeout_ms: u64,
    },

    /// Chain reorg detected affecting anchor validity.
    /// Recovery: Re-publish commitment at new chain tip.
    #[error("Chain reorg detected at checkpoint {checkpoint}: anchor may be invalid")]
    ReorgDetected {
        checkpoint: u64,
    },

    /// Network mismatch (e.g., mainnet seal on testnet).
    /// Recovery: Ensure network configuration matches chain ID.
    #[error("Network mismatch: expected chain_id {expected}, got {actual}")]
    NetworkMismatch {
        expected: String,
        actual: String,
    },

    /// Core adapter error from csv-adapter-core.
    #[error(transparent)]
    CoreError(#[from] csv_adapter_core::AdapterError),
}

impl SuiError {
    /// Returns true if this error is potentially transient and should be retried.
    pub fn is_transient(&self) -> bool {
        match self {
            SuiError::RpcError(_)
            | SuiError::ConfirmationTimeout { .. }
            | SuiError::TransactionFailed(_) => true,
            SuiError::ObjectUsed(_)
            | SuiError::StateProofFailed(_)
            | SuiError::EventProofFailed(_)
            | SuiError::CheckpointFailed(_)
            | SuiError::SerializationError(_)
            | SuiError::ReorgDetected { .. }
            | SuiError::NetworkMismatch { .. }
            | SuiError::CoreError(_) => false,
        }
    }

    /// Construct an error for transaction timeout
    pub fn timeout(tx_digest: &str, timeout_ms: u64) -> Self {
        SuiError::ConfirmationTimeout {
            tx_digest: tx_digest.to_string(),
            timeout_ms,
        }
    }

    /// Construct an error for chain reorg
    pub fn reorg(checkpoint: u64) -> Self {
        SuiError::ReorgDetected { checkpoint }
    }
}

impl From<Box<dyn std::error::Error + Send + Sync>> for SuiError {
    fn from(err: Box<dyn std::error::Error + Send + Sync>) -> Self {
        SuiError::RpcError(err.to_string())
    }
}

impl From<SuiError> for csv_adapter_core::AdapterError {
    fn from(err: SuiError) -> Self {
        match err {
            SuiError::CoreError(e) => e,
            SuiError::RpcError(msg)
            | SuiError::TransactionFailed(msg) => csv_adapter_core::AdapterError::NetworkError(msg),
            SuiError::ObjectUsed(msg) => csv_adapter_core::AdapterError::InvalidSeal(msg),
            SuiError::StateProofFailed(msg)
            | SuiError::EventProofFailed(msg) => csv_adapter_core::AdapterError::InclusionProofFailed(msg),
            SuiError::CheckpointFailed(msg) => csv_adapter_core::AdapterError::NetworkError(msg),
            SuiError::SerializationError(msg) => csv_adapter_core::AdapterError::InvalidSeal(msg),
            SuiError::ConfirmationTimeout { tx_digest, timeout_ms } => {
                csv_adapter_core::AdapterError::NetworkError(
                    format!("Timeout waiting for tx {} after {}ms", tx_digest, timeout_ms)
                )
            }
            SuiError::ReorgDetected { checkpoint } => {
                csv_adapter_core::AdapterError::ReorgInvalid(format!("Reorg at checkpoint {}", checkpoint))
            }
            sui_err => csv_adapter_core::AdapterError::NetworkError(format!("{}", sui_err)),
        }
    }
}

/// Result type for Sui adapter operations
pub type SuiResult<T> = Result<T, SuiError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transient_errors() {
        assert!(SuiError::RpcError("connection refused".to_string()).is_transient());
        assert!(SuiError::ConfirmationTimeout { tx_digest: "abc".to_string(), timeout_ms: 30000 }.is_transient());
        assert!(SuiError::TransactionFailed("execution failed".to_string()).is_transient());
    }

    #[test]
    fn test_non_transient_errors() {
        assert!(!SuiError::ObjectUsed("object consumed".to_string()).is_transient());
        assert!(!SuiError::StateProofFailed("invalid proof".to_string()).is_transient());
        assert!(!SuiError::ReorgDetected { checkpoint: 100 }.is_transient());
    }

    #[test]
    fn test_error_conversion() {
        let sui_err = SuiError::StateProofFailed("bad proof".to_string());
        let core_err: csv_adapter_core::AdapterError = sui_err.into();
        assert!(matches!(core_err, csv_adapter_core::AdapterError::InclusionProofFailed(_)));
    }
}
