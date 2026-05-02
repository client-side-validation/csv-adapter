//! Bitcoin RPC trait and test helpers
//!
//! ## Design Decision
//!
//! The `BitcoinRpc` trait defines the interface for real RPC implementations.
//! Test helpers are provided under `#[cfg(test)]` and **explicitly refuse**
//! to broadcast transactions, returning errors instead of fabricated txids.

#[cfg(test)]
use std::collections::HashSet;

/// Trait-based RPC interface for real implementations
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

    /// Clone the RPC client into a new boxed trait object.
    /// Required for the facade pattern to share RPC across operations.
    fn clone_boxed(&self) -> Box<dyn BitcoinRpc + Send + Sync>;
}

/// Test-only RPC client for unit testing
///
/// This implementation **explicitly refuses** to broadcast transactions.
/// Use this only for testing seal registry logic, not transaction broadcasting.
#[cfg(test)]
pub struct TestBitcoinRpc {
    block_count: u64,
    pub unspent_utxos: HashSet<(Vec<u8>, u32)>,
}

#[cfg(test)]
impl TestBitcoinRpc {
    pub fn new(block_count: u64) -> Self {
        Self {
            block_count,
            unspent_utxos: HashSet::new(),
        }
    }

    pub fn mark_utxo_unspent(&mut self, txid: Vec<u8>, vout: u32) {
        self.unspent_utxos.insert((txid, vout));
    }

    pub fn mark_utxo_spent(&mut self, txid: Vec<u8>, vout: u32) {
        self.unspent_utxos.remove(&(txid, vout));
    }
}

#[cfg(test)]
impl BitcoinRpc for TestBitcoinRpc {
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
        _tx_bytes: Vec<u8>,
    ) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
        // Explicit refusal — test RPCs must not fabricate txids
        Err("TestBitcoinRpc cannot broadcast transactions — use real RPC for that".into())
    }

    fn get_tx_confirmations(
        &self,
        _txid: [u8; 32],
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        Ok(0)
    }

    fn clone_boxed(&self) -> Box<dyn BitcoinRpc + Send + Sync> {
        Box::new(TestBitcoinRpc {
            block_count: self.block_count,
            unspent_utxos: self.unspent_utxos.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitcoin_rpc_block_count() {
        let rpc = TestBitcoinRpc::new(100);
        assert_eq!(rpc.get_block_count().unwrap(), 100);
    }

    #[test]
    fn test_bitcoin_rpc_utxo_lifecycle() {
        let mut rpc = TestBitcoinRpc::new(100);
        let _txid = [1u8, 2, 3].to_vec().into_boxed_slice();
        let txid_bytes: [u8; 32] = {
            let mut arr = [0u8; 32];
            arr[..3].copy_from_slice(&[1, 2, 3]);
            arr
        };
        assert!(!rpc.is_utxo_unspent(txid_bytes, 0).unwrap());
        rpc.mark_utxo_unspent(txid_bytes.to_vec(), 0);
        assert!(rpc.is_utxo_unspent(txid_bytes, 0).unwrap());
        rpc.mark_utxo_spent(txid_bytes.to_vec(), 0);
        assert!(!rpc.is_utxo_unspent(txid_bytes, 0).unwrap());
    }

    #[test]
    fn test_bitcoin_rpc_refuses_broadcast() {
        let rpc = TestBitcoinRpc::new(100);
        let result = rpc.send_raw_transaction(vec![0x01, 0x02]);
        assert!(result.is_err(), "Test RPC must refuse to broadcast");
    }
}
