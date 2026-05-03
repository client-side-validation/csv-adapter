//! Zero-Knowledge Seal Proof Module
//!
//! This module provides ZK-proof infrastructure for CSV seal verification.
//! ZK proofs enable trustless verification without relying on chain RPC responses.
//!
//! # Architecture
//!
//! ```text
//! Prover (sender):
//!   Bitcoin UTXO spend data + Merkle branch
//!   → SPV guest program
//!   → ZkSealProof { proof_bytes, public_inputs }
//!
//! Verifier (receiver):
//!   ZkSealProof
//!   → verify against known verifier key
//!   → extract seal_ref and block_hash as trusted outputs
//!   → no RPC call required
//! ```
//!
//! # Design Decisions
//!
//! - Uses a generic `ZkProver`/`ZkVerifier` trait interface for flexibility
//! - Supports multiple ZK backends (SP1, Risc0, Groth16) via feature flags
//! - Public inputs include seal_ref, block_hash, and commitment for binding
//! - Verifier keys are chain-specific and registered at runtime

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::hash::Hash;
use crate::protocol_version::Chain;
use crate::seal::SealRef;

/// Maximum ZK proof size (1MB)
pub const MAX_ZK_PROOF_SIZE: usize = 1024 * 1024;

/// Verifier key for a specific chain's ZK proof system
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VerifierKey {
    /// Chain this verifier is for
    pub chain: Chain,
    /// Verifier key bytes (backend-specific format)
    pub key_bytes: Vec<u8>,
    /// Proof system type (SP1, Risc0, Groth16, etc.)
    pub proof_system: ProofSystem,
    /// Key version for upgradeability
    pub version: u32,
    /// Whether this verifier is active
    pub active: bool,
}

impl VerifierKey {
    /// Create a new verifier key
    pub fn new(chain: Chain, key_bytes: Vec<u8>, proof_system: ProofSystem, version: u32) -> Self {
        Self {
            chain,
            key_bytes,
            proof_system,
            version,
            active: true,
        }
    }

    /// Compute a hash of this verifier key for identification
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.chain.id().as_bytes());
        hasher.update(&self.key_bytes);
        hasher.update(&self.version.to_le_bytes());
        Hash::new(hasher.finalize().into())
    }
}

/// Supported ZK proof systems
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofSystem {
    /// Succinct Labs SP1 (RISC-V zkVM)
    SP1,
    /// Risc0 RISC-V zkVM
    Risc0,
    /// Groth16 (pairing-based)
    Groth16,
    /// PlonK (permutation-based)
    PlonK,
    /// Custom/extensible proof system
    Custom,
}

impl core::fmt::Display for ProofSystem {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            ProofSystem::SP1 => write!(f, "sp1"),
            ProofSystem::Risc0 => write!(f, "risc0"),
            ProofSystem::Groth16 => write!(f, "groth16"),
            ProofSystem::PlonK => write!(f, "plonk"),
            ProofSystem::Custom => write!(f, "custom"),
        }
    }
}

/// Public inputs from a ZK seal proof
///
/// These are the trusted outputs that the ZK proof guarantees are correct.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZkPublicInputs {
    /// The seal reference being proven
    pub seal_ref: SealRef,
    /// Block hash where the seal was consumed
    pub block_hash: Hash,
    /// Commitment hash bound to the proof
    pub commitment: Hash,
    /// Source chain identifier
    pub source_chain: Chain,
    /// Block height of the seal consumption
    pub block_height: u64,
    /// Unix timestamp of the seal consumption
    pub timestamp: u64,
}

/// Complete ZK seal proof bundle
///
/// Contains the proof bytes, verifier key, and public inputs.
/// The verifier can check this proof without any chain RPC.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ZkSealProof {
    /// The ZK proof bytes (backend-specific format)
    pub proof_bytes: Vec<u8>,
    /// Verifier key for proof verification
    pub verifier_key: VerifierKey,
    /// Public inputs extracted from the proof
    pub public_inputs: ZkPublicInputs,
}

