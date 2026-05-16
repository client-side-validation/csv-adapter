//! Driver Registry — Unified Chain Driver Instantiation, Plugin System & Discovery
//!
//! This module provides a unified system for:
//! - **Driver metadata** — `DriverMetadata` describes a chain plugin
//! - **Plugin trait** — `DriverPlugin` creates chain drivers dynamically
//! - **Driver registry** — `DriverRegistry` merges factory + plugin registry roles
//! - **Chain discovery** — `DriverDiscovery` loads TOML configs, catalogs chains,
//!   and builds drivers from registered plugins
//! - **Global singleton** — `init_global_factory()` / `global_factory()` / `create_driver()`
//!
//! # Features
//!
//! The driver registry supports chain-specific features that enable real driver implementations:
//! - `bitcoin` - Enables Bitcoin driver via `csv-adapter-bitcoin`
//! - `solana` - Enables Solana driver via `csv-adapter-solana`
//! - `aptos` - Enables Aptos driver via `csv-adapter-aptos`
//! - `sui` - Enables Sui driver via `csv-adapter-sui`
//! - `ethereum` - Enables Ethereum driver via `csv-adapter-ethereum`
//! - `full` - Enables all chain drivers
//!
//! When a feature is not enabled, the registry will return `None` for that chain.

use crate::collections::HashMap;
use std::any::Any;
use std::path::Path;
use std::sync::Arc;
use std::sync::OnceLock;

use crate::chain_config::{AccountModel, ChainCapabilities, ChainConfig, ChainConfigLoader};
use crate::driver::ChainDriver;

/// Type alias for the driver factory function stored in the registry.
type DriverFactory = Arc<dyn Fn(Option<ChainConfig>) -> Box<dyn ChainDriver> + Send + Sync>;

// Driver imports are commented out temporarily due to cyclic dependency issues
// These will be re-enabled once the dependency cycle is resolved
// #[cfg(feature = "bitcoin")]
// use csv_adapter_bitcoin::BitcoinSealProtocol;
// #[cfg(feature = "solana")]
// use csv_adapter_solana::SolanaSealProtocol;
// #[cfg(feature = "aptos")]
// use csv_adapter_aptos::AptosSealProtocol;
// #[cfg(feature = "sui")]
// use csv_adapter_sui::SuiSealProtocol;
// #[cfg(feature = "ethereum")]
// use csv_adapter_ethereum::EthereumSealProtocol;

// ===========================================================================
// DriverMetadata
// ===========================================================================

/// Metadata about a chain driver plugin.
#[derive(Debug, Clone)]
pub struct DriverMetadata {
    /// Unique chain identifier
    pub chain_id: String,
    /// Human-readable chain name
    pub chain_name: String,
    /// Chain version
    pub version: String,
    /// Chain capabilities
    pub capabilities: ChainCapabilities,
    /// Author of the plugin
    pub author: Option<String>,
    /// Plugin description
    pub description: Option<String>,
    /// Homepage URL
    pub homepage: Option<String>,
}

// ===========================================================================
// DriverPlugin trait
// ===========================================================================

/// Trait for chain driver plugins.
///
/// Implement this trait to create a new chain driver plugin that can be registered
/// dynamically with the system.
pub trait DriverPlugin: Send + Sync + Any {
    /// Get plugin metadata
    fn metadata(&self) -> DriverMetadata;

    /// Create a chain driver
    ///
    /// # Arguments
    /// * `config` - Optional chain configuration
    fn create_driver(&self, config: Option<ChainConfig>) -> Box<dyn ChainDriver>;

    /// Get default configuration for this chain
    fn default_config(&self) -> ChainConfig;

    /// Validate configuration for this chain
    ///
    /// # Arguments
    /// * `config` - The configuration to validate
    ///
    /// # Returns
    /// `true` if the configuration is valid, `false` otherwise
    fn validate_config(&self, config: &ChainConfig) -> bool {
        config.chain_id == self.metadata().chain_id
    }

    /// Check if this plugin supports a specific feature
    ///
    /// # Arguments
    /// * `feature` - The feature name to check
    fn supports_feature(&self, feature: &str) -> bool {
        let caps = &self.metadata().capabilities;
        match feature {
            "nft" => caps.supports_nfts,
            "smart_contract" => caps.supports_smart_contracts,
            "cross_chain" => caps.supports_cross_chain,
            _ => false,
        }
    }
}

