//! Aptos RPC trait and mock implementation

/// Trait for Aptos RPC operations
pub trait AptosRpc: Send + Sync {
    fn get_ledger_info(&self) -> Result<AptosLedgerInfo, Box<dyn std::error::Error + Send + Sync>>;

    fn get_resource(
        &self,
        address: [u8; 32],
        resource_type: &str,
        position: Option<u64>,
    ) -> Result<Option<AptosResource>, Box<dyn std::error::Error + Send + Sync>>;

    fn get_transaction(
        &self,
        version: u64,
    ) -> Result<Option<AptosTransaction>, Box<dyn std::error::Error + Send + Sync>>;

    fn get_transactions(
        &self,
        start_version: u64,
        limit: u32,
    ) -> Result<Vec<AptosTransaction>, Box<dyn std::error::Error + Send + Sync>>;

    fn get_events(
        &self,
        event_handle: &str,
        position: &str,
        limit: u32,
    ) -> Result<Vec<AptosEvent>, Box<dyn std::error::Error + Send + Sync>>;

    fn submit_transaction(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>>;

    fn wait_for_transaction(
        &self,
        tx_hash: [u8; 32],
    ) -> Result<AptosTransaction, Box<dyn std::error::Error + Send + Sync>>;

    fn get_block_by_version(
        &self,
        version: u64,
    ) -> Result<Option<AptosBlockInfo>, Box<dyn std::error::Error + Send + Sync>>;

    fn get_events_by_account(
        &self,
        account: [u8; 32],
        start: u64,
        limit: u32,
    ) -> Result<Vec<AptosEvent>, Box<dyn std::error::Error + Send + Sync>>;

    fn get_latest_version(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;

    fn get_transaction_by_version(
        &self,
        version: u64,
    ) -> Result<Option<AptosTransaction>, Box<dyn std::error::Error + Send + Sync>>;

    fn publish_module(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>>;

    fn verify_checkpoint(
        &self,
        sequence_number: u64,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;

    fn as_any(&self) -> &dyn std::any::Any {
        unimplemented!("as_any() must be implemented by concrete types")
    }
}

/// Aptos ledger info
#[derive(Clone, Debug)]
pub struct AptosLedgerInfo {
    pub chain_id: u64,
    pub epoch: u64,
    pub ledger_version: u64,
    pub oldest_ledger_version: u64,
    pub ledger_timestamp: u64,
    pub oldest_transaction_timestamp: u64,
    pub epoch_start_timestamp: u64,
}

/// Aptos resource
#[derive(Clone, Debug)]
pub struct AptosResource {
    pub data: Vec<u8>,
}

/// Aptos transaction
#[derive(Clone, Debug)]
pub struct AptosTransaction {
    pub version: u64,
    pub hash: [u8; 32],
    pub state_change_hash: [u8; 32],
    pub event_root_hash: [u8; 32],
    pub state_checkpoint_hash: Option<[u8; 32]>,
    pub epoch: u64,
    pub round: u64,
    pub events: Vec<AptosEvent>,
    pub payload: Vec<u8>,
    pub success: bool,
    pub vm_status: String,
    pub gas_used: u64,
    pub cumulative_gas_used: u64,
}

/// Aptos event
#[derive(Clone, Debug)]
pub struct AptosEvent {
    pub event_sequence_number: u64,
    pub key: String,
    pub data: Vec<u8>,
    pub transaction_version: u64,
}

/// Aptos block info
#[derive(Clone, Debug)]
pub struct AptosBlockInfo {
    pub version: u64,
    pub block_hash: [u8; 32],
    pub epoch: u64,
    pub round: u64,
    pub timestamp_usecs: u64,
}

/// Mock Aptos RPC for testing
///
/// This implementation is only compiled in debug builds to prevent
/// accidental use in production environments.
#[cfg(debug_assertions)]
pub struct MockAptosRpc {
    pub latest_version: u64,
    pub chain_id: u64,
    pub resources: std::sync::Mutex<std::collections::HashMap<([u8; 32], String), AptosResource>>,
    pub transactions: std::sync::Mutex<std::collections::HashMap<u64, AptosTransaction>>,
    pub events: std::sync::Mutex<std::collections::HashMap<String, Vec<AptosEvent>>>,
    pub blocks: std::sync::Mutex<std::collections::HashMap<u64, AptosBlockInfo>>,
    pub sent_transactions: std::sync::Mutex<Vec<Vec<u8>>>,
}

#[cfg(debug_assertions)]
impl MockAptosRpc {
    pub fn new(latest_version: u64) -> Self {
        Self {
            latest_version,
            chain_id: 1,
            resources: std::sync::Mutex::new(std::collections::HashMap::new()),
            transactions: std::sync::Mutex::new(std::collections::HashMap::new()),
            events: std::sync::Mutex::new(std::collections::HashMap::new()),
            sent_transactions: std::sync::Mutex::new(Vec::new()),
            blocks: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    pub fn with_chain_id(latest_version: u64, chain_id: u64) -> Self {
        Self {
            latest_version,
            chain_id,
            ..Self::new(latest_version)
        }
    }

    pub fn set_resource(&self, address: [u8; 32], resource_type: &str, resource: AptosResource) {
        self.resources
            .lock()
            .unwrap()
            .insert((address, resource_type.to_string()), resource);
    }

    pub fn add_transaction(&self, version: u64, tx: AptosTransaction) {
        self.transactions.lock().unwrap().insert(version, tx);
    }

    pub fn add_event(&self, handle: &str, event: AptosEvent) {
        self.events
            .lock()
            .unwrap()
            .entry(handle.to_string())
            .or_default()
            .push(event);
    }

    pub fn add_events(&self, handle: &str, events: Vec<AptosEvent>) {
        self.events
            .lock()
            .unwrap()
            .entry(handle.to_string())
            .or_default()
            .extend(events);
    }

    pub fn set_block(&self, version: u64, block: AptosBlockInfo) {
        self.blocks.lock().unwrap().insert(version, block);
    }
}

#[cfg(debug_assertions)]
impl AptosRpc for MockAptosRpc {
    fn get_ledger_info(&self) -> Result<AptosLedgerInfo, Box<dyn std::error::Error + Send + Sync>> {
        Ok(AptosLedgerInfo {
            chain_id: self.chain_id,
            epoch: 1,
            ledger_version: self.latest_version,
            oldest_ledger_version: 0,
            ledger_timestamp: 0,
            oldest_transaction_timestamp: 0,
            epoch_start_timestamp: 0,
        })
    }

    fn get_resource(
        &self,
        address: [u8; 32],
        resource_type: &str,
        _position: Option<u64>,
    ) -> Result<Option<AptosResource>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self
            .resources
            .lock()
            .unwrap()
            .get(&(address, resource_type.to_string()))
            .cloned())
    }

    fn get_transaction(
        &self,
        version: u64,
    ) -> Result<Option<AptosTransaction>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.transactions.lock().unwrap().get(&version).cloned())
    }

    fn get_transactions(
        &self,
        start_version: u64,
        limit: u32,
    ) -> Result<Vec<AptosTransaction>, Box<dyn std::error::Error + Send + Sync>> {
        let transactions = self.transactions.lock().unwrap();
        Ok(transactions
            .iter()
            .filter(|(v, _)| **v >= start_version)
            .take(limit as usize)
            .map(|(_, tx)| tx.clone())
            .collect())
    }

    fn get_events(
        &self,
        event_handle: &str,
        _position: &str,
        limit: u32,
    ) -> Result<Vec<AptosEvent>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self
            .events
            .lock()
            .unwrap()
            .get(event_handle)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .take(limit as usize)
            .collect())
    }