impl ZkSealProof {
    /// Create a new ZK seal proof
    ///
    /// # Arguments
    /// * `proof_bytes` - ZK proof bytes (max MAX_ZK_PROOF_SIZE)
    /// * `verifier_key` - Verifier key for the target chain
    /// * `public_inputs` - Public inputs from the proof
    ///
    /// # Errors
    /// Returns an error if proof_bytes exceeds the maximum allowed size
    pub fn new(
        proof_bytes: Vec<u8>,
        verifier_key: VerifierKey,
        public_inputs: ZkPublicInputs,
    ) -> Result<Self, &'static str> {
        if proof_bytes.len() > MAX_ZK_PROOF_SIZE {
            return Err("ZK proof bytes exceed maximum allowed size (1MB)");
        }
        Ok(Self {
            proof_bytes,
            verifier_key,
            public_inputs,
        })
    }

    /// Verify the proof structure is valid
    pub fn is_structurally_valid(&self) -> bool {
        !self.proof_bytes.is_empty()
            && self.proof_bytes.len() <= MAX_ZK_PROOF_SIZE
            && self.verifier_key.active
            && !self.public_inputs.seal_ref.seal_id.is_empty()
            && self.public_inputs.block_hash.as_bytes() != &[0u8; 32]
    }

    /// Serialize the proof for storage or transmission
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Deserialize a proof from bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}

/// Trait for generating ZK seal proofs
///
/// Implementations use a specific ZK backend (SP1, Risc0, etc.)
/// to generate proofs that a seal was consumed on a specific chain.
pub trait ZkProver {
    /// Generate a ZK proof that a seal was consumed
    ///
    /// # Arguments
    /// * `seal` - The seal reference being proven
    /// * `witness` - Chain witness data (UTXO spend, Merkle branch, etc.)
    ///
    /// # Returns
    /// A ZkSealProof containing the proof bytes and public inputs
    ///
    /// # Errors
    /// Returns an error if the witness data is invalid or proving fails
    fn prove_seal_consumption(
        &self,
        seal: &SealRef,
        witness: &ChainWitness,
    ) -> Result<ZkSealProof, ZkError>;

    /// Get the proof system this prover uses
    fn proof_system(&self) -> ProofSystem;
}

/// Chain witness data for ZK proof generation
///
/// Contains the on-chain data that the ZK guest program will prove.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ChainWitness {
    /// Source chain identifier
    pub chain: Chain,
    /// Block hash of the transaction
    pub block_hash: Hash,
    /// Block height of the transaction
    pub block_height: u64,
    /// Transaction data (serialized)
    pub tx_data: Vec<u8>,
    /// Merkle/inclusion proof data
    pub inclusion_proof: Vec<u8>,
    /// Finality proof data
    pub finality_proof: Vec<u8>,
    /// Unix timestamp of the block
    pub timestamp: u64,
}

impl ChainWitness {
    /// Compute a hash of the witness for integrity verification
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(self.chain.id().as_bytes());
        hasher.update(self.block_hash.as_bytes());
        hasher.update(&self.block_height.to_le_bytes());
        hasher.update(&self.tx_data);
        hasher.update(&self.inclusion_proof);
        hasher.update(&self.finality_proof);
        hasher.update(&self.timestamp.to_le_bytes());
        Hash::new(hasher.finalize().into())
    }
}

/// Trait for verifying ZK seal proofs
///
/// Implementations use a specific ZK backend's verifier to check
/// proofs against a registered verifier key.
pub trait ZkVerifier {
    /// Verify a ZK seal proof
    ///
    /// # Arguments
    /// * `proof` - The ZkSealProof to verify
    ///
    /// # Returns
    /// The verified public inputs if the proof is valid
    ///
    /// # Errors
    /// Returns an error if the proof is invalid or the verifier key is not found
    fn verify(&self, proof: &ZkSealProof) -> Result<ZkPublicInputs, ZkError>;

    /// Get the proof system this verifier uses
    fn proof_system(&self) -> ProofSystem;
}

/// Registry of ZK verifiers for different chains
///
/// Manages verifier keys for each chain's ZK proof system.
pub struct ZkVerifierRegistry {
    verifiers: std::collections::HashMap<Chain, VerifierKey>,
}

impl ZkVerifierRegistry {
    /// Create a new empty verifier registry
    pub fn new() -> Self {
        Self {
            verifiers: std::collections::HashMap::new(),
        }
    }

