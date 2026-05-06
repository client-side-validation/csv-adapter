//! Error types for CSV adapters

use thiserror::Error;
use crate::mcp::{HasErrorSuggestion, FixAction, error_codes};

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

    /// Seal was never created on-chain (caught fake seal IDs)
    #[error("Seal {0} has no on-chain anchor — was it created via a real chain adapter?")]
    SealNotAnchored(String),

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

    /// Invalid input parameters
    #[error("Invalid input: {0}")]
    InvalidInput(String),

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

    /// Check if this error indicates a seal without on-chain anchor
    pub fn is_seal_not_anchored(&self) -> bool {
        matches!(self, AdapterError::SealNotAnchored(_))
    }

    /// Check if this error is a signature verification failure
    pub fn is_signature_error(&self) -> bool {
        matches!(self, AdapterError::SignatureVerificationFailed(_))
    }

    /// Check if this error is transient (can be retried)
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            AdapterError::NetworkError(_) |
            AdapterError::PublishFailed(_) |
            AdapterError::FinalityNotReached(_) |
            AdapterError::ReorgInvalid(_)
        )
    }
}

impl HasErrorSuggestion for AdapterError {
    fn error_code(&self) -> &'static str {
        match self {
            AdapterError::SealReplay(_) => error_codes::CORE_SEAL_REPLAY,
            AdapterError::InvalidSeal(_) => error_codes::CORE_INVALID_SEAL,
            AdapterError::SealNotAnchored(_) => error_codes::CORE_SEAL_NOT_ANCHORED,
            AdapterError::CommitmentMismatch { .. } => error_codes::CORE_COMMITMENT_MISMATCH,
            AdapterError::InclusionProofFailed(_) => error_codes::CORE_INCLUSION_PROOF_FAILED,
            AdapterError::FinalityNotReached(_) => error_codes::CORE_FINALITY_NOT_REACHED,
            AdapterError::ReorgInvalid(_) => error_codes::CORE_REORG_INVALID,
            AdapterError::NetworkError(_) => error_codes::CORE_NETWORK_ERROR,
            AdapterError::PublishFailed(_) => error_codes::CORE_PUBLISH_FAILED,
            AdapterError::SerializationError(_) => error_codes::CORE_SERIALIZATION_ERROR,
            AdapterError::InvalidConfig(_) => error_codes::CORE_INVALID_CONFIG,
            AdapterError::VersionMismatch { .. } => error_codes::CORE_VERSION_MISMATCH,
            AdapterError::DomainSeparatorMismatch => error_codes::CORE_DOMAIN_SEPARATOR_MISMATCH,
            AdapterError::SignatureVerificationFailed(_) => error_codes::CORE_SIGNATURE_VERIFICATION_FAILED,
            AdapterError::InvalidInput(_) => error_codes::CORE_INVALID_CONFIG,
            AdapterError::Generic(_) => error_codes::CORE_GENERIC,
        }
    }

    fn description(&self) -> String {
        self.to_string()
    }

    fn suggested_fix(&self) -> String {
        match self {
            AdapterError::SealReplay(_) => {
                "This seal has already been consumed. You cannot reuse a single-use seal. \
                 Check the seal state or use a different seal.".to_string()
            }
            AdapterError::InvalidSeal(_) => {
                "The seal format is invalid. Verify the seal is properly constructed \
                 and matches the expected format for this chain.".to_string()
            }
            AdapterError::SealNotAnchored(seal_id) => {
                format!(
                    "Seal {} was never created on-chain via a real chain adapter. \
                     This typically means a fake/timestamp-based seal ID was used. \
                     Create the seal using the chain adapter's create_seal() method \
                     to get a real chain-native seal reference.",
                    seal_id
                )
            }
            AdapterError::CommitmentMismatch { expected, actual } => {
                format!(
                    "Commitment hash mismatch: expected {}, got {}. \
                     Regenerate the commitment with the correct parameters.",
                    expected, actual
                )
            }
            AdapterError::InclusionProofFailed(_) => {
                "The inclusion proof could not be verified. This may indicate: \
                 1) The anchor has not been confirmed on-chain, \
                 2) The proof is for a different commitment, or \
                 3) A chain reorganization occurred. \
                 Retry with a more recent anchor.".to_string()
            }
            AdapterError::FinalityNotReached(_) => {
                "The required finality has not been reached. Wait for more confirmations \
                 or lower the required finality threshold.".to_string()
            }
            AdapterError::ReorgInvalid(_) => {
                "The anchor was invalidated by a chain reorganization. \
                 Republish the commitment at the new chain tip.".to_string()
            }
            AdapterError::NetworkError(_) => {
                "Network communication failed. Check your internet connection, \
                 verify the RPC endpoint is accessible, or try a different node.".to_string()
            }
            AdapterError::PublishFailed(_) => {
                "Failed to publish the transaction. Check: \
                 1) You have sufficient funds for gas fees, \
                 2) The transaction is properly signed, \
                 3) The chain is accepting transactions.".to_string()
            }
            AdapterError::SerializationError(_) => {
                "Data serialization/deserialization failed. Check the data format \
                 matches the expected schema for this operation.".to_string()
            }
            AdapterError::InvalidConfig(_) => {
                "The configuration is invalid. Review your config file and ensure \
                 all required fields are present and correctly formatted.".to_string()
            }
            AdapterError::VersionMismatch { expected, actual } => {
                format!(
                    "Version mismatch: expected {}, got {}. \
                     Upgrade or downgrade to the correct protocol version.",
                    expected, actual
                )
            }
            AdapterError::DomainSeparatorMismatch => {
                "The domain separator does not match. Ensure you are using \
                 the correct network (mainnet vs testnet).".to_string()
            }
            AdapterError::SignatureVerificationFailed(_) => {
                "Signature verification failed. Check: \
                 1) The message was not tampered with, \
                 2) The correct public key is being used, \
                 3) The signature algorithm matches.".to_string()
            }
            AdapterError::InvalidInput(msg) => format!("Invalid input parameters: {}", msg),
            AdapterError::Generic(_) => {
                "An unexpected error occurred. Check the logs for details \
                 or contact support with the error context.".to_string()
            }
        }
    }

    fn docs_url(&self) -> String {
        error_codes::docs_url(self.error_code())
    }

    fn fix_action(&self) -> Option<FixAction> {
        match self {
            AdapterError::NetworkError(_) => {
                Some(FixAction::Retry {
                    parameter_changes: std::collections::HashMap::new(),
                })
            }
            AdapterError::FinalityNotReached(_) => {
                Some(FixAction::WaitForConfirmations {
                    confirmations: 6,
                    estimated_seconds: 600,
                })
            }
            AdapterError::ReorgInvalid(_) => {
                Some(FixAction::CheckState {
                    url: "https://docs.csv.dev/reorg-handling".to_string(),
                    what: "Check current chain tip and republish anchor".to_string(),
                })
            }
            AdapterError::PublishFailed(_) => {
                Some(FixAction::Retry {
                    parameter_changes: std::collections::HashMap::from([
                        ("check_gas".to_string(), "true".to_string()),
                    ]),
                })
            }
            _ => None,
        }
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
