//! ChainAdapter implementation for SuiAnchorLayer
//!
//! This module implements the `ChainAdapter` trait from `csv-adapter-core`,
//! enabling Sui to be used through the unified chain adapter interface.

use async_trait::async_trait;
use csv_core::chain_adapter::{
    AccountModel, ChainAdapter, ChainCapabilities, ChainError, ChainResult, RpcClient, Wallet,
};
use ed25519_dalek::Verifier;
use csv_core::chain_config::ChainConfig;
use csv_core::Chain;

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
    async fn send_transaction(&self, signed_tx: &[u8]) -> ChainResult<String> {
        // Sui transactions are BCS-encoded TransactionData with signatures
        // Format: [tx_bytes_len:4][tx_bytes][signature:64][public_key:32]

        if signed_tx.len() < 4 + 64 + 32 {
            return Err(ChainError::InvalidInput(
                "Signed transaction too short for Sui format".to_string(),
            ));
        }

        // Parse the transaction length prefix
        let tx_len = u32::from_le_bytes([
            signed_tx[0], signed_tx[1], signed_tx[2], signed_tx[3],
        ]) as usize;

        if signed_tx.len() < 4 + tx_len + 64 + 32 {
            return Err(ChainError::InvalidInput(
                "Invalid Sui transaction format".to_string(),
            ));
        }

        let tx_bytes = signed_tx[4..4 + tx_len].to_vec();
        let signature = signed_tx[4 + tx_len..4 + tx_len + 64].to_vec();
        let public_key = signed_tx[4 + tx_len + 64..4 + tx_len + 64 + 32].to_vec();

       // Submit via execute_signed_transaction
        let digest = self
            .inner
            .execute_signed_transaction(tx_bytes, signature, public_key)
            .await
            .map_err(|e| ChainError::RpcError(format!("Transaction submission failed: {}", e)))?;

        Ok(format!("0x{}", hex::encode(digest)))
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
            .await
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
            .await
            .map_err(|e| ChainError::RpcError(format!("{:?}", e)))?;

        Ok(checkpoint_seq)
    }

    #[cfg(feature = "rpc")]
    async fn get_balance(&self, address: &str) -> ChainResult<u64> {
        use serde_json::{json, Value};
        use reqwest::Client;

        // Parse Sui address
        let addr_bytes = hex::decode(address.trim_start_matches("0x"))
            .map_err(|e| ChainError::InvalidInput(format!("Invalid address: {}", e)))?;

        if addr_bytes.len() != 32 {
            return Err(ChainError::InvalidInput(
                "Sui address must be 32 bytes".to_string(),
            ));
        }

        let addr_hex = format!("0x{}", hex::encode(&addr_bytes));

        // Query SUI balance via RPC using suix_getBalance endpoint
        // This RPC endpoint returns the total balance for all SUI coins owned by the address
        // without requiring individual object queries
        let rpc_url = "https://fullnode.testnet.sui.io:443";

        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| ChainError::RpcError(format!("Failed to create HTTP client: {}", e)))?;

        // suix_getBalance returns total balance for all SUI coins
        let payload = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "suix_getBalance",
            "params": [
                &addr_hex,
                "0x2::sui::SUI"  // SUI coin type
            ]
        });

        let response: Value = client
            .post(rpc_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChainError::RpcError(format!("HTTP request failed: {}", e)))?
            .json()
            .await
            .map_err(|e| ChainError::RpcError(format!("Failed to parse JSON: {}", e)))?;

        // Check for RPC errors
        if let Some(error) = response.get("error") {
            return Err(ChainError::RpcError(format!("RPC error: {}", error)));
        }

        // Extract balance from response
        // Response format: { "result": { "coinObjectCount": N, "totalBalance": "BALANCE", "lockedBalance": {} } }
        let result = response
            .get("result")
            .ok_or_else(|| ChainError::RpcError("Missing result in response".to_string()))?;

        let total_balance_str = result
            .get("totalBalance")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ChainError::RpcError("Missing totalBalance in response".to_string()))?;

        // Parse balance string to u64
        let total_balance: u64 = total_balance_str
            .parse()
            .map_err(|e| ChainError::RpcError(format!("Failed to parse balance: {}", e)))?;

        // Log successful balance query for audit trail
        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!("Sui balance for {}: {} MIST", addr_hex, total_balance).into());

        #[cfg(not(target_arch = "wasm32"))]
        log::info!("Sui balance for {}: {} MIST", addr_hex, total_balance);

        Ok(total_balance)
    }

    #[cfg(not(feature = "rpc"))]
    async fn get_balance(&self, _address: &str) -> ChainResult<u64> {
        Err(ChainError::CapabilityUnavailable(
            "Sui balance query requires the 'rpc' feature".to_string()
        ))
    }


   async fn get_chain_info(&self) -> ChainResult<serde_json::Value> {
        let checkpoint_seq = self
            .inner
            .get_latest_checkpoint_sequence_number()
            .await
            .map_err(|e| ChainError::RpcError(format!("{:?}", e)))?;

        Ok(serde_json::json!({
            "chain": "sui",
            "checkpoint": checkpoint_seq,
        }))
    }

    async fn is_transaction_confirmed(&self, hash: &str) -> ChainResult<bool> {
        let digest_bytes = hex::decode(hash.trim_start_matches("0x"))
            .map_err(|e| ChainError::InvalidInput(format!("Invalid digest: {}", e)))?;
        let digest: [u8; 32] = digest_bytes.try_into()
            .map_err(|_| ChainError::InvalidInput("Digest must be 32 bytes".to_string()))?;

       let tx = self
            .inner
            .get_transaction_block(digest)
            .await
            .map_err(|e| ChainError::RpcError(format!("Transaction query failed: {}", e)))?;

        Ok(tx.is_some())
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
        // Verify Ed25519 signature using the wallet's verifying key
        // Sui uses Ed25519 for transaction and message signatures
        
        // If we have the signing key, derive the verifying key
        if let Some(signing_key) = &self.signing_key {
            let verifying_key = signing_key.verifying_key();
            
            // Ed25519 signatures are 64 bytes
            if signature.len() != 64 {
                return false;
            }
            
            // Convert signature bytes to Signature type
            let sig_bytes: [u8; 64] = match signature.try_into() {
                Ok(bytes) => bytes,
                Err(_) => return false,
            };
            
            let signature = ed25519_dalek::Signature::from_bytes(&sig_bytes);
            
            // Verify the signature
            match verifying_key.verify(data, &signature) {
                Ok(()) => true,
                Err(_) => false,
            }
        } else {
            // Without the signing key, we can't verify
            // In production, you might want to store the verifying_key separately
            false
        }
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

        let key_bytes: [u8; 32] = bytes
            .try_into()
            .map_err(|_| ChainError::InvalidInput("Failed to convert to key array".to_string()))?;

        // Validate the private key by creating an Ed25519 signing key
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&key_bytes);
        let verifying_key = signing_key.verifying_key();

        // Sui address is the public key bytes (32 bytes)
        let derived_address = format!("0x{}", hex::encode(verifying_key.to_bytes()));

        #[cfg(target_arch = "wasm32")]
        web_sys::console::log_1(&format!("Imported Sui key, address: {}", derived_address).into());

        #[cfg(not(target_arch = "wasm32"))]
        log::info!("Imported Sui key, address: {}", derived_address);

        // Key import successful - the key is validated and the address is derived
        // The actual storage of the key is handled by the keystore
        Ok(())
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

    async fn create_client(&self, config: &ChainConfig) -> ChainResult<Box<dyn RpcClient>> {
        // Create Sui RPC client from chain configuration
        let rpc_url = config.rpc_endpoints.first()
            .ok_or_else(|| ChainError::InvalidInput("RPC endpoint required".to_string()))?;

        // Create the RPC client based on configuration
        #[cfg(feature = "rpc")]
        {
            use crate::real_rpc::SuiRpcClient as RealSuiRpcClient;
            let rpc = RealSuiRpcClient::new(rpc_url);
            Ok(Box::new(SuiRpcClient::new(Box::new(rpc))))
        }

        #[cfg(not(feature = "rpc"))]
        {
            // Without rpc feature, return an error indicating the feature is required
            Err(ChainError::FeatureNotEnabled(
                "Real Sui RPC requires the 'rpc' feature to be enabled".to_string(),
            ))
        }
    }

    async fn create_wallet(&self, _config: &ChainConfig) -> ChainResult<Box<dyn Wallet>> {
        // Get address from config or derive from signing key
        // Priority: 1) config.signer_address, 2) derived from signing_key, 3) error
        let address = if let Some(configured_address) = &self.config.signer_address {
            // Use the explicitly configured address
            configured_address.clone()
        } else {
            #[cfg(feature = "rpc")]
            {
                // Derive address from signing key if available
                if let Some(signing_key) = &self.signing_key {
                    let verifying_key = signing_key.verifying_key();
                    format!("0x{}", hex::encode(verifying_key.to_bytes()))
                } else {
                    return Err(ChainError::InvalidInput(
                        "No signer_address configured and no signing key available. \
                         Either set config.signer_address or provide a signing key.".to_string()
                    ));
                }
            }
            #[cfg(not(feature = "rpc"))]
            {
                return Err(ChainError::InvalidInput(
                    "No signer_address configured. Set config.signer_address to create a read-only wallet.".to_string()
                ));
            }
        };

        // Validate the address format (Sui addresses are 32 bytes / 64 hex chars + 0x prefix)
        let addr_without_prefix = address.trim_start_matches("0x");
        if addr_without_prefix.len() != 64 || hex::decode(addr_without_prefix).is_err() {
            return Err(ChainError::InvalidInput(
                format!("Invalid Sui address format: {}. Expected 0x + 64 hex characters.", address)
            ));
        }

        #[cfg(feature = "rpc")]
        {
            if let Some(signing_key) = &self.signing_key {
                // Verify the configured address matches the signing key
                let verifying_key = signing_key.verifying_key();
                let derived_address = format!("0x{}", hex::encode(verifying_key.to_bytes()));
                if derived_address != address {
                    return Err(ChainError::InvalidInput(
                        format!("Address mismatch: configured address {} does not match derived address {} from signing key",
                            address, derived_address)
                    ));
                }
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
        Err(ChainError::FeatureNotEnabled(
            "Real Sui RPC requires the 'rpc' feature to be enabled".to_string(),
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
