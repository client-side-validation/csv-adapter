//! Unified CSV client with builder pattern.
//!
//! The [`CsvClient`] is the main entry point for all CSV operations.
//! It provides access to managers for rights, transfers, proofs, wallet,
//! and event streaming.
//!
//! # Example
//!
//! ```no_run
//! use csv_adapter::prelude::*;
//!
//! #[tokio::main]
//! async fn main() -> Result<()> {
//!     let client = CsvClient::builder()
//!         .with_chain(Chain::Bitcoin)
//!         .with_store_backend(StoreBackend::InMemory)
//!         .build()?;
//!
//!     // Access managers
//!     let rights = client.rights();
//!     let transfers = client.transfers();
//!     let proofs = client.proofs();
//!
//!     // Stream events
//!     let events = client.watch();
//!
//!     Ok(())
//! }
//! ```

use std::collections::HashSet;
use std::sync::Arc;

use csv_adapter_core::Chain;
use csv_adapter_core::ChainRegistry;
use tokio::sync::broadcast;

use crate::builder::ClientBuilder;
use crate::config::Config;
use crate::deploy::DeploymentManager;
use crate::errors::CsvError;
use crate::events::EventStream;
use crate::proofs::ProofManager;
use crate::rights::RightsManager;
use crate::scalable_builder_v2::ScalableClientBuilder;
use crate::transfers::TransferManager;
use crate::wallet::Wallet;
use crate::wallet::WalletManager;

/// Handle to the underlying storage backend.
pub enum StoreHandle {
    /// In-memory seal and anchor store.
    InMemory(csv_adapter_core::InMemorySealStore),
    /// SQLite-backed store (requires `sqlite` feature).
    #[cfg(feature = "sqlite")]
    Sqlite(csv_adapter_store::SqliteSealStore),
}

/// The unified CSV client.
///
/// This is the main entry point for all CSV operations. Construct it
/// using [`CsvClient::builder()`] or [`CsvClient::scalable_builder()`] and access the various managers for
/// rights, transfers, proofs, and wallet operations.
///
/// # Thread Safety
///
/// `CsvClient` is `Send + Sync` and can be shared across threads via
/// `Arc<CsvClient>`.
pub struct CsvClient {
    /// Set of enabled chain adapters.
    pub(crate) enabled_chains: HashSet<Chain>,
    /// Optional wallet for signing and address derivation.
    pub(crate) wallet: Option<Wallet>,
    /// Storage backend for seals and anchors.
    pub(crate) store: Arc<std::sync::Mutex<StoreHandle>>,
    /// Configuration.
    pub(crate) config: Config,
    /// Event broadcast channel sender.
    pub(crate) event_tx: broadcast::Sender<crate::events::Event>,
    /// Chain registry for dynamic chain management.
    pub(crate) chain_registry: Option<Arc<ChainRegistry>>,
}

impl CsvClient {
    /// Create a new [`ClientBuilder`] for constructing a `CsvClient`.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use csv_adapter::prelude::*;
    ///
    /// let client = CsvClient::builder()
    ///     .with_chain(Chain::Bitcoin)
    ///     .with_store_backend(StoreBackend::InMemory)
    ///     .build()?;
    /// # Ok::<_, csv_adapter::CsvError>(())
    /// ```
    pub fn builder() -> ClientBuilder {
        ClientBuilder::new()
    }

    /// Create a new [`ScalableClientBuilder`] for constructing a `CsvClient` with dynamic chain support.
    ///
    /// This builder supports dynamic chain loading from configuration files and chain registries,
    /// enabling support for unlimited chains without code changes.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use csv_adapter::prelude::*;
    ///
    /// let registry = ChainRegistry::new();
    /// registry.register_chain("bitcoin".to_string(), "Bitcoin".to_string());
    /// registry.register_chain("ethereum".to_string(), "Ethereum".to_string());
    ///
    /// let client = CsvClient::scalable_builder()
    ///     .with_chain_registry(registry)
    ///     .with_chain("bitcoin")
    ///     .with_chain("ethereum")
    ///     .build()?;
    /// # Ok::<_, csv_adapter::CsvError>(())
    /// ```
    pub fn scalable_builder() -> ScalableClientBuilder {
        ScalableClientBuilder::new()
    }

