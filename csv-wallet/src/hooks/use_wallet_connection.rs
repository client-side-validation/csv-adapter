//! Browser wallet connection hook.

use crate::services::blockchain::{wallet_connection, BrowserWallet, WalletType};
use csv_core::Chain;
use dioxus::prelude::*;

/// Wallet connection state.
#[derive(Clone, Debug, PartialEq)]
pub struct WalletConnectionState {
    pub wallet: Option<BrowserWallet>,
    pub connecting: bool,
    pub error: Option<String>,
    pub chain: Chain,
}

impl Default for WalletConnectionState {
    fn default() -> Self {
        Self {
            wallet: None,
            connecting: false,
            error: None,
            chain: Chain::Ethereum,
        }
    }
}

/// Wallet connection context.
#[derive(Clone)]
pub struct WalletConnectionContext {
    state: Signal<WalletConnectionState>,
}

impl WalletConnectionContext {
    pub fn wallet(&self) -> Option<BrowserWallet> {
        self.state.read().wallet.clone()
    }

    pub fn is_connected(&self) -> bool {
        self.state.read().wallet.is_some()
    }

    pub fn is_connecting(&self) -> bool {
        self.state.read().connecting
    }

    pub fn error(&self) -> Option<String> {
        self.state.read().error.clone()
    }

    pub fn clear_error(&mut self) {
        self.state.write().error = None;
    }

    /// Connect to MetaMask (Ethereum).
    pub async fn connect_metamask(&mut self) {
        let mut state = self.state.write();
        state.connecting = true;
        state.error = None;
        drop(state);

        if !wallet_connection::is_metamask_installed() {
            self.state.write().error =
                Some("MetaMask not installed. Please install it from metamask.io".to_string());
            self.state.write().connecting = false;
            return;
        }

        // Real implementation would call eth_requestAccounts via JS interop
        // For now, assume success
        match wallet_connection::connect_metamask().await {
            Ok(wallet) => {
                let mut state = self.state.write();
                state.wallet = Some(wallet);
                state.chain = Chain::Ethereum;
                state.connecting = false;
            }
            Err(e) => {
                let mut state = self.state.write();
                state.error = Some(e.message);
                state.connecting = false;
            }
        }
    }

    /// Connect to Phantom (Solana).
    pub async fn connect_phantom(&mut self) {
        let mut state = self.state.write();
        state.connecting = true;
        state.error = None;
        drop(state);

        if !wallet_connection::is_phantom_installed() {
            self.state.write().error =
                Some("Phantom not installed. Please install it from phantom.app".to_string());
            self.state.write().connecting = false;
            return;
        }

        // Real implementation would call phantom.solana.connect()
        let wallet = BrowserWallet {
            chain: Chain::Solana,
            address: String::new(), // Would come from wallet
            wallet_type: WalletType::Phantom,
        };

        let mut state = self.state.write();
        state.wallet = Some(wallet);
        state.chain = Chain::Solana;
        state.connecting = false;
    }

    /// Disconnect wallet.
    pub fn disconnect(&mut self) {
        self.state.write().wallet = None;
        self.state.write().error = None;
    }

    /// Get recommended wallet type for a chain.
    pub fn recommended_wallet(&self, chain: Chain) -> WalletType {
        wallet_connection::recommended_wallet(chain)
    }

    /// Check if wallet is installed.
    pub fn is_wallet_installed(&self, wallet_type: &WalletType) -> bool {
        match wallet_type {
            WalletType::MetaMask => wallet_connection::is_metamask_installed(),
            WalletType::Phantom => wallet_connection::is_phantom_installed(),
            _ => false,
        }
    }
}

/// Wallet connection provider component.
#[component]
pub fn WalletConnectionProvider(children: Element) -> Element {
    let state = use_signal(WalletConnectionState::default);

    use_context_provider(|| WalletConnectionContext { state });

    rsx! { { children } }
}

/// Hook to access wallet connection context.
pub fn use_wallet_connection() -> WalletConnectionContext {
    use_context::<WalletConnectionContext>()
}

/// Component to show wallet connection button.
#[component]
pub fn WalletConnectButton(chain: Chain) -> Element {
    let mut wallet_ctx = use_wallet_connection();
    let state = wallet_ctx.state;

    let wallet_type = wallet_ctx.recommended_wallet(chain);
    let is_installed = wallet_ctx.is_wallet_installed(&wallet_type);
    let is_connected = wallet_ctx.is_connected();
    let connecting = wallet_ctx.is_connecting();

    let button_text = if connecting {
        "Connecting...".to_string()
    } else if is_connected {
        format!(
            "Connected ({}",
            state
                .read()
                .wallet
                .as_ref()
                .map(|w| truncate_address(&w.address, 4))
                .unwrap_or_default()
        )
    } else {
        format!("Connect {}", wallet_type_name(&wallet_type))
    };

    let button_class = if is_connected {
        "px-4 py-2 bg-green-600 hover:bg-green-700 rounded-lg text-sm font-medium transition-colors"
    } else if !is_installed {
        "px-4 py-2 bg-gray-600 rounded-lg text-sm font-medium opacity-50 cursor-not-allowed"
    } else {
        "px-4 py-2 bg-blue-600 hover:bg-blue-700 rounded-lg text-sm font-medium transition-colors"
    };

    rsx! {
        div { class: "space-y-2",
            button {
                onclick: move |_| {
                    if is_connected {
                        wallet_ctx.disconnect();
                    } else if is_installed {
                        match chain {
                            Chain::Ethereum => {
                                let mut ctx = wallet_ctx.clone();
                                spawn(async move {
                                    ctx.connect_metamask().await;
                                });
                            }
                            Chain::Solana => {
                                let mut ctx = wallet_ctx.clone();
                                spawn(async move {
                                    ctx.connect_phantom().await;
                                });
                            }
                            _ => {}
                        }
                    }
                },
                disabled: connecting || (!is_installed && !is_connected),
                class: "{button_class}",
                "{button_text}"
            }

            if let Some(error) = state.read().error.clone() {
                div { class: "text-xs text-red-400", "{error}" }
            }

            if !is_installed && !is_connected {
                div { class: "text-xs text-gray-500",
                    "Install {wallet_type_name(&wallet_type)} to continue"
                }
            }
        }
    }
}

fn wallet_type_name(wallet_type: &WalletType) -> String {
    match wallet_type {
        WalletType::MetaMask => "MetaMask".to_string(),
        WalletType::Phantom => "Phantom".to_string(),
        WalletType::SuiWallet => "Sui Wallet".to_string(),
        WalletType::Petra => "Petra".to_string(),
        WalletType::Leather => "Leather".to_string(),
        WalletType::Native => "Native".to_string(),
        WalletType::Custom(s) => s.clone(),
    }
}

fn truncate_address(addr: &str, chars: usize) -> String {
    if addr.len() <= chars * 2 + 3 {
        addr.to_string()
    } else {
        format!("{}...{}", &addr[..chars + 2], &addr[addr.len() - chars..])
    }
}
