//! Seals list page.

use crate::context::{use_wallet_context, SealRecord};
use crate::pages::common::*;
use crate::routes::Route;
use csv_adapter_core::Chain;
use dioxus::prelude::*;

// ===== Seals Pages =====
#[component]
pub fn Seals() -> Element {
    let wallet_ctx = use_wallet_context();
    let seals = wallet_ctx.seals();
    let mut filter_chain = use_signal(|| Option::<Chain>::None);

    let filtered = match *filter_chain.read() {
        Some(c) => seals
            .iter()
            .filter(|s| s.chain == c)
            .cloned()
            .collect::<Vec<_>>(),
        None => seals,
    };

    rsx! {
        div { class: "space-y-6",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold", "Seals" }
                Link { to: Route::CreateSeal {}, class: "{btn_primary_class()}", "+ Create Seal" }
            }

            // Filter bar
            div { class: "flex items-center gap-2 flex-wrap",
                span { class: "text-sm text-gray-400", "Filter:" }
                button {
                    onclick: move |_| filter_chain.set(None),
                    class: if filter_chain.read().is_none() { "{btn_primary_class()}" } else { "{btn_secondary_class()}" },
                    "All"
                }
                for chain in [Chain::Bitcoin, Chain::Ethereum, Chain::Sui, Chain::Aptos, Chain::Solana] {
                    button {
                        onclick: move |_| filter_chain.set(Some(chain)),
                        class: if matches!(*filter_chain.read(), Some(c) if c == chain) { "{chain_badge_class(&chain)} cursor-pointer" } else { "{chain_badge_class(&chain)} opacity-50 cursor-pointer" },
                        "{chain_icon_emoji(&chain)} {chain_name(&chain)}"
                    }
                }
            }

            if filtered.is_empty() {
                {empty_state("\u{1F512}", "No seals found", "Create a seal on a chain to get started.")}
            } else {
                div { class: "{table_class()}",
                    div { class: "{card_header_class()} flex items-center justify-between",
                        h2 { class: "font-semibold text-sm", "Consumed Seals" }
                        span { class: "text-xs text-gray-400", "{filtered.len()} total" }
                    }
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "text-left text-gray-400 border-b border-gray-800",
                                    th { class: "px-4 py-2 font-medium", "#" }
                                    th { class: "px-4 py-2 font-medium", "Seal Ref" }
                                    th { class: "px-4 py-2 font-medium", "Chain" }
                                    th { class: "px-4 py-2 font-medium", "Value" }
                                    th { class: "px-4 py-2 font-medium", "Status" }
                                }
                            }
                            tbody { class: "divide-y divide-gray-800",
                                for (i, seal) in filtered.iter().enumerate() {
                                    tr { key: "{seal.seal_ref}", class: "hover:bg-gray-800/50 transition-colors",
                                        td { class: "px-4 py-3 text-gray-400", "{i + 1}" }
                                        td { class: "px-4 py-3 font-mono text-xs", "{truncate_address(&seal.seal_ref, 12)}" }
                                        td { class: "px-4 py-3", span { class: "{chain_badge_class(&seal.chain)}", "{chain_icon_emoji(&seal.chain)} {chain_name(&seal.chain)}" } }
                                        td { class: "px-4 py-3 font-mono text-xs", "{seal.value}" }
                                        td { class: "px-4 py-3",
                                            span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium",
                                                class: if seal.consumed { "text-gray-400 bg-gray-500/20" } else { "text-green-400 bg-green-500/20" },
                                                if seal.consumed { "Consumed" } else { "Available" }
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
