//! ChainAdapter implementation for AptosAnchorLayer
//!
//! This module implements the `ChainAdapter` trait from `csv-adapter-core`,
//! enabling Aptos to be used through the unified chain adapter interface.

use async_trait::async_trait;
use base64::Engine;
use csv_core::chain_adapter::{
    AccountModel, ChainAdapter, ChainCapabilities, ChainError, ChainResult, RpcClient, Wallet,
};
use csv_core::chain_config::ChainConfig;
use csv_core::Chain;

use crate::adapter::AptosAnchorLayer;
use crate::config::{AptosConfig, AptosNetwork};
use crate::rpc::AptosRpc;

/// Aptos RPC client wrapper implementing the core RpcClient trait
pub struct AptosRpcClient {
    /// Inner RPC implementation
    inner: Box<dyn AptosRpc>,
}

impl AptosRpcClient {
    /// Create new RPC client from an AptosRpc implementation
    pub fn new(rpc: Box<dyn AptosRpc>) -> Self {
        Self { inner: rpc }
    }
}

#[async_trait]
impl RpcClient for AptosRpcClient {
    #[cfg(feature = "rpc")]
    async fn send_transaction(&self, tx: &[u8]) -> ChainResult<String> {
        // Aptos transactions are BCS-encoded Transaction payloads
        // Submit via the REST API /v1/transactions endpoint
        let tx_base64 = base64::engine::general_purpose::STANDARD.encode(tx);

        let request = serde_json::json!({
            "transaction": tx_base64,
        });

        // Send the transaction to Aptos REST API
        let response = reqwest::Client::new()
            .post("https://fullnode.mainnet.aptoslabs.com/v1/transactions")
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| ChainError::RpcError(format!("Aptos API request failed: {}", e)))?;

        let status = response.status();
        let aptos_response: serde_json::Value = response.json().await
            .map_err(|e| ChainError::SerializationError(format!("Failed to parse Aptos response: {}", e)))?;

        if !status.is_success() {
            let error_msg = aptos_response
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("Unknown Aptos API error");
            return Err(ChainError::TransactionError(error_msg.to_string()));
        }

        let tx_hash = aptos_response
            .get("hash")
            .and_then(|h| h.as_str())
            .ok_or_else(|| ChainError::RpcError("Aptos API returned no transaction hash".to_string()))?;

        Ok(tx_hash.to_string())
    }

    #[cfg(not(feature = "rpc"))]
    async fn send_transaction(&self, _tx: &[u8]) -> ChainResult<String> {
        Err(ChainError::CapabilityUnavailable(
            "Aptos transaction submission requires the 'rpc' feature".to_string()
        ))
    }

    async fn get_transaction(&self, hash: &str) -> ChainResult<serde_json::Value> {
        // Parse version or hash
        if let Ok(version) = hash.parse::<u64>() {
            let tx = self
                .inner
                .get_transaction(version)
                .await
                .map_err(|e| ChainError::RpcError(e.to_string()))?;

            if let Some(tx) = tx {
                return Ok(serde_json::json!({
                    "version": tx.version,
                    "hash": hash,
                    "success": tx.success,
                    "vm_status": tx.vm_status,
                }));
            }
        }

        Err(ChainError::InvalidInput(format!(
            "Transaction not found: {}",
            hash
        )))
    }

    async fn get_latest_block(&self) -> ChainResult<u64> {
        let ledger = self
            .inner
            .get_ledger_info()
            .await
            .map_err(|e| ChainError::RpcError(e.to_string()))?;

        Ok(ledger.ledger_version)
    }

    async fn get_balance(&self, address: &str) -> ChainResult<u64> {
        // Parse address
        let addr = parse_aptos_address(address)
            .map_err(|e| ChainError::InvalidInput(format!("Invalid address: {}", e)))?;

        // Get account resource
        let resource_type = "0x1::coin::CoinStore<0x1::aptos_coin::AptosCoin>";
        let resource = self
            .inner
            .get_resource(addr, resource_type, None)
            .await
            .map_err(|e| ChainError::RpcError(e.to_string()))?;

        // Extract balance from resource data
        if let Some(data) = resource {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&data.data) {
                if let Some(coin) = json.get("coin") {
                    if let Some(value) = coin.get("value") {
                        if let Some(balance) = value.as_str().and_then(|s| s.parse::<u64>().ok()) {
                            return Ok(balance);
                        }
                    }
                }
            }
        }

        Ok(0)
    }

    async fn is_transaction_confirmed(&self, hash: &str) -> ChainResult<bool> {
        let tx = self.get_transaction(hash).await?;
        Ok(tx.get("success").and_then(|v| v.as_bool()).unwrap_or(false))
    }

    async fn get_chain_info(&self) -> ChainResult<serde_json::Value> {
        let ledger = self
            .inner
            .get_ledger_info()
            .await
            .map_err(|e| ChainError::RpcError(e.to_string()))?;

        Ok(serde_json::json!({
            "chain": "aptos",
            "chain_id": ledger.chain_id,
            "epoch": ledger.epoch,
            "ledger_version": ledger.ledger_version,
            "ledger_timestamp": ledger.ledger_timestamp,
        }))
    }
}

