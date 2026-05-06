//! Ethereum RPC trait and test helpers
//!
//! Defines the minimal set of Ethereum JSON-RPC calls needed
//! by the CSV adapter: storage proofs, receipts, block queries, finality.

use async_trait::async_trait;

#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::Mutex;

/// Trait for Ethereum RPC operations
#[async_trait]
pub trait EthereumRpc: Send + Sync {
    /// Get current block number
    async fn block_number(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;

    /// Get block by number (returns block hash)
    async fn get_block_hash(
        &self,
        block_number: u64,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>>;

    /// Get storage proof for a contract's storage slot
    async fn get_proof(
        &self,
        address: [u8; 20],
        keys: Vec<[u8; 32]>,
        block_number: u64,
    ) -> Result<StorageProof, Box<dyn std::error::Error + Send + Sync>>;

    /// Get transaction receipt
    async fn get_transaction_receipt(
        &self,
        tx_hash: [u8; 32],
    ) -> Result<Option<TransactionReceipt>, Box<dyn std::error::Error + Send + Sync>>;

    /// Get block by hash (returns state root)
    async fn get_block_state_root(
        &self,
        block_hash: [u8; 32],
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>>;

    /// Get finalized block number (post-merge)
    async fn get_finalized_block_number(
        &self,
    ) -> Result<Option<u64>, Box<dyn std::error::Error + Send + Sync>>;

    /// Send raw transaction
    async fn send_raw_transaction(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>>;

    /// Get account balance
    async fn get_balance(
        &self,
        address: [u8; 20],
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;

    /// Get transaction count (nonce) for an address
    async fn get_transaction_count(
        &self,
        address: [u8; 20],
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;

    /// Get code at an address
    async fn get_code(
        &self,
        address: [u8; 20],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>>;

    /// Downcast to `Any` for feature-gated real implementations.
    /// Concrete types may override for explicit downcasting.
    fn as_any(&self) -> Option<&dyn std::any::Any> {
        None
    }

    /// Clone the RPC client for creating new boxed instances
    fn clone_boxed(&self) -> Box<dyn EthereumRpc>;

    /// Get current gas price
    async fn get_gas_price(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;

    /// Get block by number (returns full block info)
    async fn get_block_by_number(
        &self,
        block_number: u64,
    ) -> Result<Option<RpcBlock>, Box<dyn std::error::Error + Send + Sync>>;

    /// Get transaction by hash
    async fn get_transaction(
        &self,
        tx_hash: [u8; 32],
    ) -> Result<Option<RpcTransaction>, Box<dyn std::error::Error + Send + Sync>>;
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
    /// Gas used by the transaction
    pub gas_used: u64,
    /// Whether transaction was successful
    pub success: bool,
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

/// RPC Block representation
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RpcBlock {
    /// Block number
    pub number: u64,
    /// Block hash
    pub hash: [u8; 32],
    /// State root
    pub state_root: [u8; 32],
    /// Timestamp
    pub timestamp: u64,
}

/// RPC Transaction representation
#[derive(Clone, Debug)]
pub struct RpcTransaction {
    /// Transaction hash
    pub hash: [u8; 32],
    /// Sender address
    pub from: [u8; 20],
    /// Recipient address (None for contract creation)
    pub to: Option<[u8; 20]>,
    /// Transaction value
    pub value: Option<u64>,
    /// Gas price
    pub gas_price: Option<u64>,
    /// Gas used
    pub gas: u64,
    /// Block number
    pub block_number: Option<u64>,
}

/// Mock Ethereum RPC for testing
///
/// This implementation is only compiled in test builds to prevent
/// accidental use in production environments.
#[cfg(test)]
#[allow(clippy::type_complexity)]
pub struct MockEthereumRpc {
    pub block_number: u64,
    pub finalized_block: Option<u64>,
    pub storage_values: Mutex<HashMap<([u8; 20], [u8; 32]), Vec<u8>>>,
    pub receipts: Mutex<HashMap<[u8; 32], TransactionReceipt>>,
    pub sent_transactions: Mutex<Vec<Vec<u8>>>,
    pub state_roots: Mutex<HashMap<[u8; 32], [u8; 32]>>,
    pub blocks: Mutex<HashMap<u64, RpcBlock>>,
    pub transactions: Mutex<HashMap<[u8; 32], RpcTransaction>>,
    pub gas_price: u64,
}

#[cfg(test)]
impl MockEthereumRpc {
    pub fn new(block_number: u64) -> Self {
        let mut blocks = HashMap::new();
        // Create a default block at the current block number
        let block = RpcBlock {
            number: block_number,
            hash: [0u8; 32],
            state_root: [0u8; 32],
            timestamp: 0,
        };
        blocks.insert(block_number, block);
        
        Self {
            block_number,
            finalized_block: Some(block_number.saturating_sub(64)),
            storage_values: Mutex::new(HashMap::new()),
            receipts: Mutex::new(HashMap::new()),
            sent_transactions: Mutex::new(Vec::new()),
            state_roots: Mutex::new(HashMap::new()),
            blocks: Mutex::new(blocks),
            transactions: Mutex::new(HashMap::new()),
            gas_price: 20_000_000_000, // 20 Gwei default
        }
    }

    pub fn add_block(&self, block: RpcBlock) {
        self.blocks.lock().unwrap().insert(block.number, block);
    }

    pub fn add_transaction(&self, tx: RpcTransaction) {
        self.transactions.lock().unwrap().insert(tx.hash, tx);
    }

    pub fn set_gas_price(&mut self, price: u64) {
        self.gas_price = price;
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

#[cfg(test)]
#[async_trait]
impl EthereumRpc for MockEthereumRpc {
    async fn block_number(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.block_number)
    }

    async fn get_block_hash(
        &self,
        block_number: u64,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
        let mut hash = [0u8; 32];
        hash[..8].copy_from_slice(&block_number.to_be_bytes());
        Ok(hash)
    }

    async fn get_proof(
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

    async fn get_transaction_receipt(
        &self,
        tx_hash: [u8; 32],
    ) -> Result<Option<TransactionReceipt>, Box<dyn std::error::Error + Send + Sync>> {
        let receipts = self.receipts.lock().unwrap();
        Ok(receipts.get(&tx_hash).cloned())
    }

    async fn get_block_state_root(
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

    async fn get_finalized_block_number(
        &self,
    ) -> Result<Option<u64>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.finalized_block)
    }

    async fn send_raw_transaction(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
        self.sent_transactions.lock().unwrap().push(tx_bytes);
        Ok([0xAB; 32])
    }

    async fn get_balance(
        &self,
        _address: [u8; 20],
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        Ok(1000000000000000000u64) // Mock 1 ETH balance
    }

    async fn get_transaction_count(
        &self,
        _address: [u8; 20],
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        Ok(0u64) // Mock nonce
    }

    async fn get_code(
        &self,
        _address: [u8; 20],
    ) -> Result<Vec<u8>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(vec![]) // Mock empty code (EOA)
    }

    fn as_any(&self) -> Option<&dyn std::any::Any> {
        Some(self)
    }

    fn clone_boxed(&self) -> Box<dyn EthereumRpc> {
        Box::new(MockEthereumRpc {
            block_number: self.block_number,
            finalized_block: self.finalized_block,
            storage_values: Mutex::new(self.storage_values.lock().unwrap().clone()),
            receipts: Mutex::new(self.receipts.lock().unwrap().clone()),
            sent_transactions: Mutex::new(self.sent_transactions.lock().unwrap().clone()),
            state_roots: Mutex::new(self.state_roots.lock().unwrap().clone()),
            blocks: Mutex::new(self.blocks.lock().unwrap().clone()),
            transactions: Mutex::new(self.transactions.lock().unwrap().clone()),
            gas_price: self.gas_price,
        })
    }

    async fn get_gas_price(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.gas_price)
    }

    async fn get_block_by_number(
        &self,
        block_number: u64,
    ) -> Result<Option<RpcBlock>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.blocks.lock().unwrap().get(&block_number).cloned())
    }

    async fn get_transaction(
        &self,
        tx_hash: [u8; 32],
    ) -> Result<Option<RpcTransaction>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.transactions.lock().unwrap().get(&tx_hash).cloned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_ethereum_rpc_block_number() {
        let rpc = MockEthereumRpc::new(1000);
        assert_eq!(rpc.block_number().await.unwrap(), 1000);
    }

    #[tokio::test]
    async fn test_ethereum_rpc_storage() {
        let rpc = MockEthereumRpc::new(1000);
        let address = [1u8; 20];
        let key = [2u8; 32];
        let value = 1000u64.to_be_bytes().to_vec();

        rpc.set_storage(address, key, value.clone());

        let proof = rpc.get_proof(address, vec![key], 1000).await.unwrap();
        assert_eq!(proof.storage_proof.len(), 1);
        assert_eq!(proof.storage_proof[0].value, value);
    }

    #[tokio::test]
    async fn test_ethereum_rpc_receipt() {
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

        let fetched = rpc.get_transaction_receipt(tx_hash).await.unwrap();
        assert_eq!(fetched.logs.len(), 1);
        assert_eq!(fetched.status, 1);
    }

    #[tokio::test]
    async fn test_ethereum_rpc_finalized() {
        let rpc = MockEthereumRpc::new(1000);
        let finalized = rpc.get_finalized_block_number().await.unwrap();
        assert!(finalized.is_some());
        assert!(finalized.unwrap() <= 1000);
    }

    #[tokio::test]
    async fn test_ethereum_rpc_send_transaction() {
        let rpc = MockEthereumRpc::new(1000);
        let tx_hash = rpc.send_raw_transaction(vec![0x01, 0x02]).await.unwrap();
        assert_eq!(tx_hash, [0xAB; 32]);
    }
}
