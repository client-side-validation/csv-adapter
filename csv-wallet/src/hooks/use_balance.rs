//! Balance fetching hook.

use dioxus::prelude::*;
use dioxus_router::components::Outlet;
use csv_adapter_core::Chain;
use std::collections::HashMap;

/// Balance state for an account.
#[derive(Clone, Debug, PartialEq)]
pub struct AccountBalance {
    pub account_id: String,
    pub chain: Chain,
    pub address: String,
    pub balance: f64,
    pub loading: bool,
    pub error: Option<String>,
}

/// Balance context.
#[derive(Clone, Copy)]
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

    /// Get total balance for a chain.
    pub fn chain_total(&self, chain: Chain) -> f64 {
        self.balances
            .read()
            .values()
            .filter(|b| b.chain == chain)
            .map(|b| b.balance)
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
pub fn BalanceProvider() -> Element {
    let balances = use_signal(|| HashMap::<String, AccountBalance>::new());

    use_context_provider(|| BalanceContext { balances });

    rsx! {
        Outlet {}
    }
}

/// Hook to access balance context.
pub fn use_balance() -> BalanceContext {
    use_context::<BalanceContext>().expect("BalanceContext not found")
}

/// Format balance for display with appropriate precision.
pub fn format_balance(balance: f64, chain: Chain) -> String {
    match chain {
        Chain::Bitcoin => format!("{:.8} BTC", balance),
        Chain::Ethereum => format!("{:.6} ETH", balance),
        Chain::Sui => format!("{:.4} SUI", balance),
        Chain::Aptos => format!("{:.4} APT", balance),
        Chain::Solana => format!("{:.4} SOL", balance),
        _ => format!("{:.4}", balance),
    }
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
