//! Sui RPC trait and mock implementation

use std::collections::HashMap;
use std::sync::Mutex;

/// Trait for Sui RPC operations
pub trait SuiRpc: Send + Sync {
    /// Get object by ID
    fn get_object(
        &self,
        object_id: [u8; 32],
    ) -> Result<Option<SuiObject>, Box<dyn std::error::Error + Send + Sync>>;

    /// Get transaction block by digest
    fn get_transaction_block(
        &self,
        digest: [u8; 32],
    ) -> Result<Option<SuiTransactionBlock>, Box<dyn std::error::Error + Send + Sync>>;

    /// Get transaction events by digest
    fn get_transaction_events(
        &self,
        digest: [u8; 32],
    ) -> Result<Vec<SuiEvent>, Box<dyn std::error::Error + Send + Sync>>;

    /// Get checkpoint by sequence number
    fn get_checkpoint(
        &self,
        sequence_number: u64,
    ) -> Result<Option<SuiCheckpoint>, Box<dyn std::error::Error + Send + Sync>>;

    /// Get latest checkpoint sequence number
    fn get_latest_checkpoint_sequence_number(
        &self,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;

    /// Get the sender's address
    fn sender_address(&self) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>>;

    /// Get gas objects owned by the sender
    fn get_gas_objects(
        &self,
        owner: [u8; 32],
    ) -> Result<Vec<SuiObject>, Box<dyn std::error::Error + Send + Sync>>;

    /// Execute a signed MoveCall transaction and return the transaction digest
    ///
    /// # Arguments
    /// * `tx_bytes` - BCS-serialized TransactionData
    /// * `signature` - Ed25519 signature (64 bytes)
    /// * `public_key` - Signer's public key (32 bytes)
    fn execute_signed_transaction(
        &self,
        tx_bytes: Vec<u8>,
        signature: Vec<u8>,
        public_key: Vec<u8>,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>>;

    /// Wait for transaction confirmation
    fn wait_for_transaction(
        &self,
        digest: [u8; 32],
        timeout_ms: u64,
    ) -> Result<Option<SuiTransactionBlock>, Box<dyn std::error::Error + Send + Sync>>;

    /// Get ledger info
    fn get_ledger_info(&self) -> Result<SuiLedgerInfo, Box<dyn std::error::Error + Send + Sync>>;

    /// Downcast to Any for feature-gated real implementations
    fn as_any(&self) -> &dyn std::any::Any {
        unimplemented!("as_any() must be implemented by concrete types")
    }
}

/// Sui object representation
#[derive(Clone, Debug)]
pub struct SuiObject {
    pub object_id: [u8; 32],
    pub version: u64,
    pub owner: Vec<u8>,
    pub object_type: String,
    pub has_public_transfer: bool,
}

/// Sui object change type
#[derive(Clone, Debug)]
pub struct SuiObjectChange {
    pub object_id: [u8; 32],
    pub change_type: String,
}

/// Sui execution status
#[derive(Clone, Debug, PartialEq)]
pub enum SuiExecutionStatus {
    Success,
    Failure { error: String },
}

/// Sui transaction effects
#[derive(Clone, Debug)]
pub struct SuiTransactionEffects {
    pub status: SuiExecutionStatus,
    pub gas_used: u64,
    pub modified_objects: Vec<SuiObjectChange>,
}

/// Sui transaction block
#[derive(Clone, Debug)]
pub struct SuiTransactionBlock {
    pub digest: [u8; 32],
    pub checkpoint: Option<u64>,
    pub effects: SuiTransactionEffects,
}

/// Sui event
#[derive(Clone, Debug)]
pub struct SuiEvent {
    pub id: String,
    pub transaction_digest: [u8; 32],
    pub event_sequence_number: u64,
    pub type_field: String,
    pub data: Vec<u8>,
}

/// Sui checkpoint
#[derive(Clone, Debug)]
pub struct SuiCheckpoint {
    pub sequence_number: u64,
    pub digest: [u8; 32],
    pub epoch: u64,
    pub network_total_transactions: u64,
    pub certified: bool,
}

/// Sui ledger info
#[derive(Clone, Debug)]
pub struct SuiLedgerInfo {
    pub latest_version: u64,
    pub latest_epoch: u64,
}

/// Mock Sui RPC for testing
///
/// This implementation is only compiled in debug builds to prevent
/// accidental use in production environments.
#[cfg(debug_assertions)]
pub struct MockSuiRpc {
    objects: Mutex<HashMap<[u8; 32], SuiObject>>,
    transactions: Mutex<HashMap<[u8; 32], SuiTransactionBlock>>,
    checkpoints: Mutex<HashMap<u64, SuiCheckpoint>>,
    latest_checkpoint: u64,
    mock_address: [u8; 32],
    tx_counter: std::sync::atomic::AtomicU64,
}

#[cfg(debug_assertions)]
impl MockSuiRpc {
    pub fn new(latest_checkpoint: u64) -> Self {
        Self {
            objects: Mutex::new(HashMap::new()),
            transactions: Mutex::new(HashMap::new()),
            checkpoints: Mutex::new(HashMap::new()),
            latest_checkpoint,
            mock_address: [0x42; 32],
            tx_counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn new_with_address(latest_checkpoint: u64, address: [u8; 32]) -> Self {
        Self {
            objects: Mutex::new(HashMap::new()),
            transactions: Mutex::new(HashMap::new()),
            checkpoints: Mutex::new(HashMap::new()),
            latest_checkpoint,
            mock_address: address,
            tx_counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    pub fn add_object(&self, object: SuiObject) {
        self.objects
            .lock()
            .unwrap()
            .insert(object.object_id, object);
    }

    pub fn add_transaction(&self, tx: SuiTransactionBlock) {
        self.transactions.lock().unwrap().insert(tx.digest, tx);
    }

    pub fn add_checkpoint(&self, checkpoint: SuiCheckpoint) {
        self.checkpoints
            .lock()
            .unwrap()
            .insert(checkpoint.sequence_number, checkpoint);
    }
}

#[cfg(debug_assertions)]
impl SuiRpc for MockSuiRpc {
    fn get_object(
        &self,
        object_id: [u8; 32],
    ) -> Result<Option<SuiObject>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.objects.lock().unwrap().get(&object_id).cloned())
    }

    fn get_transaction_block(
        &self,
        digest: [u8; 32],
    ) -> Result<Option<SuiTransactionBlock>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.transactions.lock().unwrap().get(&digest).cloned())
    }

    fn get_transaction_events(
        &self,
        _digest: [u8; 32],
    ) -> Result<Vec<SuiEvent>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Vec::new())
    }

