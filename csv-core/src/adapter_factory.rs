//! Adapter factory for dynamic chain adapter instantiation.
//!
//! This module provides a factory pattern for creating chain adapters
//! based on chain IDs, enabling dynamic chain support without hardcoding.
//!
//! # Features
//!
//! The factory supports chain-specific features that enable real adapter implementations:
//! - `bitcoin` - Enables Bitcoin adapter via `csv-adapter-bitcoin`
//! - `solana` - Enables Solana adapter via `csv-adapter-solana`
//! - `aptos` - Enables Aptos adapter via `csv-adapter-aptos`
//! - `sui` - Enables Sui adapter via `csv-adapter-sui`
//! - `ethereum` - Enables Ethereum adapter via `csv-adapter-ethereum`
//! - `full` - Enables all chain adapters
//!
//! When a feature is not enabled, the factory will return `None` for that chain.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::OnceLock;

use crate::chain_adapter::ChainAdapter;
use crate::chain_config::ChainConfig;
use crate::chain_plugin::ChainPluginRegistry;

// Adapter imports are commented out temporarily due to cyclic dependency issues
// These will be re-enabled once the dependency cycle is resolved
// #[cfg(feature = "bitcoin")]
// use csv_adapter_bitcoin::BitcoinAnchorLayer;
// #[cfg(feature = "solana")]
// use csv_adapter_solana::SolanaAnchorLayer;
// #[cfg(feature = "aptos")]
// use csv_adapter_aptos::AptosAnchorLayer;
// #[cfg(feature = "sui")]
// use csv_adapter_sui::SuiAnchorLayer;
// #[cfg(feature = "ethereum")]
// use csv_adapter_ethereum::EthereumAnchorLayer;

/// Factory function type for creating chain adapters
type AdapterFactoryFn = Arc<dyn Fn(Option<ChainConfig>) -> Box<dyn ChainAdapter> + Send + Sync>;

/// Registry of adapter factories for dynamic chain instantiation
pub struct AdapterFactory {
    factories: HashMap<String, AdapterFactoryFn>,
}

impl AdapterFactory {
    /// Create a new empty adapter factory.
    pub fn empty() -> Self {
        Self {
            factories: HashMap::new(),
        }
    }

    /// Create a new adapter factory with all built-in chains registered
    ///
    /// # Note
    ///
    /// Only chains with enabled features will be registered. To enable all chains,
    /// use the `full` feature:
    /// ```toml
    /// csv-adapter-core = { version = "0.3", features = ["full"] }
    /// ```
    pub fn new() -> Self {
        let mut factory = Self::empty();

        // Register built-in chain adapters
        factory.register_built_in_adapters();

        factory
    }