// ===========================================================================
// DriverRegistry (merged from AdapterFactory + DriverRegistry)
// ===========================================================================

/// Registry of driver factories for dynamic chain instantiation.
///
/// This struct merges the roles of the former `AdapterFactory` and
/// `DriverRegistry`, providing both direct factory registration
/// and plugin-based adapter creation from a single API.
pub struct DriverRegistry {
    factories: HashMap<String, DriverFactory>,
    plugins: HashMap<String, Arc<dyn DriverPlugin>>,
}

impl DriverRegistry {
    /// Create a new empty driver registry.
    pub fn empty() -> Self {
        Self {
            factories: HashMap::new(),
            plugins: HashMap::new(),
        }
    }

    /// Create a new driver registry with all built-in chains registered.
    ///
    /// # Note
    ///
    /// Only chains with enabled features will be registered. To enable all chains,
    /// use the `full` feature:
    /// ```toml
    /// csv-adapter-core = { version = "0.3", features = ["full"] }
    /// ```
    pub fn new() -> Self {
        let mut registry = Self::empty();
        registry.register_built_in_drivers();
        registry
    }

    /// Register all built-in chain drivers.
    fn register_built_in_drivers(&mut self) {
        // Driver registration is commented out temporarily due to cyclic dependency issues
        // These will be re-enabled once the dependency cycle is resolved

        // Bitcoin
        // #[cfg(feature = "bitcoin")]
        // self.register_with_config("bitcoin", Arc::new(|config| {
        //     if let Some(cfg) = config {
        //         match csv_adapter_bitcoin::create_bitcoin_adapter(&cfg) {
        //             Ok(adapter) => Box::new(adapter),
        //             Err(_) => Box::new(BitcoinSealProtocol::signet().unwrap_or_else(|_| {
        //                 let config = csv_adapter_bitcoin::BitcoinConfig::default();
        //                 let wallet = csv_adapter_bitcoin::SealWallet::generate_random(
        //                     csv_adapter_bitcoin::bitcoin::Network::Signet
        //                 );
        //                 BitcoinSealProtocol::with_wallet(config, wallet).unwrap()
        //             })),
        //         }
        //     } else {
        //         Box::new(BitcoinSealProtocol::signet().unwrap_or_else(|_| {
        //             let config = csv_adapter_bitcoin::BitcoinConfig::default();
        //             let wallet = csv_adapter_bitcoin::SealWallet::generate_random(
        //                 csv_adapter_bitcoin::bitcoin::Network::Signet
        //             );
        //             BitcoinSealProtocol::with_wallet(config, wallet).unwrap()
        //         }))
        //     }
        // }));

        // Ethereum
        // #[cfg(feature = "ethereum")]
        // self.register_with_config("ethereum", Arc::new(|config| {
        //     if let Some(cfg) = config {
        //         match csv_adapter_ethereum::create_ethereum_adapter(&cfg) {
        //             Ok(adapter) => Box::new(adapter),
        //             Err(_) => Box::new(EthereumSealProtocol::with_test().unwrap()),
        //         }
        //     } else {
        //         Box::new(EthereumSealProtocol::with_test().unwrap())
        //     }
        // }));

        // Solana
        // #[cfg(feature = "solana")]
        // self.register_with_config("solana", Arc::new(|config| {
        //     if let Some(cfg) = config {
        //         match csv_adapter_solana::create_solana_adapter(&cfg) {
        //             Ok(adapter) => Box::new(adapter),
        //             Err(_) => {
        //                 let config = csv_adapter_solana::SolanaConfig::default();
        //                 Box::new(SolanaSealProtocol::new(config))
        //             }
        //         }
        //     } else {
        //         let config = csv_adapter_solana::SolanaConfig::default();
        //         Box::new(SolanaSealProtocol::new(config))
        //     }
        // }));

        // Sui
        // #[cfg(feature = "sui")]
        // self.register_with_config("sui", Arc::new(|config| {
        //     if let Some(cfg) = config {
        //         match csv_adapter_sui::create_sui_adapter(&cfg) {
        //             Ok(adapter) => Box::new(adapter),
        //             Err(_) => Box::new(SuiSealProtocol::with_test().unwrap()),
        //         }
        //     } else {
        //         Box::new(SuiSealProtocol::with_test().unwrap())
        //     }
        // }));

        // Aptos
        // #[cfg(feature = "aptos")]
        // self.register_with_config("aptos", Arc::new(|config| {
        //     if let Some(cfg) = config {
        //         match csv_adapter_aptos::create_aptos_adapter(&cfg) {
        //             Ok(adapter) => Box::new(adapter),
        //             Err(_) => Box::new(AptosSealProtocol::with_test().unwrap()),
        //         }
        //     } else {
        //         Box::new(AptosSealProtocol::with_test().unwrap())
        //     }
        // }));
    }