    fn get_checkpoint(
        &self,
        sequence_number: u64,
    ) -> Result<Option<SuiCheckpoint>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self
            .checkpoints
            .lock()
            .unwrap()
            .get(&sequence_number)
            .cloned())
    }

    fn get_latest_checkpoint_sequence_number(
        &self,
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.latest_checkpoint)
    }

    fn sender_address(&self) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.mock_address)
    }

    fn get_gas_objects(
        &self,
        _owner: [u8; 32],
    ) -> Result<Vec<SuiObject>, Box<dyn std::error::Error + Send + Sync>> {
        // Return mock gas objects
        Ok(vec![SuiObject {
            object_id: [0x01; 32],
            version: 1,
            owner: self.mock_address.to_vec(),
            object_type: "0x2::coin::Coin<0x2::sui::SUI>".to_string(),
            has_public_transfer: true,
        }])
    }

    fn execute_signed_transaction(
        &self,
        _tx_bytes: Vec<u8>,
        _signature: Vec<u8>,
        _public_key: Vec<u8>,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
        // Mock: return a deterministic digest with incrementing counter
        let counter = self.tx_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let mut digest = [0u8; 32];
        digest[..4].copy_from_slice(b"mock");
        digest[4..12].copy_from_slice(&counter.to_le_bytes());
        Ok(digest)
    }

    fn wait_for_transaction(
        &self,
        _digest: [u8; 32],
        _timeout_ms: u64,
    ) -> Result<Option<SuiTransactionBlock>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(None)
    }

    fn get_ledger_info(&self) -> Result<SuiLedgerInfo, Box<dyn std::error::Error + Send + Sync>> {
        Ok(SuiLedgerInfo {
            latest_version: self.latest_checkpoint,
            latest_epoch: 1,
        })
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_object() {
        let rpc = MockSuiRpc::new(1000);
        let obj = SuiObject {
            object_id: [1u8; 32],
            version: 1,
            owner: vec![2, 3],
            object_type: "CSV::Seal".to_string(),
            has_public_transfer: false,
        };
        rpc.add_object(obj.clone());

        let fetched = rpc.get_object([1u8; 32]).unwrap();
        assert_eq!(fetched.unwrap().version, 1);
    }

    #[test]
    fn test_mock_checkpoint() {
        let rpc = MockSuiRpc::new(1000);
        let cp = SuiCheckpoint {
            sequence_number: 500,
            digest: [1u8; 32],
            epoch: 1,
            network_total_transactions: 50000,
            certified: true,
        };
        rpc.add_checkpoint(cp.clone());

        let fetched = rpc.get_checkpoint(500).unwrap();
        assert!(fetched.unwrap().certified);
    }
}
