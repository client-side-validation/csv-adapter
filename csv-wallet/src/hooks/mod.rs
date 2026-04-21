//! Dioxus hooks for state management - STUB IMPLEMENTATIONS.

use dioxus::prelude::*;
use csv_adapter_core::Chain;

// ===== use_balance stub =====
#[derive(Clone)]
pub struct AccountBalance {
    pub account_id: String,
    pub chain: Chain,
    pub address: String,
    pub balance: f64,
    pub loading: bool,
    pub error: Option<String>,
}

#[derive(Clone, Copy)]
pub struct BalanceContext;

impl BalanceContext {
    pub fn get_balance(&self, _account_id: &str) -> Option<AccountBalance> { None }
    pub fn set_balance(&mut self, _account_id: String, _balance: AccountBalance) {}
    pub fn chain_total(&self, _chain: Chain) -> f64 { 0.0 }
}

pub fn use_balance() -> BalanceContext {
    BalanceContext
}

pub fn format_balance(balance: f64, chain: Chain) -> String {
    match chain {
        Chain::Bitcoin => format!("{:.8} BTC", balance),
        Chain::Ethereum => format!("{:.6} ETH", balance),
        Chain::Sui => format!("{:.9} SUI", balance),
        Chain::Aptos => format!("{:.9} APT", balance),
        Chain::Solana => format!("{:.9} SOL", balance),
        _ => format!("{:.6} UNK", balance),
    }
}

// ===== use_wallet_connection stub =====
#[derive(Clone, Copy)]
pub struct WalletConnectionContext;

impl WalletConnectionContext {
    pub fn is_connected(&self) -> bool { false }
    pub fn connect(&mut self, _chain: Chain) {}
    pub fn disconnect(&mut self) {}
    pub fn wallet(&self) -> Option<()> { None }
}

pub fn use_wallet_connection() -> WalletConnectionContext {
    WalletConnectionContext
}

#[component]
pub fn WalletConnectButton(chain: Chain) -> Element {
    rsx! {
        button {
            class: "px-4 py-2 bg-gray-700 text-gray-300 rounded-lg text-sm opacity-50 cursor-not-allowed",
            disabled: true,
            "Wallet Connect (Coming Soon)"
        }
    }
}
