//! Celestia blob submission and DAS verification

use crate::rpc::CelestiaRpc;
use csv_adapter_core::hash::Hash;

/// Blob submission result
#[derive(Clone, Debug)]
pub struct BlobSubmission {
    pub tx_hash: [u8; 32],
    pub height: u64,
    pub commitment: [u8; 32],
    pub share_index: u64,
}

/// Blob submitter
pub struct BlobSubmitter;

impl BlobSubmitter {
    pub fn new() -> Self {
        Self
    }

    /// Submit a blob to Celestia via PayForBlob
    pub fn submit(
        &self,
        rpc: &dyn CelestiaRpc,
        namespace_id: [u8; 8],
        commitment: Hash,
        payload: Vec<u8>,
    ) -> Result<BlobSubmission, Box<dyn std::error::Error + Send + Sync>> {
        let tx_hash = rpc.submit_pay_for_blob(namespace_id, payload)?;
        let latest_height = rpc.get_latest_block_height()?;

        Ok(BlobSubmission {
            tx_hash,
            height: latest_height + 1, // Blob included in next block
            commitment: *commitment.as_bytes(),
            share_index: 0,
        })
    }

    /// Verify blob inclusion after submission
    pub fn verify_inclusion(
        &self,
        rpc: &dyn CelestiaRpc,
        namespace_id: [u8; 8],
        height: u64,
        commitment: [u8; 32],
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        let blob = rpc.get_blob(namespace_id, height, commitment)?;
        Ok(blob.is_some())
    }
}

impl Default for BlobSubmitter {
    fn default() -> Self {
        Self::new()
    }
}

/// DAS (Data Availability Sampling) verifier
pub struct DasVerifier;

impl DasVerifier {
    pub fn new() -> Self {
        Self
    }

    /// Verify data availability via sampling
    ///
    /// This implements Celestia's Data Availability Sampling (DAS) verification.
    /// It samples multiple shares across the data square to verify availability.
    pub fn verify(
        &self,
        rpc: &dyn CelestiaRpc,
        height: u64,
        namespace_id: [u8; 8],
        share_index: u64,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        // Sample shares at regular intervals across the namespace
        let sample_count = 5;
        let stride = 10;

        for i in 0..sample_count {
            let sample_share = share_index + (i as u64 * stride);

            let available = rpc.verify_data_availability(height, namespace_id, sample_share)?;
            if !available {
                return Ok(false);
            }
        }

        // Verify data root consistency
        let data_root = rpc.get_block_data_root(height)?;
        if data_root == [0u8; 32] {
            return Ok(false);
        }

        Ok(true)
    }
}

impl Default for DasVerifier {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rpc::MockCelestiaRpc;

    #[test]
    fn test_blob_submission() {
        let rpc = MockCelestiaRpc::new(100);
        let submitter = BlobSubmitter::new();
        let result = submitter
            .submit(&rpc, [1u8; 8], Hash::new([2u8; 32]), vec![3, 4, 5])
            .unwrap();
        assert_eq!(result.tx_hash, [0xAB; 32]);
    }

    #[test]
    fn test_das_verification() {
        let rpc = MockCelestiaRpc::new(100);
        // Set a data root for the test height
        rpc.set_data_root(100, [1u8; 32]);
        let verifier = DasVerifier::new();
        assert!(verifier.verify(&rpc, 100, [1u8; 8], 0).unwrap());
    }
}