    // -- Plugin registry methods --

    /// Register a chain plugin.
    pub fn register_plugin(&mut self, plugin: Arc<dyn DriverPlugin>) {
        let metadata = plugin.metadata();
        self.plugins.insert(metadata.chain_id.clone(), plugin);
    }

    /// Unregister a chain plugin.
    ///
    /// # Returns
    /// `true` if the plugin was removed, `false` if it didn't exist.
    pub fn unregister_plugin(&mut self, chain_id: &str) -> bool {
        self.plugins.remove(chain_id).is_some()
    }

    /// Check if a plugin is registered.
    pub fn is_plugin_registered(&self, chain_id: &str) -> bool {
        self.plugins.contains_key(chain_id)
    }

    /// Get a plugin by chain ID.
    pub fn get_plugin(&self, chain_id: &str) -> Option<Arc<dyn DriverPlugin>> {
        self.plugins.get(chain_id).cloned()
    }

    /// Get plugin metadata.
    pub fn get_plugin_metadata(&self, chain_id: &str) -> Option<DriverMetadata> {
        self.plugins.get(chain_id).map(|p| p.metadata())
    }

    /// Get all registered chain IDs from plugins.
    pub fn registered_chains(&self) -> Vec<&str> {
        self.plugins.keys().map(|k| k.as_str()).collect()
    }

    /// Get all plugins.
    pub fn all_plugins(&self) -> &HashMap<String, Arc<dyn DriverPlugin>> {
        &self.plugins
    }

    /// Create a driver for a registered plugin chain.
    pub fn create_driver_from_plugin(
        &self,
        chain_id: &str,
        config: Option<ChainConfig>,
    ) -> Option<Box<dyn ChainDriver>> {
        self.plugins.get(chain_id).map(|p| p.create_driver(config))
    }

    /// Get chains that support a specific feature.
    pub fn chains_with_feature(&self, feature: &str) -> Vec<&str> {
        self.plugins
            .iter()
            .filter(|(_, plugin)| plugin.supports_feature(feature))
            .map(|(id, _)| id.as_str())
            .collect()
    }

