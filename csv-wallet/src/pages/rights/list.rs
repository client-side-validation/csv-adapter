//! Sanads list page.

use crate::context::{use_wallet_context, ProofStatus, SealStatus};
use crate::pages::common::*;
use crate::routes::Route;
use csv_core::Chain;
use dioxus::prelude::*;

#[component]
pub fn Sanads() -> Element {
    let wallet_ctx = use_wallet_context();
    let sanads = wallet_ctx.sanads();
    let mut filter_chain = use_signal(|| Option::<Chain>::None);

    let filtered = match *filter_chain.read() {
        Some(c) => sanads
            .iter()
            .filter(|r| r.chain == c)
            .cloned()
            .collect::<Vec<_>>(),
        None => sanads,
    };

    rsx! {
        div { class: "space-y-6",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold", "Sanads" }
                Link { to: Route::CreateSanad {}, class: "{btn_primary_class()}", "+ Create Sanad" }
            }

            // Filter bar
            div { class: "flex items-center gap-2",
                span { class: "text-sm text-gray-400", "Filter:" }
                button {
                    onclick: move |_| filter_chain.set(None),
                    class: if filter_chain.read().is_none() { "{btn_primary_class()}" } else { "{btn_secondary_class()}" },
                    "All"
                }
                for chain in [Chain::Bitcoin, Chain::Ethereum, Chain::Sui, Chain::Aptos, Chain::Solana] {
                    button {
                        key: "sanad-filter-{chain:?}",
                        onclick: move |_| filter_chain.set(Some(chain)),
                        class: if matches!(*filter_chain.read(), Some(c) if c == chain) { "{chain_badge_class(&chain)} cursor-pointer" } else { "{chain_badge_class(&chain)} opacity-50 cursor-pointer" },
                        "{chain_icon_emoji(&chain)} {chain_name(&chain)}"
                    }
                }
            }

            if filtered.is_empty() {
                {empty_state("\u{1F48E}", "No Sanads found", "Create a Sanad to get started.")}
            } else {
                div { class: "{table_class()}",
                    div { class: "{card_header_class()} flex items-center justify-between",
                        h2 { class: "font-semibold text-sm", "Tracked Sanads" }
                        span { class: "text-xs text-gray-400", "{filtered.len()} total" }
                    }
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "text-left text-gray-400 border-b border-gray-800",
                                    th { class: "px-4 py-2 font-medium", "Sanad ID" }
                                    th { class: "px-4 py-2 font-medium", "Chain" }
                                    th { class: "px-4 py-2 font-medium", "Value" }
                                    th { class: "px-4 py-2 font-medium", "Status" }
                                    th { class: "px-4 py-2 font-medium", "Seal/Proof" }
                                    th { class: "px-4 py-2 font-medium", "Actions" }
                                }
                            }
                            tbody { class: "divide-y divide-gray-800",
                                for (idx, sanad) in filtered.iter().enumerate() {
                                    {
                                        let seal = wallet_ctx.seal_for_sanad(&sanad.id);
                                        let proofs = wallet_ctx.proofs_for_sanad(&sanad.id);
                                        let verified_count = proofs.iter().filter(|p| p.status == ProofStatus::Verified).count();
                                        rsx! {
                                            tr { key: "{idx}-{sanad.id}", class: "hover:bg-gray-800/50 transition-colors",
                                                td { class: "px-4 py-3 font-mono text-xs text-gray-300", "{truncate_address(&sanad.id, 8)}" }
                                                td { class: "px-4 py-3", span { class: "{chain_badge_class(&sanad.chain)}", "{chain_icon_emoji(&sanad.chain)} {chain_name(&sanad.chain)}" } }
                                                td { class: "px-4 py-3 font-mono text-xs", "{sanad.value}" }
                                                td { class: "px-4 py-3",
                                                    span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {sanad_status_class(&sanad.status)}",
                                                        "{sanad.status}"
                                                    }
                                                }
                                                td { class: "px-4 py-3",
                                                    div { class: "flex gap-1",
                                                        // Seal indicator
                                                        if let Some(ref s) = seal {
                                                            span { class: "inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium {seal_status_badge_class(&s.status)}",
                                                                "\u{1F512}"
                                                            }
                                                        }
                                                        // Proof indicator
                                                        if !proofs.is_empty() {
                                                            if verified_count > 0 {
                                                                span { class: "inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium text-green-400 bg-green-500/20",
                                                                    "\u{1F4C4} {verified_count}"
                                                                }
                                                            } else {
                                                                span { class: "inline-flex items-center px-1.5 py-0.5 rounded text-[10px] font-medium text-yellow-400 bg-yellow-500/20",
                                                                    "\u{1F4C4} {proofs.len()}"
                                                                }
                                                            }
                                                        }
                                                    }
                                                }
                                                td { class: "px-4 py-3 flex gap-2",
                                                    Link { to: Route::SanadJourney { id: sanad.id.clone() }, class: "text-purple-400 hover:text-purple-300 text-xs font-medium", "Journey" }
                                                    Link { to: Route::ShowSanad { id: sanad.id.clone() }, class: "text-blue-400 hover:text-blue-300 text-xs", "View" }
                                                    button {
                                                        onclick: {
                                                            let mut wallet_ctx = wallet_ctx.clone();
                                                            let sanad_id = sanad.id.clone();
                                                            move |_| {
                                                                wallet_ctx.remove_sanad(&sanad_id);
                                                            }
                                                        },
                                                        class: "text-red-400 hover:text-red-300 text-xs",
                                                        "Remove"
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
    }
}

fn seal_status_badge_class(status: &SealStatus) -> &'static str {
    match status {
        SealStatus::Active => "text-yellow-400 bg-yellow-500/20",
        SealStatus::Locked => "text-orange-400 bg-orange-500/20",
        SealStatus::Consumed => "text-gray-400 bg-gray-500/20",
        SealStatus::Transferred => "text-green-400 bg-green-500/20",
    }
}
