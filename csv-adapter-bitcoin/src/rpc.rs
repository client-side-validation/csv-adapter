//! Bitcoin RPC trait and mock implementation

use std::collections::HashSet;
use std::sync::Mutex;

/// Trait-based RPC interface for mocking in tests
pub trait BitcoinRpc: Send + Sync {
    fn get_block_count(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;
    fn get_block_hash(
        &self,
        height: u64,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>>;
    fn is_utxo_unspent(
        &self,
        txid: [u8; 32],
        vout: u32,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>>;
    fn send_raw_transaction(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>>;
    fn get_tx_confirmations(
        &self,
        txid: [u8; 32],
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>>;
}

/// Mock RPC client for testing
///
/// This implementation is only compiled in debug builds to prevent
/// accidental use in production environments.
#[cfg(debug_assertions)]
pub struct MockBitcoinRpc {
    block_count: u64,
    pub unspent_utxos: std::collections::HashSet<(Vec<u8>, u32)>,
    pub sent_transactions: Mutex<Vec<Vec<u8>>>,
}

#[cfg(debug_assertions)]
impl MockBitcoinRpc {
    pub fn new(block_count: u64) -> Self {
        Self {
            block_count,
            unspent_utxos: HashSet::new(),
            sent_transactions: Mutex::new(Vec::new()),
        }
    }

    pub fn mark_utxo_unspent(&mut self, txid: Vec<u8>, vout: u32) {
        self.unspent_utxos.insert((txid, vout));
    }

    pub fn mark_utxo_spent(&mut self, txid: Vec<u8>, vout: u32) {
        self.unspent_utxos.remove(&(txid, vout));
    }
}

#[cfg(debug_assertions)]
impl BitcoinRpc for MockBitcoinRpc {
    fn get_block_count(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.block_count)
    }

    fn get_block_hash(
        &self,
        height: u64,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
        let mut hash = [0u8; 32];
        hash[..8].copy_from_slice(&height.to_le_bytes());
        Ok(hash)
    }

    fn is_utxo_unspent(
        &self,
        txid: [u8; 32],
        vout: u32,
    ) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
        Ok(self.unspent_utxos.contains(&(txid.to_vec(), vout)))
    }

    fn send_raw_transaction(
        &self,
        tx_bytes: Vec<u8>,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
        self.sent_transactions.lock().unwrap().push(tx_bytes);
        Ok([0xAB; 32])
    }

    fn get_tx_confirmations(
        &self,
        _txid: [u8; 32],
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        Ok(6)
    }
}

#[cfg(not(debug_assertions))]
compile_error!(
    "MockBitcoinRpc can only be used in debug builds. \
    This prevents accidental use of mock implementations in production."
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_rpc_block_count() {
        let rpc = MockBitcoinRpc::new(100);
        assert_eq!(rpc.get_block_count().unwrap(), 100);
    }

    #[test]
    fn test_mock_rpc_utxo_lifecycle() {
        let mut rpc = MockBitcoinRpc::new(100);
        let mut txid = [0u8; 32];
        txid[0] = 1;
        txid[1] = 2;
        txid[2] = 3;
        assert!(!rpc.is_utxo_unspent(txid, 0).unwrap());
        rpc.mark_utxo_unspent(txid.to_vec(), 0);
        assert!(rpc.is_utxo_unspent(txid, 0).unwrap());
        rpc.mark_utxo_spent(txid.to_vec(), 0);
        assert!(!rpc.is_utxo_unspent(txid, 0).unwrap());
    }

    #[test]
    fn test_mock_rpc_send_transaction() {
        let rpc = MockBitcoinRpc::new(100);
        let txid = rpc.send_raw_transaction(vec![0x01, 0x02]).unwrap();
        assert_eq!(txid, [0xAB; 32]);
    }
}
