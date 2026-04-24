//! Transaction detail page.

use crate::context::use_wallet_context;
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

#[component]
pub fn TransactionDetail(id: String) -> Element {
    let wallet_ctx = use_wallet_context();
    let transaction = wallet_ctx.transaction_by_id(&id);

    if let Some(tx) = transaction {
        let explorer_url = wallet_ctx.get_explorer_url(tx.chain, &tx.tx_hash);
        
        let status_class = match tx.status {
            crate::context::TransactionStatus::Confirmed => "text-green-400 bg-green-500/20",
            crate::context::TransactionStatus::Pending => "text-yellow-400 bg-yellow-500/20",
            crate::context::TransactionStatus::Failed => "text-red-400 bg-red-500/20",
        };

        rsx! {
            div { class: "space-y-6",
                div { class: "flex items-center justify-between",
                    h1 { class: "text-2xl font-bold", "Transaction Details" }
                    Link { to: Route::Transactions {}, class: "text-sm text-blue-400 hover:text-blue-300", "\u{2190} Back" }
                }

                div { class: "{card_class()} p-6 space-y-6",
                    div { class: "flex items-center justify-between pb-6 border-b border-gray-800",
                        div { class: "flex items-center gap-3",
                            span { class: "{chain_badge_class(&tx.chain)}",
                                "{chain_icon_emoji(&tx.chain)} {chain_name(&tx.chain)}"
                            }
                            span { class: "px-3 py-1 rounded-full text-sm font-medium {status_class}",
                                "{tx.status.to_string()}"
                            }
                        }
                        if let Some(url) = explorer_url.clone() {
                            a {
                                href: "{url}",
                                target: "_blank",
                                rel: "noopener noreferrer",
                                class: "text-sm text-blue-400 hover:text-blue-300 flex items-center gap-1",
                                "View on Explorer \u{2197}"
                            }
                        }
                    }

                    div { class: "grid grid-cols-2 gap-4",
                        div {
                            p { class: "text-xs text-gray-500 mb-1", "Transaction Type" }
                            p { class: "text-sm font-medium", "{tx.tx_type.to_string()}" }
                        }
                        div {
                            p { class: "text-xs text-gray-500 mb-1", "Transaction Hash" }
                            p { class: "font-mono text-sm text-gray-300", "{truncate_address(&tx.tx_hash, 16)}" }
                        }
                        div {
                            p { class: "text-xs text-gray-500 mb-1", "From" }
                            p { class: "font-mono text-sm text-gray-300", "{truncate_address(&tx.from_address, 12)}" }
                        }
                        if let Some(to) = tx.to_address.as_ref() {
                            div {
                                p { class: "text-xs text-gray-500 mb-1", "To" }
                                p { class: "font-mono text-sm text-gray-300", "{truncate_address(to, 12)}" }
                            }
                        }
                        div {
                            p { class: "text-xs text-gray-500 mb-1", "Value" }
                            p { class: "text-sm font-medium", "{tx.amount.unwrap_or(0)} {chain_name(&tx.chain)}" }
                        }
                        div {
                            p { class: "text-xs text-gray-500 mb-1", "Time" }
                            p { class: "text-sm text-gray-300", "{format_timestamp(tx.created_at)}" }
                        }
                    }
                }
            }
        }
    } else {
        rsx! {
            div { class: "space-y-6",
                h1 { class: "text-2xl font-bold", "Transaction Details" }
                div { class: "{card_class()} p-6",
                    p { class: "text-gray-400", "Transaction not found." }
                    Link { to: Route::Transactions {}, class: "text-blue-400 hover:text-blue-300 text-sm", "\u{2190} Back to Transactions" }
                }
            }
        }
    }
}
