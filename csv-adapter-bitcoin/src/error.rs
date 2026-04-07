//! Bitcoin adapter error types

use thiserror::Error;

/// Bitcoin adapter specific errors
#[derive(Error, Debug)]
pub enum BitcoinError {
    /// Bitcoin RPC error
    #[error("RPC error: {0}")]
    RpcError(String),

    /// Transaction not found
    #[error("Transaction not found: {0}")]
    TransactionNotFound(String),

    /// UTXO already spent
    #[error("UTXO already spent: {0}")]
    UTXOSpent(String),

    /// Invalid Merkle proof
    #[error("Invalid Merkle proof: {0}")]
    InvalidMerkleProof(String),

    /// Registry full (max size reached)
    #[error("Registry full: {0}")]
    RegistryFull(String),

    /// Reorg detected
    #[error("Reorg detected at height {height}, depth {depth}")]
    ReorgDetected { height: u64, depth: u64 },

    /// Insufficient confirmations
    #[error("Insufficient confirmations: got {got}, need {need}")]
    InsufficientConfirmations { got: u64, need: u64 },

    /// Wrapper for core adapter errors
    #[error(transparent)]
    CoreError(#[from] csv_adapter_core::AdapterError),
}

impl From<BitcoinError> for csv_adapter_core::AdapterError {
    fn from(err: BitcoinError) -> Self {
        match err {
            BitcoinError::CoreError(e) => e,
            BitcoinError::RpcError(msg) => csv_adapter_core::AdapterError::NetworkError(msg),
            BitcoinError::TransactionNotFound(msg) => csv_adapter_core::AdapterError::Generic(msg),
            BitcoinError::UTXOSpent(msg) => csv_adapter_core::AdapterError::InvalidSeal(msg),
            BitcoinError::InvalidMerkleProof(msg) => {
                csv_adapter_core::AdapterError::InclusionProofFailed(msg)
            }
            BitcoinError::RegistryFull(msg) => csv_adapter_core::AdapterError::Generic(msg),
            BitcoinError::ReorgDetected { height, depth } => {
                csv_adapter_core::AdapterError::ReorgInvalid(format!(
                    "Reorg at height {}, depth {}",
                    height, depth
                ))
            }
            BitcoinError::InsufficientConfirmations { got, need } => {
                csv_adapter_core::AdapterError::FinalityNotReached(format!(
                    "Got {} confirmations, need {}",
                    got, need
                ))
            }
        }
    }
}

impl BitcoinError {
    /// Whether this error is transient and may be retried
    pub fn is_transient(&self) -> bool {
        match self {
            BitcoinError::RpcError(_) => true,
            BitcoinError::TransactionNotFound(_) => true,
            BitcoinError::InsufficientConfirmations { .. } => true,
            BitcoinError::ReorgDetected { .. } => true,
            BitcoinError::UTXOSpent(_) => false,
            BitcoinError::InvalidMerkleProof(_) => false,
            BitcoinError::RegistryFull(_) => false,
            BitcoinError::CoreError(_) => false,
        }
    }
}

/// Result type for Bitcoin adapter operations
pub type BitcoinResult<T> = Result<T, BitcoinError>;