    /// Register a verifier key for a chain
    pub fn register(&mut self, key: VerifierKey) {
        self.verifiers.insert(key.chain, key);
    }

    /// Get the verifier key for a chain
    pub fn get(&self, chain: &Chain) -> Option<&VerifierKey> {
        self.verifiers.get(chain)
    }

    /// Check if a chain has a registered verifier
    pub fn has_verifier(&self, chain: &Chain) -> bool {
        self.verifiers.contains_key(chain)
    }

    /// Remove a verifier for a chain
    pub fn remove(&mut self, chain: &Chain) -> Option<VerifierKey> {
        self.verifiers.remove(chain)
    }

    /// Get all registered chains
    pub fn registered_chains(&self) -> Vec<Chain> {
        self.verifiers.keys().cloned().collect()
    }
}

impl Default for ZkVerifierRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur in ZK proof operations
#[derive(Debug, Clone, thiserror::Error)]
pub enum ZkError {
    /// Invalid proof bytes
    #[error("Invalid proof bytes: {0}")]
    InvalidProof(String),

    /// Verifier key not found for chain
    #[error("Verifier key not found for chain: {0}")]
    VerifierNotFound(Chain),

    /// Proof verification failed
    #[error("Proof verification failed: {0}")]
    VerificationFailed(String),

    /// Proof generation failed
    #[error("Proof generation failed: {0}")]
    GenerationFailed(String),

    /// Unsupported proof system
    #[error("Unsupported proof system: {0}")]
    UnsupportedSystem(String),

    /// Proof size exceeds limit
    #[error("Proof size exceeds maximum: {0} bytes")]
    ProofTooLarge(usize),

    /// Backend-specific error
    #[error("Backend error: {0}")]
    BackendError(String),
}

