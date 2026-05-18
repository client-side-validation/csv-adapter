use alloc::vec::Vec;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};


use crate::protocol_version::ChainId;

/// A signed package that establishes a trusted checkpoint for offline verification.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrustPackage {
    /// Chain identifier for which this trust package applies.
    pub chain_id: ChainId,
    /// Trusted checkpoint hash for the chain.
    pub trusted_checkpoint: crate::hash::Hash,
    /// Height of the checkpoint.
    pub checkpoint_height: u64,
    /// Validator commitment payload (opaque bytes).
    pub validator_commitment: Vec<u8>,
    /// Generation time of the package.
    pub generated_at: DateTime<Utc>,
    /// Expiration time of the package.
    pub expires_at: DateTime<Utc>,
    /// Raw package signature bytes (codec-specific).
    pub package_signature: Vec<u8>,
}

/// Context required to perform offline verification using a trust package.
#[derive(Clone, Debug)]
pub struct OfflineVerificationContext {
    /// The trust package to use for verification.
    pub trust_package: TrustPackage,
    /// The effective verification timestamp.
    pub verification_time: DateTime<Utc>,
}
