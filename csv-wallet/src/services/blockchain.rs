//! Blockchain service for wallet operations.
//!
//! Provides blockchain operations by delegating to csv-sdk.
//! Supports both native and browser wallet contexts.

use csv_sdk::{client::NetworkType, CsvClient};
use csv_core::{ChainId, SanadId};

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

#[cfg(feature = "cross-chain-persist")]
use csv_sdk::cross_chain::PersistentTransferRegistry;
#[cfg(feature = "cross-chain-persist")]
use sqlx::SqlitePool;

/// Blockchain error type.
#[derive(Debug, Clone)]
pub struct BlockchainError {
    pub message: String,
    pub chain: Option<ChainId>,
    pub code: Option<u32>,
}

impl std::fmt::Display for BlockchainError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "BlockchainError: {}", self.message)
    }
}

impl std::error::Error for BlockchainError {}

impl From<csv_sdk::CsvError> for BlockchainError {
    fn from(err: csv_sdk::CsvError) -> Self {
        Self {
            message: err.to_string(),
            chain: None,
            code: None,
        }
    }
}

/// Wallet type enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalletType {
    MetaMask,
    Phantom,
    Petra,
    Leather,
    Native,
    Custom,
    SuiWallet,
    AptosWallet,
    SolanaWallet,
}

/// Native wallet.
#[derive(Debug, Clone)]
pub struct NativeWallet {
    pub address: String,
}

impl NativeWallet {
    /// Create a new native wallet.
    pub fn new(address: String) -> Self {
        Self { address }
    }

    /// Get the wallet address.
    pub fn address(&self) -> &str {
        &self.address
    }
}

/// Browser wallet.
#[derive(Debug, Clone, PartialEq)]
pub struct BrowserWallet {
    pub address: String,
    pub chain: Option<ChainId>,
    pub wallet_type: WalletType,
}

/// Contract type enum.
#[derive(Debug, Clone, Copy)]
pub enum ContractType {
    Registry,
    Bridge,
    Lock,
}

/// Contract deployment info.
#[derive(Debug, Clone)]
pub struct ContractDeployment {
    pub address: String,
    pub tx_hash: String,
    pub chain: Option<ChainId>,
    pub contract_address: String,
    pub contract_type: ContractType,
    pub deployed_at: u64,
}

/// Blockchain service using csv-sdk.
pub struct BlockchainService {
    client: CsvClient,
    #[cfg(feature = "cross-chain-persist")]
    transfer_registry: Option<PersistentTransferRegistry>,
}

impl std::fmt::Debug for BlockchainService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockchainService")
            .field("client", &"<CsvClient>")
            .finish()
    }
}

impl BlockchainService {
    /// Create a new blockchain service.
    /// 
    /// Note: This uses InMemory storage for the CSV client's internal operations.
    /// For sensitive wallet data (mnemonics, private keys), the wallet should use
    /// csv-store's EncryptedStorageManager separately via the wallet context.
    pub fn new(_config: BlockchainConfig) -> Self {
        // Initialize csv-sdk client with default configuration
        let client = CsvClient::builder()
            .with_store_backend(csv_sdk::builder::StoreBackend::InMemory)
            .build()
            .expect("Failed to create CSV client");

        Self {
            client,
            #[cfg(feature = "cross-chain-persist")]
            transfer_registry: None,
        }
    }

    /// Create a new blockchain service with persistent transfer registry.
    /// 
    /// Note: This uses InMemory storage for the CSV client's internal operations.
    /// For sensitive wallet data (mnemonics, private keys), the wallet should use
    /// csv-store's EncryptedStorageManager separately via the wallet context.
    #[cfg(feature = "cross-chain-persist")]
    pub fn with_registry(_config: BlockchainConfig, pool: SqlitePool) -> Self {
        let client = CsvClient::builder()
            .with_store_backend(csv_sdk::builder::StoreBackend::InMemory)
            .build()
            .expect("Failed to create CSV client");

        Self {
            client,
            transfer_registry: Some(PersistentTransferRegistry::new(pool)),
        }
    }

}

impl Clone for BlockchainService {
    fn clone(&self) -> Self {
        // Clone the client reference by re-creating a client with the same enabled chains.
        // The runtime is stateless and the client builder is cheap for wallet contexts.
        let client = CsvClient::builder()
            .with_store_backend(csv_sdk::builder::StoreBackend::InMemory)
            .build()
            .expect("Failed to create CSV client");

        Self {
            client,
            #[cfg(feature = "cross-chain-persist")]
            transfer_registry: None,
        }
    }
}

/// Blockchain configuration.
#[derive(Debug, Clone, Default)]
pub struct BlockchainConfig {
    _private: (),
}

/// Transfer result returned by wallet transfer helpers.
#[derive(Debug, Clone)]
pub struct TransferResult {
    pub transfer_id: String,
    pub source_fee: Option<u64>,
    pub dest_fee: Option<u64>,
    pub lock_tx_hash: Option<String>,
    pub mint_tx_hash: Option<String>,
}

