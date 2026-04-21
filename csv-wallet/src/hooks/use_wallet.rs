//! Wallet state hook.

use dioxus::prelude::*;
use crate::wallet_core::WalletData as Wallet;

/// Wallet state.
#[derive(Clone, PartialEq)]
pub struct WalletState {
    /// Whether wallet is initialized
    pub initialized: bool,
    /// Whether wallet is unlocked
    pub unlocked: bool,
    /// Current wallet
    pub wallet: Option<Wallet>,
    /// Wallet addresses
    pub addresses: std::collections::HashMap<csv_adapter_core::Chain, String>,
}

/// Wallet context.
#[derive(Clone, Copy)]
pub struct WalletContext {
    pub state: Signal<WalletState>,
}

impl WalletContext {
    pub fn create_wallet(&mut self) -> Result<Wallet, String> {
        // TODO: Implement wallet generation
        // For now, just create empty wallet
        let wallet = Wallet::default();
        self.state.write().wallet = Some(wallet.clone());
        self.state.write().unlocked = true;
        self.state.write().initialized = true;
        
        Ok(wallet)
    }

    pub fn import_wallet(&mut self, _mnemonic: &str) -> Result<Wallet, String> {
        // TODO: Implement mnemonic import
        let wallet = Wallet::default();
        self.state.write().wallet = Some(wallet.clone());
        self.state.write().unlocked = true;
        self.state.write().initialized = true;
        
        Ok(wallet)
    }

    pub fn lock(&mut self) {
        self.state.write().unlocked = false;
    }

    pub fn unlock(&mut self, password: &str) -> Result<(), String> {
        // In production, decrypt wallet with password
        let _ = password;
        self.state.write().unlocked = true;
        Ok(())
    }
}

/// Wallet provider component.
#[component]
pub fn WalletProvider(children: Element) -> Element {
    let mut state = use_signal(|| WalletState {
        initialized: false,
        unlocked: false,
        wallet: None,
        addresses: std::collections::HashMap::new(),
    });

    use_context_provider(|| WalletContext { state });
    
    rsx! { { children } }
}

/// Hook to access wallet state.
pub fn use_wallet() -> WalletContext {
    use_context::<WalletContext>()
}
