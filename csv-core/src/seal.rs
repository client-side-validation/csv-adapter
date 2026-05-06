//! Seal and Anchor reference types
//!
//! Seals represent single-use sanads to authorize state transitions.
//! Anchors represent on-chain references containing commitments.

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};

/// Maximum allowed size for seal identifiers (1KB)
pub const MAX_SEAL_ID_SIZE: usize = 1024;

/// Maximum allowed size for anchor identifiers (1KB)
pub const MAX_ANCHOR_ID_SIZE: usize = 1024;

/// Maximum allowed size for anchor metadata (4KB)
pub const MAX_ANCHOR_METADATA_SIZE: usize = 4096;

/// A specific point on any chain that acts as a seal.
///
/// Bitcoin uses `OutPoint` (txid + vout) to identify a specific output.
/// A Bitcoin seal IS an OutPoint. `SealPoint` generalizes this concept.
///
/// The concrete meaning is chain-specific:
/// - Bitcoin: UTXO OutPoint
/// - Ethereum: Contract address + storage slot
/// - Sui: Object ID
/// - Aptos: Resource address + key
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SealPoint {
    /// Chain-specific seal identifier
    pub id: Vec<u8>,
    /// Optional nonce for replay resistance
    pub nonce: Option<u64>,
}

impl SealPoint {
    /// Create a new SealPoint from raw bytes
    ///
    /// # Arguments
    /// * `id` - Chain-specific seal identifier (max 1KB)
    /// * `nonce` - Optional nonce for replay resistance
    ///
    /// # Errors
    /// Returns an error if the id exceeds the maximum allowed size
    pub fn new(id: Vec<u8>, nonce: Option<u64>) -> Result<Self, &'static str> {
        if id.len() > MAX_SEAL_ID_SIZE {
            return Err("id exceeds maximum allowed size (1KB)");
        }
        if id.is_empty() {
            return Err("id cannot be empty");
        }
        Ok(Self { id, nonce })
    }

    /// Create a new SealPoint without validation.
    ///
    /// # Safety
    /// This bypasses size validation. Use only for internal protocol conversions
    /// where the input is already known to be valid.
    pub fn new_unchecked(id: Vec<u8>, nonce: Option<u64>) -> Self {
        Self { id, nonce }
    }

    /// Serialize to bytes
    ///
    /// Format: `[nonce_flag(1) | nonce_bytes(8 if flag=1) | id_len(varuint) | id]`
    /// The nonce_flag is 1 for `Some(nonce)`, 0 for `None`.
    pub fn to_vec(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(9 + self.id.len());
        if let Some(nonce) = self.nonce {
            out.push(1);
            out.extend_from_slice(&nonce.to_le_bytes());
        } else {
            out.push(0);
        }
        out.extend_from_slice(&(self.id.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.id);
        out
    }

    /// Deserialize from bytes
    ///
    /// # Errors
    /// Returns an error if the bytes are malformed or id exceeds the maximum allowed size.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.is_empty() {
            return Err("empty bytes");
        }
        let mut pos = 0;

        let nonce = match bytes[pos] {
            0 => {
                pos += 1;
                None
            }
            1 => {
                pos += 1;
                if bytes.len() < pos + 8 {
                    return Err("truncated nonce");
                }
                let nonce_bytes: [u8; 8] = bytes[pos..pos + 8]
                    .try_into()
                    .map_err(|_| "truncated nonce")?;
                pos += 8;
                Some(u64::from_le_bytes(nonce_bytes))
            }
            _ => return Err("invalid nonce flag"),
        };

        if bytes.len() < pos + 4 {
            return Err("truncated id length");
        }
        let id_len = u32::from_le_bytes(
            bytes[pos..pos + 4]
                .try_into()
                .map_err(|_| "truncated id length")?,
        ) as usize;
        pos += 4;

        if id_len > MAX_SEAL_ID_SIZE {
            return Err("id exceeds maximum allowed size (1KB)");
        }
        if id_len == 0 {
            return Err("id cannot be empty");
        }
        if bytes.len() < pos + id_len {
            return Err("truncated id");
        }
        let id = bytes[pos..pos + id_len].to_vec();

        Ok(Self { id, nonce })
    }

}

/// The anchor for a commitment on-chain.
///
/// Represents where a commitment was anchored on-chain.
///
/// The concrete meaning is chain-specific:
/// - Bitcoin: Transaction ID + output index
/// - Ethereum: Transaction hash + log index
/// - Sui: Object ID + version
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CommitAnchor {
    /// Chain-specific anchor identifier
    pub anchor_id: Vec<u8>,
    /// Block height or equivalent ordering
    pub block_height: u64,
    /// Optional chain-specific metadata
    pub metadata: Vec<u8>,
}

impl CommitAnchor {
    /// Create a new CommitAnchor
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

    /// Create a new CommitAnchor without validation.
    ///
    /// # Safety
    /// This bypasses validation. Use only for internal protocol conversions.
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
    fn test_seal_point_creation() {
        let seal = SealPoint::new(vec![1, 2, 3], Some(42)).unwrap();
        assert_eq!(seal.id, vec![1, 2, 3]);
        assert_eq!(seal.nonce, Some(42));
    }

    #[test]
    fn test_commit_anchor_creation() {
        let anchor = CommitAnchor::new(vec![4, 5, 6], 100, vec![7, 8]).unwrap();
        assert_eq!(anchor.anchor_id, vec![4, 5, 6]);
        assert_eq!(anchor.block_height, 100);
        assert_eq!(anchor.metadata, vec![7, 8]);
    }

