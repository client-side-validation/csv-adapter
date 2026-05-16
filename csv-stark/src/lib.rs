//! csv-stark — STARK Batch Verification for IoT Sensor Streams
//!
//! Provides infrastructure for batching and verifying IoT sensor readings
//! using STARK (Scalable Transparent ARgument of Knowledge) proofs.
//!
//! # Architecture
//!
//! ```text
//! IoT Device Stream:
//!   [Reading 1, Reading 2, ..., Reading N]
//!       ↓
//!   CSVBatchProver (AIR execution)
//!       ↓
//!   StarkProof { proof_bytes, batch_commitment }
//!       ↓
//!   Posted to Celestia DA layer (csv-celestia)
//!       ↓
//!   CSVBatchVerifier verifies against batch_commitment
//! ```
//!
//! # Design Decisions
//!
//! - **AIR (Algebraic Intermediate Representation)**: Enforces device signature
//!   validity, value bounds, and timestamp ordering for each reading in the batch.
//! - **Batch size target**: 1024 readings per proof (configurable).
//! - **Prover backend**: Currently uses a mock/stub implementation. Production
//!   should integrate `winterfell` or `stone-prover`. See Open Question #1.
//! - **DA layer**: Proofs are posted to Celestia via blob transactions.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::vec::Vec;

use csv_core::hash::Hash;

// ============================================================================
// Types
// ============================================================================

/// Maximum number of readings per STARK proof batch.
pub const MAX_BATCH_SIZE: usize = 1024;

/// Minimum batch size for efficient proving.
pub const MIN_BATCH_SIZE: usize = 16;

/// IoT sensor reading to be included in a STARK proof batch.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct IoTReading {
    /// Device identifier (SHA-256 hash of device certificate).
    pub device_id: [u8; 32],
    /// Sensor value (64-bit unsigned integer).
    pub value: u64,
    /// Unix timestamp when the reading was taken.
    pub timestamp: u64,
    /// Device signature over (value || timestamp) — Ed25519 or similar.
    pub signature: Vec<u8>,
    /// Optional metadata payload (max 256 bytes).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<Vec<u8>>,
}

impl IoTReading {
    /// Create a new IoT reading.
    ///
    /// # Errors
    /// Returns an error if metadata exceeds 256 bytes or signature is not exactly 64 bytes.
    pub fn new(
        device_id: [u8; 32],
        value: u64,
        timestamp: u64,
        signature: Vec<u8>,
        metadata: Option<Vec<u8>>,
    ) -> Result<Self, StarkError> {
        if signature.len() != 64 {
            return Err(StarkError::InvalidSignatureLength(signature.len()));
        }
        if let Some(ref m) = metadata {
            if m.len() > 256 {
                return Err(StarkError::MetadataTooLarge(m.len()));
            }
        }
        Ok(Self {
            device_id,
            value,
            timestamp,
            signature,
            metadata,
        })
    }

    /// Compute a hash of this reading for Merkle tree leaf construction.
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(b"CSV-IOT-READING::");
        hasher.update(self.device_id);
        hasher.update(self.value.to_le_bytes());
        hasher.update(self.timestamp.to_le_bytes());
        hasher.update(&self.signature);
        if let Some(ref m) = self.metadata {
            hasher.update(b"HAS_META");
            hasher.update(m);
        } else {
            hasher.update(b"NO_META");
        }
        Hash::new(hasher.finalize().into())
    }

    /// Verify the device signature over (value || timestamp).
    ///
    /// Note: This is a lightweight check. In production, full signature
    /// verification would use the actual public key (not included here).
    pub fn verify_signature_light(&self) -> bool {
        // Check that the signature is non-zero (placeholder).
        self.signature.iter().any(|&b| b != 0)
    }

    /// Check if the reading value is within reasonable bounds.
    pub fn is_value_valid(&self) -> bool {
        // Values must be non-negative (u64 is always non-negative)
        // Additional domain-specific range checks should be applied by the prover AIR.
        true
    }

    /// Serialize the reading to bytes for Merkle tree inclusion.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(32 + 8 + 8 + 64 + 1);
        out.extend_from_slice(&self.device_id);
        out.extend_from_slice(&self.value.to_le_bytes());
        out.extend_from_slice(&self.timestamp.to_le_bytes());
        out.extend_from_slice(&self.signature);
        if self.metadata.is_some() {
            out.push(1);
            if let Some(ref m) = self.metadata {
                out.extend_from_slice(m);
            }
        } else {
            out.push(0);
        }
        out
    }

    /// Deserialize a reading from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, StarkError> {
        if bytes.len() < 112 {
            return Err(StarkError::ReadingTooShort(bytes.len()));
        }
        let mut device_id = [0u8; 32];
        device_id.copy_from_slice(&bytes[..32]);
        let value = u64::from_le_bytes(
            bytes[32..40]
                .try_into()
                .map_err(|_| StarkError::InvalidReading)?,
        );
        let timestamp = u64::from_le_bytes(
            bytes[40..48]
                .try_into()
                .map_err(|_| StarkError::InvalidReading)?,
        );
        let signature = bytes[48..112].to_vec();

        let metadata = if bytes.len() > 112 {
            if bytes[112] == 1 {
                Some(bytes[113..].to_vec())
            } else {
                None
            }
        } else {
            None
        };

        IoTReading::new(device_id, value, timestamp, signature, metadata)
    }
}