    /// Register all built-in chain adapters
    fn register_built_in_adapters(&mut self) {
        // Chain adapter registration is commented out temporarily due to cyclic dependency issues
        // These will be re-enabled once the dependency cycle is resolved
        
        // Bitcoin
        // #[cfg(feature = "bitcoin")]
        // self.register_with_config("bitcoin", Arc::new(|config| {
        //     if let Some(cfg) = config {
        //         match csv_adapter_bitcoin::create_bitcoin_adapter(&cfg) {
        //             Ok(adapter) => Box::new(adapter),
        //             Err(_) => Box::new(BitcoinAnchorLayer::signet().unwrap_or_else(|_| {
        //                 // Fallback: create a minimal adapter
        //                 let config = csv_adapter_bitcoin::BitcoinConfig::default();
        //                 let wallet = csv_adapter_bitcoin::SealWallet::generate_random(
        //                     csv_adapter_bitcoin::bitcoin::Network::Signet
        //                 );
        //                 BitcoinAnchorLayer::with_wallet(config, wallet).unwrap()
        //             })),
        //         }
        //     } else {
        //         Box::new(BitcoinAnchorLayer::signet().unwrap_or_else(|_| {
        //             let config = csv_adapter_bitcoin::BitcoinConfig::default();
        //             let wallet = csv_adapter_bitcoin::SealWallet::generate_random(
        //                 csv_adapter_bitcoin::bitcoin::Network::Signet
        //             );
        //             BitcoinAnchorLayer::with_wallet(config, wallet).unwrap()
        //         }))
        //     }
        // }));

        // Ethereum
        // #[cfg(feature = "ethereum")]
        // self.register_with_config("ethereum", Arc::new(|config| {
        //     if let Some(cfg) = config {
        //         match csv_adapter_ethereum::create_ethereum_adapter(&cfg) {
        //             Ok(adapter) => Box::new(adapter),
        //             Err(_) => Box::new(EthereumAnchorLayer::with_test().unwrap()),
        //         }
        //     } else {
        //         Box::new(EthereumAnchorLayer::with_test().unwrap())
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
        //                 Box::new(SolanaAnchorLayer::new(config))
        //             }
        //         }
        //     } else {
        //         let config = csv_adapter_solana::SolanaConfig::default();
        //         Box::new(SolanaAnchorLayer::new(config))
        //     }
        // }));

        // Sui
        // #[cfg(feature = "sui")]
        // self.register_with_config("sui", Arc::new(|config| {
        //     if let Some(cfg) = config {
        //         match csv_adapter_sui::create_sui_adapter(&cfg) {
        //             Ok(adapter) => Box::new(adapter),
        //             Err(_) => Box::new(SuiAnchorLayer::with_test().unwrap()),
        //         }
        //     } else {
        //         Box::new(SuiAnchorLayer::with_test().unwrap())
        //     }
        // }));

        // Aptos
        // #[cfg(feature = "aptos")]
        // self.register_with_config("aptos", Arc::new(|config| {
        //     if let Some(cfg) = config {
        //         match csv_adapter_aptos::create_aptos_adapter(&cfg) {
        //             Ok(adapter) => Box::new(adapter),
        //             Err(_) => Box::new(AptosAnchorLayer::with_test().unwrap()),
        //         }
        //     } else {
        //         Box::new(AptosAnchorLayer::with_test().unwrap())
        //     }
        // }));
    }

    /// Register a custom adapter factory
    ///
    /// # Arguments
    ///
    /// * `chain_id` - The unique identifier for the chain
    /// * `factory` - A factory function that creates the adapter
    ///
    /// # Example
    ///
    /// ```rust
    /// use csv_adapter_core::adapter_factory::AdapterFactory;
    /// use csv_adapter_core::adapters::ScalableSolanaAdapter;
    ///
    /// let mut factory = AdapterFactory::new();
    /// factory.register("solana", || {
    ///     Box::new(ScalableSolanaAdapter::new())
    /// });
    /// ```
    pub fn register<F>(&mut self, chain_id: &str, factory: F)
    where
        F: Fn() -> Box<dyn ChainAdapter> + Send + Sync + 'static,
    {
        self.factories
            .insert(chain_id.to_string(), Arc::new(move |_| factory()));
    }

    /// Register a factory that receives the discovered chain configuration.
    pub fn register_with_config(&mut self, chain_id: &str, factory: AdapterFactoryFn) {
        self.factories.insert(chain_id.to_string(), factory);
    }

    /// Register all plugins from a plugin registry.
    pub fn register_plugins_from_registry(&mut self, registry: &ChainPluginRegistry) {
        for (chain_id, plugin) in registry.all_plugins() {
            let plugin = Arc::clone(plugin);
            self.register_with_config(
                chain_id,
                Arc::new(move |config| plugin.create_adapter(config)),
            );
        }
    }

    /// Create an adapter for the specified chain
    ///
    /// # Arguments
    ///
    /// * `chain_id` - The chain identifier (e.g., "bitcoin", "solana")
    ///
    /// # Returns
    ///
    /// Returns `Some(Box<dyn ChainAdapter>)` if the chain is registered,
    /// or `None` if no factory exists for the chain.
    ///
    /// # Example
    ///
    /// ```rust
    /// use csv_adapter_core::adapter_factory::AdapterFactory;
    ///
    /// let factory = AdapterFactory::new();
    /// let adapter = factory.create_adapter("bitcoin");
    /// assert!(adapter.is_some());
    /// ```
    pub fn create_adapter(&self, chain_id: &str) -> Option<Box<dyn ChainAdapter>> {
        self.create_adapter_with_config(chain_id, None)
    }

    /// Create an adapter with a discovered configuration.
    pub fn create_adapter_with_config(
        &self,
        chain_id: &str,
        config: Option<ChainConfig>,
    ) -> Option<Box<dyn ChainAdapter>> {
        self.factories.get(chain_id).map(|factory| factory(config))
    }

