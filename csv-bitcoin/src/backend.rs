//! ChainDriver implementation for BitcoinSealProtocol
//!
//! This module implements the `ChainDriver` trait from `csv-adapter-core`,
//! enabling Bitcoin to be used through the unified chain adapter interface.

use async_trait::async_trait;
use csv_core::ChainId;
use csv_core::chain_config::ChainConfig;
use csv_core::driver::{
    AccountModel, ChainCapabilities, ChainDriver, ChainError, ChainResult, RpcClient, Wallet,
};

use crate::config::{BitcoinConfig, Network};
use crate::rpc::BitcoinRpc;
use crate::seal_protocol::BitcoinSealProtocol;
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
            .map(hex::encode)
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
        // CRITICAL FIX: Do NOT silently return Ok(0) - this masked balance query failures
        // and caused the "fake-zero balance" bug. Instead, query a real UTXO set
        // from the configured RPC provider.
        //
        // Use get_utxos_for_address to sum UTXO amounts for balance.
        // If no UTXO index is configured, return an error rather than a fake zero.
        let utxos = self.inner.get_utxos_for_address(address).map_err(|e| {
            ChainError::RpcError(format!(
                "Balance unavailable for {}: {}. Bitcoin requires a UTXO index (Electrum/mempool.space).",
                address, e
            ))
        })?;

        let total_balance: u64 = utxos.iter().map(|utxo| utxo.amount_sat).sum();
        Ok(total_balance)
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
        use sha2::{Digest, Sha256};

        let mut digest = [0u8; 32];
        digest.copy_from_slice(&Sha256::digest(_data));

        self.wallet
            .sign_taproot_keypath(&crate::wallet::Bip86Path::external(0, 0), &digest)
            .map_err(|e| ChainError::WalletError(format!("Failed to sign transaction: {}", e)))
    }

    fn verify_signature(&self, data: &[u8], signature: &[u8]) -> bool {
        use secp256k1::{Message, Secp256k1, XOnlyPublicKey};
        use sha2::{Digest, Sha256};

        if signature.len() != 64 {
            return false;
        }

        let secp = Secp256k1::new();
        let derived = match self.wallet.get_funding_address(0, 0) {
            Ok(derived) => derived,
            Err(_) => return false,
        };
        let public_key = match XOnlyPublicKey::from_slice(&derived.internal_xonly.serialize()) {
            Ok(public_key) => public_key,
            Err(_) => return false,
        };
        let signature = match secp256k1::schnorr::Signature::from_slice(signature) {
            Ok(signature) => signature,
            Err(_) => return false,
        };
        let digest = Sha256::digest(data);
        let message = match Message::from_digest_slice(&digest) {
            Ok(message) => message,
            Err(_) => return false,
        };

        secp.verify_schnorr(&signature, &message, &public_key)
            .is_ok()
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
impl ChainDriver for BitcoinSealProtocol {
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
        let rpc = self.rpc.as_ref().ok_or_else(|| {
            ChainError::FeatureNotEnabled(
                "Bitcoin RPC client is not configured for this adapter".to_string(),
            )
        })?;

        Ok(Box::new(BitcoinRpcClient::new(rpc.clone_boxed())))
    }

    async fn create_wallet(&self, _config: &ChainConfig) -> ChainResult<Box<dyn Wallet>> {
        Err(ChainError::WalletError(
            "Bitcoin wallet export through ChainDriver is disabled: refusing to create a random replacement wallet. Use BitcoinSealProtocol directly with configured wallet material.".to_string(),
        ))
    }

    fn csv_program_id(&self) -> Option<&'static str> {
        // Bitcoin doesn't have smart contracts in the traditional sense
        // CSV commitments use Taproot scripts
        None
    }

    fn to_core_chain(&self) -> ChainId {
        ChainId::new("bitcoin")
    }

    fn default_network(&self) -> &'static str {
        "signet"
    }
}

/// Create a new Bitcoin adapter from chain configuration
pub fn create_bitcoin_adapter(config: &ChainConfig) -> ChainResult<BitcoinSealProtocol> {
    // Parse network from config
    let network = match config.default_network.as_str() {
        "mainnet" => Network::Mainnet,
        "testnet" => Network::Testnet,
        "signet" => Network::Signet,
        "regtest" => Network::Regtest,
        _ => Network::Signet,
    };

    let xpub = config
        .custom_settings
        .get("xpub")
        .and_then(|value| value.as_str())
        .ok_or_else(|| {
            ChainError::WalletError(
                "Bitcoin adapter creation requires custom_settings.xpub; refusing to generate a throwaway wallet".to_string(),
            )
        })?
        .to_string();

    let btc_config = BitcoinConfig {
        network,
        finality_depth: config.capabilities.confirmation_blocks as u32,
        xpub: Some(xpub),
        ..Default::default()
    };

    let wallet = SealWallet::from_xpub(
        btc_config
            .xpub
            .as_deref()
            .ok_or_else(|| ChainError::WalletError("Missing Bitcoin xpub".to_string()))?,
        network.to_bitcoin_network(),
    )
    .map_err(|e| ChainError::WalletError(format!("Failed to create wallet from xpub: {}", e)))?;

    BitcoinSealProtocol::with_wallet(btc_config, wallet)
        .map_err(|e| ChainError::WalletError(format!("Failed to create wallet: {}", e)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bitcoin_adapter_chain_id() {
        let adapter = BitcoinSealProtocol::signet().unwrap();
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
            chain_name: "Bitcoin".to_string(),
            default_network: "signet".to_string(),
            rpc_endpoints: vec!["https://blockstream.info/testnet/api".to_string()],
            program_id: None,
            block_explorer_urls: vec![],
            start_block: 0,
            capabilities: ChainCapabilities {
                supports_nfts: false,
                supports_smart_contracts: false,
                account_model: AccountModel::UTXO,
                confirmation_blocks: 6,
                max_batch_size: 100,
                supported_networks: vec!["mainnet".to_string(), "signet".to_string()],
                supports_cross_chain: false,
                custom_features: std::collections::HashMap::new(),
            },
            custom_settings: std::collections::HashMap::new(),
        };

        let adapter = create_bitcoin_adapter(&config);
        assert!(adapter.is_err());
    }
}
