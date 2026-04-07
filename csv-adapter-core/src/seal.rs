//! Seal and Anchor reference types
//!
//! Seals represent single-use rights to authorize state transitions.
//! Anchors represent on-chain references containing commitments.

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Maximum allowed size for seal identifiers (1KB)
pub const MAX_SEAL_ID_SIZE: usize = 1024;

/// Maximum allowed size for anchor identifiers (1KB)
pub const MAX_ANCHOR_ID_SIZE: usize = 1024;

/// Maximum allowed size for anchor metadata (4KB)
pub const MAX_ANCHOR_METADATA_SIZE: usize = 4096;

/// A reference to a single-use seal
///
/// The concrete meaning is chain-specific:
/// - Bitcoin: UTXO OutPoint
/// - Ethereum: Contract address + storage slot
/// - Sui: Object ID
/// - Aptos: Resource address + key
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SealRef {
    /// Chain-specific seal identifier
    pub seal_id: Vec<u8>,
    /// Optional nonce for replay resistance
    pub nonce: Option<u64>,
}

impl SealRef {
    /// Create a new SealRef from raw bytes
    ///
    /// # Arguments
    /// * `seal_id` - Chain-specific seal identifier (max 1KB)
    /// * `nonce` - Optional nonce for replay resistance
    ///
    /// # Errors
    /// Returns an error if the seal_id exceeds the maximum allowed size
    pub fn new(seal_id: Vec<u8>, nonce: Option<u64>) -> Result<Self, &'static str> {
        if seal_id.len() > MAX_SEAL_ID_SIZE {
            return Err("seal_id exceeds maximum allowed size (1KB)");
        }
        if seal_id.is_empty() {
            return Err("seal_id cannot be empty");
        }
        Ok(Self { seal_id, nonce })
    }

    /// Create a new SealRef without validation (for internal use only)
    ///
    /// # Safety
    /// This bypasses size validation and should only be used when
    /// the input is already known to be valid.
    pub fn new_unchecked(seal_id: Vec<u8>, nonce: Option<u64>) -> Self {
        Self { seal_id, nonce }
    }

    /// Serialize to bytes
    pub fn to_vec(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + self.seal_id.len());
        if let Some(nonce) = self.nonce {
            out.extend_from_slice(&nonce.to_le_bytes());
        } else {
            out.extend_from_slice(&[0u8; 8]);
        }
        out.extend_from_slice(&self.seal_id);
        out
    }
}

/// A reference to an on-chain anchor containing a commitment
///
/// The concrete meaning is chain-specific:
/// - Bitcoin: Transaction ID + output index
/// - Ethereum: Transaction hash + log index
/// - Sui: Object ID + version
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct AnchorRef {
    /// Chain-specific anchor identifier
    pub anchor_id: Vec<u8>,
    /// Block height or equivalent ordering
    pub block_height: u64,
    /// Optional chain-specific metadata
    pub metadata: Vec<u8>,
}