/// A batch of IoT readings to be proven together.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IoTReadingsBatch {
    /// The readings in this batch.
    pub readings: Vec<IoTReading>,
    /// Merkle root of the reading hashes.
    pub merkle_root: Hash,
}

impl IoTReadingsBatch {
    /// Create a new batch from readings.
    ///
    /// # Errors
    /// Returns an error if:
    /// - The batch is empty or exceeds MAX_BATCH_SIZE
    /// - Any reading has invalid metadata size
    pub fn new(readings: Vec<IoTReading>) -> Result<Self, StarkError> {
        if readings.is_empty() {
            return Err(StarkError::EmptyBatch);
        }
        if readings.len() > MAX_BATCH_SIZE {
            return Err(StarkError::BatchTooLarge(readings.len()));
        }

        // Validate each reading
        for r in &readings {
            if let Some(ref m) = r.metadata {
                if m.len() > 256 {
                    return Err(StarkError::MetadataTooLarge(m.len()));
                }
            }
        }

        // Compute Merkle root
        let merkle_root = Self::compute_merkle_root(&readings);

        Ok(Self {
            readings,
            merkle_root,
        })
    }

    /// Compute the Merkle root of reading hashes.
    pub fn compute_merkle_root(readings: &[IoTReading]) -> Hash {
        if readings.is_empty() {
            return Hash::zero();
        }

        // Build Merkle tree iteratively
        let mut leaves: Vec<[u8; 32]> = readings.iter().map(|r| r.hash().into_inner()).collect();

        while leaves.len() > 1 {
            let mut next_level = Vec::with_capacity(leaves.len().div_ceil(2));
            let mut i = 0;
            while i < leaves.len() {
                if i + 1 < leaves.len() {
                    // Pair: hash(0x01 || left || right)
                    let mut hasher = Sha256::new();
                    hasher.update([0x01u8]);
                    hasher.update(leaves[i]);
                    hasher.update(leaves[i + 1]);
                    next_level.push(hasher.finalize().into());
                    i += 2;
                } else {
                    // Odd node: hash(0x00 || node)
                    let mut hasher = Sha256::new();
                    hasher.update([0x00u8]);
                    hasher.update(leaves[i]);
                    next_level.push(hasher.finalize().into());
                    i += 1;
                }
            }
            leaves = next_level;
        }

        Hash::new(leaves[0])
    }

    /// Get the number of readings in the batch.
    pub fn len(&self) -> usize {
        self.readings.len()
    }

    /// Check if the batch is empty.
    pub fn is_empty(&self) -> bool {
        self.readings.is_empty()
    }

    /// Verify each reading's signature and value bounds.
    pub fn verify_readings(&self) -> Result<(), Vec<usize>> {
        let mut invalid = Vec::new();
        for (i, r) in self.readings.iter().enumerate() {
            if !r.verify_signature_light() || !r.is_value_valid() {
                invalid.push(i);
            }
        }
        if invalid.is_empty() {
            Ok(())
        } else {
            Err(invalid)
        }
    }
}

/// A STARK proof for a batch of IoT readings.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StarkProofBundle {
    /// The STARK proof bytes (backend-specific format).
    pub proof_bytes: Vec<u8>,
    /// Commitment to the batch: SHA-256(merkle_root || batch_size || timestamp_range).
    pub batch_commitment: Hash,
    /// Source chain where this proof was posted (e.g., Celestia blob).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_chain: Option<String>,
}