    /// Get number of registered plugins.
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }

    // -- Factory methods --

    /// Register a custom driver factory (no config).
    ///
    /// # Arguments
    ///
    /// * `chain_id` - The unique identifier for the chain
    /// * `factory` - A factory function that creates the driver
    ///
    /// # Example
    ///
    /// ```rust
    /// use csv_core::driver_registry::DriverRegistry;
    ///
    /// let mut registry = DriverRegistry::new();
    /// registry.register("solana", || {
    ///     panic!("Not implemented");
    /// });
    /// ```
    pub fn register<F>(&mut self, chain_id: &str, factory: F)
    where
        F: Fn() -> Box<dyn ChainDriver> + Send + Sync + 'static,
    {
        self.factories
            .insert(chain_id.to_string(), Arc::new(move |_| factory()));
    }

    /// Register a factory that receives the discovered chain configuration.
    pub fn register_with_config(&mut self, chain_id: &str, factory: DriverFactory) {
        self.factories.insert(chain_id.to_string(), factory);
    }

    /// Register all plugins from the plugin registry into the factory.
    pub fn register_plugins_from_registry(&mut self, registry: &DriverRegistry) {
        for (chain_id, plugin) in registry.all_plugins() {
            let plugin = Arc::clone(plugin);
            self.register_with_config(
                chain_id,
                Arc::new(move |config| plugin.create_driver(config)),
            );
        }
    }

    /// Create a driver for the specified chain (factory lookup).
    ///
    /// # Arguments
    ///
    /// * `chain_id` - The chain identifier (e.g., "bitcoin", "solana")
    ///
    /// # Returns
    ///
    /// Returns `Some(Box<dyn ChainDriver>)` if the chain is registered,
    /// or `None` if no factory exists for the chain.
    pub fn create_driver(&self, chain_id: &str) -> Option<Box<dyn ChainDriver>> {
        self.create_driver_with_config(chain_id, None)
    }

    /// Create a driver with a discovered configuration.
    pub fn create_driver_with_config(
        &self,
        chain_id: &str,
        config: Option<ChainConfig>,
    ) -> Option<Box<dyn ChainDriver>> {
        // First try plugin-based creation
        if let Some(driver) = self.create_driver_from_plugin(chain_id, config.clone()) {
            return Some(driver);
        }
        // Fall back to direct factory
        self.factories.get(chain_id).map(|factory| factory(config))
    }

    /// Check if a chain is supported.
    pub fn is_supported(&self, chain_id: &str) -> bool {
        self.factories.contains_key(chain_id) || self.is_plugin_registered(chain_id)
    }

    /// Get all supported chain IDs.
    pub fn supported_chains(&self) -> Vec<&str> {
        let mut chains: Vec<&str> = self.factories.keys().map(|k| k.as_str()).collect();
        for id in self.registered_chains() {
            if !chains.contains(&id) {
                chains.push(id);
            }
        }
        chains
    }

    /// Get the number of registered adapters.
    pub fn adapter_count(&self) -> usize {
        self.factories.len()
    }

    /// Create adapters for all supported chains.
    pub fn create_all_adapters(&self) -> Vec<(String, Box<dyn ChainDriver>)> {
        self.factories
            .iter()
            .map(|(chain_id, factory)| (chain_id.clone(), factory(None)))
            .collect()
    }
}

impl Default for DriverRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// ChainInfo
// ===========================================================================

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

// ===========================================================================
// DriverDiscovery
// ===========================================================================

/// Internal registry of discovered chain metadata.
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
/// - Plugin-based driver instantiation
/// - Factory building for adapter creation
pub struct DriverDiscovery {
    config_loader: ChainConfigLoader,
    catalog: ChainCatalog,
    registry: DriverRegistry,
}

impl DriverDiscovery {
    /// Create a new empty discovery system.
    pub fn new() -> Self {
        Self {
            config_loader: ChainConfigLoader::new(),
            catalog: ChainCatalog::new(),
            registry: DriverRegistry::new(),
        }
    }

    /// Discover and load all chains from the given directory.
    ///
    /// Reads all `*.toml` files in `chains_dir`, loads their configurations,
    /// and registers them in the internal catalog.
    ///
    /// # Arguments
    /// * `chains_dir` — Path to directory containing chain configuration TOML files
    pub fn discover_chains(&mut self, chains_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        #[cfg(debug_assertions)]
        eprintln!("Discovering chains from: {}", chains_dir.display());

        self.config_loader.load_from_directory(chains_dir)?;

        for (chain_id, config) in self.config_loader.all_configs() {
            #[cfg(debug_assertions)]
            eprintln!("Registering chain: {} ({})", chain_id, config.chain_name);
            self.catalog
                .register(chain_id.clone(), config.chain_name.clone());
        }

        #[cfg(debug_assertions)]
        eprintln!("Successfully discovered {} chains", self.catalog.len());

        Ok(())
    }

    /// Register a plugin so discovered configs can create adapters dynamically.
    pub fn register_plugin(&mut self, plugin: Arc<dyn DriverPlugin>) {
        self.registry.register_plugin(plugin);
    }

    /// Access the driver registry.
    pub fn registry(&self) -> &DriverRegistry {
        &self.registry
    }

    /// Create a driver for a discovered chain using its loaded configuration.
    ///
    /// Looks up the chain configuration and asks the appropriate plugin
    /// to instantiate the driver.
    ///
    /// # Arguments
    /// * `chain_id` — Chain identifier (e.g., "bitcoin")
    ///
    /// # Returns
    /// * `Some(driver)` — If a plugin can create a driver for this chain
    /// * `None` — If no plugin is registered for this chain
    pub fn create_driver(&self, chain_id: &str) -> Option<Box<dyn ChainDriver>> {
        let config = self.get_chain_config(chain_id).cloned();
        self.registry.create_driver_with_config(chain_id, config)
    }

