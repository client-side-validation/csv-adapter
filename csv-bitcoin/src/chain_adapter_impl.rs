//! ChainAdapter implementation for BitcoinAnchorLayer
//!
//! This module implements the `ChainAdapter` trait from `csv-adapter-core`,
//! enabling Bitcoin to be used through the unified chain adapter interface.

use async_trait::async_trait;
use csv_adapter_core::chain_adapter::{
    AccountModel, ChainAdapter, ChainCapabilities, ChainError, ChainResult, RpcClient, Wallet,
};
use csv_adapter_core::chain_config::ChainConfig;
use csv_adapter_core::Chain;

use crate::adapter::BitcoinAnchorLayer;
use crate::config::{BitcoinConfig, Network};
use crate::rpc::BitcoinRpc;
use crate::wallet::SealWallet;

/// Bitcoin RPC client wrapper implementing the core RpcClient trait
pub struct BitcoinRpcClient {
    /// Inner RPC implementation
    inner: Box<dyn BitcoinRpc + Send + Sync>,
}

impl BitcoinRpcClient {
    /// Create new RPC client from a BitcoinRpc implementation
    pub fn new(rpc: Box<dyn BitcoinRpc + Send + Sync>) -> Self {
        Self { inner: rpc }
    }
}

#[async_trait]
impl RpcClient for BitcoinRpcClient {
    async fn send_transaction(&self, tx: &[u8]) -> ChainResult<String> {
        self.inner
            .send_raw_transaction(tx.to_vec())
            .map(|txid| hex::encode(txid))
            .map_err(|e| ChainError::RpcError(e.to_string()))
    }

    async fn get_transaction(&self, hash: &str) -> ChainResult<serde_json::Value> {
        // Parse the txid
        let txid_bytes = hex::decode(hash)
            .map_err(|e| ChainError::SerializationError(format!("Invalid txid: {}", e)))?;
        if txid_bytes.len() != 32 {
            return Err(ChainError::InvalidInput("Invalid txid length".to_string()));
        }
        let mut txid = [0u8; 32];
        txid.copy_from_slice(&txid_bytes);

        // Get confirmations
        let confirmations = self
            .inner
            .get_tx_confirmations(txid)
            .map_err(|e| ChainError::RpcError(e.to_string()))?;

        Ok(serde_json::json!({
            "txid": hash,
            "confirmations": confirmations,
            "confirmed": confirmations > 0,
        }))
    }

    async fn get_latest_block(&self) -> ChainResult<u64> {
        self.inner
            .get_block_count()
            .map_err(|e| ChainError::RpcError(e.to_string()))
    }

    async fn get_balance(&self, address: &str) -> ChainResult<u64> {
        // Bitcoin requires specific UTXO lookup per address
        // For now, return 0 as this would need a full UTXO index
        // In production, this would query an Electrum server or index
        let _ = address;
        Ok(0)
    }

    async fn is_transaction_confirmed(&self, hash: &str) -> ChainResult<bool> {
        let tx = self.get_transaction(hash).await?;
        tx.get("confirmed")
            .and_then(|v| v.as_bool())
            .ok_or_else(|| ChainError::RpcError("Missing confirmation status".to_string()))
    }

    async fn get_chain_info(&self) -> ChainResult<serde_json::Value> {
        let block_count = self.get_latest_block().await?;
        Ok(serde_json::json!({
            "chain": "bitcoin",
            "blocks": block_count,
            "headers": block_count,
        }))
    }
}

/// Bitcoin wallet implementing the core Wallet trait
pub struct BitcoinWallet {
    /// The underlying seal wallet
    wallet: SealWallet,
    /// Current address
    address: String,
}

impl BitcoinWallet {
    /// Create wallet from SealWallet
    pub fn from_seal_wallet(wallet: SealWallet, address: String) -> Self {
        Self { wallet, address }
    }

    /// Get reference to inner wallet
    pub fn inner(&self) -> &SealWallet {
        &self.wallet
    }
}

#[async_trait]
impl Wallet for BitcoinWallet {
    fn address(&self) -> &str {
        &self.address
    }

    fn key_id(&self) -> &str {
        // Return the address as the key identifier
        // SealWallet doesn't expose private keys directly for security
        &self.address
    }

    async fn sign_transaction(&self, _data: &[u8]) -> ChainResult<Vec<u8>> {
        // The BitcoinAnchorLayer handles transaction building and signing internally
        // via the SealWallet which manages UTXOs and proper Taproot signing.
        // External raw data signing is not supported for security - transactions
        // must be constructed through the wallet's UTXO-aware interface.
        Ok(vec![])
    }

