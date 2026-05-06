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

    /// Create a new SealRef without validation.
    ///
    /// # Safety
    /// This bypasses size validation. Use only for internal protocol conversions
    /// where the input is already known to be valid.
    pub fn new_unchecked(seal_id: Vec<u8>, nonce: Option<u64>) -> Self {
        Self { seal_id, nonce }
    }

    /// Serialize to bytes
    ///
    /// Format: `[nonce_flag(1) | nonce_bytes(8 if flag=1) | seal_id_len(varuint) | seal_id]`
    /// The nonce_flag is 1 for `Some(nonce)`, 0 for `None`.
    pub fn to_vec(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(9 + self.seal_id.len());
        if let Some(nonce) = self.nonce {
            out.push(1);
            out.extend_from_slice(&nonce.to_le_bytes());
        } else {
            out.push(0);
        }
        out.extend_from_slice(&(self.seal_id.len() as u32).to_le_bytes());
        out.extend_from_slice(&self.seal_id);
        out
    }

    /// Deserialize from bytes
    ///
    /// # Errors
    /// Returns an error if the bytes are malformed or seal_id exceeds the maximum allowed size.
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
            return Err("truncated seal_id length");
        }
        let seal_id_len = u32::from_le_bytes(
            bytes[pos..pos + 4]
                .try_into()
                .map_err(|_| "truncated seal_id length")?,
        ) as usize;
        pos += 4;

        if seal_id_len > MAX_SEAL_ID_SIZE {
            return Err("seal_id exceeds maximum allowed size (1KB)");
        }
        if seal_id_len == 0 {
            return Err("seal_id cannot be empty");
        }
        if bytes.len() < pos + seal_id_len {
            return Err("truncated seal_id");
        }
        let seal_id = bytes[pos..pos + seal_id_len].to_vec();

        Ok(Self { seal_id, nonce })
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

    /// Create a new $1 without validation.
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
    fn test_seal_ref_roundtrip() {
        // Test with Some(nonce)
        let seal1 = SealRef::new(vec![1, 2, 3], Some(42)).unwrap();
        let bytes1 = seal1.to_vec();
        let restored1 = SealRef::from_bytes(&bytes1).unwrap();
        assert_eq!(restored1.seal_id, vec![1, 2, 3]);
        assert_eq!(restored1.nonce, Some(42));

        // Test with None nonce
        let seal2 = SealRef::new(vec![4, 5, 6], None).unwrap();
        let bytes2 = seal2.to_vec();
        let restored2 = SealRef::from_bytes(&bytes2).unwrap();
        assert_eq!(restored2.seal_id, vec![4, 5, 6]);
        assert_eq!(restored2.nonce, None);

        // Verify that None and Some(0) produce different bytes
        let seal_none = SealRef::new(vec![1, 2, 3], None).unwrap();
        let seal_zero = SealRef::new(vec![1, 2, 3], Some(0)).unwrap();
        assert_ne!(seal_none.to_vec(), seal_zero.to_vec());
    }

    #[test]
    fn test_seal_ref_from_bytes_errors() {
        // Empty bytes
        assert_eq!(SealRef::from_bytes(&[]), Err("empty bytes"));

        // Invalid nonce flag
        assert_eq!(SealRef::from_bytes(&[5]), Err("invalid nonce flag"));

        // Truncated nonce
        assert_eq!(SealRef::from_bytes(&[1, 0, 0]), Err("truncated nonce"));

        // Truncated seal_id length
        assert_eq!(SealRef::from_bytes(&[0]), Err("truncated seal_id length"));

        // Truncated seal_id data
        assert_eq!(
            SealRef::from_bytes(&[0, 3, 0, 0, 0, 1]),
            Err("truncated seal_id")
        );

        // Seal_id too large
        let mut large = vec![0, 0x01, 0x04, 0x00, 0x00]; // length 1025
        large.extend(vec![0u8; 1025]);
        assert_eq!(
            SealRef::from_bytes(&large),
            Err("seal_id exceeds maximum allowed size (1KB)")
        );

        // Empty seal_id
        assert_eq!(
            SealRef::from_bytes(&[0, 0, 0, 0, 0]),
            Err("seal_id cannot be empty")
        );
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
