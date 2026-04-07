//! Error types for CSV adapters

use thiserror::Error;

/// Result type alias for adapter operations
pub type Result<T> = core::result::Result<T, AdapterError>;

/// Error types for CSV adapter operations
#[derive(Error, Debug)]
pub enum AdapterError {
    /// Seal has already been used (replay attack)
    #[error("Seal replay detected: seal {0:?}")]
    SealReplay(String),

    /// Seal is invalid or malformed
    #[error("Invalid seal: {0}")]
    InvalidSeal(String),

    /// Commitment hash mismatch
    #[error("Commitment hash mismatch: expected {expected}, got {actual}")]
    CommitmentMismatch {
        /// Expected commitment hash
        expected: String,
        /// Actual commitment hash
        actual: String,
    },

    /// Inclusion proof verification failed
    #[error("Inclusion proof failed: {0}")]
    InclusionProofFailed(String),

    /// Finality not reached
    #[error("Finality not reached: {0}")]
    FinalityNotReached(String),

    /// Chain reorg invalidated anchor
    #[error("Anchor invalidated by reorg: {0:?}")]
    ReorgInvalid(String),

    /// Network or RPC error
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Transaction publishing failed
    #[error("Publish failed: {0}")]
    PublishFailed(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    SerializationError(String),

    /// Invalid configuration
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    /// Version mismatch
    #[error("Version mismatch: expected {expected}, got {actual}")]
    VersionMismatch {
        /// Expected version
        expected: u8,
        /// Actual version
        actual: u8,
    },

    /// Domain separator mismatch
    #[error("Domain separator mismatch")]
    DomainSeparatorMismatch,

    /// Signature verification failed
    #[error("Signature verification failed: {0}")]
    SignatureVerificationFailed(String),

    /// Generic error with message
    #[error("Adapter error: {0}")]
    Generic(String),
}

impl AdapterError {
    /// Check if this error is a reorg-related error
    pub fn is_reorg(&self) -> bool {
        matches!(self, AdapterError::ReorgInvalid(_))
    }

    /// Check if this error is a replay attack detection
    pub fn is_replay(&self) -> bool {
        matches!(self, AdapterError::SealReplay(_))
    }

    /// Check if this error is a signature verification failure
    pub fn is_signature_error(&self) -> bool {
        matches!(self, AdapterError::SignatureVerificationFailed(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = AdapterError::SealReplay("abc123".to_string());
        assert!(err.to_string().contains("replay"));
    }

    #[test]
    fn test_error_is_reorg() {
        let err = AdapterError::ReorgInvalid("anchor".to_string());
        assert!(err.is_reorg());
    }

    #[test]
    fn test_error_is_replay() {
        let err = AdapterError::SealReplay("seal".to_string());
        assert!(err.is_replay());
    }

    #[test]
    fn test_error_is_signature_error() {
        let err = AdapterError::SignatureVerificationFailed("invalid sig".to_string());
        assert!(err.is_signature_error());
    }

    #[test]
    fn test_error_signature_verification_failed() {
        let err = AdapterError::SignatureVerificationFailed("bad signature".to_string());
        assert!(err.to_string().contains("Signature verification failed"));
    }
}
