//! Ethereum adapter error types

use thiserror::Error;

/// Ethereum adapter specific errors
#[derive(Error, Debug)]
pub enum EthereumError {
    /// Ethereum RPC error
    #[error("RPC error: {0}")]
    RpcError(String),

    /// Storage slot already used
    #[error("Storage slot already used: {0}")]
    SlotUsed(String),

    /// Invalid receipt proof
    #[error("Invalid receipt proof: {0}")]
    InvalidReceiptProof(String),

    /// Reorg detected
    #[error("Reorg detected at block {block}, depth {depth}")]
    ReorgDetected { block: u64, depth: u64 },

    /// Insufficient confirmations
    #[error("Insufficient confirmations: got {got}, need {need}")]
    InsufficientConfirmations { got: u64, need: u64 },

    /// Wrapper for core adapter errors
    #[error(transparent)]
    CoreError(#[from] csv_adapter_core::AdapterError),
}

impl EthereumError {
    /// Whether this error is transient and may be retried
    pub fn is_transient(&self) -> bool {
        match self {
            EthereumError::RpcError(_) => true,
            EthereumError::InsufficientConfirmations { .. } => true,
            EthereumError::ReorgDetected { .. } => true,
            EthereumError::SlotUsed(_) => false,
            EthereumError::InvalidReceiptProof(_) => false,
            EthereumError::CoreError(_) => false,
        }
    }
}

impl From<EthereumError> for csv_adapter_core::AdapterError {
    fn from(err: EthereumError) -> Self {
        match err {
            EthereumError::CoreError(e) => e,
            EthereumError::RpcError(msg) => csv_adapter_core::AdapterError::NetworkError(msg),
            EthereumError::SlotUsed(msg) => csv_adapter_core::AdapterError::InvalidSeal(msg),
            EthereumError::InvalidReceiptProof(msg) => csv_adapter_core::AdapterError::InclusionProofFailed(msg),
            EthereumError::ReorgDetected { block, depth } => csv_adapter_core::AdapterError::ReorgInvalid(
                format!("Reorg at block {}, depth {}", block, depth)
            ),
            EthereumError::InsufficientConfirmations { got, need } => {
                csv_adapter_core::AdapterError::FinalityNotReached(
                    format!("Got {} confirmations, need {}", got, need)
                )
            }
        }
    }
}

/// Result type for Ethereum adapter operations
pub type EthereumResult<T> = Result<T, EthereumError>;
