//! Scalable builder implementation using dynamic chain registry.

use std::collections::HashSet;
use std::sync::Arc;

use csv_adapter_core::Chain;
use csv_adapter_core::ChainRegistry;

use crate::config::Config;
use crate::errors::CsvError;
use crate::wallet::Wallet;

/// Storage backend for seal and anchor persistence.
#[derive(Debug, Clone)]
pub enum StoreBackend {
    /// In-memory store (non-persistent, for testing).
    InMemory,
    /// SQLite file at the given path.
    #[cfg(feature = "sqlite")]
    Sqlite {
        /// Path to the SQLite database.
        path: String,
    },
}

/// Internal state for the scalable client builder.
#[derive(Default)]
struct ScalableBuilderState {
    enabled_chains: HashSet<String>,
    wallet: Option<Wallet>,
    store_backend: Option<StoreBackend>,
    config: Option<Config>,
    chain_registry: Option<Arc<ChainRegistry>>,
}

/// Scalable fluent builder for constructing a CsvClient with dynamic chain support.
///
/// Use [`CsvClient::scalable_builder()`](crate::client::CsvClient::scalable_builder) to create a new builder.
pub struct ScalableClientBuilder {
    state: ScalableBuilderState,
}

impl ScalableClientBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self {
            state: ScalableBuilderState::default(),
        }
    }

    /// Enable a specific chain by ID.
    ///
    /// This method can be called multiple times to enable multiple chains.
    ///
    /// # Arguments
    ///
    /// * `chain_id` — The chain ID to enable (e.g., "bitcoin", "ethereum", "solana").
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use csv_adapter::prelude::*;
    ///
    /// let client = CsvClient::scalable_builder()
    ///     .with_chain("bitcoin")
    ///     .with_chain("ethereum")
    ///     .with_chain("solana")
    ///     .build()?;
    /// # Ok::<_, csv_adapter::CsvError>(())
    /// ```
    pub fn with_chain(mut self, chain_id: &str) -> Self {
        self.state.enabled_chains.insert(chain_id.to_string());
        self
    }

    /// Enable all chains from the registry.
    ///
    /// This method automatically enables all chains that have been registered
    /// in the chain registry.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use csv_adapter::prelude::*;
    ///
    /// let registry = ChainRegistry::new();
    /// // Register chains dynamically (in real implementation)
    /// registry.register_chain("bitcoin".to_string(), "Bitcoin".to_string());
    /// registry.register_chain("ethereum".to_string(), "Ethereum".to_string());
    ///
    /// let client = CsvClient::scalable_builder()
    ///     .with_chain_registry(registry)
    ///     .with_all_available_chains()
    ///     .build()?;
    /// # Ok::<_, csv_adapter::CsvError>(())
    /// ```
    pub fn with_all_available_chains(mut self) -> Self {
        if let Some(ref registry) = self.state.chain_registry {
            let available_chains = registry.supported_chains();

            for chain_id in available_chains {
                self.state.enabled_chains.insert(chain_id.to_string());
            }
        }

        self
    }

    /// Set the chain registry for dynamic chain discovery.
    ///
    /// The registry contains all registered chain adapters and their capabilities.
    ///
    /// # Arguments
    ///
    /// * `registry` — The chain registry to use.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use csv_adapter::prelude::*;
    ///
    /// let registry = ChainRegistry::new();
    /// // Register chains dynamically
    /// registry.register_chain("polygon".to_string(), "Polygon".to_string());
    ///
    /// let client = CsvClient::scalable_builder()
    ///     .with_chain_registry(registry)
    ///     .with_chain("polygon")
    ///     .build()?;
    /// # Ok::<_, csv_adapter::CsvError>(())
    /// ```
    pub fn with_chain_registry(mut self, registry: ChainRegistry) -> Self {
        self.state.chain_registry = Some(Arc::new(registry));
        self
    }

    /// Attach a wallet to the client.
    ///
    /// The wallet is used for signing transactions and deriving addresses.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use csv_adapter::prelude::*;
    ///
    /// let wallet = Wallet::generate();
    /// let client = CsvClient::scalable_builder()
    ///     .with_wallet(wallet)
    ///     .build()?;
    /// # Ok::<_, csv_adapter::CsvError>(())
    /// ```
    pub fn with_wallet(mut self, wallet: Wallet) -> Self {
        self.state.wallet = Some(wallet);
        self
    }

    /// Set the storage backend.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use csv_adapter::prelude::*;
    ///
    /// let client = CsvClient::scalable_builder()
    ///     .with_store_backend(StoreBackend::InMemory)
    ///     .build()?;
    /// # Ok::<_, csv_adapter::CsvError>(())
    /// ```
    pub fn with_store_backend(mut self, backend: StoreBackend) -> Self {
        self.state.store_backend = Some(backend);
        self
    }

    /// Load configuration from a [`Config`] struct.
    ///
    /// This overrides any previously set chain or store settings.
    pub fn with_config(mut self, config: Config) -> Self {
        // Enable chains from config before moving config into state
        for (chain_id, chain_config) in &config.chains {
            if chain_config.enabled {
                self.state.enabled_chains.insert(chain_id.clone());
            }
        }

        self.state.config = Some(config);
        self
    }

    /// Build the [`CsvClient`](crate::client::CsvClient), validating all settings.
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - No chains are enabled
    /// - The store backend cannot be initialized
    /// - Chain registry is not set when enabling chains from registry
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use csv_adapter::prelude::*;
    ///
    /// let client = CsvClient::scalable_builder()
    ///     .with_chain("bitcoin")
    ///     .with_chain("ethereum")
    ///     .with_chain("solana")
    ///     .build()?;
    /// # Ok::<_, csv_adapter::CsvError>(())
    /// ```
    pub fn build(self) -> Result<crate::client::CsvClient, CsvError> {
        if self.state.enabled_chains.is_empty() {
            return Err(CsvError::BuilderError(
                "At least one chain must be enabled. Use .with_chain() to enable a chain."
                    .to_string(),
            ));
        }

        // Validate that chain registry is set if using registry-based chains
        if self.state.enabled_chains.iter().any(|chain_id| {
            !matches!(
                chain_id.as_str(),
                "bitcoin" | "ethereum" | "solana" | "sui" | "aptos"
            )
        }) && self.state.chain_registry.is_none()
        {
            return Err(CsvError::BuilderError(
                "Chain registry must be set when enabling custom chains. Use .with_chain_registry()."
                    .to_string(),
            ));
        }

        // Apply config overrides if present
        let config = self.state.config.unwrap_or_default();

        // Initialize store backend
        let store = match self.state.store_backend.unwrap_or(StoreBackend::InMemory) {
            StoreBackend::InMemory => {
                crate::client::StoreHandle::InMemory(csv_adapter_core::InMemorySealStore::new())
            }
            #[cfg(feature = "sqlite")]
            StoreBackend::Sqlite { ref path } => crate::client::StoreHandle::Sqlite(
                csv_adapter_store::SqliteSealStore::open(path)
                    .map_err(|e| CsvError::StoreError(e.to_string()))?,
            ),
        };

        // Convert enabled chain IDs to core Chain enum
        let mut core_chains = HashSet::new();
        for chain_id in &self.state.enabled_chains {
            if let Ok(core_chain) = chain_id.parse::<Chain>() {
                core_chains.insert(core_chain);
            }
        }

        // Create the chain facade
        let chain_facade = crate::facade::ChainFacade::new(Arc::new(crate::client::ClientRef::new()));

        #[cfg(feature = "tokio")]
        let event_tx = tokio::sync::broadcast::channel(256).0;
        #[cfg(not(feature = "tokio"))]
        let event_tx = ();

        Ok(crate::client::CsvClient {
            enabled_chains: core_chains,
            wallet: self.state.wallet,
            store: Arc::new(std::sync::Mutex::new(store)),
            config,
            event_tx,
            chain_registry: self.state.chain_registry,
            chain_facade,
        })
    }
}

impl Default for ScalableClientBuilder {
    fn default() -> Self {
        Self::new()
    }
}
