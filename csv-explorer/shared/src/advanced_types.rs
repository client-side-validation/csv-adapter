/// Advanced commitment and proof types for the CSV Explorer.
///
/// This module provides extended types for indexing and querying
/// commitments and proofs with metadata about scheme types, proof types, etc.
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

// Re-export core commitment types
pub use csv_core::{CommitmentScheme, FinalityProofType, InclusionProofType};

// ---------------------------------------------------------------------------
// Enhanced Record Types for Indexer/API
// ---------------------------------------------------------------------------

/// Enhanced right record with commitment scheme and proof metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedRightRecord {
    // Basic fields (same as RightRecord)
    pub id: String,
    pub chain: String,
    pub seal_ref: String,
    pub commitment: String,
    pub owner: String,
    pub created_at: DateTime<Utc>,
    pub created_tx: String,
    pub status: String,
    pub metadata: Option<JsonValue>,
    pub transfer_count: u64,
    pub last_transfer_at: Option<DateTime<Utc>>,

    // Advanced commitment fields
    /// Commitment scheme used (hash_based, pedersen, kzg, etc.)
    pub commitment_scheme: CommitmentScheme,
    /// Commitment version
    pub commitment_version: u8,
    /// Protocol ID
    pub protocol_id: String,
    /// MPC root hash (for multi-protocol commitments)
    pub mpc_root: Option<String>,
    /// Domain separator
    pub domain_separator: Option<String>,
    /// Inclusion proof type
    pub inclusion_proof_type: InclusionProofType,
    /// Finality proof type
    pub finality_proof_type: FinalityProofType,
    /// Proof size in bytes
    pub proof_size_bytes: Option<u64>,
    /// Number of confirmations
    pub confirmations: Option<u64>,
}

/// Enhanced seal record with proof metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedSealRecord {
    // Basic fields (same as SealRecord)
    pub id: String,
    pub chain: String,
    pub seal_type: String,
    pub seal_ref: String,
    pub right_id: Option<String>,
    pub status: String,
    pub consumed_at: Option<DateTime<Utc>>,
    pub consumed_tx: Option<String>,
    pub block_height: u64,

    // Advanced fields
    /// Seal proof type (merkle, merkle_patricia, object_proof, etc.)
    pub seal_proof_type: String,
    /// Seal proof verification status
    pub seal_proof_verified: Option<bool>,
}

/// Enhanced inclusion proof record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedInclusionProof {
    /// Right ID this proof is for
    pub right_id: String,
    /// Chain where proof is included
    pub chain: String,
    /// Block/anchor reference
    pub anchor_ref: String,
    /// Proof type (Merkle, MPT, Accumulator, etc.)
    pub proof_type: String,
    /// Proof data (hex-encoded)
    pub proof_data: String,
    /// Proof size in bytes
    pub proof_size_bytes: u64,
    /// Verification status
    pub verified: Option<bool>,
    /// Created at
    pub created_at: DateTime<Utc>,
}

/// Enhanced transfer record with cross-chain proof metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedTransferRecord {
    // Basic fields (same as TransferRecord)
    pub id: String,
    pub right_id: String,
    pub from_chain: String,
    pub to_chain: String,
    pub from_owner: String,
    pub to_owner: String,
    pub lock_tx: String,
    pub mint_tx: Option<String>,
    pub proof_ref: Option<String>,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub duration_ms: Option<u64>,

    // Advanced fields
    /// Cross-chain proof type
    pub cross_chain_proof_type: Option<String>,
    /// Bridge contract used
    pub bridge_contract: Option<String>,
    /// Bridge proof verification status
    pub bridge_proof_verified: Option<bool>,
}

// ---------------------------------------------------------------------------
// Filter Types for Advanced Queries
// ---------------------------------------------------------------------------

/// Filter for querying rights by commitment scheme and proof type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RightProofFilter {
    pub chain: Option<String>,
    pub owner: Option<String>,
    pub commitment_scheme: Option<CommitmentScheme>,
    pub inclusion_proof_type: Option<InclusionProofType>,
    pub finality_proof_type: Option<FinalityProofType>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Filter for querying seals by proof type.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SealProofFilter {
    pub chain: Option<String>,
    pub seal_type: Option<String>,
    pub seal_proof_type: Option<String>,
    pub seal_proof_verified: Option<bool>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

// ---------------------------------------------------------------------------
// Statistics Types
// ---------------------------------------------------------------------------

/// Statistics on commitment scheme and proof usage.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProofStatistics {
    /// Total rights indexed
    pub total_rights: u64,
    /// Total seals indexed
    pub total_seals: u64,
    /// Rights by commitment scheme
    pub rights_by_commitment_scheme: Vec<SchemeCount>,
    /// Rights by inclusion proof type
    pub rights_by_inclusion_proof: Vec<InclusionProofCount>,
    /// Rights by finality proof type
    pub rights_by_finality_proof: Vec<FinalityProofCount>,
    /// Seals by proof type
    pub seals_by_proof_type: Vec<SealProofCount>,
}

/// Count of items by commitment scheme.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemeCount {
    pub scheme: CommitmentScheme,
    pub count: u64,
}

/// Count of items by inclusion proof type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InclusionProofCount {
    pub proof_type: InclusionProofType,
    pub count: u64,
}

/// Count of items by finality proof type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FinalityProofCount {
    pub proof_type: FinalityProofType,
    pub count: u64,
}

/// Count of seals by proof type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SealProofCount {
    pub proof_type: String,
    pub count: u64,
}

// ---------------------------------------------------------------------------
// Verification Status Types
// ---------------------------------------------------------------------------

/// Proof verification status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProofVerificationStatus {
    /// Proof has not been verified
    Unverified,
    /// Proof verification in progress
    Verifying,
    /// Proof verified successfully
    Verified,
    /// Proof verification failed
    Invalid,
    /// Verification could not be completed (error/timeout)
    Error,
}

impl std::fmt::Display for ProofVerificationStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProofVerificationStatus::Unverified => write!(f, "unverified"),
            ProofVerificationStatus::Verifying => write!(f, "verifying"),
            ProofVerificationStatus::Verified => write!(f, "verified"),
            ProofVerificationStatus::Invalid => write!(f, "invalid"),
            ProofVerificationStatus::Error => write!(f, "error"),
        }
    }
}
