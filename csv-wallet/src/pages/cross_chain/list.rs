//! Cross-chain transfers list page.

use crate::context::use_wallet_context;
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

#[component]
pub fn CrossChain() -> Element {
    let wallet_ctx = use_wallet_context();
    let transfers = wallet_ctx.transfers();

    rsx! {
        div { class: "space-y-6",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold", "Cross-Chain Transfers" }
                Link { to: Route::CrossChainTransfer {}, class: "{btn_primary_class()}", "+ New Transfer" }
            }

            if transfers.is_empty() {
                {empty_state("\u{21C4}", "No transfers recorded", "Start a cross-chain transfer to move Rights between chains.")}
            } else {
                div { class: "{table_class()}",
                    div { class: "{card_header_class()} flex items-center justify-between",
                        h2 { class: "font-semibold text-sm", "Transfers" }
                        span { class: "text-xs text-gray-400", "{transfers.len()} total" }
                    }
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "text-left text-gray-400 border-b border-gray-800",
                                    th { class: "px-4 py-2 font-medium", "Transfer ID" }
                                    th { class: "px-4 py-2 font-medium", "From" }
                                    th { class: "px-4 py-2 font-medium", "To" }
                                    th { class: "px-4 py-2 font-medium", "Right ID" }
                                    th { class: "px-4 py-2 font-medium", "Status" }
                                }
                            }
                            tbody { class: "divide-y divide-gray-800",
                                for (idx, t) in transfers.iter().enumerate() {
                                    tr { key: "{idx}-{t.id}", class: "hover:bg-gray-800/50 transition-colors",
                                        td { class: "px-4 py-3 font-mono text-xs", "{truncate_address(&t.id, 6)}" }
                                        td { class: "px-4 py-3", span { class: "{chain_badge_class(&t.from_chain)}", "{chain_icon_emoji(&t.from_chain)}" } }
                                        td { class: "px-4 py-3", span { class: "{chain_badge_class(&t.to_chain)}", "{chain_icon_emoji(&t.to_chain)}" } }
                                        td { class: "px-4 py-3 font-mono text-xs", "{truncate_address(&t.right_id, 8)}" }
                                        td { class: "px-4 py-3",
                                            span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {transfer_status_class(&t.status)}",
                                                "{t.status}"
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
}
