//! Test adapters for testing.

use crate::chain_adapter::{ChainAdapter, ChainResult, RpcClient, Wallet};
use crate::chain_config::{AccountModel, ChainCapabilities, ChainConfig};
use crate::protocol_version::Chain;

/// A mock adapter for testing purposes.
pub struct MockAdapter;

#[async_trait::async_trait]
impl ChainAdapter for MockAdapter {
    fn chain_id(&self) -> &'static str {
        "mock"
    }

    fn chain_name(&self) -> &'static str {
        "Mock Chain"
    }

    fn capabilities(&self) -> ChainCapabilities {
        ChainCapabilities {
            supports_nfts: true,
            supports_smart_contracts: true,
            account_model: AccountModel::Account,
            confirmation_blocks: 1,
            max_batch_size: 100,
            supported_networks: vec!["mainnet".to_string(), "testnet".to_string()],
            supports_cross_chain: true,
            custom_features: std::collections::HashMap::new(),
        }
    }

    async fn create_client(&self, _config: &ChainConfig) -> ChainResult<Box<dyn RpcClient>> {
        Ok(Box::new(MockRpcClient))
    }

    async fn create_wallet(&self, _config: &ChainConfig) -> ChainResult<Box<dyn Wallet>> {
        Ok(Box::new(MockWallet))
    }

    fn csv_program_id(&self) -> Option<&'static str> {
        None
    }

    fn to_core_chain(&self) -> Chain {
        Chain::Bitcoin
    }

    fn default_network(&self) -> &'static str {
        "mainnet"
    }
}

struct MockRpcClient;

#[async_trait::async_trait]
impl RpcClient for MockRpcClient {
    async fn send_transaction(&self, _tx: &[u8]) -> ChainResult<String> {
        Ok("mock_tx_hash".to_string())
    }

    async fn get_transaction(&self, _hash: &str) -> ChainResult<serde_json::Value> {
        Ok(serde_json::json!({}))
    }

    async fn get_latest_block(&self) -> ChainResult<u64> {
        Ok(1000)
    }

    async fn get_chain_info(&self) -> ChainResult<serde_json::Value> {
        Ok(serde_json::json!({}))
    }

    async fn get_balance(&self, _address: &str) -> ChainResult<u64> {
        Ok(1000000)
    }

    async fn is_transaction_confirmed(&self, _hash: &str) -> ChainResult<bool> {
        Ok(true)
    }
}

struct MockWallet;

#[async_trait::async_trait]
impl Wallet for MockWallet {
    fn address(&self) -> &'static str {
        "mock_address"
    }

    fn key_id(&self) -> &'static str {
        "mock_key_id"
    }

    fn generate_address(&self) -> ChainResult<String> {
        Ok("mock_address".to_string())
    }

    async fn sign_transaction(&self, _data: &[u8]) -> ChainResult<Vec<u8>> {
        Ok(vec![0x01, 0x02, 0x03])
    }

    fn verify_signature(&self, _data: &[u8], _signature: &[u8]) -> bool {
        true
    }

    fn import_from_private_key(&self, _private_key: &str) -> ChainResult<()> {
        Ok(())
    }
}