    fn submit_transaction(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
        self.sent_transactions.lock().unwrap().push(tx_bytes);
        Ok([0xAB; 32])
    }

    fn wait_for_transaction(
        &self,
        _tx_hash: [u8; 32],
    ) -> Result<AptosTransaction, Box<dyn std::error::Error + Send + Sync>> {
        Ok(AptosTransaction {
            version: self.latest_version,
            hash: [0xCD; 32],
            state_change_hash: [0xEF; 32],
            event_root_hash: [0x1F; 32],
            state_checkpoint_hash: None,
            epoch: 1,
            round: 0,
            events: Vec::new(),
            payload: Vec::new(),
            success: true,
            vm_status: "Executed".to_string(),
            gas_used: 0,
            cumulative_gas_used: 0,
        })
    }

    fn get_block_by_version(
        &self,
        version: u64,
    ) -> Result<Option<AptosBlockInfo>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.blocks.lock().unwrap().get(&version).cloned())
    }

    fn get_events_by_account(
        &self,
        _account: [u8; 32],
        start: u64,
        limit: u32,
    ) -> Result<Vec<AptosEvent>, Box<dyn std::error::Error + Send + Sync>> {
        let events = self.events.lock().unwrap();
        Ok(events
            .values()
            .flatten()
            .skip(start as usize)
            .take(limit as usize)
            .cloned()
            .collect())
    }

    fn get_latest_version(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.latest_version)
    }

    fn get_transaction_by_version(
        &self,
        version: u64,
    ) -> Result<Option<AptosTransaction>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.transactions.lock().unwrap().get(&version).cloned())
    }

    fn publish_module(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
        self.sent_transactions.lock().unwrap().push(tx_bytes);
        Ok([0xAB; 32])
    }

    fn verify_checkpoint(
        &self,
        _sequence_number: u64,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        Ok(true)
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}

