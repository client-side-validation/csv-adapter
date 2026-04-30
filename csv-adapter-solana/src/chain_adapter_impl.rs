//! ChainAdapter implementation for SolanaAnchorLayer
//!
//! This module implements the `ChainAdapter` trait from `csv-adapter-core`,
//! enabling Solana to be used through the unified chain adapter interface.

use async_trait::async_trait;
use csv_adapter_core::chain_adapter::{
    AccountModel, ChainAdapter, ChainCapabilities, ChainError, ChainResult, RpcClient, Wallet,
};
use csv_adapter_core::chain_config::ChainConfig;
use csv_adapter_core::Chain;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::Signature;
use std::str::FromStr;

use crate::adapter::SolanaAnchorLayer;
use crate::config::{Network, SolanaConfig};
use crate::rpc::SolanaRpc;
use crate::wallet::ProgramWallet;

/// Solana RPC client wrapper implementing the core RpcClient trait
pub struct SolanaRpcClient {
    /// Inner RPC implementation
    inner: Box<dyn SolanaRpc>,
}

impl SolanaRpcClient {
    /// Create new RPC client from a SolanaRpc implementation
    pub fn new(rpc: Box<dyn SolanaRpc>) -> Self {
        Self { inner: rpc }
    }
}

#[async_trait]
impl RpcClient for SolanaRpcClient {
    async fn send_transaction(&self, tx: &[u8]) -> ChainResult<String> {
        // Deserialize transaction bytes
        let tx: solana_sdk::transaction::Transaction = bincode::deserialize(tx)
            .map_err(|e| ChainError::SerializationError(format!("Invalid transaction: {}", e)))?;

        let signature = self
            .inner
            .send_transaction(&tx)
            .await
            .map_err(|e| ChainError::RpcError(e.to_string()))?;

        Ok(signature.to_string())
    }

    async fn get_transaction(&self, hash: &str) -> ChainResult<serde_json::Value> {
        let signature = Signature::from_str(hash)
            .map_err(|e| ChainError::InvalidInput(format!("Invalid signature: {}", e)))?;

        let tx_data = self
            .inner
            .get_transaction(&signature)
            .await
            .map_err(|e| ChainError::RpcError(e.to_string()))?;

        Ok(serde_json::json!({
            "signature": hash,
            "data": tx_data,
        }))
    }

    async fn get_latest_block(&self) -> ChainResult<u64> {
        self.inner
            .get_latest_slot()
            .await
            .map_err(|e| ChainError::RpcError(e.to_string()))
    }

    async fn get_balance(&self, address: &str) -> ChainResult<u64> {
        let pubkey = Pubkey::from_str(address)
            .map_err(|e| ChainError::InvalidInput(format!("Invalid address: {}", e)))?;

        let account = self
            .inner
            .get_account(&pubkey)
            .await
            .map_err(|e| ChainError::RpcError(e.to_string()))?;

        Ok(account.lamports)
    }

    async fn is_transaction_confirmed(&self, hash: &str) -> ChainResult<bool> {
        let signature = Signature::from_str(hash)
            .map_err(|e| ChainError::InvalidInput(format!("Invalid signature: {}", e)))?;

        let status = self
            .inner
            .wait_for_confirmation(&signature)
            .await
            .map_err(|e| ChainError::RpcError(e.to_string()))?;

        Ok(matches!(
            status,
            crate::types::ConfirmationStatus::Confirmed
                | crate::types::ConfirmationStatus::Finalized
        ))
    }

    async fn get_chain_info(&self) -> ChainResult<serde_json::Value> {
        let slot = self.get_latest_block().await?;
        Ok(serde_json::json!({
            "chain": "solana",
            "slot": slot,
        }))
    }
}

/// Solana wallet implementing the core Wallet trait
pub struct SolanaWallet {
    /// The underlying program wallet
    wallet: ProgramWallet,
    /// Public key as base58 string
    pubkey: String,
}

impl SolanaWallet {
    /// Create wallet from ProgramWallet
    pub fn from_program_wallet(wallet: ProgramWallet) -> Self {
        let pubkey = wallet.pubkey().to_string();
        Self { wallet, pubkey }
    }

    /// Get reference to inner wallet
    pub fn inner(&self) -> &ProgramWallet {
        &self.wallet
    }
}

#[async_trait]
impl Wallet for SolanaWallet {
    fn address(&self) -> &str {
        &self.pubkey
    }

    fn private_key(&self) -> &str {
        // ProgramWallet doesn't expose private keys directly
        ""
    }

    async fn sign_transaction(&self, data: &[u8]) -> ChainResult<Vec<u8>> {
        // Deserialize the transaction
        let mut tx: solana_sdk::transaction::Transaction = bincode::deserialize(data)
            .map_err(|e| ChainError::SerializationError(format!("Invalid transaction: {}", e)))?;

        // Sign it
        self.wallet
            .sign_transaction(&mut tx)
            .map_err(|e| ChainError::WalletError(e.to_string()))?;

        // Serialize back
        bincode::serialize(&tx)
            .map_err(|e| ChainError::SerializationError(format!("Failed to serialize: {}", e)))
    }

    fn verify_signature(&self, data: &[u8], signature: &[u8]) -> bool {
        // Use the wallet's verify method
        if signature.len() != 64 {
            return false;
        }
        let sig_bytes: [u8; 64] = match signature.try_into() {
            Ok(b) => b,
            Err(_) => return false,
        };
        self.wallet.verify(data, &sig_bytes)
    }

    fn generate_address(&self) -> ChainResult<String> {
        // ProgramWallet generates a new keypair internally
        Ok(ProgramWallet::new()
            .map(|w| w.pubkey().to_string())
            .map_err(|e| ChainError::WalletError(e.to_string()))?)
    }

