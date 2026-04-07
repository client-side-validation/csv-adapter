//! Celestia-specific type definitions

use serde::{Serialize, Deserialize};

/// Celestia seal reference (namespaced blob ID)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CelestiaSealRef {
    /// Namespace ID (8 bytes for Celestia namespaces)
    pub namespace_id: [u8; 8],
    /// Blob commitment hash
    pub blob_hash: [u8; 32],
    /// Nonce for replay resistance
    pub nonce: u64,
}

impl CelestiaSealRef {
    pub fn new(namespace_id: [u8; 8], blob_hash: [u8; 32], nonce: u64) -> Self {
        Self { namespace_id, blob_hash, nonce }
    }

    pub fn to_vec(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(8 + 32 + 8);
        out.extend_from_slice(&self.namespace_id);
        out.extend_from_slice(&self.blob_hash);
        out.extend_from_slice(&self.nonce.to_le_bytes());
        out
    }
}

/// Celestia anchor reference (PayForBlob transaction)
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct CelestiaAnchorRef {
    /// Transaction hash
    pub tx_hash: [u8; 32],
    /// Height where blob was published
    pub height: u64,
    /// Namespace ID
    pub namespace_id: [u8; 8],
    /// Share index within blob
    pub share_index: u64,
}

impl CelestiaAnchorRef {
    pub fn new(tx_hash: [u8; 32], height: u64, namespace_id: [u8; 8], share_index: u64) -> Self {
        Self { tx_hash, height, namespace_id, share_index }
    }
}

/// Celestia inclusion proof (data availability sampling proof)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CelestiaInclusionProof {
    /// Blob data
    pub blob_data: Vec<u8>,
    /// Row/column proofs for DAS
    pub das_proofs: Vec<Vec<u8>>,
    /// Block height
    pub height: u64,
}

impl CelestiaInclusionProof {
    pub fn new(blob_data: Vec<u8>, das_proofs: Vec<Vec<u8>>, height: u64) -> Self {
        Self { blob_data, das_proofs, height }
    }
}

/// Celestia finality proof (k confirmations + DAS)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CelestiaFinalityProof {
    /// Number of confirmations
    pub confirmations: u64,
    /// Whether data availability was verified
    pub das_verified: bool,
}

impl CelestiaFinalityProof {
    pub fn new(confirmations: u64, das_verified: bool) -> Self {
        Self { confirmations, das_verified }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seal_ref_creation() {
        let seal = CelestiaSealRef::new([1u8; 8], [2u8; 32], 42);
        assert_eq!(seal.nonce, 42);
    }

    #[test]
    fn test_anchor_ref_creation() {
        let anchor = CelestiaAnchorRef::new([3u8; 32], 100, [4u8; 8], 0);
        assert_eq!(anchor.height, 100);
    }
}