    /// Get a [`RightsManager`] for creating, querying, and managing Rights.
    pub fn rights(&self) -> RightsManager {
        RightsManager::new(Arc::new(self.clone_ref()))
    }

    /// Get a [`TransferManager`] for cross-chain transfer operations.
    pub fn transfers(&self) -> TransferManager {
        TransferManager::new(Arc::new(self.clone_ref()))
    }

    /// Get a [`ProofManager`] for generating and verifying proofs.
    pub fn proofs(&self) -> ProofManager {
        ProofManager::new(Arc::new(self.clone_ref()))
    }

    /// Get a [`WalletManager`] for wallet operations.
    ///
    /// # Errors
    ///
    /// Returns an error if no wallet was attached to the client.
    pub fn wallet(&self) -> Result<WalletManager, CsvError> {
        self.wallet
            .as_ref()
            .map(|w| WalletManager::new(w.clone()))
            .ok_or_else(|| {
                CsvError::BuilderError(
                    "No wallet attached. Use .with_wallet() when building the client.".to_string(),
                )
            })
    }

    /// Get a [`DeploymentManager`] for deploying CSV contracts.
    ///
    /// Provides a unified interface for deploying CSV seal contracts
    /// across all supported blockchains using their respective SDKs.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use csv_adapter::prelude::*;
    ///
    /// # async fn example() -> Result<()> {
    /// let client = CsvClient::builder()
    ///     .with_chain(Chain::Ethereum)
    ///     .build()?;
    ///
    /// // Deploy a CSV Lock contract on Ethereum
    /// let deployment = client.deploy()
    ///     .deploy_csv_lock("https://rpc.example.com", "0x...", &bytecode)
    ///     .await?;
    ///
    /// println!("Deployed at: {:?}", deployment.address);
    /// # Ok(())
    /// # }
    /// ```
    pub fn deploy(&self) -> DeploymentManager {
        DeploymentManager::new(Arc::new(self.clone_ref()))
    }

    /// Get an [`EventStream`] for watching CSV events.
    ///
    /// Returns a stream that receives events emitted by this client
    /// and its managers.
    pub fn watch(&self) -> EventStream {
        EventStream::new(self.event_tx.subscribe())
    }

    /// Check if a specific chain is enabled.
    pub fn is_chain_enabled(&self, chain: Chain) -> bool {
        self.enabled_chains.contains(&chain)
    }

    /// Get the set of enabled chains.
    pub fn enabled_chains(&self) -> &HashSet<Chain> {
        &self.enabled_chains
    }

    /// Get a reference to the attached wallet, if any.
    pub fn wallet_ref(&self) -> Option<&Wallet> {
        self.wallet.as_ref()
    }

    /// Get a reference to the configuration.
    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Emit an event to all event stream subscribers.
    #[allow(dead_code)]
    pub(crate) fn emit_event(&self, event: crate::events::Event) {
        // Best-effort: ignore if no receivers
        let _ = self.event_tx.send(event);
    }

    // Internal: create a cheap clone reference for managers
    fn clone_ref(&self) -> ClientRef {
        ClientRef {
            enabled_chains: self.enabled_chains.clone(),
            wallet: self.wallet.clone(),
            store: Arc::clone(&self.store),
            config: self.config.clone(),
            event_tx: self.event_tx.clone(),
            chain_registry: self.chain_registry.clone(),
        }
    }
}

/// A shareable reference to the client's state, used by managers.
///
/// This is an internal type that allows managers to hold a reference
/// to the client without the full `CsvClient` struct.
#[allow(dead_code)]
pub(crate) struct ClientRef {
    pub(crate) enabled_chains: HashSet<Chain>,
    #[allow(dead_code)]
    pub(crate) wallet: Option<Wallet>,
    #[allow(dead_code)]
    pub(crate) store: Arc<std::sync::Mutex<StoreHandle>>,
    #[allow(dead_code)]
    pub(crate) config: Config,
    pub(crate) event_tx: broadcast::Sender<crate::events::Event>,
    #[allow(dead_code)]
    pub(crate) chain_registry: Option<Arc<ChainRegistry>>,
}

impl ClientRef {
    pub(crate) fn is_chain_enabled(&self, chain: Chain) -> bool {
        self.enabled_chains.contains(&chain)
    }

    pub(crate) fn emit_event(&self, event: crate::events::Event) {
        let _ = self.event_tx.send(event);
    }
}