#[cfg(not(debug_assertions))]
compile_error!(
    "MockAptosRpc can only be used in debug builds. \
    This prevents accidental use of mock implementations in production."
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_ledger_info() {
        let rpc = MockAptosRpc::new(1000);
        let info = rpc.get_ledger_info().unwrap();
        assert_eq!(info.chain_id, 1);
        assert_eq!(info.ledger_version, 1000);
    }

    #[test]
    fn test_mock_resource() {
        let rpc = MockAptosRpc::new(1000);
        let address = [1u8; 32];
        let resource = AptosResource {
            data: vec![0xAB, 0xCD],
        };
        rpc.set_resource(address, "CSV::Seal", resource.clone());

        let fetched = rpc.get_resource(address, "CSV::Seal", None).unwrap();
        assert_eq!(fetched.unwrap().data, vec![0xAB, 0xCD]);
    }

    #[test]
    fn test_mock_transaction() {
        let rpc = MockAptosRpc::new(1000);
        let tx = AptosTransaction {
            version: 500,
            hash: [1u8; 32],
            state_change_hash: [2u8; 32],
            event_root_hash: [3u8; 32],
            state_checkpoint_hash: None,
            epoch: 1,
            round: 0,
            events: vec![],
            payload: vec![0x01, 0x02],
            success: true,
            vm_status: "Executed".to_string(),
            gas_used: 100,
            cumulative_gas_used: 100,
        };
        rpc.add_transaction(500, tx.clone());

        let fetched = rpc.get_transaction(500).unwrap();
        assert_eq!(fetched.unwrap().version, 500);
    }

    #[test]
    fn test_mock_events() {
        let rpc = MockAptosRpc::new(1000);
        let event = AptosEvent {
            event_sequence_number: 1,
            key: "CSV::Seal".to_string(),
            data: vec![0xAB, 0xCD],
            transaction_version: 500,
        };
        rpc.add_event("CSV::Seal", event.clone());

        let fetched = rpc.get_events("CSV::Seal", "0", 10).unwrap();
        assert_eq!(fetched.len(), 1);
    }

    #[test]
    fn test_mock_submit_transaction() {
        let rpc = MockAptosRpc::new(1000);
        let tx_hash = rpc.submit_transaction(vec![0x01, 0x02]).unwrap();
        assert_eq!(tx_hash, [0xAB; 32]);
    }

    #[test]
    fn test_mock_wait_for_transaction() {
        let rpc = MockAptosRpc::new(1000);
        let tx_hash = [1u8; 32];
        let tx = rpc.wait_for_transaction(tx_hash).unwrap();
        assert_eq!(tx.version, 1000);
        assert!(tx.success);
    }
}
