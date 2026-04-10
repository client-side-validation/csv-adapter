//! Bitcoin RPC trait and mock implementation
//!
//! ## Design Decision
//!
//! The `BitcoinRpc` trait defines the interface for real RPC implementations.
//! **There is no mock that silently returns fake transaction IDs.**
//!
//! Mock implementations that silently succeed are how people think things work
//! when they don't. If you attempt to call `send_raw_transaction()` on a mock,
//! it returns an explicit error — not a fabricated txid.

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
}

/// Stub RPC client for unit testing
///
/// This implementation **explicitly refuses** to broadcast transactions.
/// Mock implementations that silently return fake txids are how people think
/// things work when they don't. Use this only for testing seal registry logic,
/// not transaction broadcasting.
#[cfg(test)]
pub struct StubBitcoinRpc {
    block_count: u64,
    pub unspent_utxos: HashSet<(Vec<u8>, u32)>,
}

#[cfg(test)]
impl StubBitcoinRpc {
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
impl BitcoinRpc for StubBitcoinRpc {
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
        // Explicit refusal — mock RPCs must not fabricate txids
        Err("StubBitcoinRpc cannot broadcast transactions — use real RPC for that".into())
    }

    fn get_tx_confirmations(
        &self,
        _txid: [u8; 32],
    ) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
        Ok(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stub_rpc_block_count() {
        let rpc = StubBitcoinRpc::new(100);
        assert_eq!(rpc.get_block_count().unwrap(), 100);
    }

    #[test]
    fn test_stub_rpc_utxo_lifecycle() {
        let mut rpc = StubBitcoinRpc::new(100);
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
    fn test_stub_rpc_refuses_broadcast() {
        let rpc = StubBitcoinRpc::new(100);
        let result = rpc.send_raw_transaction(vec![0x01, 0x02]);
        assert!(result.is_err(), "Stub RPC must refuse to broadcast");
    }
}