    /// Build a driver registry from the registered plugins.
    pub fn build_registry(&self) -> DriverRegistry {
        let mut registry = DriverRegistry::empty();
        registry.register_plugins_from_registry(&self.registry);
        registry
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

impl Default for DriverDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

// ===========================================================================
// Builder (renamed from ChainPluginBuilder)
// ===========================================================================

/// Type alias for config factory functions
type ConfigFactoryFn = dyn Fn() -> ChainConfig + Send + Sync;

/// Builder for creating driver plugins.
pub struct DriverPluginBuilder {
    metadata: DriverMetadata,
    config_factory: Option<Box<ConfigFactoryFn>>,
}

impl DriverPluginBuilder {
    /// Create a new plugin builder.
    pub fn new(chain_id: &str, chain_name: &str) -> Self {
        Self {
            metadata: DriverMetadata {
                chain_id: chain_id.to_string(),
                chain_name: chain_name.to_string(),
                version: "1.0.0".to_string(),
                capabilities: ChainCapabilities {
                    supports_nfts: false,
                    supports_smart_contracts: false,
                    account_model: AccountModel::Account,
                    confirmation_blocks: 12,
                    max_batch_size: 100,
                    supported_networks: vec!["mainnet".to_string()],
                    supports_cross_chain: false,
                    custom_features: HashMap::new(),
                },
                author: None,
                description: None,
                homepage: None,
            },
            config_factory: None,
        }
    }

    /// Set the plugin version.
    pub fn version(mut self, version: &str) -> Self {
        self.metadata.version = version.to_string();
        self
    }

    /// Set the plugin author.
    pub fn author(mut self, author: &str) -> Self {
        self.metadata.author = Some(author.to_string());
        self
    }

    /// Set the plugin description.
    pub fn description(mut self, description: &str) -> Self {
        self.metadata.description = Some(description.to_string());
        self
    }

    /// Set the plugin homepage.
    pub fn homepage(mut self, homepage: &str) -> Self {
        self.metadata.homepage = Some(homepage.to_string());
        self
    }

    /// Set the chain capabilities.
    pub fn capabilities(mut self, capabilities: ChainCapabilities) -> Self {
        self.metadata.capabilities = capabilities;
        self
    }

    /// Set the config factory.
    pub fn config_factory<F>(mut self, factory: F) -> Self
    where
        F: Fn() -> ChainConfig + Send + Sync + 'static,
    {
        self.config_factory = Some(Box::new(factory));
        self
    }

    /// Build the plugin.
    pub fn build(self) -> Result<Arc<dyn DriverPlugin>, DriverPluginBuildError> {
        if self.config_factory.is_none() {
            return Err(DriverPluginBuildError::MissingConfigFactory);
        }

        Ok(Arc::new(BuiltDriverPlugin {
            metadata: self.metadata,
            config_factory: self.config_factory.unwrap(),
        }))
    }
}

/// Errors that can occur when building a plugin.
#[derive(Debug, Clone, PartialEq)]
pub enum DriverPluginBuildError {
    /// Missing config factory
    MissingConfigFactory,
}

impl std::fmt::Display for DriverPluginBuildError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingConfigFactory => write!(f, "Config factory is required"),
        }
    }
}

impl std::error::Error for DriverPluginBuildError {}

/// Built driver plugin from builder.
pub struct BuiltDriverPlugin {
    metadata: DriverMetadata,
    config_factory: Box<dyn Fn() -> ChainConfig + Send + Sync>,
}

impl DriverPlugin for BuiltDriverPlugin {
    fn metadata(&self) -> DriverMetadata {
        self.metadata.clone()
    }

    fn create_driver(&self, _config: Option<ChainConfig>) -> Box<dyn ChainDriver> {
        // This should be implemented by the actual plugin
        // The config_factory is used to provide default config
        let _config = (self.config_factory)();
        // Default implementation - should be overridden
        panic!("create_driver not implemented for BuiltDriverPlugin")
    }

    fn default_config(&self) -> ChainConfig {
        (self.config_factory)()
    }
}

// ===========================================================================
// Global factory singleton
// ===========================================================================

