//! Runtime error types

#![allow(missing_docs)]

use thiserror::Error;

use csv_core::proof::ReplayId;
use csv_core::verified::VerificationFailure;

/// Runtime errors that can occur during transfer execution
#[derive(Error, Debug)]
pub enum RuntimeError {
    /// Storage backend error
    #[error("Storage error: {0}")]
    Storage(String),

    /// Adapter operation failed
    #[error("Adapter error: {0}")]
    Adapter(String),

    /// Transfer not found
    #[error("Transfer not found: {0}")]
    TransferNotFound(String),

    /// Invalid transfer state
    #[error("Invalid transfer state: {0}")]
    InvalidState(String),

    /// Concurrent coordinator conflict
    #[error("Concurrent coordinator conflict: {0}")]
    ConcurrentConflict(String),

    /// Lease conflict — another coordinator holds the lease
    #[error("Lease conflict: {0}")]
    LeaseConflict(String),

    /// Lease expired during mint operation
    #[error("Lease expired: {0}")]
    LeaseExpired(String),

    /// Replay detected — transfer already executed
    #[error("Replay detected: transfer with this ReplayId already exists")]
    ReplayDetected(ReplayId),

    /// Mint failed after insert — needs recovery
    #[error("Mint failed: {cause}")]
    MintFailed { cause: String },

    /// Finality not met for chain
    #[error("Finality not met for chain {chain}: guarantee={guarantee:?}, required={required:?}")]
    FinalityNotMet {
        chain: csv_core::mcp::ChainId,
        guarantee: csv_core::finality_guarantee::FinalityGuarantee,
        required: csv_core::finality_guarantee::FinalityPolicy,
    },

    /// No policy registered for chain
    #[error("No finality policy registered for chain {0}")]
    NoPolicyForChain(csv_core::mcp::ChainId),
}

/// Errors specific to the transfer coordinator
#[derive(Error, Debug)]
pub enum TransferCoordinatorError {
    /// Replay detected — transfer already executed
    #[error("Replay detected: transfer with this ReplayId already exists")]
    ReplayDetected(ReplayId),

    /// Unknown chain — adapter not registered
    #[error("Unknown chain: {0}")]
    UnknownChain(String),

    /// Unsupported operation for chain
    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    /// Verification failed thresholds check
    #[error("Verification failed: {0}")]
    VerificationFailed(VerificationFailure),

    /// Lock on source chain failed
    #[error("Lock failed: {0}")]
    LockFailed(String),

    /// Finality verification failed
    #[error("Finality verification failed: {0}")]
    FinalityFailed(String),

    /// Proof building failed
    #[error("Proof building failed: {0}")]
    ProofBuildFailed(String),

    /// Mint on destination chain failed
    #[error("Mint failed: {0}")]
    MintFailed(String),

    /// Replay database error
    #[error("Replay database error: {0}")]
    ReplayDbError(String),

    /// Runtime error
    #[error("Runtime error: {0}")]
    RuntimeError(String),
}

impl From<RuntimeError> for TransferCoordinatorError {
    fn from(e: RuntimeError) -> Self {
        TransferCoordinatorError::RuntimeError(e.to_string())
    }
}
