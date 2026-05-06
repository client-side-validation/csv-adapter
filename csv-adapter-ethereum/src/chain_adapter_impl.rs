//! ChainAdapter implementation for EthereumAnchorLayer
//!
//! This module implements the `ChainAdapter` trait from `csv-adapter-core`,
//! enabling Ethereum to be used through the unified chain adapter interface.

use async_trait::async_trait;
use csv_adapter_core::chain_adapter::{
    AccountModel, ChainAdapter, ChainCapabilities, ChainError, ChainResult, RpcClient, Wallet,
};
use csv_adapter_core::chain_config::ChainConfig;
use csv_adapter_core::Chain;

use crate::adapter::EthereumAnchorLayer;
use crate::config::{EthereumConfig, Network};
use crate::rpc::EthereumRpc;

/// Ethereum RPC client wrapper implementing the core RpcClient trait
pub struct EthereumRpcClient {
    /// Inner RPC implementation
    inner: Box<dyn EthereumRpc>,
}

impl EthereumRpcClient {
    /// Create new RPC client from an EthereumRpc implementation
    pub fn new(rpc: Box<dyn EthereumRpc>) -> Self {
        Self { inner: rpc }
    }
}

#[async_trait]
impl RpcClient for EthereumRpcClient {
    async fn send_transaction(&self, tx: &[u8]) -> ChainResult<String> {
        let tx_hash = self
            .inner
            .send_raw_transaction(tx.to_vec())
            .await
            .map_err(|e| ChainError::RpcError(format!("{:?}", e)))?;

        Ok(format!("0x{}", hex::encode(tx_hash)))
    }

    async fn get_transaction(&self, hash: &str) -> ChainResult<serde_json::Value> {
        // Parse tx hash
        let hex_str = hash.trim_start_matches("0x");
        let tx_bytes = hex::decode(hex_str)
            .map_err(|e| ChainError::InvalidInput(format!("Invalid hash: {}", e)))?;

        if tx_bytes.len() != 32 {
            return Err(ChainError::InvalidInput(
                "Transaction hash must be 32 bytes".to_string(),
            ));
        }

        let mut tx_hash = [0u8; 32];
        tx_hash.copy_from_slice(&tx_bytes);

        let receipt = self
            .inner
            .get_transaction_receipt(tx_hash)
            .await
            .map_err(|e| ChainError::RpcError(format!("{:?}", e)))?
            .ok_or_else(|| ChainError::RpcError("Transaction receipt not found".to_string()))?;

        Ok(serde_json::json!({
            "hash": hash,
            "block_number": receipt.block_number,
            "status": if receipt.success { "success" } else { "failed" },
            "gas_used": receipt.gas_used,
        }))
    }

    async fn get_latest_block(&self) -> ChainResult<u64> {
        self.inner
            .block_number()
            .await
            .map_err(|e| ChainError::RpcError(format!("{:?}", e)))
    }

    async fn get_balance(&self, address: &str) -> ChainResult<u64> {
        // Parse address
        let hex_str = address.trim_start_matches("0x");
        let addr_bytes = hex::decode(hex_str)
            .map_err(|e| ChainError::InvalidInput(format!("Invalid address: {}", e)))?;

        if addr_bytes.len() != 20 {
            return Err(ChainError::InvalidInput(
                "Ethereum address must be 20 bytes".to_string(),
            ));
        }

        let mut addr = [0u8; 20];
        addr.copy_from_slice(&addr_bytes);

        let balance = self
            .inner
            .get_balance(addr)
            .await
            .map_err(|e| ChainError::RpcError(format!("{:?}", e)))?;

        // Convert U256 to u64 (will truncate large balances)
        let balance_bytes = balance.to_le_bytes();
        let mut u64_bytes = [0u8; 8];
        u64_bytes.copy_from_slice(&balance_bytes[..8.min(balance_bytes.len())]);
        Ok(u64::from_le_bytes(u64_bytes))
    }

    async fn is_transaction_confirmed(&self, hash: &str) -> ChainResult<bool> {
        let receipt = self.get_transaction(hash).await?;
        Ok(receipt.get("block_number").is_some())
    }

    async fn get_chain_info(&self) -> ChainResult<serde_json::Value> {
        let block_number = self.get_latest_block().await?;
        Ok(serde_json::json!({
            "chain": "ethereum",
            "block_number": block_number,
        }))
    }
}

/// Ethereum wallet implementing the core Wallet trait
pub struct EthereumWallet {
    /// Account address (20 bytes)
    address: String,
    /// Signing key (optional)
    #[allow(dead_code)]
    signing_key: Option<secp256k1::SecretKey>,
}