impl StarkProofBundle {
    /// Create a new proof bundle from a batch and proof bytes.
    pub fn from_batch(readings: &[IoTReading], proof_bytes: Vec<u8>) -> Self {
        let merkle_root = IoTReadingsBatch::compute_merkle_root(readings);
        let mut hasher = Sha256::new();
        hasher.update(b"CSV-STARK-BATCH-COMMITMENT::");
        hasher.update(merkle_root.as_bytes());
        hasher.update((readings.len() as u64).to_le_bytes());
        if readings.len() >= 2 {
            let min_ts = readings.iter().map(|r| r.timestamp).min().unwrap_or(0);
            let max_ts = readings.iter().map(|r| r.timestamp).max().unwrap_or(0);
            hasher.update(min_ts.to_le_bytes());
            hasher.update(max_ts.to_le_bytes());
        }
        let batch_commitment = Hash::new(hasher.finalize().into());

        Self {
            proof_bytes,
            batch_commitment,
            source_chain: None,
        }
    }

    /// Compute a hash of this bundle for identification.
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(b"CSV-STARK-PROOF-HASH::");
        hasher.update(&self.proof_bytes);
        hasher.update(self.batch_commitment.as_bytes());
        Hash::new(hasher.finalize().into())
    }

    /// Serialize the proof bundle to bytes.
    pub fn to_bytes(&self) -> Result<Vec<u8>, bincode::Error> {
        bincode::serialize(self)
    }

    /// Deserialize a proof bundle from bytes.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, bincode::Error> {
        bincode::deserialize(bytes)
    }
}

// ============================================================================
// Prover and Verifier Traits
// ============================================================================

/// Trait for proving a batch of IoT readings using STARKs.
pub trait IoTBatchProver {
    /// Generate a STARK proof for a batch of IoT readings.
    ///
    /// The AIR (Algebraic Intermediate Representation) enforces:
    /// 1. Each reading's device signature is valid
    /// 2. Each reading's value is within domain-specific bounds
    /// 3. Timestamps are monotonically increasing per device
    ///
    /// # Arguments
    /// * `batch` — The batch of readings to prove
    ///
    /// # Returns
    /// A StarkProofBundle containing the proof and batch commitment
    fn prove(&self, batch: &IoTReadingsBatch) -> Result<StarkProofBundle, StarkError>;

    /// Get the target batch size for this prover.
    fn target_batch_size(&self) -> usize {
        MIN_BATCH_SIZE
    }

    /// Get the maximum batch size for this prover.
    fn max_batch_size(&self) -> usize {
        MAX_BATCH_SIZE
    }
}

/// Trait for verifying STARK proofs of IoT reading batches.
pub trait IoTBatchVerifier {
    /// Verify a STARK proof against expected batch data.
    ///
    /// # Arguments
    /// * `proof` — The proof bundle to verify
    /// * `expected_commitment` — Expected batch commitment (for binding)
    ///
    /// # Returns
    /// True if the proof is valid and the commitment matches
    fn verify(
        &self,
        proof: &StarkProofBundle,
        expected_commitment: Hash,
    ) -> Result<bool, StarkError>;

    /// Verify a proof without checking the commitment (quick check).
    fn verify_proof_structure(&self, proof: &StarkProofBundle) -> Result<bool, StarkError>;

    /// Estimate proof verification time in milliseconds.
    fn estimated_verify_time_ms(&self, batch_size: usize) -> u64 {
        // Linear estimate: ~0.1ms per reading
        (batch_size as u64) / 10
    }
}

// ============================================================================
// Mock Implementation (Stub — replace with winterfell/stone-prover in production)
// ============================================================================

/// A mock/stub STARK prover for development and testing.
///
/// In production, replace this with a real STARK backend such as:
/// - `winterfell` (Rust-native STARK prover)
/// - `stone-prover` (Cairo-compatible STARK prover)
/// - `SP1` (Succinct Labs zkVM — already partially integrated in csv-bitcoin)
pub struct MockStarkProver;

impl Default for MockStarkProver {
    fn default() -> Self {
        Self
    }
}

