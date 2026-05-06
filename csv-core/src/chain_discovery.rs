//! Chain Discovery — Canonical Entry Point for Chain Management
//!
//! `ChainDiscovery` is the single public API for discovering, registering,
//! and instantiating chain adapters. It absorbs the roles of the former
//! `ChainRegistry` and `SimpleChainRegistry`:
//!
//! - **Configuration loading** — reads `chains/*.toml` files
//! - **Chain catalog** — tracks chain IDs, names, and capabilities
//! - **Plugin system** — registers `ChainPlugin`s for dynamic adapter creation
//! - **Adapter factory** — builds `AdapterFactory` from registered plugins
//!
//! # Usage
//!
//! ```no_run
//! use csv_adapter_core::chain_discovery::ChainDiscovery;
//! use csv_adapter_core::chain_plugin::ChainPlugin;
//! use std::sync::Arc;
//!
//! let mut discovery = ChainDiscovery::new();
//! discovery.register_plugin(Arc::new(my_plugin));
//! discovery.load_default_chains().expect("chains dir required");
//!
//! // Create an adapter for a discovered chain
//! let adapter = discovery.create_adapter("bitcoin");
//! ```

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use super::adapter_factory::AdapterFactory;
use super::chain_adapter::ChainAdapter;
use super::chain_config::{ChainConfig, ChainConfigLoader};
use super::chain_plugin::{ChainPlugin, ChainPluginRegistry};

/// Basic chain information discovered from configuration.
#[derive(Debug, Clone)]
pub struct ChainInfo {
    /// Unique identifier for the chain (e.g., "bitcoin", "ethereum")
    pub chain_id: String,
    /// Human-readable name (e.g., "Bitcoin", "Ethereum Mainnet")
    pub chain_name: String,
    /// Whether the chain supports NFTs
    pub supports_nfts: bool,
    /// Whether the chain supports smart contracts
    pub supports_smart_contracts: bool,
}

/// Internal registry of discovered chain metadata.
///
/// Replaces the former `SimpleChainRegistry` — now embedded directly
/// in `ChainDiscovery` to eliminate the three-registry overlap.
#[derive(Clone)]
struct ChainCatalog {
    chains: HashMap<String, ChainInfo>,
}

impl ChainCatalog {
    fn new() -> Self {
        Self {
            chains: HashMap::new(),
        }
    }

    fn register(&mut self, chain_id: String, chain_name: String) {
        let info = ChainInfo {
            chain_id: chain_id.clone(),
            chain_name,
            supports_nfts: true,
            supports_smart_contracts: true,
        };
        self.chains.insert(chain_id, info);
    }

    fn supported_chains(&self) -> Vec<String> {
        self.chains.keys().cloned().collect()
    }

    fn get_chain_info(&self, chain_id: &str) -> Option<&ChainInfo> {
        self.chains.get(chain_id)
    }

    fn supports_nfts(&self, chain_id: &str) -> bool {
        self.chains
            .get(chain_id)
            .map(|info| info.supports_nfts)
            .unwrap_or(false)
    }

    fn len(&self) -> usize {
        self.chains.len()
    }
}

/// Chain discovery system for automatic chain loading and adapter creation.
///
/// This is the **only** public entry point for chain management. It combines:
/// - TOML configuration loading from `chains/` directory
/// - Chain metadata catalog (ID, name, capabilities)
/// - Plugin-based adapter instantiation
/// - Factory building for adapter creation
pub struct ChainDiscovery {
    config_loader: ChainConfigLoader,
    catalog: ChainCatalog,
    plugins: ChainPluginRegistry,
}

impl ChainDiscovery {
    /// Create a new empty discovery system.
    pub fn new() -> Self {
        Self {
            config_loader: ChainConfigLoader::new(),
            catalog: ChainCatalog::new(),
            plugins: ChainPluginRegistry::new(),
        }
    }