impl BlockchainService {
    /// Execute a cross-chain transfer through the CSV SDK transfer manager.
    pub async fn execute_cross_chain_transfer(
        &self,
        sanad_id: SanadId,
        from_chain: ChainId,
        to_chain: ChainId,
        destination_address: String,
    ) -> Result<TransferResult, BlockchainError> {
        #[cfg(feature = "cross-chain-persist")]
        if let Some(registry) = &self.transfer_registry {
            // Double-spend check: verify sanad hasn't been transferred already
            let sanad_id_str = hex::encode(sanad_id.0.as_bytes());
            if registry
                .is_transferred(&sanad_id_str)
                .await
                .map_err(|e| BlockchainError {
                    message: format!("Failed to check transfer status: {}", e),
                    chain: Some(from_chain.clone()),
                    code: None,
                })?
            {
                return Err(BlockchainError {
                    message: format!("Sanad {} has already been transferred", sanad_id_str),
                    chain: Some(from_chain),
                    code: Some(400),
                });
            }
        }

        self.client
            .init_adapters(NetworkType::Testnet)
            .await
            .map_err(BlockchainError::from)?;

        let transfer_id = self.client
            .transfers()
            .cross_chain(sanad_id.clone(), to_chain.clone())
            .from_chain(from_chain.clone())
            .to_address(destination_address)
            .execute()
            .await
            .map_err(BlockchainError::from)?;

        let details = self.client
            .transfers()
            .details(&transfer_id)
            .map_err(BlockchainError::from)?;

        #[cfg(feature = "cross-chain-persist")]
        if let Some(registry) = &self.transfer_registry {
            // Record the completed transfer for double-spend protection
            let sanad_id_str = hex::encode(sanad_id.0.as_bytes());
            let from_chain_str = from_chain.as_str();
            let to_chain_str = to_chain.as_str();
            let lock_tx = details.lock_tx_hash.as_deref().unwrap_or("");
            
            registry
                .record_transfer(
                    &sanad_id_str,
                    from_chain_str,
                    to_chain_str,
                    lock_tx,
                    None, // mint_tx not available yet
                    "wallet", // from_owner placeholder
                    "wallet", // to_owner placeholder
                )
                .await
                .map_err(|e| BlockchainError {
                    message: format!("Failed to record transfer: {}", e),
                    chain: Some(from_chain),
                    code: None,
                })?;
        }

        Ok(TransferResult {
            transfer_id,
            source_fee: None,
            dest_fee: None,
            lock_tx_hash: details.lock_tx_hash,
            mint_tx_hash: None,
        })
    }

    /// Execute a local transfer on the same source and destination chain.
    pub async fn transfer_sanad_local(
        &self,
        sanad_id: SanadId,
        chain: ChainId,
        destination_address: String,
    ) -> Result<TransferResult, BlockchainError> {
        self.execute_cross_chain_transfer(sanad_id, chain.clone(), chain, destination_address)
            .await
    }
}

/// Wallet connection utilities.
pub mod wallet_connection {
    use super::{ChainId, NativeWallet, WalletType};
    use wasm_bindgen::JsCast;
    use wasm_bindgen::JsValue;
    use wasm_bindgen_futures::JsFuture;
    use web_sys::window;

    /// Get recommended wallet type for a chain.
    pub fn recommended_wallet(_chain: ChainId) -> WalletType {
        WalletType::MetaMask
    }

    /// Check if MetaMask is installed.
    pub fn is_metamask_installed() -> bool {
        if let Some(window) = window() {
            if let Ok(ethereum) = js_sys::Reflect::get(&window, &JsValue::from_str("ethereum")) {
                return !ethereum.is_undefined();
            }
        }
        false
    }

    /// Check if Phantom is installed.
    pub fn is_phantom_installed() -> bool {
        if let Some(window) = window() {
            if let Ok(phantom) = js_sys::Reflect::get(&window, &JsValue::from_str("phantom")) {
                return !phantom.is_undefined();
            }
        }
        false
    }

    /// Connect to MetaMask using window.ethereum.request({ method: 'eth_requestAccounts' }).
    #[cfg(target_arch = "wasm32")]
    pub async fn connect_metamask() -> Result<NativeWallet, String> {
        let window = window().ok_or("Window not available")?;

        let ethereum = js_sys::Reflect::get(&window, &JsValue::from_str("ethereum"))
            .map_err(|_| "MetaMask not available")?;

        if ethereum.is_undefined() {
            return Err("MetaMask not installed".to_string());
        }

        let request = js_sys::Reflect::get(&ethereum, &JsValue::from_str("request"))
            .map_err(|_| "MetaMask request method not available")?;

        let request_fn = request
            .dyn_ref::<js_sys::Function>()
            .ok_or("request is not a function")?;

        let request_params = js_sys::Object::new();
        js_sys::Reflect::set(
            &request_params,
            &JsValue::from_str("method"),
            &JsValue::from_str("eth_requestAccounts"),
        )
        .map_err(|_| "Failed to set request params")?;

        let promise_value = request_fn
            .call1(&ethereum, &request_params)
            .map_err(|_| "Failed to call request")?;

        let promise = promise_value.unchecked_into::<js_sys::Promise>();

        let result = JsFuture::from(promise)
            .await
            .map_err(|e| format!("Failed to connect to MetaMask: {:?}", e))?;

        let accounts = result
            .dyn_ref::<js_sys::Array>()
            .ok_or("Expected array of accounts")?;

        if accounts.length() == 0 {
            return Err("No accounts returned from MetaMask".to_string());
        }

        let account = accounts.get(0);
        let address = account
            .as_string()
            .ok_or("Account address is not a string")?;

        Ok(NativeWallet::new(address))
    }

    /// Connect to MetaMask (non-wasm stub).
    #[cfg(not(target_arch = "wasm32"))]
    pub async fn connect_metamask() -> Result<NativeWallet, String> {
        Err("MetaMask connection only available in WASM builds".to_string())
    }

    /// Create a native wallet from address.
    pub fn native_wallet(address: &str) -> NativeWallet {
        NativeWallet::new(address.to_string())
    }

    /// Check if wallet is installed.
    pub fn is_wallet_installed(wallet_type: &WalletType) -> bool {
        match wallet_type {
            WalletType::MetaMask => is_metamask_installed(),
            WalletType::Phantom => is_phantom_installed(),
            _ => false,
        }
    }
}
