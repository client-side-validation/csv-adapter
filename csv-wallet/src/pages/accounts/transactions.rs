//! Account-specific transactions page.

use crate::context::use_wallet_context;
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

#[component]
pub fn AccountTransactions(id: String) -> Element {
    let wallet_ctx = use_wallet_context();

    // Find the account by ID
    let account = wallet_ctx.accounts().into_iter().find(|a| a.id == id);

    let Some(account) = account else {
        return rsx! {
            div { class: "space-y-6",
                div { class: "flex items-center gap-3",
                    Link { to: Route::Dashboard {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                    h1 { class: "text-xl font-bold", "Account Not Found" }
                }
                div { class: "{card_class()} p-6",
                    p { class: "text-gray-400", "The requested account could not be found." }
                }
            }
        };
    };

    // Get all transactions for this account
    let all_transactions = wallet_ctx.transactions();
    let account_transactions: Vec<_> = all_transactions
        .into_iter()
        .filter(|tx| {
            tx.from_address == account.address || tx.to_address.as_ref() == Some(&account.address)
        })
        .collect();

    let chain_name = match account.chain {
        csv_core::Chain::Bitcoin => "Bitcoin",
        csv_core::Chain::Ethereum => "Ethereum",
        csv_core::Chain::Sui => "Sui",
        csv_core::Chain::Aptos => "Aptos",
        csv_core::Chain::Solana => "Solana",
        _ => "Unknown",
    };

    rsx! {
        div { class: "space-y-6",
            // Header
            div { class: "flex items-center gap-3",
                Link { to: Route::Dashboard {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Account Transactions" }
            }

            // Account Info Card
            div { class: "{card_class()} p-6",
                div { class: "flex items-center gap-3 mb-4",
                    span { class: "{chain_badge_class(&account.chain)}",
                        "{chain_icon_emoji(&account.chain)} {chain_name}"
                    }
                }
                div { class: "space-y-2",
                    div {
                        p { class: "text-xs text-gray-400", "Address" }
                        p { class: "font-mono text-sm text-gray-200 break-all", "{account.address}" }
                    }
                    div {
                        p { class: "text-xs text-gray-400", "Account ID" }
                        p { class: "font-mono text-sm text-gray-300", "{truncate_address(&account.id, 8)}" }
                    }
                }
            }

            // Transactions List
            div { class: "{card_class()} p-6",
                h2 { class: "text-lg font-semibold mb-4",
                    "Transactions ({account_transactions.len()})"
                }

                if account_transactions.is_empty() {
                    div { class: "text-center py-8",
                        p { class: "text-gray-400", "No transactions found for this account." }
                    }
                } else {
                    div { class: "space-y-3",
                        for tx in account_transactions {
                            Link {
                                key: "{tx.id}",
                                to: Route::TransactionDetail { id: tx.id.clone() },
                                class: "block p-4 bg-gray-800/50 rounded-lg hover:bg-gray-800 transition-colors",
                                div { class: "flex items-center justify-between",
                                    div { class: "flex items-center gap-3",
                                        span { class: "{chain_badge_class(&tx.chain)}",
                                            "{chain_icon_emoji(&tx.chain)}"
                                        }
                                        div {
                                            p { class: "font-medium text-sm",
                                                "{tx.tx_type.to_string()}"
                                            }
                                            p { class: "text-xs text-gray-400 font-mono",
                                                "{truncate_address(&tx.tx_hash, 8)}"
                                            }
                                        }
                                    }
                                    div { class: "text-right",
                                        p { class: "text-sm text-gray-300",
                                            {tx.amount.map_or("-".to_string(), |a| format!("{} {}", a, chain_name))}
                                        }
                                        p { class: "text-xs text-gray-500",
                                            "{format_timestamp(tx.created_at)}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
