//! Real Bitcoin RPC client implementation
//!
//! Wraps `bitcoincore-rpc` behind the `BitcoinRpc` trait for production use.
//! Only compiled when the `rpc` feature is enabled.

#[cfg(feature = "rpc")]
pub mod real_rpc {
    use bitcoincore_rpc::{RpcApi, Client, Auth};
    use bitcoin::{Network, Txid};
    use bitcoin_hashes::Hash;

    use crate::rpc::BitcoinRpc;

    /// Real Bitcoin RPC client backed by bitcoincore-rpc
    pub struct RealBitcoinRpc {
        client: Client,
        network: Network,
    }

    impl RealBitcoinRpc {
        /// Create a new real RPC client (no auth — for local/public nodes)
        pub fn new(url: &str, network: Network) -> Result<Self, RealRpcError> {
            let client = Client::new(url, Auth::None)?;
            Ok(Self { client, network })
        }

        /// Create with authentication
        pub fn with_auth(
            url: &str,
            user: &str,
            pass: &str,
            network: Network,
        ) -> Result<Self, RealRpcError> {
            let client = Client::new(url, Auth::UserPass(user.into(), pass.into()))?;
            Ok(Self { client, network })
        }
    }

    impl BitcoinRpc for RealBitcoinRpc {
        fn get_block_count(&self) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
            Ok(self.client.get_block_count()?)
        }

        fn get_block_hash(&self, height: u64) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
            let hash = self.client.get_block_hash(height)?;
            let bytes = hash.as_ref();
            let mut result = [0u8; 32];
            result.copy_from_slice(bytes);
            Ok(result)
        }

        fn is_utxo_unspent(&self, txid: [u8; 32], vout: u32) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
            let txid = Txid::from_slice(&txid)
                .map_err(|e| format!("Invalid txid: {}", e))?;
            let result = self.client.get_tx_out(&txid, vout, Some(true))?;
            Ok(result.is_some())
        }

        fn send_raw_transaction(&self, tx_bytes: Vec<u8>) -> Result<[u8; 32], Box<dyn std::error::Error + Send + Sync>> {
            let tx = bitcoin::consensus::encode::deserialize::<bitcoin::Transaction>(&tx_bytes)
                .map_err(|e| format!("Failed to deserialize transaction: {}", e))?;
            let txid = self.client.send_raw_transaction(&tx)?;
            let bytes = txid.as_ref();
            let mut result = [0u8; 32];
            result.copy_from_slice(bytes);
            Ok(result)
        }

        fn get_tx_confirmations(&self, txid: [u8; 32]) -> Result<u64, Box<dyn std::error::Error + Send + Sync>> {
            let txid = Txid::from_slice(&txid)
                .map_err(|e| format!("Invalid txid: {}", e))?;
            let info = self.client.get_raw_transaction_info(&txid, None)?;
            Ok(info.confirmations.map(|c| c as u64).unwrap_or(0))
        }
    }

    /// Real RPC error type
    #[derive(Debug, thiserror::Error)]
    pub enum RealRpcError {
        #[error("RPC error: {0}")]
        Rpc(#[from] bitcoincore_rpc::Error),
    }
}