impl IoTBatchProver for MockStarkProver {
    fn prove(&self, batch: &IoTReadingsBatch) -> Result<StarkProofBundle, StarkError> {
        if batch.readings.is_empty() {
            return Err(StarkError::EmptyBatch);
        }
        if batch.readings.len() > MAX_BATCH_SIZE {
            return Err(StarkError::BatchTooLarge(batch.readings.len()));
        }

        // Mock: produce deterministic "proof" bytes
        let mut proof_bytes = Vec::with_capacity(256);
        proof_bytes.extend_from_slice(b"CSV-MOCK-STARK-PROOF");
        proof_bytes.extend_from_slice(batch.merkle_root.as_bytes());
        let len_bytes = (batch.readings.len() as u64).to_le_bytes();
        proof_bytes.extend_from_slice(&len_bytes);

        Ok(StarkProofBundle::from_batch(&batch.readings, proof_bytes))
    }

    fn target_batch_size(&self) -> usize {
        MIN_BATCH_SIZE
    }
    fn max_batch_size(&self) -> usize {
        MAX_BATCH_SIZE
    }
}

/// A mock/stub STARK verifier for development and testing.
pub struct MockStarkVerifier;

impl Default for MockStarkVerifier {
    fn default() -> Self {
        Self
    }
}

impl IoTBatchVerifier for MockStarkVerifier {
    fn verify(
        &self,
        proof: &StarkProofBundle,
        expected_commitment: Hash,
    ) -> Result<bool, StarkError> {
        // Verify structure first
        if !self.verify_proof_structure(proof)? {
            return Ok(false);
        }
        // Check commitment matches
        Ok(proof.batch_commitment == expected_commitment)
    }

    fn verify_proof_structure(&self, proof: &StarkProofBundle) -> Result<bool, StarkError> {
        // Mock verification: check that proof bytes are non-empty and commitment is valid
        if proof.proof_bytes.len() < 16 {
            return Ok(false);
        }
        if proof.batch_commitment.as_bytes() == &[0u8; 32] {
            return Ok(false);
        }
        // Check mock proof header
        if proof.proof_bytes.starts_with(b"CSV-MOCK-STARK-PROOF") {
            Ok(true)
        } else {
            // In production, this would run the actual STARK verifier
            Ok(false)
        }
    }

    fn estimated_verify_time_ms(&self, batch_size: usize) -> u64 {
        (batch_size as u64).max(1) / 10
    }
}

// ============================================================================
// Batch Builder Utilities
// ============================================================================

/// Utility for building batches from a stream of readings.
pub struct BatchBuilder {
    current: Vec<IoTReading>,
    max_size: usize,
}

impl BatchBuilder {
    /// Create a new batch builder with the given max batch size.
    pub fn new(max_size: usize) -> Self {
        Self {
            current: Vec::with_capacity(max_size),
            max_size: max_size.min(MAX_BATCH_SIZE),
        }
    }

    /// Add a reading to the current batch.
    ///
    /// Returns `Ok(())` if the reading was added, or `Err(batch)` with the
    /// completed batch if adding this reading would exceed the limit.
    pub fn add(&mut self, reading: IoTReading) -> Result<(), IoTReadingsBatch> {
        if self.current.len() >= self.max_size {
            let current = core::mem::take(&mut self.current);
            match IoTReadingsBatch::new(current) {
                Ok(batch) => {
                    self.current.push(reading);
                    Err(batch)
                }
                Err(_) => {
                    // If batch creation fails, just push and return error with empty batch
                    self.current.push(reading);
                    Err(IoTReadingsBatch::new(vec![]).unwrap())
                }
            }
        } else {
            self.current.push(reading);
            Ok(())
        }
    }

    /// Finalize and return the current batch.
    pub fn finalize(mut self) -> Option<IoTReadingsBatch> {
        if self.current.is_empty() {
            None
        } else {
            IoTReadingsBatch::new(core::mem::take(&mut self.current)).ok()
        }
    }

    /// Get the number of readings currently in the batch.
    pub fn len(&self) -> usize {
        self.current.len()
    }

    /// Check if the current batch is empty.
    pub fn is_empty(&self) -> bool {
        self.current.is_empty()
    }

    /// Check if the current batch is full.
    pub fn is_full(&self) -> bool {
        self.current.len() >= self.max_size
    }
}

impl Default for BatchBuilder {
    fn default() -> Self {
        Self::new(MIN_BATCH_SIZE)
    }
}

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur in STARK batch operations.
#[derive(Debug, Clone, thiserror::Error)]
pub enum StarkError {
    /// Batch is empty.
    #[error("Batch must contain at least one reading")]
    EmptyBatch,

    /// Batch exceeds maximum size.
    #[error("Batch size {0} exceeds maximum of {MAX_BATCH_SIZE}")]
    BatchTooLarge(usize),

