//! Account management and dashboard pages.

use crate::context::types::{RightStatus, TransferStatus};
use crate::context::use_wallet_context;
use crate::hooks::{format_balance_display, AccountBalance};
use crate::pages::common::*;
use crate::routes::Route;
pub use csv_adapter_core::Chain;
use dioxus::prelude::*;
use std::collections::HashMap;

pub mod transactions;

pub use transactions::AccountTransactions;

#[component]
pub fn Dashboard() -> Element {
    let wallet_ctx = use_wallet_context();
    let accounts = wallet_ctx.accounts();
    let rights = wallet_ctx.rights();
    let transfers = wallet_ctx.transfers();
    let has_wallet = wallet_ctx.is_initialized();

    // Balance state for all accounts
    let mut account_balances = use_signal(HashMap::<String, AccountBalance>::new);
    let mut balances_loading = use_signal(|| false);

    // Fetch balances for all accounts when dashboard loads
    use_effect({
        let accounts_to_fetch = accounts.clone();
        move || {
            if accounts_to_fetch.is_empty() {
                return;
            }

            balances_loading.set(true);
            let accounts_to_fetch = accounts_to_fetch.clone();

            spawn(async move {
                use crate::services::chain_api::ChainApi;
                let api = ChainApi::default();
                let mut balances = HashMap::new();

                for account in accounts_to_fetch {
                    let balance_result = api.get_balance(account.chain, &account.address).await;

                    let balance_raw = match balance_result {
                        Ok(b) => b,
                        Err(_) => 0u64,
                    };
                    let error = balance_result.err().map(|e| e.to_string());

                    let balance_data = AccountBalance {
                        account_id: account.id.clone(),
                        chain: account.chain,
                        address: account.address.clone(),
                        balance_raw,
                        loading: false,
                        error,
                    };

                    balances.insert(account.id.clone(), balance_data);
                }

                account_balances.set(balances);
                balances_loading.set(false);
            });
        }
    });

    if !has_wallet {
        return rsx! {
            div { class: "flex items-center justify-center min-h-[calc(100vh-8rem)]",
                div { class: "relative z-10 w-full max-w-lg mx-4",
                    div { class: "{card_class()} p-8 space-y-6",
                        div { class: "text-center space-y-2",
                            div { class: "text-5xl mb-2 inline-block", "\u{1F510}" }
                            h2 { class: "text-2xl font-bold", "CSV Wallet" }
                            p { class: "text-gray-400 text-sm", "Manage accounts per-chain." }
                        }
                        p { class: "text-center text-gray-500", "Use the Wallet page to add accounts" }
                    }
                }
            }
        };
    }

    let active_rights = rights
        .iter()
        .filter(|r| r.status == RightStatus::Active)
        .count();
    let completed_transfers = transfers
        .iter()
        .filter(|t| t.status == TransferStatus::Completed)
        .count();

    rsx! {
        div { class: "space-y-6",
            h1 { class: "text-2xl font-bold", "Dashboard" }

            // Stats row
            div { class: "grid grid-cols-2 lg:grid-cols-4 gap-4",
                {stat_card("Accounts", &accounts.len().to_string(), "\u{1F4B3}")}
                {stat_card("Active Rights", &active_rights.to_string(), "\u{1F48E}")}
                {stat_card("Transfers", &completed_transfers.to_string(), "\u{21C4}")}
                {stat_card("Network", "Testnet", "\u{1F310}")}
            }

            // Chain Addresses Section with Balances
            if !accounts.is_empty() {
                div { class: "{card_class()} p-5",
                    div { class: "flex items-center justify-between mb-4",
                        h2 { class: "text-lg font-semibold", "Your Accounts" }
                        if *balances_loading.read() {
                            span { class: "text-xs text-gray-400 animate-pulse", "Loading balances..." }
                        }
                    }
                    div { class: "space-y-3",
                        for account in accounts {
                            div { key: "{account.id}", class: "flex items-center justify-between p-3 bg-gray-800/50 rounded-lg",
                                div { class: "flex items-center gap-3 flex-1",
                                    span { class: "{chain_badge_class(&account.chain)}",
                                        "{chain_icon_emoji(&account.chain)} {chain_name(&account.chain)}"
                                    }
                                    div { class: "flex-1",
                                        p { class: "font-mono text-sm text-gray-300", "{truncate_address(&account.address, 12)}" }
                                        // Display balance
                                        if let Some(balance_data) = account_balances.read().get(&account.id) {
                                            if balance_data.loading {
                                                p { class: "text-xs text-gray-500", "Loading..." }
                                            } else if let Some(ref error) = balance_data.error {
                                                p { class: "text-xs text-red-400", "Error fetching balance" }
                                                // Show detailed error for Bitcoin debugging
                                                if account.chain == Chain::Bitcoin {
                                                    p { class: "text-[10px] text-gray-500 max-w-[200px] truncate", "{error}" }
                                                }
                                            } else if balance_data.balance_raw > 0 {
                                                p { class: "text-xs text-green-400 font-medium",
                                                    "{format_balance_display(balance_data.balance_raw, account.chain)}"
                                                }
                                            } else {
                                                p { class: "text-xs text-yellow-400 font-medium",
                                                    "{format_balance_display(balance_data.balance_raw, account.chain)} (No funds)"
                                                }
                                            }
                                        } else {
                                            p { class: "text-xs text-gray-500", "Balance: -" }
                                        }
                                    }
                                }
                                Link { to: Route::AccountTransactions { id: account.id.clone() }, class: "text-xs text-blue-400 hover:text-blue-300 ml-4",
                                    "View Transactions \u{2192}"
                                }
                            }
                        }
                    }
                }
            }

            // Quick Actions
            div { class: "grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-4 gap-4",
                Link { to: Route::CreateRight {}, class: "{card_class()} p-5 block",
                    div { class: "flex items-center gap-3",
                        span { class: "text-2xl", "\u{1F48E}" },
                        h3 { class: "font-semibold text-sm", "Create Right" }
                    }
                }
                Link { to: Route::CrossChainTransfer {}, class: "{card_class()} p-5 block",
                    div { class: "flex items-center gap-3",
                        span { class: "text-2xl", "\u{21C4}" },
                        h3 { class: "font-semibold text-sm", "Cross-Chain" }
                    }
                }
                Link { to: Route::GenerateProof {}, class: "{card_class()} p-5 block",
                    div { class: "flex items-center gap-3",
                        span { class: "text-2xl", "\u{1F4C4}" },
                        h3 { class: "font-semibold text-sm", "Generate Proof" }
                    }
                }
                Link { to: Route::CreateSeal {}, class: "{card_class()} p-5 block",
                    div { class: "flex items-center gap-3",
                        span { class: "text-2xl", "\u{1F512}" },
                        h3 { class: "font-semibold text-sm", "Create Seal" }
                    }
                }
            }
        }
    }
}

fn stat_card(label: &str, value: &str, icon: &str) -> Element {
    rsx! {
        div { class: "{card_class()} p-5",
            div { class: "flex items-center justify-between",
                div {
                    p { class: "text-xs text-gray-400", "{label}" }
                    p { class: "text-xl font-bold", "{value}" }
                }
                span { class: "text-2xl", "{icon}" }
            }
        }
    }
}