impl EthereumWallet {
    /// Create new wallet with address
    pub fn new(address: String) -> Self {
        Self {
            address,
            signing_key: None,
        }
    }

    /// Create wallet with signing capability
    #[cfg(feature = "rpc")]
    pub fn with_signing_key(address: String, signing_key: secp256k1::SecretKey) -> Self {
        Self {
            address,
            signing_key: Some(signing_key),
        }
    }
}

#[async_trait]
impl Wallet for EthereumWallet {
    fn address(&self) -> &str {
        &self.address
    }

    fn key_id(&self) -> &str {
        // Return address as key identifier
        &self.address
    }

    async fn sign_transaction(&self, data: &[u8]) -> ChainResult<Vec<u8>> {
        if let Some(signing_key) = &self.signing_key {
            let secp = secp256k1::Secp256k1::new();
            // Use from_digest_slice for 32-byte data, fall back to hash if needed
            let message_bytes: [u8; 32] = if data.len() == 32 {
                data.try_into().unwrap_or_else(|_| {
                    let mut arr = [0u8; 32];
                    arr.copy_from_slice(&data[..32.min(data.len())]);
                    arr
                })
            } else {
                // Hash the data if not exactly 32 bytes
                use sha2::{Digest, Sha256};
                let hash = Sha256::digest(data);
                hash.into()
            };
            let message = secp256k1::Message::from_digest_slice(&message_bytes)
                .map_err(|e| ChainError::WalletError(format!("Invalid message: {}", e)))?;

            let signature = secp.sign_ecdsa(&message, signing_key);
            Ok(signature.serialize_compact().to_vec())
        } else {
            Err(ChainError::WalletError(
                "No signing key available (read-only wallet)".to_string(),
            ))
        }
    }

    fn verify_signature(&self, _data: &[u8], _signature: &[u8]) -> bool {
        // Would verify ECDSA signature
        false
    }

    fn generate_address(&self) -> ChainResult<String> {
        // Generate new keypair
        let secp = secp256k1::Secp256k1::new();
        let (_secret_key, public_key) = secp.generate_keypair(&mut rand::thread_rng());

        // Address is last 20 bytes of keccak256(pubkey)
        use sha3::{Digest, Keccak256};
        let pubkey_bytes = public_key.serialize_uncompressed();
        let hash = Keccak256::digest(&pubkey_bytes[1..]); // Skip 0x04 prefix

        let mut addr = [0u8; 20];
        addr.copy_from_slice(&hash[12..]);

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

        let key: [u8; 32] = bytes
            .try_into()
            .map_err(|_| ChainError::InvalidInput("Failed to convert to key array".to_string()))?;

        // Validate the private key by creating a secp256k1 SecretKey
        let secret_key = secp256k1::SecretKey::from_slice(&key)
            .map_err(|e| ChainError::InvalidInput(format!("Invalid private key: {}", e)))?;

        // Derive the public key
        let secp = secp256k1::Secp256k1::new();
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);

        // Derive the Ethereum address from the public key
        // Address is last 20 bytes of keccak256(pubkey)
        use sha3::{Digest, Keccak256};
        let pubkey_bytes = public_key.serialize_uncompressed();
        let hash = Keccak256::digest(&pubkey_bytes[1..]); // Skip 0x04 prefix

        let mut addr = [0u8; 20];
        addr.copy_from_slice(&hash[12..]);
        let derived_address = format!("0x{}", hex::encode(addr));

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!("Imported Ethereum key, address: {}", derived_address).into());

        #[cfg(not(target_arch = "wasm32"))]
        log::info!("Imported Ethereum key, address: {}", derived_address);

        // Key import successful - the key is validated and the address is derived
        // The actual storage of the key is handled by the keystore
        Ok(())
    }
}

/// Chain capabilities for Ethereum
fn ethereum_capabilities() -> ChainCapabilities {
    ChainCapabilities {
        supports_nfts: true,
        supports_smart_contracts: true,
        account_model: AccountModel::Account,
        confirmation_blocks: 12, // Ethereum finality after ~12 blocks
        max_batch_size: 100,
        supported_networks: vec![
            "mainnet".to_string(),
            "sepolia".to_string(),
            "goerli".to_string(),
            "holesky".to_string(),
            "localhost".to_string(),
        ],
        supports_cross_chain: true,
        custom_features: Default::default(),
    }
}