    /// Discover and load all chains from the given directory.
    ///
    /// Reads all `*.toml` files in `chains_dir`, loads their configurations,
    /// and registers them in the internal catalog.
    ///
    /// # Arguments
    /// * `chains_dir` — Path to directory containing chain configuration TOML files
    pub fn discover_chains(
        &mut self,
        chains_dir: &Path,
    ) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(debug_assertions)]
        eprintln!("Discovering chains from: {}", chains_dir.display());

        self.config_loader.load_from_directory(chains_dir)?;

        for (chain_id, config) in self.config_loader.all_configs() {
            #[cfg(debug_assertions)]
            eprintln!(
                "Registering chain: {} ({})",
                chain_id, config.chain_name
            );
            self.catalog
                .register(chain_id.clone(), config.chain_name.clone());
        }

        #[cfg(debug_assertions)]
        eprintln!(
            "Successfully discovered {} chains",
            self.catalog.len()
        );

        Ok(())
    }

    /// Register a plugin so discovered configs can create adapters dynamically.
    pub fn register_plugin(&mut self, plugin: Arc<dyn ChainPlugin>) {
        self.plugins.register(plugin);
    }

    /// Access the plugin registry.
    pub fn plugins(&self) -> &ChainPluginRegistry {
        &self.plugins
    }

    /// Create an adapter for a discovered chain using its loaded configuration.
    ///
    /// Looks up the chain configuration and asks the appropriate plugin
    /// to instantiate the adapter.
    ///
    /// # Arguments
    /// * `chain_id` — Chain identifier (e.g., "bitcoin")
    ///
    /// # Returns
    /// * `Some(adapter)` — If a plugin can create an adapter for this chain
    /// * `None` — If no plugin is registered for this chain
    pub fn create_adapter(
        &self,
        chain_id: &str,
    ) -> Option<Box<dyn ChainAdapter>> {
        let config = self.get_chain_config(chain_id).cloned();
        self.plugins.create_adapter(chain_id, config)
    }

    /// Build an adapter factory from the registered plugins.
    pub fn build_factory(&self) -> AdapterFactory {
        let mut factory = AdapterFactory::empty();
        factory.register_plugins_from_registry(&self.plugins);
        factory
    }

    /// Get chain information by ID.
    pub fn get_chain_info(&self, chain_id: &str) -> Option<&ChainInfo> {
        self.catalog.get_chain_info(chain_id)
    }

    /// Get configuration for a specific chain.
    pub fn get_chain_config(&self, chain_id: &str) -> Option<&ChainConfig> {
        self.config_loader.get_config(chain_id)
    }

    /// Get all chain configurations.
    pub fn all_chain_configs(&self) -> &HashMap<String, ChainConfig> {
        self.config_loader.all_configs()
    }

    /// Get all registered chain IDs.
    pub fn supported_chain_ids(&self) -> Vec<String> {
        self.catalog.supported_chains()
    }

    /// Check if a chain supports NFTs.
    pub fn supports_nfts(&self, chain_id: &str) -> bool {
        self.catalog.supports_nfts(chain_id)
    }

    /// Get chains that support NFTs.
    pub fn nft_supported_chains(&self) -> Vec<String> {
        self.catalog
            .supported_chains()
            .into_iter()
            .filter(|chain_id| self.catalog.supports_nfts(chain_id))
            .collect()
    }

    /// Load chains from the default directory (`./chains/`).
    pub fn load_default_chains(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        if let Some(chains_dir) = self.config_loader.load_from_default_locations()? {
            self.refresh_catalog_from_configs();
            #[cfg(debug_assertions)]
            eprintln!(
                "Loaded chains from default directory: {}",
                chains_dir.display()
            );
            Ok(())
        } else {
            #[cfg(debug_assertions)]
            eprintln!("Default chains directory not found, no chains loaded");
            Ok(())
        }
    }

    fn refresh_catalog_from_configs(&mut self) {
        self.catalog = ChainCatalog::new();
        for (chain_id, config) in self.config_loader.all_configs() {
            self.catalog
                .register(chain_id.clone(), config.chain_name.clone());
        }
    }
}

impl Default for ChainDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_chain_discovery() {
        let temp_dir = TempDir::new().unwrap();
        let chains_dir = temp_dir.path();

        let test_config = r#"
chain_id = "test-chain"
chain_name = "Test Chain"
default_network = "testnet"
rpc_endpoints = ["https://test-rpc.example.com"]
block_explorer_urls = ["https://test-explorer.example.com"]

[capabilities]
supports_nfts = true
supports_smart_contracts = true
account_model = "Account"
confirmation_blocks = 12
max_batch_size = 100
supported_networks = ["mainnet", "testnet"]
supports_cross_chain = false

[custom_settings]
test_setting = "value"
"#;

        let config_path = chains_dir.join("test-chain.toml");
        fs::write(&config_path, test_config).unwrap();

        let mut discovery = ChainDiscovery::new();
        discovery.discover_chains(chains_dir).unwrap();

        let supported_chains = discovery.supported_chain_ids();
        assert_eq!(supported_chains.len(), 1);
        assert_eq!(supported_chains[0], "test-chain");

        let config = discovery.get_chain_config("test-chain");
        assert!(config.is_some());
        assert_eq!(config.unwrap().chain_name, "Test Chain");

        let info = discovery.get_chain_info("test-chain");
        assert!(info.is_some());
        assert_eq!(info.unwrap().chain_name, "Test Chain");
    }

    #[test]
    fn test_chain_info_capabilities() {
        let temp_dir = TempDir::new().unwrap();
        let chains_dir = temp_dir.path();

        let test_config = r#"
chain_id = "nft-chain"
chain_name = "NFT Chain"
default_network = "mainnet"
rpc_endpoints = ["https://nft-rpc.example.com"]
block_explorer_urls = []

[capabilities]
supports_nfts = true
supports_smart_contracts = false
account_model = "Account"
confirmation_blocks = 1
max_batch_size = 50
supported_networks = ["mainnet"]
supports_cross_chain = true
"#;

        let config_path = chains_dir.join("nft-chain.toml");
        fs::write(&config_path, test_config).unwrap();

        let mut discovery = ChainDiscovery::new();
        discovery.discover_chains(chains_dir).unwrap();

        assert!(discovery.supports_nfts("nft-chain"));
        let nft_chains = discovery.nft_supported_chains();
        assert_eq!(nft_chains.len(), 1);
        assert_eq!(nft_chains[0], "nft-chain");
    }
}
