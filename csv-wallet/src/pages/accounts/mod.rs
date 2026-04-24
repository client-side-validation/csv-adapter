//! Account management and dashboard pages.

use crate::context::types::{RightStatus, TransferStatus};
use crate::context::use_wallet_context;
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

pub mod transactions;

pub use transactions::AccountTransactions;

#[component]
pub fn Dashboard() -> Element {
    let wallet_ctx = use_wallet_context();
    let accounts = wallet_ctx.accounts();
    let rights = wallet_ctx.rights();
    let transfers = wallet_ctx.transfers();
    let has_wallet = wallet_ctx.is_initialized();

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

    let active_rights = rights.iter().filter(|r| r.status == RightStatus::Active).count();
    let completed_transfers = transfers.iter().filter(|t| t.status == TransferStatus::Completed).count();

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

            // Chain Addresses Section
            if !accounts.is_empty() {
                div { class: "{card_class()} p-5",
                    h2 { class: "text-lg font-semibold mb-4", "Your Addresses" }
                    div { class: "space-y-3",
                        for account in accounts {
                            div { key: "{account.id}", class: "flex items-center justify-between p-3 bg-gray-800/50 rounded-lg",
                                div { class: "flex items-center gap-3",
                                    span { class: "{chain_badge_class(&account.chain)}",
                                        "{chain_icon_emoji(&account.chain)} {chain_name(&account.chain)}"
                                    }
                                    p { class: "font-mono text-sm text-gray-300", "{truncate_address(&account.address, 12)}" }
                                }
                                Link { to: Route::AccountTransactions { id: account.id.clone() }, class: "text-xs text-blue-400 hover:text-blue-300",
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
