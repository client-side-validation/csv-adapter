//! ChainAdapter implementation for SuiAnchorLayer
//!
//! This module implements the `ChainAdapter` trait from `csv-adapter-core`,
//! enabling Sui to be used through the unified chain adapter interface.

use async_trait::async_trait;
use csv_adapter_core::chain_adapter::{
    AccountModel, ChainAdapter, ChainCapabilities, ChainError, ChainResult, RpcClient, Wallet,
};
use csv_adapter_core::chain_config::ChainConfig;
use csv_adapter_core::Chain;

use crate::adapter::SuiAnchorLayer;
use crate::config::{SuiConfig, SuiNetwork};
use crate::rpc::SuiRpc;

/// Sui RPC client wrapper implementing the core RpcClient trait
pub struct SuiRpcClient {
    /// Inner RPC implementation
    inner: Box<dyn SuiRpc>,
}

impl SuiRpcClient {
    /// Create new RPC client from a SuiRpc implementation
    pub fn new(rpc: Box<dyn SuiRpc>) -> Self {
        Self { inner: rpc }
    }
}

#[async_trait]
impl RpcClient for SuiRpcClient {
    async fn send_transaction(&self, tx: &[u8]) -> ChainResult<String> {
        // Sui transactions are BCS-encoded TransactionData
        // Submit via the RPC
        let _ = tx;
        Err(ChainError::NotImplemented(
            "Sui transaction submission".to_string(),
        ))
    }

    async fn get_transaction(&self, hash: &str) -> ChainResult<serde_json::Value> {
        // Parse digest
        let digest_bytes = hex::decode(hash.trim_start_matches("0x"))
            .map_err(|e| ChainError::InvalidInput(format!("Invalid digest: {}", e)))?;

        let _tx = self
            .inner
            .get_transaction_block(
                digest_bytes
                    .try_into()
                    .map_err(|_| ChainError::InvalidInput("Invalid digest length".to_string()))?,
            )
            .map_err(|e| ChainError::RpcError(format!("{:?}", e)))?;

        // SuiTransactionBlock doesn't implement Serialize, so we just return the digest
        Ok(serde_json::json!({
            "digest": hash,
            "transaction": "SuiTransactionBlock",
        }))
    }

    async fn get_latest_block(&self) -> ChainResult<u64> {
        let checkpoint_seq = self
            .inner
            .get_latest_checkpoint_sequence_number()
            .map_err(|e| ChainError::RpcError(format!("{:?}", e)))?;

        Ok(checkpoint_seq)
    }

    async fn get_balance(&self, address: &str) -> ChainResult<u64> {
        // Parse Sui address
        let addr_bytes = hex::decode(address.trim_start_matches("0x"))
            .map_err(|e| ChainError::InvalidInput(format!("Invalid address: {}", e)))?;

        if addr_bytes.len() != 32 {
            return Err(ChainError::InvalidInput(
                "Sui address must be 32 bytes".to_string(),
            ));
        }

        let mut addr = [0u8; 32];
        addr.copy_from_slice(&addr_bytes);

        // Get all gas objects owned by address
        let objects = self
            .inner
            .get_gas_objects(addr)
            .map_err(|e| ChainError::RpcError(format!("{:?}", e)))?;

        // Sum up SUI coins
        let mut total_balance = 0u64;
        for obj in objects {
            if obj.object_type == "0x2::coin::Coin<0x2::sui::SUI>" {
                // Coin objects have balance in their data
                // Simplified - actual parsing would depend on object structure
                total_balance += obj.version; // Placeholder
            }
        }

        Ok(total_balance)
    }

    async fn is_transaction_confirmed(&self, hash: &str) -> ChainResult<bool> {
        // In Sui, transactions are checkpointed for finality
        let tx = self.get_transaction(hash).await?;
        Ok(tx.get("transaction").is_some())
    }

    async fn get_chain_info(&self) -> ChainResult<serde_json::Value> {
        let checkpoint_seq = self
            .inner
            .get_latest_checkpoint_sequence_number()
            .map_err(|e| ChainError::RpcError(format!("{:?}", e)))?;

        Ok(serde_json::json!({
            "chain": "sui",
            "checkpoint": checkpoint_seq,
        }))
    }
}

/// Sui wallet implementing the core Wallet trait
pub struct SuiWallet {
    /// Account address
    address: String,
    /// Signing key (optional, for read-only wallets)
    #[allow(dead_code)]
    signing_key: Option<ed25519_dalek::SigningKey>,
}

impl SuiWallet {
    /// Create new wallet with address
    pub fn new(address: String) -> Self {
        Self {
            address,
            signing_key: None,
        }
    }

    /// Create wallet with signing capability
    pub fn with_signing_key(address: String, signing_key: ed25519_dalek::SigningKey) -> Self {
        Self {
            address,
            signing_key: Some(signing_key),
        }
    }
}

#[async_trait]
impl Wallet for SuiWallet {
    fn address(&self) -> &str {
        &self.address
    }

    fn private_key(&self) -> &str {
        // Ed25519 keys not exposed directly
        ""
    }

    async fn sign_transaction(&self, data: &[u8]) -> ChainResult<Vec<u8>> {
        if let Some(signing_key) = &self.signing_key {
            use ed25519_dalek::Signer;
            let signature = signing_key.sign(data);
            Ok(signature.to_bytes().to_vec())
        } else {
            Err(ChainError::WalletError(
                "No signing key available (read-only wallet)".to_string(),
            ))
        }
    }

    fn verify_signature(&self, _data: &[u8], _signature: &[u8]) -> bool {
        // Would verify Ed25519 signature
        false
    }

