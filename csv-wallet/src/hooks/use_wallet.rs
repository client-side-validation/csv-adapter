//! Wallet state hook.

use dioxus::prelude::*;
use csv_adapter::wallet::Wallet;

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
pub struct WalletContext {
    pub state: Signal<WalletState>,
}

impl WalletContext {
    pub fn create_wallet(&mut self) -> Result<Wallet, String> {
        let wallet = Wallet::generate();
        self.state.write().wallet = Some(wallet.clone());
        self.state.write().unlocked = true;
        self.state.write().initialized = true;
        
        // Populate addresses
        use csv_adapter_core::Chain;
        for chain in [Chain::Bitcoin, Chain::Ethereum, Chain::Sui, Chain::Aptos] {
            self.state.write().addresses.insert(chain, wallet.address(chain));
        }
        
        Ok(wallet)
    }

    pub fn import_wallet(&mut self, mnemonic: &str) -> Result<Wallet, String> {
        let wallet = Wallet::from_mnemonic(mnemonic, "")
            .map_err(|e| format!("Failed to import wallet: {}", e))?;
        self.state.write().wallet = Some(wallet.clone());
        self.state.write().unlocked = true;
        self.state.write().initialized = true;
        
        use csv_adapter_core::Chain;
        for chain in [Chain::Bitcoin, Chain::Ethereum, Chain::Sui, Chain::Aptos] {
            self.state.write().addresses.insert(chain, wallet.address(chain));
        }
        
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
pub fn WalletProvider() -> Element {
    let mut state = use_signal(|| WalletState {
        initialized: false,
        unlocked: false,
        wallet: None,
        addresses: std::collections::HashMap::new(),
    });

    use_context_provider(|| WalletContext { state });
    
    rsx! {
        Outlet {}
    }
}

/// Hook to access wallet state.
pub fn use_wallet() -> WalletContext {
    use_context::<WalletContext>().unwrap()
}