    #[test]
    fn test_seal_point_serialization() {
        let seal = SealPoint::new(vec![1, 2, 3], Some(42)).unwrap();
        let bytes = seal.to_vec();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_seal_point_roundtrip() {
        // Test with Some(nonce)
        let seal1 = SealPoint::new(vec![1, 2, 3], Some(42)).unwrap();
        let bytes1 = seal1.to_vec();
        let restored1 = SealPoint::from_bytes(&bytes1).unwrap();
        assert_eq!(restored1.id, vec![1, 2, 3]);
        assert_eq!(restored1.nonce, Some(42));

        // Test with None nonce
        let seal2 = SealPoint::new(vec![4, 5, 6], None).unwrap();
        let bytes2 = seal2.to_vec();
        let restored2 = SealPoint::from_bytes(&bytes2).unwrap();
        assert_eq!(restored2.id, vec![4, 5, 6]);
        assert_eq!(restored2.nonce, None);

        // Verify that None and Some(0) produce different bytes
        let seal_none = SealPoint::new(vec![1, 2, 3], None).unwrap();
        let seal_zero = SealPoint::new(vec![1, 2, 3], Some(0)).unwrap();
        assert_ne!(seal_none.to_vec(), seal_zero.to_vec());
    }

    #[test]
    fn test_seal_point_from_bytes_errors() {
        // Empty bytes
        assert_eq!(SealPoint::from_bytes(&[]), Err("empty bytes"));

        // Invalid nonce flag
        assert_eq!(SealPoint::from_bytes(&[5]), Err("invalid nonce flag"));

        // Truncated nonce
        assert_eq!(SealPoint::from_bytes(&[1, 0, 0]), Err("truncated nonce"));

        // Truncated id length
        assert_eq!(SealPoint::from_bytes(&[0]), Err("truncated id length"));

        // Truncated id data
        assert_eq!(
            SealPoint::from_bytes(&[0, 3, 0, 0, 0, 1]),
            Err("truncated id")
        );

        // id too large
        let mut large = vec![0, 0x01, 0x04, 0x00, 0x00]; // length 1025
        large.extend(vec![0u8; 1025]);
        assert_eq!(
            SealPoint::from_bytes(&large),
            Err("id exceeds maximum allowed size (1KB)")
        );

        // Empty id
        assert_eq!(
            SealPoint::from_bytes(&[0, 0, 0, 0, 0]),
            Err("id cannot be empty")
        );
    }

    #[test]
    fn test_commit_anchor_serialization() {
        let anchor = CommitAnchor::new(vec![4, 5, 6], 100, vec![7, 8]).unwrap();
        let bytes = anchor.to_vec();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_seal_point_empty_id() {
        let result = SealPoint::new(vec![], Some(42));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "id cannot be empty");
    }

    #[test]
    fn test_seal_point_too_large() {
        let large_id = vec![0u8; MAX_SEAL_ID_SIZE + 1];
        let result = SealPoint::new(large_id, Some(42));
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "id exceeds maximum allowed size (1KB)"
        );
    }

    #[test]
    fn test_seal_point_at_max_size() {
        let max_id = vec![0u8; MAX_SEAL_ID_SIZE];
        let result = SealPoint::new(max_id, Some(42));
        assert!(result.is_ok());
    }

    #[test]
    fn test_commit_anchor_empty_id() {
        let result = CommitAnchor::new(vec![], 100, vec![7, 8]);
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "anchor_id cannot be empty");
    }

    #[test]
    fn test_commit_anchor_id_too_large() {
        let large_id = vec![0u8; MAX_ANCHOR_ID_SIZE + 1];
        let result = CommitAnchor::new(large_id, 100, vec![7, 8]);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "anchor_id exceeds maximum allowed size (1KB)"
        );
    }

    #[test]
    fn test_commit_anchor_metadata_too_large() {
        let large_metadata = vec![0u8; MAX_ANCHOR_METADATA_SIZE + 1];
        let result = CommitAnchor::new(vec![1, 2, 3], 100, large_metadata);
        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err(),
            "metadata exceeds maximum allowed size (4KB)"
        );
    }

    #[test]
    fn test_commit_anchor_at_max_sizes() {
        let max_id = vec![0u8; MAX_ANCHOR_ID_SIZE];
        let max_metadata = vec![0u8; MAX_ANCHOR_METADATA_SIZE];
        let result = CommitAnchor::new(max_id, 100, max_metadata);
        assert!(result.is_ok());
    }

    #[test]
    fn test_seal_point_new_unchecked() {
        let seal = SealPoint::new_unchecked(vec![1, 2, 3], Some(42));
        assert_eq!(seal.id, vec![1, 2, 3]);
        assert_eq!(seal.nonce, Some(42));
    }

    #[test]
    fn test_commit_anchor_new_unchecked() {
        let anchor = CommitAnchor::new_unchecked(vec![4, 5, 6], 100, vec![7, 8]);
        assert_eq!(anchor.anchor_id, vec![4, 5, 6]);
        assert_eq!(anchor.block_height, 100);
        assert_eq!(anchor.metadata, vec![7, 8]);
    }
}

// Backward compatibility type aliases
#[deprecated(since = "0.4.0", note = "Use SealPoint instead")]
pub type SealPoint = SealPoint;

#[deprecated(since = "0.4.0", note = "Use CommitAnchor instead")]
pub type CommitAnchor = CommitAnchor;