/// Create a default ZkVerifierRegistry with common chains
///
/// This is a convenience function that registers placeholder verifiers.
/// In production, these would be loaded from on-chain verifier contracts.
pub fn default_verifier_registry() -> ZkVerifierRegistry {
    let mut registry = ZkVerifierRegistry::new();

    // Register placeholder verifiers for common chains
    // In production, these would be loaded from on-chain contracts
    for chain in [
        Chain::Bitcoin,
        Chain::Ethereum,
        Chain::Solana,
        Chain::Sui,
        Chain::Aptos,
    ] {
        registry.register(VerifierKey::new(
            chain,
            vec![0u8; 64], // Placeholder verifier key
            ProofSystem::SP1,
            1,
        ));
    }

    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verifier_key_hash() {
        let key1 = VerifierKey::new(Chain::Bitcoin, vec![1u8; 32], ProofSystem::SP1, 1);
        let key2 = VerifierKey::new(Chain::Bitcoin, vec![1u8; 32], ProofSystem::SP1, 1);
        let key3 = VerifierKey::new(Chain::Ethereum, vec![1u8; 32], ProofSystem::SP1, 1);

        assert_eq!(key1.hash(), key2.hash());
        assert_ne!(key1.hash(), key3.hash());
    }

    #[test]
    fn test_zk_seal_proof_structure() {
        let verifier_key = VerifierKey::new(Chain::Bitcoin, vec![1u8; 32], ProofSystem::SP1, 1);
        let seal_ref = SealRef::new(vec![0xAB; 32], Some(42)).unwrap();
        let public_inputs = ZkPublicInputs {
            seal_ref: seal_ref.clone(),
            block_hash: Hash::new([1u8; 32]),
            commitment: Hash::new([2u8; 32]),
            source_chain: Chain::Bitcoin,
            block_height: 800_000,
            timestamp: 1_000_000,
        };

        let proof = ZkSealProof::new(vec![0xCD; 128], verifier_key, public_inputs).unwrap();
        assert!(proof.is_structurally_valid());
    }

    #[test]
    fn test_zk_seal_proof_too_large() {
        let verifier_key = VerifierKey::new(Chain::Bitcoin, vec![1u8; 32], ProofSystem::SP1, 1);
        let seal_ref = SealRef::new(vec![0xAB; 32], Some(42)).unwrap();
        let public_inputs = ZkPublicInputs {
            seal_ref,
            block_hash: Hash::new([1u8; 32]),
            commitment: Hash::new([2u8; 32]),
            source_chain: Chain::Bitcoin,
            block_height: 800_000,
            timestamp: 1_000_000,
        };

        let result = ZkSealProof::new(vec![0u8; MAX_ZK_PROOF_SIZE + 1], verifier_key, public_inputs);
        assert!(result.is_err());
    }

    #[test]
    fn test_zk_seal_proof_invalid() {
        let verifier_key = VerifierKey::new(Chain::Bitcoin, vec![1u8; 32], ProofSystem::SP1, 1);
        let seal_ref = SealRef::new(vec![0xAB; 32], Some(42)).unwrap();
        let public_inputs = ZkPublicInputs {
            seal_ref,
            block_hash: Hash::zero(), // Invalid: zero block hash
            commitment: Hash::new([2u8; 32]),
            source_chain: Chain::Bitcoin,
            block_height: 800_000,
            timestamp: 1_000_000,
        };

        let proof = ZkSealProof::new(vec![0xCD; 128], verifier_key, public_inputs).unwrap();
        assert!(!proof.is_structurally_valid());
    }

    #[test]
    fn test_witness_hash() {
        let witness = ChainWitness {
            chain: Chain::Bitcoin,
            block_hash: Hash::new([1u8; 32]),
            block_height: 800_000,
            tx_data: vec![0xAB; 64],
            inclusion_proof: vec![0xCD; 32],
            finality_proof: vec![0xEF; 16],
            timestamp: 1_000_000,
        };

        let hash = witness.hash();
        assert_ne!(hash.as_bytes(), &[0u8; 32]);

        // Same witness should produce same hash
        let witness2 = witness.clone();
        assert_eq!(witness.hash(), witness2.hash());
    }

    #[test]
    fn test_verifier_registry() {
        let mut registry = ZkVerifierRegistry::new();

        assert!(!registry.has_verifier(&Chain::Bitcoin));

        registry.register(VerifierKey::new(Chain::Bitcoin, vec![1u8; 32], ProofSystem::SP1, 1));
        assert!(registry.has_verifier(&Chain::Bitcoin));
        assert_eq!(registry.registered_chains().len(), 1);

        let key = registry.get(&Chain::Bitcoin).unwrap();
        assert_eq!(key.chain, Chain::Bitcoin);

        registry.remove(&Chain::Bitcoin);
        assert!(!registry.has_verifier(&Chain::Bitcoin));
    }

    #[test]
    fn test_default_verifier_registry() {
        let registry = default_verifier_registry();
        let chains = registry.registered_chains();
        assert_eq!(chains.len(), 5);
        assert!(chains.contains(&Chain::Bitcoin));
        assert!(chains.contains(&Chain::Ethereum));
        assert!(chains.contains(&Chain::Solana));
    }

    #[test]
    fn test_proof_system_display() {
        assert_eq!(ProofSystem::SP1.to_string(), "sp1");
        assert_eq!(ProofSystem::Risc0.to_string(), "risc0");
        assert_eq!(ProofSystem::Groth16.to_string(), "groth16");
        assert_eq!(ProofSystem::PlonK.to_string(), "plonk");
        assert_eq!(ProofSystem::Custom.to_string(), "custom");
    }

    #[test]
    fn test_zk_seal_proof_serialization() {
        let verifier_key = VerifierKey::new(Chain::Bitcoin, vec![1u8; 32], ProofSystem::SP1, 1);
        let seal_ref = SealRef::new(vec![0xAB; 32], Some(42)).unwrap();
        let public_inputs = ZkPublicInputs {
            seal_ref,
            block_hash: Hash::new([1u8; 32]),
            commitment: Hash::new([2u8; 32]),
            source_chain: Chain::Bitcoin,
            block_height: 800_000,
            timestamp: 1_000_000,
        };

        let proof = ZkSealProof::new(vec![0xCD; 128], verifier_key, public_inputs).unwrap();
        let bytes = proof.to_bytes().unwrap();
        let restored = ZkSealProof::from_bytes(&bytes).unwrap();

        assert_eq!(proof, restored);
    }
}
