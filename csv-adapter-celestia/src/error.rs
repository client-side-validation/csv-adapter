//! Celestia adapter error types

use thiserror::Error;

#[derive(Error, Debug)]
pub enum CelestiaError {
    #[error("RPC error: {0}")]
    RpcError(String),
    #[error("Blob already published: {0}")]
    BlobAlreadyPublished(String),
    #[error("Data availability check failed: {0}")]
    DASFailed(String),
    #[error(transparent)]
    CoreError(#[from] csv_adapter_core::AdapterError),
}

impl CelestiaError {
    /// Whether this error is transient and may be retried
    pub fn is_transient(&self) -> bool {
        match self {
            CelestiaError::RpcError(_) => true,
            CelestiaError::BlobAlreadyPublished(_) => false,
            CelestiaError::DASFailed(_) => true, // May succeed on retry
            CelestiaError::CoreError(_) => false,
        }
    }
}

impl From<CelestiaError> for csv_adapter_core::AdapterError {
    fn from(err: CelestiaError) -> Self {
        match err {
            CelestiaError::CoreError(e) => e,
            CelestiaError::RpcError(msg) => csv_adapter_core::AdapterError::NetworkError(msg),
            CelestiaError::BlobAlreadyPublished(msg) => csv_adapter_core::AdapterError::InvalidSeal(msg),
            CelestiaError::DASFailed(msg) => csv_adapter_core::AdapterError::InclusionProofFailed(msg),
        }
    }
}

pub type CelestiaResult<T> = Result<T, CelestiaError>;