    /// Metadata exceeds 256 bytes.
    #[error("Metadata size {0} exceeds maximum of 256 bytes")]
    MetadataTooLarge(usize),

    /// Reading data is too short for deserialization.
    #[error("Reading data too short: {0} bytes")]
    ReadingTooShort(usize),

    /// Invalid reading format.
    #[error("Invalid reading data")]
    InvalidReading,

    /// Invalid proof bytes are too short or malformed.
    #[error("Invalid proof: bytes too short")]
    InvalidProof,

    /// Signature is not exactly 64 bytes.
    #[error("Signature length {0} is not 64 bytes")]
    InvalidSignatureLength(usize),

    /// Batch commitment mismatch.
    #[error("Batch commitment mismatch")]
    CommitmentMismatch,

    /// Batch size below minimum for efficient proving.
    #[error("Batch size {0} is below minimum of {MIN_BATCH_SIZE}")]
    BatchTooSmall(usize),
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn test_device_id() -> [u8; 32] {
        [0x01; 32]
    }

    fn test_signature() -> Vec<u8> {
        vec![0xAB; 64]
    }

    fn make_reading(device_id: u8, value: u64, timestamp: u64) -> IoTReading {
        let did = [device_id; 32];
        IoTReading::new(did, value, timestamp, test_signature(), None).unwrap()
    }