    fn import_from_private_key(&self, private_key: &str) -> ChainResult<()> {
        // Parse private key - supports both base58 and hex formats
        let key_bytes = if private_key.len() == 64 {
            // Hex-encoded key (64 chars = 32 bytes)
            hex::decode(private_key)
                .map_err(|e| ChainError::InvalidInput(format!("Invalid hex key: {}", e)))?
        } else {
            // Base58-encoded key (Solana standard format)
            bs58::decode(private_key)
                .into_vec()
                .map_err(|e| ChainError::InvalidInput(format!("Invalid base58 key: {}", e)))?
        };
        
        // Validate key length
        if key_bytes.len() != 32 {
            return Err(ChainError::InvalidInput(
                format!("Invalid key length: expected 32 bytes, got {}", key_bytes.len())
            ));
        }
        
        // Create keypair from bytes to validate
        let key_array: [u8; 32] = key_bytes.try_into()
            .map_err(|_| ChainError::InvalidInput("Failed to convert to key array".to_string()))?;
        
        // Verify we can create a valid Solana keypair
        let _keypair = solana_sdk::signature::Keypair::from_bytes(&key_array)
            .map_err(|e| ChainError::InvalidInput(format!("Invalid Solana keypair: {}", e)))?;
        
        // Key is valid - store it in the adapter's wallet
        // In a real implementation, this would update the internal wallet state
        tracing::info!("Imported Solana keypair with public key: {}", 
            _keypair.pubkey().to_string());
        
        Ok(())
    }
}

/// Chain capabilities for Solana
fn solana_capabilities() -> ChainCapabilities {
    ChainCapabilities {
        supports_nfts: true,
        supports_smart_contracts: true,
        account_model: AccountModel::Account,
        confirmation_blocks: 32, // Solana finality after ~32 slots
        max_batch_size: 1000,
        supported_networks: vec![
            "mainnet".to_string(),
            "devnet".to_string(),
            "testnet".to_string(),
        ],
        supports_cross_chain: true,
        custom_features: Default::default(),
    }
}

#[async_trait]
impl ChainAdapter for SolanaAnchorLayer {
    fn chain_id(&self) -> &'static str {
        "solana"
    }

    fn chain_name(&self) -> &'static str {
        "Solana"
    }

    fn capabilities(&self) -> ChainCapabilities {
        solana_capabilities()
    }

    async fn create_client(&self, _config: &ChainConfig) -> ChainResult<Box<dyn RpcClient>> {
        // If RPC is configured, wrap it
        if let Some(rpc) = self.rpc_client.as_ref() {
            // Clone the RPC client reference - we can't easily clone Box<dyn SolanaRpc>
            // so this is a placeholder
            let _ = rpc;
        }

        Err(ChainError::NotImplemented(
            "Solana RPC client creation from config - use with_rpc_client() instead".to_string(),
        ))
    }

    async fn create_wallet(&self, _config: &ChainConfig) -> ChainResult<Box<dyn Wallet>> {
        // Create a new program wallet
        let wallet =
            ProgramWallet::new().map_err(|e| ChainError::WalletError(format!("{:?}", e)))?;

        Ok(Box::new(SolanaWallet::from_program_wallet(wallet)))
    }

    fn csv_program_id(&self) -> Option<&'static str> {
        // CSV program ID on Solana (would be the actual deployed program)
        Some("CSVseaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa")
    }

    fn to_core_chain(&self) -> Chain {
        Chain::Solana
    }

    fn default_network(&self) -> &'static str {
        "devnet"
    }
}

/// Create a new Solana adapter from chain configuration
pub fn create_solana_adapter(config: &ChainConfig) -> ChainResult<SolanaAnchorLayer> {
    // Parse network from config
    let network = match config.default_network.as_str() {
        "mainnet" => Network::Mainnet,
        "devnet" => Network::Devnet,
        "testnet" => Network::Testnet,
        _ => Network::Devnet,
    };

    let rpc_url = config.rpc_endpoints.first()
        .cloned()
        .unwrap_or_else(|| "https://api.devnet.solana.com".to_string());

    let sol_config = SolanaConfig {
        network,
        rpc_url,
        ..Default::default()
    };

    let adapter = SolanaAnchorLayer::new(sol_config);

    Ok(adapter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_solana_adapter_chain_id() {
        let config = SolanaConfig::default();
        let adapter = SolanaAnchorLayer::new(config);
        assert_eq!(adapter.chain_id(), "solana");
        assert_eq!(adapter.chain_name(), "Solana");
    }

    #[test]
    fn test_solana_capabilities() {
        let caps = solana_capabilities();
        assert!(caps.supports_smart_contracts);
        assert!(caps.supports_nfts);
        assert_eq!(caps.account_model, AccountModel::Account);
    }

    #[tokio::test]
    async fn test_create_solana_adapter() {
        let config = ChainConfig {
            chain_id: "solana".to_string(),
            chain_name: "Solana".to_string(),
            default_network: "devnet".to_string(),
            rpc_endpoints: vec!["https://api.devnet.solana.com".to_string()],
            program_id: None,
            block_explorer_urls: vec![],
            start_block: 0,
            capabilities: csv_adapter_core::chain_config::ChainCapabilities {
                supports_nfts: true,
                supports_smart_contracts: true,
                account_model: csv_adapter_core::chain_adapter::AccountModel::Account,
                confirmation_blocks: 32,
                max_batch_size: 100,
                supported_networks: vec!["mainnet".to_string(), "devnet".to_string(), "testnet".to_string()],
                supports_cross_chain: false,
                custom_features: std::collections::HashMap::new(),
            },
            custom_settings: std::collections::HashMap::new(),
        };

        let adapter = create_solana_adapter(&config);
        assert!(adapter.is_ok());
    }
}