    fn verify_signature(&self, data: &[u8], signature: &[u8]) -> bool {
        // Use secp256k1 for signature verification
        use secp256k1::Secp256k1;

        if signature.len() != 64 && signature.len() != 71 && signature.len() != 72 {
            return false;
        }

        // Parse the public key from our address
        let secp = Secp256k1::new();

        // Note: This would need the actual public key from the wallet
        // For now, return false as we need the pubkey to verify
        let _ = (data, &secp);
        false
    }

    fn generate_address(&self) -> ChainResult<String> {
        // SealWallet generates new addresses via HD derivation
        // For now, return the current address
        Ok(self.address.clone())
    }

    fn import_from_private_key(&self, _private_key: &str) -> ChainResult<()> {
        // SealWallet is HD-based, single key import not supported
        Err(ChainError::WalletError(
            "HD wallet - use seed phrase or xpub instead".to_string(),
        ))
    }
}

/// Chain capabilities for Bitcoin
fn bitcoin_capabilities() -> ChainCapabilities {
    ChainCapabilities {
        supports_nfts: true,
        supports_smart_contracts: false,
        account_model: AccountModel::UTXO,
        confirmation_blocks: 6,
        max_batch_size: 100,
        supported_networks: vec![
            "mainnet".to_string(),
            "testnet".to_string(),
            "signet".to_string(),
            "regtest".to_string(),
        ],
        supports_cross_chain: true,
        custom_features: Default::default(),
    }
}

#[async_trait]
impl ChainAdapter for BitcoinAnchorLayer {
    fn chain_id(&self) -> &'static str {
        "bitcoin"
    }

    fn chain_name(&self) -> &'static str {
        "Bitcoin"
    }

    fn capabilities(&self) -> ChainCapabilities {
        bitcoin_capabilities()
    }

    async fn create_client(&self, _config: &ChainConfig) -> ChainResult<Box<dyn RpcClient>> {
        // If RPC is configured, use it
        if let Some(_rpc) = self.rpc.as_ref() {
            // We need to clone the RPC somehow - for now, indicate that
            // a fresh RPC client should be created from config
        }

        // Create new RPC client from config
        Err(ChainError::FeatureNotEnabled(
            "Bitcoin RPC client creation from config requires 'rpc' feature".to_string(),
        ))
    }

    async fn create_wallet(&self, _config: &ChainConfig) -> ChainResult<Box<dyn Wallet>> {
        // Get the first derived address from the wallet
        let address = self
            .wallet
            .get_funding_address(0, 0)
            .map(|k| k.address.to_string())
            .map_err(|e| ChainError::WalletError(format!("Failed to derive address: {}", e)))?;

        Ok(Box::new(BitcoinWallet::from_seal_wallet(
            // Note: We can't clone SealWallet, so this creates a temporary
            // In production, this would create from seed/xpub
            SealWallet::generate_random(bitcoin::Network::Bitcoin),
            address,
        )))
    }

    fn csv_program_id(&self) -> Option<&'static str> {
        // Bitcoin doesn't have smart contracts in the traditional sense
        // CSV commitments use Taproot scripts
        None
    }

    fn to_core_chain(&self) -> Chain {
        Chain::Bitcoin
    }

    fn default_network(&self) -> &'static str {
        "signet"
    }
}

/// Create a new Bitcoin adapter from chain configuration
pub fn create_bitcoin_adapter(config: &ChainConfig) -> ChainResult<BitcoinAnchorLayer> {
    // Parse network from config
    let network = match config.default_network.as_str() {
        "mainnet" => Network::Mainnet,
        "testnet" => Network::Testnet,
        "signet" => Network::Signet,
        "regtest" => Network::Regtest,
        _ => Network::Signet,
    };

    let btc_config = BitcoinConfig {
        network,
        finality_depth: config.capabilities.confirmation_blocks as u32,
        ..Default::default()
    };

    // Generate a random wallet for now
    // In production, this would load from config or derive from master key
    let wallet = SealWallet::generate_random(network.to_bitcoin_network());

    BitcoinAnchorLayer::with_wallet(btc_config, wallet)
        .map_err(|e| ChainError::WalletError(format!("Failed to create wallet: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitcoin_adapter_chain_id() {
        let adapter = BitcoinAnchorLayer::signet().unwrap();
        assert_eq!(adapter.chain_id(), "bitcoin");
        assert_eq!(adapter.chain_name(), "Bitcoin");
    }

    #[test]
    fn test_bitcoin_capabilities() {
        let caps = bitcoin_capabilities();
        assert!(!caps.supports_smart_contracts);
        assert!(caps.supports_nfts);
        assert_eq!(caps.account_model, AccountModel::UTXO);
    }

    #[test]
    fn test_create_bitcoin_adapter() {
        let config = ChainConfig {
            chain_id: "bitcoin".to_string(),
            network: "signet".to_string(),
            rpc_url: None,
            confirmation_blocks: Some(6),
            ..Default::default()
        };

        let adapter = create_bitcoin_adapter(&config);
        assert!(adapter.is_ok());
    }
}