    #[test]
    fn test_reading_hash() {
        let r = make_reading(1, 42, 1000);
        let h = r.hash();
        assert_ne!(h.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn test_reading_hash_deterministic() {
        let r1 = make_reading(1, 42, 1000);
        let r2 = make_reading(1, 42, 1000);
        assert_eq!(r1.hash(), r2.hash());
    }

    #[test]
    fn test_reading_different_values() {
        let r1 = make_reading(1, 42, 1000);
        let r2 = make_reading(1, 43, 1000);
        assert_ne!(r1.hash(), r2.hash());
    }

    #[test]
    fn test_reading_signature_verify() {
        let r = make_reading(1, 42, 1000);
        assert!(r.verify_signature_light());
    }

    #[test]
    fn test_reading_zero_signature_fails() {
        let mut r = make_reading(1, 42, 1000);
        r.signature = vec![0u8; 64];
        assert!(!r.verify_signature_light());
    }

    #[test]
    fn test_reading_serialization_roundtrip() {
        let r = make_reading(1, 42, 1000);
        let bytes = r.to_bytes();
        let restored = IoTReading::from_bytes(&bytes).unwrap();
        assert_eq!(r, restored);
    }

    #[test]
    fn test_batch_creation() {
        let readings = vec![make_reading(1, 10, 100), make_reading(1, 20, 200)];
        let batch = IoTReadingsBatch::new(readings).unwrap();
        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn test_batch_empty_fails() {
        let result = IoTReadingsBatch::new(vec![]);
        assert!(matches!(result, Err(StarkError::EmptyBatch)));
    }

    #[test]
    fn test_batch_too_large() {
        let readings: Vec<IoTReading> = (0..=MAX_BATCH_SIZE)
            .map(|i| make_reading(1, i as u64, i as u64 * 100))
            .collect();
        let result = IoTReadingsBatch::new(readings);
        assert!(matches!(result, Err(StarkError::BatchTooLarge(_))));
    }

    #[test]
    fn test_batch_merkle_root() {
        let readings = vec![make_reading(1, 10, 100), make_reading(2, 20, 200)];
        let batch = IoTReadingsBatch::new(readings).unwrap();
        assert_ne!(batch.merkle_root.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn test_batch_merkle_deterministic() {
        let readings = vec![make_reading(1, 10, 100), make_reading(2, 20, 200)];
        let batch1 = IoTReadingsBatch::new(readings.clone()).unwrap();
        let batch2 = IoTReadingsBatch::new(readings).unwrap();
        assert_eq!(batch1.merkle_root, batch2.merkle_root);
    }

    #[test]
    fn test_batch_readings_verification() {
        let readings = vec![make_reading(1, 10, 100), make_reading(2, 20, 200)];
        let batch = IoTReadingsBatch::new(readings).unwrap();
        assert!(batch.verify_readings().is_ok());
    }

    #[test]
    fn test_batch_prover_and_verifier() {
        let readings = vec![make_reading(1, 10, 100), make_reading(2, 20, 200)];
        let batch = IoTReadingsBatch::new(readings).unwrap();

        let prover = MockStarkProver;
        let proof = prover.prove(&batch).unwrap();

        let verifier = MockStarkVerifier;
        let valid = verifier.verify(&proof, proof.batch_commitment).unwrap();
        assert!(valid);
    }

    #[test]
    fn test_proof_bundle_serialization() {
        let readings = vec![make_reading(1, 10, 100)];
        let batch = IoTReadingsBatch::new(readings).unwrap();
        let prover = MockStarkProver;
        let proof = prover.prove(&batch).unwrap();

        // Test batch_commitment serialization
        let bytes = bincode::serialize(&proof.batch_commitment).unwrap();
        let restored: Hash = bincode::deserialize(&bytes).unwrap();
        assert_eq!(proof.batch_commitment, restored);
    }

    #[test]
    fn test_batch_builder() {
        let mut builder = BatchBuilder::new(4);

        for i in 0..3 {
            assert!(builder.add(make_reading(1, i, (i + 1) * 100)).is_ok());
        }
        assert_eq!(builder.len(), 3);
        assert!(!builder.is_full());

        // Adding 4th should succeed
        assert!(builder.add(make_reading(1, 3, 400)).is_ok());
        assert!(builder.is_full());

        // Adding 5th should return completed batch
        let result = builder.add(make_reading(1, 4, 500));
        assert!(result.is_err());
        let completed = result.unwrap_err();
        assert_eq!(completed.len(), 4);
    }

    #[test]
    fn test_batch_builder_finalize() {
        let mut builder = BatchBuilder::new(4);
        builder.add(make_reading(1, 10, 100)).unwrap();
        builder.add(make_reading(1, 20, 200)).unwrap();

        let batch = builder.finalize().unwrap();
        assert_eq!(batch.len(), 2);
    }

    #[test]
    fn test_batch_builder_empty_finalize() {
        let builder = BatchBuilder::new(4);
        assert!(builder.finalize().is_none());
    }

    #[test]
    fn test_readings_at_max_batch_size() {
        let readings: Vec<IoTReading> = (0..MAX_BATCH_SIZE)
            .map(|i| make_reading(1, i as u64, i as u64 * 100))
            .collect();
        let batch = IoTReadingsBatch::new(readings).unwrap();
        assert_eq!(batch.len(), MAX_BATCH_SIZE);
    }

    #[test]
    fn test_single_reading_batch() {
        let readings = vec![make_reading(1, 42, 1000)];
        let batch = IoTReadingsBatch::new(readings).unwrap();
        assert_eq!(batch.len(), 1);
        assert_ne!(batch.merkle_root, Hash::zero());
    }

    #[test]
    fn test_metadata_too_large() {
        let large_meta = vec![0u8; 257];
        let result = IoTReading::new(
            test_device_id(),
            42,
            1000,
            test_signature(),
            Some(large_meta),
        );
        assert!(matches!(result, Err(StarkError::MetadataTooLarge(257))));
    }

    #[test]
    fn test_metadata_within_limit() {
        let meta = vec![0u8; 256];
        let result = IoTReading::new(test_device_id(), 42, 1000, test_signature(), Some(meta));
        assert!(result.is_ok());
    }

    #[test]
    fn test_proof_hash() {
        let readings = vec![make_reading(1, 10, 100)];
        let batch = IoTReadingsBatch::new(readings).unwrap();
        let prover = MockStarkProver;
        let proof = prover.prove(&batch).unwrap();
        assert_ne!(proof.hash(), Hash::zero());
    }

    #[test]
    fn test_verifier_wrong_commitment() {
        let readings = vec![make_reading(1, 10, 100)];
        let batch = IoTReadingsBatch::new(readings).unwrap();
        let prover = MockStarkProver;
        let proof = prover.prove(&batch).unwrap();

        let verifier = MockStarkVerifier;
        let wrong_commitment = Hash::new([0xFF; 32]);
        let valid = verifier.verify(&proof, wrong_commitment).unwrap();
        assert!(!valid);
    }

    #[test]
    fn test_builder_with_metadata() {
        let meta = vec![0x01, 0x02];
        let reading =
            IoTReading::new(test_device_id(), 42, 1000, test_signature(), Some(meta)).unwrap();
        let mut builder = BatchBuilder::new(4);
        assert!(builder.add(reading).is_ok());
        assert_eq!(builder.len(), 1);
    }
}