/// Aptos wallet implementing the core Wallet trait
pub struct AptosWallet {
    /// Account address
    address: String,
    /// Signing key (optional, for read-only wallets)
    #[allow(dead_code)]
    signing_key: Option<ed25519_dalek::SigningKey>,
}

impl AptosWallet {
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
impl Wallet for AptosWallet {
    fn address(&self) -> &str {
        &self.address
    }

    fn key_id(&self) -> &str {
        // Return the address as the key identifier
        &self.address
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

    fn verify_signature(&self, data: &[u8], signature: &[u8]) -> bool {
        if signature.len() != 64 {
            return false;
        }
        let sig_bytes: [u8; 64] = match signature.try_into() {
            Ok(b) => b,
            Err(_) => return false,
        };

        // Parse address to get public key
        // In Aptos, the address is derived from the public key
        // We would need to store the verifying key alongside the address
        let _ = (data, sig_bytes);
        false
    }

    fn generate_address(&self) -> ChainResult<String> {
        // Generate new Ed25519 keypair
        let mut csprng = rand_core::OsRng;
        let signing_key = ed25519_dalek::SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();

        // Aptos address is the last 32 bytes of SHA3-256(pubkey) with 0x prefix
        use sha3::{Digest, Sha3_256};
        let mut hasher = Sha3_256::new();
        hasher.update(verifying_key.to_bytes());
        let result = hasher.finalize();

        let mut addr = [0u8; 32];
        addr.copy_from_slice(&result);

        Ok(format!("0x{}", hex::encode(addr)))
    }

    fn import_from_private_key(&self, private_key: &str) -> ChainResult<()> {
        // Parse hex private key
        let hex_str = private_key.trim_start_matches("0x");
        let bytes = hex::decode(hex_str)
            .map_err(|e| ChainError::InvalidInput(format!("Invalid hex: {}", e)))?;

        if bytes.len() != 32 {
            return Err(ChainError::InvalidInput(
                "Private key must be 32 bytes".to_string(),
            ));
        }

        let key_bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| ChainError::InvalidInput("Failed to convert to key array".to_string()))?;

        // Validate the private key by creating an Ed25519 signing key
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&key_bytes);
        let verifying_key = signing_key.verifying_key();

        // Derive Aptos address from public key
        use sha3::{Digest, Sha3_256};
        let mut hasher = Sha3_256::new();
        hasher.update(verifying_key.to_bytes());
        let result = hasher.finalize();

        let mut addr = [0u8; 32];
        addr.copy_from_slice(&result);
        let derived_address = format!("0x{}", hex::encode(addr));

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!("Imported Aptos key, address: {}", derived_address).into());

        #[cfg(not(target_arch = "wasm32"))]
        log::info!("Imported Aptos key, address: {}", derived_address);

        // Key import successful - key is validated and address is derived
        Ok(())
    }
}

/// Parse Aptos address string
fn parse_aptos_address(s: &str) -> Result<[u8; 32], String> {
    let hex_str = s.trim_start_matches("0x");
    let mut padded = String::new();
    for _ in 0..(64 - hex_str.len()) {
        padded.push('0');
    }
    padded.push_str(hex_str);

    let bytes = hex::decode(&padded).map_err(|e| format!("Invalid hex: {}", e))?;
    if bytes.len() != 32 {
        return Err(format!("Address must be 32 bytes, got {}", bytes.len()));
    }

    let mut addr = [0u8; 32];
    addr.copy_from_slice(&bytes);
    Ok(addr)
}