impl AnchorRef {
    /// Create a new AnchorRef
    ///
    /// # Arguments
    /// * `anchor_id` - Chain-specific anchor identifier (max 1KB)
    /// * `block_height` - Block height or equivalent ordering
    /// * `metadata` - Optional chain-specific metadata (max 4KB)
    ///
    /// # Errors
    /// Returns an error if anchor_id or metadata exceeds the maximum allowed size
    pub fn new(
        anchor_id: Vec<u8>,
        block_height: u64,
        metadata: Vec<u8>,
    ) -> Result<Self, &'static str> {
        if anchor_id.len() > MAX_ANCHOR_ID_SIZE {
            return Err("anchor_id exceeds maximum allowed size (1KB)");
        }
        if anchor_id.is_empty() {
            return Err("anchor_id cannot be empty");
        }
        if metadata.len() > MAX_ANCHOR_METADATA_SIZE {
            return Err("metadata exceeds maximum allowed size (4KB)");
        }
        Ok(Self {
            anchor_id,
            block_height,
            metadata,
        })
    }

    /// Create a new AnchorRef without validation (for internal use only)
    ///
    /// # Safety
    /// This bypasses size validation and should only be used when
    /// the input is already known to be valid.
    pub fn new_unchecked(anchor_id: Vec<u8>, block_height: u64, metadata: Vec<u8>) -> Self {
        Self {
            anchor_id,
            block_height,
            metadata,
        }
    }

    /// Serialize to bytes
    pub fn to_vec(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + self.anchor_id.len() + self.metadata.len());
        out.extend_from_slice(&self.block_height.to_le_bytes());
        out.extend_from_slice(&self.anchor_id);
        out.extend_from_slice(&self.metadata);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seal_ref_creation() {
        let seal = SealRef::new(vec![1, 2, 3], Some(42)).unwrap();
        assert_eq!(seal.seal_id, vec![1, 2, 3]);
        assert_eq!(seal.nonce, Some(42));
    }

    #[test]
    fn test_anchor_ref_creation() {
        let anchor = AnchorRef::new(vec![4, 5, 6], 100, vec![7, 8]).unwrap();
        assert_eq!(anchor.anchor_id, vec![4, 5, 6]);
        assert_eq!(anchor.block_height, 100);
        assert_eq!(anchor.metadata, vec![7, 8]);
    }

    #[test]
    fn test_seal_ref_serialization() {
        let seal = SealRef::new(vec![1, 2, 3], Some(42)).unwrap();
        let bytes = seal.to_vec();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_anchor_ref_serialization() {
        let anchor = AnchorRef::new(vec![4, 5, 6], 100, vec![7, 8]).unwrap();
        let bytes = anchor.to_vec();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_seal_ref_empty_id() {
        let result = SealRef::new(vec![], Some(42));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "seal_id cannot be empty");
    }

    #[test]
    fn test_seal_ref_too_large() {
        let large_id = vec![0u8; MAX_SEAL_ID_SIZE + 1];
        let result = SealRef::new(large_id, Some(42));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "seal_id exceeds maximum allowed size (1KB)"
        );
    }

    #[test]
    fn test_seal_ref_at_max_size() {
        let max_id = vec![0u8; MAX_SEAL_ID_SIZE];
        let result = SealRef::new(max_id, Some(42));
        assert!(result.is_ok());
    }

    #[test]
    fn test_anchor_ref_empty_id() {
        let result = AnchorRef::new(vec![], 100, vec![7, 8]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "anchor_id cannot be empty");
    }

    #[test]
    fn test_anchor_ref_id_too_large() {
        let large_id = vec![0u8; MAX_ANCHOR_ID_SIZE + 1];
        let result = AnchorRef::new(large_id, 100, vec![7, 8]);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "anchor_id exceeds maximum allowed size (1KB)"
        );
    }

    #[test]
    fn test_anchor_ref_metadata_too_large() {
        let large_metadata = vec![0u8; MAX_ANCHOR_METADATA_SIZE + 1];
        let result = AnchorRef::new(vec![1, 2, 3], 100, large_metadata);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "metadata exceeds maximum allowed size (4KB)"
        );
    }

    #[test]
    fn test_anchor_ref_at_max_sizes() {
        let max_id = vec![0u8; MAX_ANCHOR_ID_SIZE];
        let max_metadata = vec![0u8; MAX_ANCHOR_METADATA_SIZE];
        let result = AnchorRef::new(max_id, 100, max_metadata);
        assert!(result.is_ok());
    }

    #[test]
    fn test_seal_ref_new_unchecked() {
        let seal = SealRef::new_unchecked(vec![1, 2, 3], Some(42));
        assert_eq!(seal.seal_id, vec![1, 2, 3]);
        assert_eq!(seal.nonce, Some(42));
    }

    #[test]
    fn test_anchor_ref_new_unchecked() {
        let anchor = AnchorRef::new_unchecked(vec![4, 5, 6], 100, vec![7, 8]);
        assert_eq!(anchor.anchor_id, vec![4, 5, 6]);
        assert_eq!(anchor.block_height, 100);
        assert_eq!(anchor.metadata, vec![7, 8]);
    }
}