    fn generate_address(&self) -> ChainResult<String> {
        // Generate new Ed25519 keypair
        let signing_key = ed25519_dalek::SigningKey::generate(&mut rand::rngs::OsRng);
        let verifying_key = signing_key.verifying_key();

        // Sui address is derived from public key (32 bytes)
        let addr = verifying_key.to_bytes();
        Ok(format!("0x{}", hex::encode(addr)))
    }

    fn import_from_private_key(&self, private_key: &str) -> ChainResult<()> {
        let hex_str = private_key.trim_start_matches("0x");
        let bytes = hex::decode(hex_str)
            .map_err(|e| ChainError::InvalidInput(format!("Invalid hex: {}", e)))?;

        if bytes.len() != 32 {
            return Err(ChainError::InvalidInput(
                "Private key must be 32 bytes".to_string(),
            ));
        }

        let _key: [u8; 32] = bytes
            .try_into()
            .map_err(|_| ChainError::InvalidInput("Failed to convert to key array".to_string()))?;

        Err(ChainError::NotImplemented(
            "Key import - use key derivation instead".to_string(),
        ))
    }
}

/// Chain capabilities for Sui
fn sui_capabilities() -> ChainCapabilities {
    ChainCapabilities {
        supports_nfts: true,
        supports_smart_contracts: true,
        account_model: AccountModel::Account,
        confirmation_blocks: 1, // Sui has immediate finality via checkpoint
        max_batch_size: 1000,
        supported_networks: vec![
            "mainnet".to_string(),
            "testnet".to_string(),
            "devnet".to_string(),
        ],
        supports_cross_chain: true,
        custom_features: Default::default(),
    }
}

#[async_trait]
impl ChainAdapter for SuiAnchorLayer {
    fn chain_id(&self) -> &'static str {
        "sui"
    }

    fn chain_name(&self) -> &'static str {
        "Sui"
    }

    fn capabilities(&self) -> ChainCapabilities {
        sui_capabilities()
    }

    async fn create_client(&self, _config: &ChainConfig) -> ChainResult<Box<dyn RpcClient>> {
        Err(ChainError::NotImplemented(
            "Sui RPC client creation from config - use from_config() instead".to_string(),
        ))
    }

    async fn create_wallet(&self, _config: &ChainConfig) -> ChainResult<Box<dyn Wallet>> {
        // Get sender address from config - use sender_address method or default
        let address = format!("0x{}", hex::encode([0u8; 32])); // Placeholder

        #[cfg(feature = "rpc")]
        {
            if let Some(signing_key) = &self.signing_key {
                return Ok(Box::new(SuiWallet::with_signing_key(
                    address,
                    signing_key.clone(),
                )));
            }
        }

        Ok(Box::new(SuiWallet::new(address)))
    }

    fn csv_program_id(&self) -> Option<&'static str> {
        // CSV seal package ID on Sui
        Some("0xcsvsui")
    }

    fn to_core_chain(&self) -> Chain {
        Chain::Sui
    }

    fn default_network(&self) -> &'static str {
        "testnet"
    }
}

/// Create a new Sui adapter from chain configuration
pub fn create_sui_adapter(config: &ChainConfig) -> ChainResult<SuiAnchorLayer> {
    // Parse network from config
    let network = match config.default_network.as_str() {
        "mainnet" => SuiNetwork::Mainnet,
        "testnet" => SuiNetwork::Testnet,
        "devnet" => SuiNetwork::Devnet,
        _ => SuiNetwork::Testnet,
    };

    let sui_config = SuiConfig::new(network);

    // In test builds, use test RPC
    #[cfg(test)]
    {
        use crate::rpc::MockSuiRpc;
        let rpc = Box::new(MockSuiRpc::new(1)); // Use checkpoint sequence number
        SuiAnchorLayer::from_config(sui_config, rpc)
            .map_err(|e| ChainError::RpcError(format!("{:?}", e)))
    }

    // When rpc feature is enabled, use real RPC
    #[cfg(all(not(test), feature = "rpc"))]
    {
        use crate::real_rpc::SuiRpcClient;
        let rpc_url = config.rpc_endpoints.first()
            .ok_or_else(|| ChainError::InvalidInput("RPC endpoint required".to_string()))?;
        let rpc = Box::new(SuiRpcClient::new(rpc_url));
        SuiAnchorLayer::from_config(sui_config, rpc)
            .map_err(|e| ChainError::RpcError(format!("{:?}", e)))
    }

    // Otherwise, return error indicating rpc feature is needed
    #[cfg(not(any(test, feature = "rpc")))]
    {
        Err(ChainError::NotImplemented(
            "Real Sui RPC requires the rpc feature to be enabled".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sui_adapter_chain_id() {
        let adapter = SuiAnchorLayer::with_test().unwrap();
        assert_eq!(adapter.chain_id(), "sui");
        assert_eq!(adapter.chain_name(), "Sui");
    }

    #[test]
    fn test_sui_capabilities() {
        let caps = sui_capabilities();
        assert!(caps.supports_smart_contracts);
        assert!(caps.supports_nfts);
        assert_eq!(caps.account_model, AccountModel::Account);
    }

    #[test]
    fn test_create_sui_adapter() {
        let config = ChainConfig {
            chain_id: "sui".to_string(),
            network: "testnet".to_string(),
            rpc_url: None,
            confirmation_blocks: Some(1),
            ..Default::default()
        };

        let adapter = create_sui_adapter(&config);
        assert!(adapter.is_ok());
    }
}