#[async_trait]
impl ChainAdapter for EthereumAnchorLayer {
    fn chain_id(&self) -> &'static str {
        "ethereum"
    }

    fn chain_name(&self) -> &'static str {
        "Ethereum"
    }

    fn capabilities(&self) -> ChainCapabilities {
        ethereum_capabilities()
    }

    async fn create_client(&self, config: &ChainConfig) -> ChainResult<Box<dyn RpcClient>> {
        // Create Ethereum RPC client from chain configuration
        let rpc_url = config.rpc_endpoints.first()
            .ok_or_else(|| ChainError::InvalidInput("RPC endpoint required".to_string()))?;

        // Create the RPC client based on configuration
        #[cfg(feature = "rpc")]
        {
            use crate::real_rpc::RealEthereumRpc;
            use crate::chain_adapter_impl::EthereumRpcClient;
            let csv_seal_address = self.csv_seal_address;
            let rpc = RealEthereumRpc::new(rpc_url, csv_seal_address)
                .await
                .map_err(|e| ChainError::RpcError(format!("Failed to create RPC client: {}", e)))?;
            Ok(Box::new(EthereumRpcClient::new(Box::new(rpc))))
        }

        #[cfg(not(feature = "rpc"))]
        {
            // Without rpc feature, return an error indicating the feature is required
            Err(ChainError::FeatureNotEnabled(
                "Real Ethereum RPC requires the 'rpc' feature to be enabled".to_string(),
            ))
        }
    }

    async fn create_wallet(&self, _config: &ChainConfig) -> ChainResult<Box<dyn Wallet>> {
        // Create from CSV seal address
        let address = format!("0x{}", hex::encode(self.csv_seal_address));
        Ok(Box::new(EthereumWallet::new(address)))
    }

    fn csv_program_id(&self) -> Option<&'static str> {
        // CSV seal contract address
        Some("0xCsvSeaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    }

    fn to_core_chain(&self) -> Chain {
        Chain::Ethereum
    }

    fn default_network(&self) -> &'static str {
        "sepolia"
    }
}

/// Create a new Ethereum adapter from chain configuration
pub async fn create_ethereum_adapter(config: &ChainConfig) -> ChainResult<EthereumAnchorLayer> {
    // Parse network from config (use default_network field)
    let network = match config.default_network.as_str() {
        "mainnet" => Network::Mainnet,
        "sepolia" => Network::Sepolia,
        "goerli" => Network::Goerli,
        "dev" | "localhost" => Network::Dev,
        _ => Network::Sepolia,
    };

    let eth_config = EthereumConfig {
        network,
        finality_depth: 12, // Default finality depth
        ..Default::default()
    };

    // In test builds, use test RPC
    #[cfg(test)]
    {
        use crate::rpc::MockEthereumRpc;
        let rpc: Box<dyn EthereumRpc> = Box::new(MockEthereumRpc::new(1000));
        let csv_seal_address = [0u8; 20];
        EthereumAnchorLayer::from_config(eth_config, rpc, csv_seal_address)
            .map_err(|e| ChainError::RpcError(format!("{:?}", e)))
    }

    // When rpc feature is enabled, use real RPC
    #[cfg(all(not(test), feature = "rpc"))]
    {
        use crate::RealEthereumRpc;
        let rpc_url = config.rpc_endpoints.first()
            .ok_or_else(|| ChainError::InvalidInput("RPC endpoint required".to_string()))?;
        let csv_seal_address = [0u8; 20]; // Could be configured
        let rpc: Box<dyn EthereumRpc> = Box::new(
            RealEthereumRpc::new(rpc_url, csv_seal_address)
                .await
                .map_err(|e| ChainError::RpcError(format!("{:?}", e)))?
        );
        EthereumAnchorLayer::from_config(eth_config, rpc, csv_seal_address)
            .map_err(|e| ChainError::RpcError(format!("{:?}", e)))
    }

    // Otherwise, return error indicating rpc feature is needed
    #[cfg(not(any(test, feature = "rpc")))]
    {
        Err(ChainError::FeatureNotEnabled(
            "Real Ethereum RPC requires the 'rpc' feature to be enabled in csv-adapter-ethereum".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ethereum_adapter_chain_id() {
        let adapter = EthereumAnchorLayer::with_test().unwrap();
        assert_eq!(adapter.chain_id(), "ethereum");
        assert_eq!(adapter.chain_name(), "Ethereum");
    }

    #[test]
    fn test_ethereum_capabilities() {
        let caps = ethereum_capabilities();
        assert!(caps.supports_smart_contracts);
        assert!(caps.supports_nfts);
        assert_eq!(caps.account_model, AccountModel::Account);
    }

    #[test]
    fn test_create_ethereum_adapter() {
        let config = ChainConfig {
            chain_id: "ethereum".to_string(),
            network: "sepolia".to_string(),
            rpc_url: None,
            confirmation_blocks: Some(12),
            ..Default::default()
        };

        let adapter = create_ethereum_adapter(&config);
        assert!(adapter.is_ok());
    }
}
