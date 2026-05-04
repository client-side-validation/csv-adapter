//! Balance fetching hook.

use csv_adapter_core::Chain;
use dioxus::prelude::*;
use std::collections::HashMap;

/// Balance state for an account.
#[derive(Clone, Debug, PartialEq)]
pub struct AccountBalance {
    pub account_id: String,
    pub chain: Chain,
    pub address: String,
    /// Balance in raw chain units (satoshis, lamports, MIST, octas, wei).
    /// Use `format_balance_display()` for human-readable display.
    pub balance_raw: u64,
    pub loading: bool,
    pub error: Option<String>,
}

/// Balance context.
#[derive(Clone)]
pub struct BalanceContext {
    balances: Signal<HashMap<String, AccountBalance>>,
}

impl BalanceContext {
    /// Get balance for a specific account.
    pub fn get_balance(&self, account_id: &str) -> Option<AccountBalance> {
        self.balances.read().get(account_id).cloned()
    }

    /// Get all balances.
    pub fn all_balances(&self) -> Vec<AccountBalance> {
        self.balances.read().values().cloned().collect()
    }

    /// Get total raw balance for a chain (in chain-native units).
    pub fn chain_total_raw(&self, chain: Chain) -> u64 {
        self.balances
            .read()
            .values()
            .filter(|b| b.chain == chain)
            .map(|b| b.balance_raw)
            .sum()
    }

    /// Set balance for an account.
    pub fn set_balance(&mut self, account_id: String, balance_data: AccountBalance) {
        self.balances.write().insert(account_id, balance_data);
    }

    /// Clear all balances.
    pub fn clear(&mut self) {
        self.balances.write().clear();
    }
}

/// Balance provider component.
#[component]
pub fn BalanceProvider(children: Element) -> Element {
    let balances = use_signal(HashMap::<String, AccountBalance>::new);

    use_context_provider(|| BalanceContext { balances });

    rsx! { { children } }
}

/// Hook to access balance context.
pub fn use_balance() -> BalanceContext {
    use_context::<BalanceContext>()
}

/// Format raw balance for display with appropriate precision.
/// Takes raw chain units (satoshis, lamports, etc.) and returns human-readable string.
pub fn format_balance_display(balance_raw: u64, chain: Chain) -> String {
    match chain {
        Chain::Bitcoin => {
            let btc = balance_raw as f64 / 100_000_000.0;
            format!("{:.8} BTC", btc)
        }
        Chain::Ethereum => {
            let eth = balance_raw as f64 / 1e18;
            format!("{:.6} ETH", eth)
        }
        Chain::Sui => {
            let sui = balance_raw as f64 / 1e9;
            format!("{:.4} SUI", sui)
        }
        Chain::Aptos => {
            let apt = balance_raw as f64 / 1e8;
            format!("{:.4} APT", apt)
        }
        Chain::Solana => {
            let sol = balance_raw as f64 / 1e9;
            format!("{:.4} SOL", sol)
        }
        _ => format!("{} units", balance_raw),
    }
}

/// Legacy alias for backward compatibility - prefer format_balance_display.
#[deprecated(since = "0.4.0", note = "Use format_balance_display with raw u64 instead")]
pub fn format_balance(balance: f64, chain: Chain) -> String {
    format_balance_display(balance as u64, chain)
}

/// Get chain symbol.
pub fn chain_symbol(chain: Chain) -> &'static str {
    match chain {
        Chain::Bitcoin => "BTC",
        Chain::Ethereum => "ETH",
        Chain::Sui => "SUI",
        Chain::Aptos => "APT",
        Chain::Solana => "SOL",
        _ => "",
    }
}