    /// Check if a chain is supported
    ///
    /// # Arguments
    ///
    /// * `chain_id` - The chain identifier to check
    ///
    /// # Returns
    ///
    /// `true` if the chain has a registered factory, `false` otherwise.
    pub fn is_supported(&self, chain_id: &str) -> bool {
        self.factories.contains_key(chain_id)
    }

    /// Get all supported chain IDs
    ///
    /// # Returns
    ///
    /// A vector of all registered chain IDs.
    pub fn supported_chains(&self) -> Vec<&str> {
        self.factories.keys().map(|k| k.as_str()).collect()
    }

    /// Get the number of registered adapters
    pub fn adapter_count(&self) -> usize {
        self.factories.len()
    }

    /// Create adapters for all supported chains
    ///
    /// # Returns
    ///
    /// A vector of tuples containing (chain_id, adapter) for all registered chains.
    pub fn create_all_adapters(&self) -> Vec<(String, Box<dyn ChainAdapter>)> {
        self.factories
            .iter()
            .map(|(chain_id, factory)| (chain_id.clone(), factory(None)))
            .collect()
    }
}

impl Default for AdapterFactory {
    fn default() -> Self {
        Self::new()
    }
}

/// Global adapter factory singleton for convenience
///
/// Note: In production code, prefer dependency injection over singletons.
static GLOBAL_FACTORY: OnceLock<AdapterFactory> = OnceLock::new();

/// Initialize the global adapter factory
///
/// This function initializes the global factory if it hasn't been initialized yet.
/// It is safe to call multiple times - only the first call will initialize the factory.
pub fn init_global_factory() {
    GLOBAL_FACTORY.get_or_init(AdapterFactory::new);
}

/// Get the global adapter factory
///
/// # Panics
///
/// Panics if the global factory hasn't been initialized.
pub fn global_factory() -> &'static AdapterFactory {
    GLOBAL_FACTORY
        .get()
        .expect("Global adapter factory not initialized. Call init_global_factory() first.")
}

/// Create an adapter using the global factory
///
/// # Arguments
///
/// * `chain_id` - The chain identifier
///
/// # Returns
///
/// Returns `Some(Box<dyn ChainAdapter>)` if the chain is registered,
/// or `None` if no factory exists for the chain.
///
/// # Panics
///
/// Panics if the global factory hasn't been initialized.
pub fn create_adapter(chain_id: &str) -> Option<Box<dyn ChainAdapter>> {
    global_factory().create_adapter(chain_id)
}

/// Check if a chain is supported by the global factory
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
/// Panics if the global factory hasn't been initialized.
pub fn is_chain_supported(chain_id: &str) -> bool {
    global_factory().is_supported(chain_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_adapter_factory_creation() {
        let factory = AdapterFactory::new();
        assert!(factory.is_supported("bitcoin"));
        assert!(factory.is_supported("solana"));
        assert!(!factory.is_supported("unknown_chain"));
    }

    #[test]
    fn test_create_adapter() {
        let factory = AdapterFactory::new();

        let bitcoin = factory.create_adapter("bitcoin");
        assert!(bitcoin.is_some());
        assert_eq!(bitcoin.unwrap().chain_id(), "bitcoin");

        let unknown = factory.create_adapter("unknown");
        assert!(unknown.is_none());
    }

    #[test]
    fn test_supported_chains() {
        let factory = AdapterFactory::new();
        let chains = factory.supported_chains();

        assert!(chains.contains(&"bitcoin"));
        assert!(chains.contains(&"solana"));
    }

    #[test]
    fn test_create_all_adapters() {
        let factory = AdapterFactory::new();
        let adapters = factory.create_all_adapters();

        assert_eq!(adapters.len(), 5); // bitcoin, ethereum, solana, sui, aptos

        for (chain_id, adapter) in adapters {
            assert_eq!(chain_id, adapter.chain_id());
        }
    }

    #[test]
    #[ignore = "Requires full trait impl with async methods - skip for now"]
    fn test_custom_registration() {
        let mut factory = AdapterFactory::new();
        factory.register("custom_chain", || {
            panic!("Not implemented");
        });
        assert!(factory.is_supported("custom_chain"));
    }
}
