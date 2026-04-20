//! Advanced commitment types and proof metadata for the CSV protocol.
//!
//! This module extends the basic commitment and proof types to support:
//! - Multiple commitment scheme versions (V2, V3, hash-based, KZG, etc.)
//! - Advanced proof metadata (inclusion proof types, finality proof types)
//! - Extensible commitment scheme registry
//!
//! **Note:** ZK-proof verification is NOT implemented yet.
//! This module provides type infrastructure for indexing and querying.

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// Commitment Scheme Types
// ---------------------------------------------------------------------------

/// Commitment scheme type - identifies the cryptographic construction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum CommitmentScheme {
    /// Simple hash-based commitment (SHA-256)
    #[default]
    HashBased,
    /// Pedersen commitment (hiding, binding)
    Pedersen,
    /// KZG polynomial commitment (used in PLONK, Ethereum)
    KZG,
    /// Inner product argument (Bulletproofs)
    Bulletproofs,
    /// Multilinear polynomial commitment (Hyrax, Spartan)
    Multilinear,
    /// FRI-based commitment (STARKs)
    FRI,
    /// Custom/extensible scheme
    Custom,
}


impl core::fmt::Display for CommitmentScheme {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            CommitmentScheme::HashBased => write!(f, "hash_based"),
            CommitmentScheme::Pedersen => write!(f, "pedersen"),
            CommitmentScheme::KZG => write!(f, "kzg"),
            CommitmentScheme::Bulletproofs => write!(f, "bulletproofs"),
            CommitmentScheme::Multilinear => write!(f, "multilinear"),
            CommitmentScheme::FRI => write!(f, "fri"),
            CommitmentScheme::Custom => write!(f, "custom"),
        }
    }
}

impl CommitmentScheme {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "hash_based" => Some(CommitmentScheme::HashBased),
            "pedersen" => Some(CommitmentScheme::Pedersen),
            "kzg" => Some(CommitmentScheme::KZG),
            "bulletproofs" => Some(CommitmentScheme::Bulletproofs),
            "multilinear" => Some(CommitmentScheme::Multilinear),
            "fri" => Some(CommitmentScheme::FRI),
            "custom" => Some(CommitmentScheme::Custom),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            CommitmentScheme::HashBased => "hash_based",
            CommitmentScheme::Pedersen => "pedersen",
            CommitmentScheme::KZG => "kzg",
            CommitmentScheme::Bulletproofs => "bulletproofs",
            CommitmentScheme::Multilinear => "multilinear",
            CommitmentScheme::FRI => "fri",
            CommitmentScheme::Custom => "custom",
        }
    }
}

// ---------------------------------------------------------------------------
// Inclusion Proof Types
// ---------------------------------------------------------------------------

/// Type of inclusion proof used to anchor commitment on-chain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum InclusionProofType {
    /// Bitcoin-style: Merkle proof (double-SHA256)
    #[default]
    Merkle,
    /// Ethereum-style: Merkle-Patricia Trie proof
    MerklePatricia,
    /// Sui-style: Object proof with checkpoint signature
    ObjectProof,
    /// Aptos-style: Accumulator proof
    Accumulator,
    /// Solana-style: Account state proof
    AccountState,
    /// Custom proof type
    Custom,
}


impl core::fmt::Display for InclusionProofType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            InclusionProofType::Merkle => write!(f, "merkle"),
            InclusionProofType::MerklePatricia => write!(f, "merkle_patricia"),
            InclusionProofType::ObjectProof => write!(f, "object_proof"),
            InclusionProofType::Accumulator => write!(f, "accumulator"),
            InclusionProofType::AccountState => write!(f, "account_state"),
            InclusionProofType::Custom => write!(f, "custom"),
        }
    }
}

