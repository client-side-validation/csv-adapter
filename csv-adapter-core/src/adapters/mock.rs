//! Mock chain adapter for testing.

use async_trait::async_trait;
use serde_json::json;
use std::collections::HashMap;

use super::super::{
    ChainAdapter, ChainConfig, ChainCapabilities, ChainResult, ChainError,
    RpcClient, Wallet, AccountModel,
};

/// Mock chain adapter for testing
pub struct MockAdapter {
    chain_id: &'static str,
    chain_name: &'static str,
    capabilities: ChainCapabilities,
}

impl MockAdapter {
    /// Create new mock adapter
    pub fn new(chain_id: &'static str, chain_name: &'static str) -> Self {
        let capabilities = ChainCapabilities {
            supports_nfts: true,
            supports_smart_contracts: true,
            account_model: AccountModel::Account,
            confirmation_blocks: 6,
            max_batch_size: 50,
            supported_networks: vec!["testnet".to_string(), "mainnet".to_string()],
            supports_cross_chain: true,
            custom_features: HashMap::new(),
        };
        
        Self {
            chain_id,
            chain_name,
            capabilities,
        }
    }
}

#[async_trait]
impl ChainAdapter for MockAdapter {
    fn chain_id(&self) -> &'static str {
        self.chain_id
    }
    
    fn chain_name(&self) -> &'static str {
        self.chain_name
    }
    
    fn capabilities(&self) -> ChainCapabilities {
        self.capabilities.clone()
    }
    
    async fn create_client(&self, _config: &ChainConfig) -> ChainResult<Box<dyn RpcClient>> {
        Ok(Box::new(MockRpcClient::new()))
    }
    
    async fn create_wallet(&self, _config: &ChainConfig) -> ChainResult<Box<dyn Wallet>> {
        Ok(Box::new(MockWallet::new()))
    }
    
    fn csv_program_id(&self) -> Option<&'static str> {
        Some("MockProgram11111111111111111111111111111111")
    }
    
    fn to_core_chain(&self) -> crate::Chain {
        match self.chain_id {
            "mock-bitcoin" => crate::Chain::Bitcoin,
            "mock-ethereum" => crate::Chain::Ethereum,
            "mock-solana" => crate::Chain::Solana,
            "mock-sui" => crate::Chain::Sui,
            "mock-aptos" => crate::Chain::Aptos,
            _ => crate::Chain::Bitcoin, // Default fallback
        }
    }
    
    fn default_network(&self) -> &'static str {
        "testnet"
    }
}

/// Mock RPC client for testing
pub struct MockRpcClient {
    transactions: std::sync::Arc<std::sync::Mutex<Vec<serde_json::Value>>>,
}

impl MockRpcClient {
    /// Create a new mock RPC client.
    pub fn new() -> Self {
        Self {
            transactions: std::sync::Arc::new(std::sync::Mutex::new(Vec::new())),
        }
    }
    
    /// Add mock transaction for testing
    pub fn add_transaction(&self, tx: serde_json::Value) {
        let mut transactions = self.transactions.lock().unwrap();
        transactions.push(tx);
    }
}

#[async_trait]
impl RpcClient for MockRpcClient {
    async fn send_transaction(&self, tx: &[u8]) -> ChainResult<String> {
        let tx_hash = format!("mock_tx_{}", tx.len());
        Ok(tx_hash)
    }
    
    async fn get_transaction(&self, hash: &str) -> ChainResult<serde_json::Value> {
        let transactions = self.transactions.lock().unwrap();
        for tx in transactions.iter() {
            if let Some(h) = tx.get("hash") {
                if h.as_str() == Some(hash) {
                    return Ok(tx.clone());
                }
            }
        }
        Err(ChainError::RpcError("Transaction not found".to_string()))
    }
    
    async fn get_latest_block(&self) -> ChainResult<u64> {
        Ok(12345)
    }
    
    async fn get_balance(&self, _address: &str) -> ChainResult<u64> {
        Ok(1000000)
    }
    
    async fn is_transaction_confirmed(&self, _hash: &str) -> ChainResult<bool> {
        Ok(true)
    }
    
    async fn get_chain_info(&self) -> ChainResult<serde_json::Value> {
        Ok(json!({
            "chain_id": "mock",
            "block_height": 12345,
            "network": "testnet"
        }))
    }
}

/// Mock wallet for testing
pub struct MockWallet {
    address: String,
}

impl MockWallet {
    /// Create a new mock wallet.
    pub fn new() -> Self {
        Self {
            address: "mock_address_12345".to_string(),
        }
    }
}

#[async_trait]
impl Wallet for MockWallet {
    fn address(&self) -> &str {
        &self.address
    }
    
    fn private_key(&self) -> &str {
        "mock_private_key_encrypted"
    }
    
    async fn sign_transaction(&self, data: &[u8]) -> ChainResult<Vec<u8>> {
        let mut signature = data.to_vec();
        signature.extend_from_slice(&[0, 1, 2, 3]); // Mock signature
        Ok(signature)
    }
    
    fn verify_signature(&self, data: &[u8], signature: &[u8]) -> bool {
        // Simple mock verification
        data.len() == signature.len() - 4
    }
    
    fn generate_address(&self) -> ChainResult<String> {
        Ok(format!("mock_address_{}", rand::random::<u32>()))
    }
    
    fn from_private_key(&self, _private_key: &str) -> ChainResult<()> {
        Ok(())
    }
}
