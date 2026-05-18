//! Blockchain service for wallet operations.
//!
//! Provides blockchain operations through csv-runtime.
//! Supports both native and browser wallet contexts.

use csv_core::ChainId;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen::JsCast;

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

/// Blockchain service using csv-runtime.
pub struct BlockchainService;

impl std::fmt::Debug for BlockchainService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BlockchainService").finish()
    }
}

impl BlockchainService {
    /// Create a new blockchain service.
    pub fn new(_config: BlockchainConfig) -> Self {
        Self
    }
}

impl Clone for BlockchainService {
    fn clone(&self) -> Self {
        Self
    }
}

/// Blockchain configuration.
#[derive(Debug, Clone, Default)]
pub struct BlockchainConfig {
    _private: (),
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
