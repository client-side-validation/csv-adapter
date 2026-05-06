//! Transaction card component.

use crate::context::{use_wallet_context, TransactionRecord, TransactionStatus};
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

#[component]
pub fn TransactionCard(transaction: TransactionRecord) -> Element {
    let wallet_ctx = use_wallet_context();
    let _tx_clone = transaction.clone();

    let status_class = match transaction.status {
        TransactionStatus::Confirmed => "text-green-400 bg-green-500/20",
        TransactionStatus::Pending => "text-yellow-400 bg-yellow-500/20",
        TransactionStatus::Failed => "text-red-400 bg-red-500/20",
    };

    let explorer_url = wallet_ctx.get_explorer_url(transaction.chain, &transaction.tx_hash);

    rsx! {
        Link {
            to: Route::TransactionDetail { id: transaction.id.clone() },
            class: "{card_class()} p-4 block card-hover",
            div { class: "flex items-center justify-between",
                div { class: "flex items-center gap-3",
                    span { class: "{chain_badge_class(&transaction.chain)}",
                        "{chain_icon_emoji(&transaction.chain)}"
                    }
                    div {
                        p { class: "font-medium text-sm", "{transaction.tx_type.to_string()}" }
                        p { class: "text-xs text-gray-500",
                            {format!("{} → {}",
                                truncate_address(&transaction.from_address, 6),
                                transaction.to_address.as_ref().map_or("?".to_string(), |a| truncate_address(a, 6))
                            )}
                        }
                    }
                }
                div { class: "text-sanad",
                    span { class: "inline-flex items-center px-2 py-1 rounded text-xs font-medium {status_class}",
                        "{transaction.status.to_string()}"
                    }
                    if let Some(block) = transaction.block_number {
                        p { class: "text-xs text-gray-500 mt-1", "Block {block}" }
                    }
                }
            }
            if let Some(url) = explorer_url {
                div { class: "mt-3 pt-3 border-t border-gray-800 flex items-center justify-between",
                    span { class: "text-xs text-gray-500", "{truncate_address(&transaction.tx_hash, 8)}" }
                    a {
                        href: "{url}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        class: "text-xs text-blue-400 hover:text-blue-300 flex items-center gap-1",
                        onclick: |e| e.stop_propagation(),
                        "View on Explorer \u{2197}"
                    }
                }
            }
        }
    }
}

// ===== Transaction Detail Page =====
