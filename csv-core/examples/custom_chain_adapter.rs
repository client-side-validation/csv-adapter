use std::collections::HashMap;

use async_trait::async_trait;

use csv_adapter_core::{
    AccountModel, AdapterFactory, Chain, ChainAdapter, ChainCapabilities, ChainConfig, ChainError,
    ChainPluginBuilder, ChainResult, RpcClient, Wallet,
};

#[derive(Debug, Clone)]
struct ExampleChainAdapter;

#[async_trait]
impl ChainAdapter for ExampleChainAdapter {
    fn chain_id(&self) -> &'static str {
        "example-chain"
    }

    fn chain_name(&self) -> &'static str {
        "Example Chain"
    }

    fn capabilities(&self) -> ChainCapabilities {
        ChainCapabilities {
            supports_nfts: true,
            supports_smart_contracts: true,
            account_model: AccountModel::Account,
            confirmation_blocks: 8,
            max_batch_size: 128,
            supported_networks: vec!["devnet".to_string(), "mainnet".to_string()],
            supports_cross_chain: true,
            custom_features: HashMap::new(),
        }
    }

    async fn create_client(&self, _config: &ChainConfig) -> ChainResult<Box<dyn RpcClient>> {
        Err(ChainError::FeatureNotEnabled("Example chain RPC client not available".to_string()))
    }

    async fn create_wallet(&self, _config: &ChainConfig) -> ChainResult<Box<dyn Wallet>> {
        Err(ChainError::FeatureNotEnabled("Example chain wallet not available".to_string()))
    }

    fn csv_program_id(&self) -> Option<&'static str> {
        Some("example-program")
    }

    fn to_core_chain(&self) -> Chain {
        Chain::Solana
    }

    fn default_network(&self) -> &'static str {
        "devnet"
    }
}

fn example_config() -> ChainConfig {
    ChainConfig {
        chain_id: "example-chain".to_string(),
        chain_name: "Example Chain".to_string(),
        default_network: "devnet".to_string(),
        rpc_endpoints: vec!["https://rpc.example-chain.devnet".to_string()],
        program_id: Some("example-program".to_string()),
        block_explorer_urls: vec!["https://explorer.example-chain.devnet".to_string()],
        capabilities: ExampleChainAdapter.capabilities(),
        custom_settings: HashMap::new(),
    }
}

fn main() {
    let plugin = ChainPluginBuilder::new("example-chain", "Example Chain")
        .version(csv_adapter_core::PROTOCOL_VERSION)
        .author("csv-adapter")
        .description("Example custom chain plugin registered at runtime")
        .capabilities(ExampleChainAdapter.capabilities())
        .adapter_factory(|config| {
            if let Some(config) = config {
                println!(
                    "Creating adapter for {} on {}",
                    config.chain_name, config.default_network
                );
            }
            Box::new(ExampleChainAdapter)
        })
        .config_factory(example_config)
        .build()
        .expect("plugin should build");

    let mut factory = AdapterFactory::empty();
    let mut registry = csv_adapter_core::ChainPluginRegistry::new();
    registry.register(plugin);
    factory.register_plugins_from_registry(&registry);

    let adapter = factory
        .create_adapter_with_config("example-chain", Some(example_config()))
        .expect("custom adapter should be created");

    println!(
        "Registered {} ({}) with default network {}",
        adapter.chain_name(),
        adapter.chain_id(),
        adapter.default_network()
    );
}