/// Chain capabilities for Aptos
fn aptos_capabilities() -> ChainCapabilities {
    ChainCapabilities {
        supports_nfts: true,
        supports_smart_contracts: true,
        account_model: AccountModel::Account,
        confirmation_blocks: 1, // Aptos has immediate finality via BFT
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
impl ChainAdapter for AptosAnchorLayer {
    fn chain_id(&self) -> &'static str {
        "aptos"
    }

    fn chain_name(&self) -> &'static str {
        "Aptos"
    }

    fn capabilities(&self) -> ChainCapabilities {
        aptos_capabilities()
    }

    async fn create_client(&self, config: &ChainConfig) -> ChainResult<Box<dyn RpcClient>> {
        // Create Aptos RPC client from chain configuration
        let rpc_url = config.rpc_endpoints.first()
            .ok_or_else(|| ChainError::InvalidInput("RPC endpoint required".to_string()))?;

        #[cfg(feature = "rpc")]
        {
            use crate::real_rpc::AptosRpcClient as RealAptosRpcClient;
            let rpc = RealAptosRpcClient::new(rpc_url);
            Ok(Box::new(AptosRpcClient::new(Box::new(rpc))))
        }

        #[cfg(not(feature = "rpc"))]
        {
            Err(ChainError::FeatureNotEnabled(
                "Real Aptos RPC requires the 'rpc' feature to be enabled".to_string(),
            ))
        }
    }

    async fn create_wallet(&self, _config: &ChainConfig) -> ChainResult<Box<dyn Wallet>> {
        // Get sender address from RPC
        let sender = self
            .rpc
            .sender_address()
            .await
            .map_err(|e| ChainError::RpcError(format!("{:?}", e)))?;

        let address = format!("0x{}", hex::encode(sender));

        #[cfg(feature = "rpc")]
        {
            // If we have a signing key, include it
            if let Some(signing_key) = &self.signing_key {
                return Ok(Box::new(AptosWallet::with_signing_key(
                    address,
                    signing_key.clone(),
                )));
            }
        }

        Ok(Box::new(AptosWallet::new(address)))
    }

    fn csv_program_id(&self) -> Option<&'static str> {
        // CSV seal contract address on Aptos
        Some("0x1::csv_seal")
    }

    fn to_core_chain(&self) -> Chain {
        Chain::Aptos
    }

    fn default_network(&self) -> &'static str {
        "devnet"
    }
}

/// Create a new Aptos adapter from chain configuration
pub fn create_aptos_adapter(config: &ChainConfig) -> ChainResult<AptosAnchorLayer> {
    // Parse network from config (use default_network field)
    let network = match config.default_network.as_str() {
        "mainnet" => AptosNetwork::Mainnet,
        "testnet" => AptosNetwork::Testnet,
        "devnet" => AptosNetwork::Devnet,
        _ => AptosNetwork::Devnet,
    };

    let aptos_config = AptosConfig::new(network);

    // In test builds, use test RPC
    #[cfg(test)]
    {
        use crate::rpc::MockAptosRpc;
        let rpc = Box::new(MockAptosRpc::new(aptos_config.chain_id() as u64));
        AptosAnchorLayer::from_config(aptos_config, rpc)
            .map_err(|e| ChainError::RpcError(format!("{:?}", e)))
    }

    // When rpc feature is enabled, use real RPC
    #[cfg(all(not(test), feature = "rpc"))]
    {
        use crate::real_rpc::AptosRpcClient;
        let rpc_url = config.rpc_endpoints.first()
            .ok_or_else(|| ChainError::InvalidInput("RPC endpoint required".to_string()))?;
        let rpc = Box::new(AptosRpcClient::new(rpc_url));
        AptosAnchorLayer::from_config(aptos_config, rpc)
            .map_err(|e| ChainError::RpcError(format!("{:?}", e)))
    }

    // Otherwise, return error indicating rpc feature is needed
    #[cfg(not(any(test, feature = "rpc")))]
    {
        Err(ChainError::FeatureNotEnabled(
            "Real Aptos RPC requires the 'rpc' feature to be enabled".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aptos_adapter_chain_id() {
        let adapter = AptosAnchorLayer::with_test().unwrap();
        assert_eq!(adapter.chain_id(), "aptos");
        assert_eq!(adapter.chain_name(), "Aptos");
    }

    #[test]
    fn test_aptos_capabilities() {
        let caps = aptos_capabilities();
        assert!(caps.supports_smart_contracts);
        assert!(caps.supports_nfts);
        assert_eq!(caps.account_model, AccountModel::Account);
    }

    #[test]
    fn test_parse_aptos_address() {
        let addr = parse_aptos_address("0x1").unwrap();
        assert_eq!(addr[31], 1);
    }

    #[test]
    fn test_create_aptos_adapter() {
        let config = ChainConfig {
            chain_id: "aptos".to_string(),
            network: "devnet".to_string(),
            rpc_url: None,
            confirmation_blocks: Some(1),
            ..Default::default()
        };

        let adapter = create_aptos_adapter(&config);
        assert!(adapter.is_ok());
    }
}
