//! Celestia RPC trait and mock
//!
//! This module provides RPC abstraction for Celestia, including:
//! * Blob submission via PayForBlob
//! * Blob retrieval and verification
//! * Data Availability Sampling (DAS) verification
//! * Block data root queries

use std::collections::HashMap;
use std::sync::Mutex;

/// Trait for Celestia RPC operations
pub trait CelestiaRpc: Send + Sync {
    fn get_latest_block_height(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;
    fn submit_pay_for_blob(
        &self,
        namespace_id: [u8; 8],
        blob_data: Vec<u8>,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>>;
    fn get_blob(
        &self,
        namespace_id: [u8; 8],
        height: u64,
        commitment: [u8; 32],
    ) -> Result<Option<BlobResponse>, Box<dyn std::error::Error + Send + Sync>>;
    fn get_block_data_root(
        &self,
        height: u64,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>>;
    fn verify_data_availability(
        &self,
        height: u64,
        namespace_id: [u8; 8],
        share_index: u64,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
}

#[derive(Clone, Debug)]
pub struct BlobResponse {
    pub namespace_id: [u8; 8],
    pub data: Vec<u8>,
    pub height: u64,
    pub commitment: [u8; 32],
    pub share_index: u64,
}

/// Mock Celestia RPC
///
/// This implementation is only compiled in debug builds to prevent
/// accidental use in production environments.
#[cfg(debug_assertions)]
pub struct MockCelestiaRpc {
    pub latest_height: u64,
    pub blobs: Mutex<HashMap<([u8; 8], u64, [u8; 32]), BlobResponse>>,
    pub data_roots: Mutex<HashMap<u64, [u8; 32]>>,
    pub sent_transactions: Mutex<Vec<([u8; 8], Vec<u8>)>>,
}

#[cfg(debug_assertions)]
impl MockCelestiaRpc {
    pub fn new(latest_height: u64) -> Self {
        Self {
            latest_height,
            blobs: Mutex::new(HashMap::new()),
            data_roots: Mutex::new(HashMap::new()),
            sent_transactions: Mutex::new(Vec::new()),
        }
    }

    pub fn add_blob(&self, response: BlobResponse) {
        self.blobs.lock().unwrap().insert(
            (response.namespace_id, response.height, response.commitment),
            response,
        );
    }

    pub fn set_data_root(&self, height: u64, root: [u8; 32]) {
        self.data_roots.lock().unwrap().insert(height, root);
    }
}

#[cfg(debug_assertions)]
impl CelestiaRpc for MockCelestiaRpc {
    fn get_latest_block_height(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.latest_height)
    }

    fn submit_pay_for_blob(
        &self,
        namespace_id: [u8; 8],
        blob_data: Vec<u8>,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
        self.sent_transactions
            .lock()
            .unwrap()
            .push((namespace_id, blob_data));
        Ok([0xAB; 32])
    }

    fn get_blob(
        &self,
        namespace_id: [u8; 8],
        height: u64,
        commitment: [u8; 32],
    ) -> Result<Option<BlobResponse>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self
            .blobs
            .lock()
            .unwrap()
            .get(&(namespace_id, height, commitment))
            .cloned())
    }

    fn get_block_data_root(
        &self,
        height: u64,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
        self.data_roots
            .lock()
            .unwrap()
            .get(&height)
            .copied()
            .ok_or_else(|| -> Box<dyn std::error::Error + Send + Sync> {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    format!("Block data root not found for height {}", height),
                ))
            })
    }

    fn verify_data_availability(
        &self,
        _height: u64,
        _namespace_id: [u8; 8],
        _share_index: u64,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        Ok(true)
    }
}

#[cfg(not(debug_assertions))]
compile_error!(
    "MockCelestiaRpc can only be used in debug builds. \
    This prevents accidental use of mock implementations in production."
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_height() {
        let rpc = MockCelestiaRpc::new(1000);
        assert_eq!(rpc.get_latest_block_height().unwrap(), 1000);
    }

    #[test]
    fn test_mock_blob() {
        let rpc = MockCelestiaRpc::new(1000);
        let blob = BlobResponse {
            namespace_id: [1u8; 8],
            data: vec![0xAB, 0xCD],
            height: 100,
            commitment: [2u8; 32],
            share_index: 0,
        };
        rpc.add_blob(blob.clone());

        let fetched = rpc.get_blob([1u8; 8], 100, [2u8; 32]).unwrap();
        assert_eq!(fetched.unwrap().data, vec![0xAB, 0xCD]);
    }

    #[test]
    fn test_mock_data_root() {
        let rpc = MockCelestiaRpc::new(1000);
        rpc.set_data_root(100, [3u8; 32]);

        let root = rpc.get_block_data_root(100).unwrap();
        assert_eq!(root, [3u8; 32]);
    }

    #[test]
    fn test_mock_submit_blob() {
        let rpc = MockCelestiaRpc::new(1000);
        let tx_hash = rpc.submit_pay_for_blob([1u8; 8], vec![0xAB, 0xCD]).unwrap();
        assert_eq!(tx_hash, [0xAB; 32]);
    }

    #[test]
    fn test_mock_das_verify() {
        let rpc = MockCelestiaRpc::new(1000);
        let verified = rpc.verify_data_availability(100, [1u8; 8], 0).unwrap();
        assert!(verified);
    }
}