/// Global driver registry singleton for convenience.
///
/// Note: In production code, prefer dependency injection over singletons.
static GLOBAL_REGISTRY: OnceLock<DriverRegistry> = OnceLock::new();

/// Initialize the global driver registry.
///
/// This function initializes the global registry if it hasn't been initialized yet.
/// It is safe to call multiple times - only the first call will initialize the registry.
pub fn init_global_factory() {
    GLOBAL_REGISTRY.get_or_init(DriverRegistry::new);
}

/// Get the global driver registry.
///
/// # Panics
///
/// Panics if the global registry hasn't been initialized.
pub fn global_factory() -> &'static DriverRegistry {
    GLOBAL_REGISTRY
        .get()
        .expect("Global driver registry not initialized. Call init_global_factory() first.")
}

/// Create a driver using the global registry.
///
/// # Arguments
///
/// * `chain_id` - The chain identifier
///
/// # Returns
///
/// Returns `Some(Box<dyn ChainDriver>)` if the chain is registered,
/// or `None` if no factory exists for the chain.
///
/// # Panics
///
/// Panics if the global registry hasn't been initialized.
pub fn create_driver(chain_id: &str) -> Option<Box<dyn ChainDriver>> {
    global_factory().create_driver(chain_id)
}

/// Check if a chain is supported by the global registry.
///
/// # Arguments
///
/// * `chain_id` - The chain identifier to check
///
/// # Returns
///
/// `true` if the chain has a registered factory, `false` otherwise.
///
/// # Panics
///
/// Panics if the global registry hasn't been initialized.
pub fn is_chain_supported(chain_id: &str) -> bool {
    global_factory().is_supported(chain_id)
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_adapter_factory_creation() {
        let registry = DriverRegistry::new();
        assert!(!registry.is_supported("bitcoin"));
        assert!(!registry.is_supported("solana"));
        assert!(!registry.is_supported("unknown_chain"));
    }

    #[test]
    fn test_create_driver() {
        let registry = DriverRegistry::new();

        let bitcoin = registry.create_driver("bitcoin");
        assert!(bitcoin.is_none());

        let unknown = registry.create_driver("unknown");
        assert!(unknown.is_none());
    }

    #[test]
    fn test_supported_chains() {
        let registry = DriverRegistry::new();
        let chains = registry.supported_chains();

        assert!(chains.is_empty());
    }

    #[test]
    fn test_create_all_adapters() {
        let registry = DriverRegistry::new();
        let adapters = registry.create_all_adapters();

        assert_eq!(adapters.len(), 0);
    }

    #[test]
    #[ignore = "Requires full trait impl with async methods - skip for now"]
    fn test_custom_registration() {
        let mut registry = DriverRegistry::new();
        registry.register("custom_chain", || {
            panic!("Not implemented");
        });
        assert!(registry.is_supported("custom_chain"));
    }

    #[test]
    #[ignore = "Requires ScalableBitcoinAdapter which is not yet available"]
    fn test_plugin_registry() {
        // TODO: Implement test when ScalableBitcoinAdapter is available
    }

    #[test]
    #[ignore = "Requires ScalableBitcoinAdapter which is not yet available"]
    fn test_plugin_features() {
        // TODO: Implement test when ScalableBitcoinAdapter is available
    }

    #[test]
    #[ignore = "Requires ScalableBitcoinAdapter which is not yet available"]
    fn test_plugin_builder() {
        // TODO: Implement test when ScalableBitcoinAdapter is available
    }

    #[test]
    #[ignore = "Requires ScalableBitcoinAdapter which is not yet available"]
    fn test_plugin_builder_missing_factories() {
        // TODO: Implement test when ScalableBitcoinAdapter is available
    }

    #[test]
    fn test_driver_discovery() {
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

        let mut discovery = DriverDiscovery::new();
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

[custom_settings]
"#;

        let config_path = chains_dir.join("nft-chain.toml");
        fs::write(&config_path, test_config).unwrap();

        let mut discovery = DriverDiscovery::new();
        discovery.discover_chains(chains_dir).unwrap();

        assert!(discovery.supports_nfts("nft-chain"));
        let nft_chains = discovery.nft_supported_chains();
        assert_eq!(nft_chains.len(), 1);
        assert_eq!(nft_chains[0], "nft-chain");
    }
}