impl InclusionProofType {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "merkle" => Some(InclusionProofType::Merkle),
            "merkle_patricia" => Some(InclusionProofType::MerklePatricia),
            "object_proof" => Some(InclusionProofType::ObjectProof),
            "accumulator" => Some(InclusionProofType::Accumulator),
            "account_state" => Some(InclusionProofType::AccountState),
            "custom" => Some(InclusionProofType::Custom),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            InclusionProofType::Merkle => "merkle",
            InclusionProofType::MerklePatricia => "merkle_patricia",
            InclusionProofType::ObjectProof => "object_proof",
            InclusionProofType::Accumulator => "accumulator",
            InclusionProofType::AccountState => "account_state",
            InclusionProofType::Custom => "custom",
        }
    }
}

// ---------------------------------------------------------------------------
// Finality Proof Types
// ---------------------------------------------------------------------------

/// Type of finality proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum FinalityProofType {
    /// Confirmation depth (probabilistic)
    #[default]
    ConfirmationDepth,
    /// Checkpoint finality (deterministic, 2f+1)
    Checkpoint,
    /// Finalized block (Ethereum post-merge)
    FinalizedBlock,
    /// Slot-based (Solana)
    SlotBased,
    /// Custom finality proof
    Custom,
}


impl core::fmt::Display for FinalityProofType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            FinalityProofType::ConfirmationDepth => write!(f, "confirmation_depth"),
            FinalityProofType::Checkpoint => write!(f, "checkpoint"),
            FinalityProofType::FinalizedBlock => write!(f, "finalized_block"),
            FinalityProofType::SlotBased => write!(f, "slot_based"),
            FinalityProofType::Custom => write!(f, "custom"),
        }
    }
}

impl FinalityProofType {
    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "confirmation_depth" => Some(FinalityProofType::ConfirmationDepth),
            "checkpoint" => Some(FinalityProofType::Checkpoint),
            "finalized_block" => Some(FinalityProofType::FinalizedBlock),
            "slot_based" => Some(FinalityProofType::SlotBased),
            "custom" => Some(FinalityProofType::Custom),
            _ => None,
        }
    }

    /// Convert to string
    pub fn as_str(&self) -> &'static str {
        match self {
            FinalityProofType::ConfirmationDepth => "confirmation_depth",
            FinalityProofType::Checkpoint => "checkpoint",
            FinalityProofType::FinalizedBlock => "finalized_block",
            FinalityProofType::SlotBased => "slot_based",
            FinalityProofType::Custom => "custom",
        }
    }
}

// ---------------------------------------------------------------------------
// Proof Metadata
// ---------------------------------------------------------------------------

/// Metadata associated with a proof.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProofMetadata {
    /// Inclusion proof type
    pub inclusion_proof_type: Option<InclusionProofType>,
    /// Finality proof type
    pub finality_proof_type: Option<FinalityProofType>,
    /// Commitment scheme used
    pub commitment_scheme: Option<CommitmentScheme>,
    /// Proof size in bytes
    pub proof_size_bytes: Option<u64>,
    /// Number of confirmations
    pub confirmations: Option<u64>,
    /// Additional metadata (chain-specific)
    pub extra: alloc::vec::Vec<u8>,
}

// ---------------------------------------------------------------------------
// Enhanced Commitment Structure
// ---------------------------------------------------------------------------

/// Enhanced commitment with scheme and metadata tracking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnhancedCommitment {
    // Basic fields (same as core Commitment)
    /// Protocol version.
    pub version: u8,
    /// Unique protocol identifier.
    pub protocol_id: [u8; 32],
    /// Merkle root of the MPC tree.
    pub mpc_root: [u8; 32],
    /// Unique contract identifier.
    pub contract_id: [u8; 32],
    /// Hash of the previous commitment.
    pub previous_commitment: [u8; 32],
    /// Hash of the transition payload.
    pub transition_payload_hash: [u8; 32],
    /// Unique seal identifier.
    pub seal_id: [u8; 32],
    /// Domain separator for disambiguation.
    pub domain_separator: [u8; 32],

    // Advanced fields
    /// Commitment scheme used
    pub commitment_scheme: CommitmentScheme,
    /// Inclusion proof type
    pub inclusion_proof_type: InclusionProofType,
    /// Finality proof type
    pub finality_proof_type: FinalityProofType,
    /// Proof metadata
    pub proof_metadata: ProofMetadata,
}
