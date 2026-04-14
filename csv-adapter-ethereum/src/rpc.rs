//! Ethereum RPC trait and mock implementation
//!
//! Defines the minimal set of Ethereum JSON-RPC calls needed
//! by the CSV adapter: storage proofs, receipts, block queries, finality.

#[cfg(debug_assertions)]
use std::collections::HashMap;
#[cfg(debug_assertions)]
use std::sync::Mutex;

/// Trait for Ethereum RPC operations
pub trait EthereumRpc: Send + Sync {
    /// Get current block number
    fn block_number(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;

    /// Get block by number (returns block hash)
    fn get_block_hash(
        &self,
        block_number: u64,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>>;

    /// Get storage proof for a contract's storage slot
    fn get_proof(
        &self,
        address: [u8; 20],
        keys: Vec<[u8; 32]>,
        block_number: u64,
    ) -> Result<StorageProof, Box<dyn std::error::Error + Send + Sync>>;

    /// Get transaction receipt
    fn get_transaction_receipt(
        &self,
        tx_hash: [u8; 32],
    ) -> Result<TransactionReceipt, Box<dyn std::error::Error + Send + Sync>>;

    /// Get block by hash (returns state root)
    fn get_block_state_root(
        &self,
        block_hash: [u8; 32],
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>>;

    /// Get finalized block number (post-merge)
    fn get_finalized_block_number(
        &self,
    ) -> Result<Option<u64>, Box<dyn std::error::Error + Send + Sync>>;

    /// Send raw transaction
    fn send_raw_transaction(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>>;

    /// Downcast to `Any` for feature-gated real implementations.
    /// Default returns self; concrete types may override for explicit downcasting.
    fn as_any(&self) -> &dyn std::any::Any
    where
        Self: Sized + 'static,
    {
        self
    }
}

/// Storage proof response (matches eth_getProof format)
#[derive(Clone, Debug)]
pub struct StorageProof {
    /// Account proof (MPT branch from state root to account)
    pub account_proof: Vec<Vec<u8>>,
    /// Account balance
    pub balance: String,
    /// Account code hash
    pub code_hash: [u8; 32],
    /// Account nonce
    pub nonce: String,
    /// Storage root hash
    pub storage_hash: [u8; 32],
    /// Storage proofs for each requested key
    pub storage_proof: Vec<SingleStorageProof>,
}

/// Storage proof for a single key
#[derive(Clone, Debug)]
pub struct SingleStorageProof {
    /// Storage key
    pub key: [u8; 32],
    /// Storage value (RLP-encoded)
    pub value: Vec<u8>,
    /// MPT proof nodes
    pub proof: Vec<Vec<u8>>,
}

/// Transaction receipt
#[derive(Clone, Debug)]
pub struct TransactionReceipt {
    /// Transaction hash
    pub tx_hash: [u8; 32],
    /// Block number
    pub block_number: u64,
    /// Block hash
    pub block_hash: [u8; 32],
    /// Contract address (if deployment)
    pub contract_address: Option<[u8; 20]>,
    /// Logs emitted
    pub logs: Vec<LogEntry>,
    /// Status (1 = success, 0 = failure)
    pub status: u64,
}

/// LOG event entry
#[derive(Clone, Debug)]
pub struct LogEntry {
    /// Emitter contract address
    pub address: [u8; 20],
    /// Event topics (indexed parameters)
    pub topics: Vec<[u8; 32]>,
    /// Event data (non-indexed parameters)
    pub data: Vec<u8>,
    /// Log index within transaction
    pub log_index: u64,
}

/// Mock Ethereum RPC for testing
///
/// This implementation is only compiled in debug builds to prevent
/// accidental use in production environments.
#[cfg(debug_assertions)]
#[allow(clippy::type_complexity)]
pub struct MockEthereumRpc {
    pub block_number: u64,
    pub finalized_block: Option<u64>,
    pub storage_values: Mutex<HashMap<([u8; 20], [u8; 32]), Vec<u8>>>,
    pub receipts: Mutex<HashMap<[u8; 32], TransactionReceipt>>,
    pub sent_transactions: Mutex<Vec<Vec<u8>>>,
    pub state_roots: Mutex<HashMap<[u8; 32], [u8; 32]>>,
}

#[cfg(debug_assertions)]
impl MockEthereumRpc {
    pub fn new(block_number: u64) -> Self {
        Self {
            block_number,
            finalized_block: Some(block_number.saturating_sub(64)),
            storage_values: Mutex::new(HashMap::new()),
            receipts: Mutex::new(HashMap::new()),
            sent_transactions: Mutex::new(Vec::new()),
            state_roots: Mutex::new(HashMap::new()),
        }
    }

    pub fn set_storage(&self, address: [u8; 20], key: [u8; 32], value: Vec<u8>) {
        self.storage_values
            .lock()
            .unwrap()
            .insert((address, key), value);
    }

    pub fn add_receipt(&self, tx_hash: [u8; 32], receipt: TransactionReceipt) {
        self.receipts.lock().unwrap().insert(tx_hash, receipt);
    }

    pub fn set_state_root(&self, block_hash: [u8; 32], state_root: [u8; 32]) {
        self.state_roots
            .lock()
            .unwrap()
            .insert(block_hash, state_root);
    }
}

#[cfg(debug_assertions)]
impl EthereumRpc for MockEthereumRpc {
    fn block_number(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.block_number)
    }

    fn get_block_hash(
        &self,
        block_number: u64,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
        let mut hash = [0u8; 32];
        hash[..8].copy_from_slice(&block_number.to_be_bytes());
        Ok(hash)
    }

    fn get_proof(
        &self,
        address: [u8; 20],
        keys: Vec<[u8; 32]>,
        _block_number: u64,
    ) -> Result<StorageProof, Box<dyn std::error::Error + Send + Sync>> {
        let storage_proof: Vec<_> = keys
            .iter()
            .map(|key| {
                let value = self
                    .storage_values
                    .lock()
                    .unwrap()
                    .get(&(address, *key))
                    .cloned()
                    .unwrap_or_default();
                SingleStorageProof {
                    key: *key,
                    value,
                    proof: vec![vec![0xAB; 32]], // Mock MPT proof nodes
                }
            })
            .collect();

        Ok(StorageProof {
            account_proof: vec![vec![0xCD; 32]],
            balance: "0".to_string(),
            code_hash: [0u8; 32],
            nonce: "0".to_string(),
            storage_hash: [0xEF; 32],
            storage_proof,
        })
    }

    fn get_transaction_receipt(
        &self,
        tx_hash: [u8; 32],
    ) -> Result<TransactionReceipt, Box<dyn std::error::Error + Send + Sync>> {
        let receipts = self.receipts.lock().unwrap();
        receipts.get(&tx_hash).cloned().ok_or_else(
            || -> Box<dyn std::error::Error + Send + Sync> {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Receipt not found",
                ))
            },
        )
    }

    fn get_block_state_root(
        &self,
        block_hash: [u8; 32],
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
        let roots = self.state_roots.lock().unwrap();
        roots.get(&block_hash).copied().ok_or_else(
            || -> Box<dyn std::error::Error + Send + Sync> {
                Box::new(std::io::Error::new(
                    std::io::ErrorKind::NotFound,
                    "Block not found",
                ))
            },
        )
    }

    fn get_finalized_block_number(
        &self,
    ) -> Result<Option<u64>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.finalized_block)
    }

    fn send_raw_transaction(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
        self.sent_transactions.lock().unwrap().push(tx_bytes);
        Ok([0xAB; 32])
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_block_number() {
        let rpc = MockEthereumRpc::new(1000);
        assert_eq!(rpc.block_number().unwrap(), 1000);
    }

    #[test]
    fn test_mock_storage() {
        let rpc = MockEthereumRpc::new(1000);
        let address = [1u8; 20];
        let key = [2u8; 32];
        let value = 1000u64.to_be_bytes().to_vec();

        rpc.set_storage(address, key, value.clone());

        let proof = rpc.get_proof(address, vec![key], 1000).unwrap();
        assert_eq!(proof.storage_proof.len(), 1);
        assert_eq!(proof.storage_proof[0].value, value);
    }

    #[test]
    fn test_mock_receipt() {
        let rpc = MockEthereumRpc::new(1000);
        let tx_hash = [3u8; 32];
        let receipt = TransactionReceipt {
            tx_hash,
            block_number: 500,
            block_hash: [4u8; 32],
            contract_address: None,
            logs: vec![LogEntry {
                address: [5u8; 20],
                topics: vec![[6u8; 32]],
                data: vec![7, 8, 9],
                log_index: 0,
            }],
            status: 1,
        };

        rpc.add_receipt(tx_hash, receipt.clone());

        let fetched = rpc.get_transaction_receipt(tx_hash).unwrap();
        assert_eq!(fetched.logs.len(), 1);
        assert_eq!(fetched.status, 1);
    }

    #[test]
    fn test_mock_finalized() {
        let rpc = MockEthereumRpc::new(1000);
        let finalized = rpc.get_finalized_block_number().unwrap();
        assert!(finalized.is_some());
        assert!(finalized.unwrap() <= 1000);
    }

    #[test]
    fn test_mock_send_transaction() {
        let rpc = MockEthereumRpc::new(1000);
        let tx_hash = rpc.send_raw_transaction(vec![0x01, 0x02]).unwrap();
        assert_eq!(tx_hash, [0xAB; 32]);
    }
}
