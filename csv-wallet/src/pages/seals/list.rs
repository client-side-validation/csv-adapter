//! Seals list page.

use crate::context::{use_wallet_context, SealRecord, SealStatus};
use crate::pages::common::*;
use crate::routes::Route;
use csv_adapter_core::Chain;
use dioxus::prelude::*;

// ===== Seals Pages =====
#[component]
pub fn Seals() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let seals = wallet_ctx.seals();
    let mut filter_chain = use_signal(|| Option::<Chain>::None);
    let mut selected_seal = use_signal(|| None::<SealRecord>);
    let mut show_delete_confirm = use_signal(|| None::<SealRecord>);

    // Collect seals into owned vector for use in closures
    let seals_owned: Vec<_> = seals.into_iter().collect();

    let filtered: Vec<_> = match *filter_chain.read() {
        Some(c) => seals_owned
            .iter()
            .filter(|s| s.chain == c)
            .cloned()
            .collect(),
        None => seals_owned.clone(),
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
                    key: "all",
                    onclick: move |_| filter_chain.set(None),
                    class: if filter_chain.read().is_none() { "{btn_primary_class()}" } else { "{btn_secondary_class()}" },
                    "All"
                }
                for chain in [Chain::Bitcoin, Chain::Ethereum, Chain::Sui, Chain::Aptos, Chain::Solana] {
                    button {
                        key: "seal-filter-{chain:?}",
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
                        h2 { class: "font-semibold text-sm", "Seals" }
                        span { class: "text-xs text-gray-400", "{filtered.len()} total" }
                    }
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "text-left text-gray-400 border-b border-gray-800",
                                    th { class: "px-4 py-2 font-medium", "#" }
                                    th { class: "px-4 py-2 font-medium", "Seal Ref" }
                                    th { class: "px-4 py-2 font-medium", "Chain" }
                                    th { class: "px-4 py-2 font-medium", "Protects Right" }
                                    th { class: "px-4 py-2 font-medium", "Value" }
                                    th { class: "px-4 py-2 font-medium", "Status" }
                                    th { class: "px-4 py-2 font-medium", "Actions" }
                                }
                            }
                            tbody { class: "divide-y divide-gray-800",
                                for (i, seal) in filtered.iter().enumerate() {
                                    let status_class = match seal.status {
                                        SealStatus::Active => "text-yellow-400 bg-yellow-500/20",
                                        SealStatus::Locked => "text-orange-400 bg-orange-500/20",
                                        SealStatus::Consumed => "text-gray-400 bg-gray-500/20",
                                        SealStatus::Transferred => "text-green-400 bg-green-500/20",
                                    };
                                    tr { key: "seal-row-{i}", class: "hover:bg-gray-800/50 transition-colors",
                                        td { class: "px-4 py-3 text-gray-400", "{i + 1}" }
                                        td { class: "px-4 py-3 font-mono text-xs", "{truncate_address(&seal.seal_ref, 12)}" }
                                        td { class: "px-4 py-3", span { class: "{chain_badge_class(&seal.chain)}", "{chain_icon_emoji(&seal.chain)} {chain_name(&seal.chain)}" } }
                                        td { class: "px-4 py-3 font-mono text-xs",
                                            Link { to: Route::RightJourney { id: seal.right_id.clone() }, class: "text-purple-400 hover:text-purple-300",
                                                "{truncate_address(&seal.right_id, 8)}"
                                            }
                                        }
                                        td { class: "px-4 py-3 font-mono text-xs", "{seal.value}" }
                                        td { class: "px-4 py-3",
                                            span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {status_class}",
                                                "{seal.status}"
                                            }
                                        }
                                        td { class: "px-4 py-3",
                                            div { class: "flex gap-2",
                                                {
                                                    let seal_for_view = seal.clone();
                                                    rsx! {
                                                        button {
                                                            onclick: move |_| selected_seal.set(Some(seal_for_view.clone())),
                                                            class: "px-2 py-1 rounded text-xs bg-blue-900/30 text-blue-400 hover:bg-blue-900/50 transition-colors",
                                                            "View"
                                                        }
                                                    }
                                                }
                                                if seal.status != SealStatus::Consumed && seal.status != SealStatus::Transferred {
                                                    {
                                                        let seal_ref_clone = seal.seal_ref.clone();
                                                        rsx! {
                                                            Link {
                                                                to: Route::ConsumeSeal { seal_ref: Some(seal_ref_clone) },
                                                                class: "px-2 py-1 rounded text-xs bg-orange-900/30 text-orange-400 hover:bg-orange-900/50 transition-colors",
                                                                "Consume"
                                                            }
                                                        }
                                                    }
                                                }
                                                {
                                                    let seal_for_delete = seal.clone();
                                                    rsx! {
                                                        button {
                                                            onclick: move |_| show_delete_confirm.set(Some(seal_for_delete.clone())),
                                                            class: "px-2 py-1 rounded text-xs bg-red-900/30 text-red-400 hover:bg-red-900/50 transition-colors",
                                                            "Delete"
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

            // Seal Detail Modal
            {
                let seal_opt = selected_seal.read().clone();
                let mut close_modal = selected_seal.clone();
                match seal_opt {
                    Some(seal) => rsx! {
                        div { class: "fixed inset-0 bg-black/50 flex items-center justify-center z-50",
                            div { class: "{card_class()} max-w-lg w-full mx-4",
                                div { class: "{card_header_class()} flex items-center justify-between",
                                    h3 { class: "font-semibold", "Seal Details" }
                                    button { onclick: move |_| close_modal.set(None), class: "text-gray-400 hover:text-gray-200", "\u{2715}" }
                                }
                                div { class: "p-4 space-y-4",
                                    div { class: "space-y-2",
                                        p { class: "text-sm text-gray-400", "Seal Reference" }
                                        p { class: "text-sm font-mono break-all", "{seal.seal_ref}" }
                                    }
                                    div { class: "space-y-2",
                                        p { class: "text-sm text-gray-400", "Chain" }
                                        p { class: "text-sm", span { class: "{chain_badge_class(&seal.chain)}", "{chain_icon_emoji(&seal.chain)} {chain_name(&seal.chain)}" } }
                                    }
                                    div { class: "space-y-2",
                                        p { class: "text-sm text-gray-400", "Value" }
                                        p { class: "text-sm font-mono", "{seal.value}" }
                                    }
                                    div { class: "space-y-2",
                                        p { class: "text-sm text-gray-400", "Status" }
                                        p { class: "text-sm",
                                            span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {seal_status_class(&seal.status)}",
                                                "{seal.status}"
                                            }
                                        }
                                    }
                                    div { class: "space-y-2",
                                        p { class: "text-sm text-gray-400", "Protects Right" }
                                        p { class: "text-sm font-mono break-all",
                                            Link { to: Route::RightJourney { id: seal.right_id.clone() }, class: "text-purple-400 hover:text-purple-300",
                                                "{&seal.right_id}"
                                            }
                                        }
                                    }
                                    if seal.created_at > 0 {
                                        div { class: "space-y-2",
                                            p { class: "text-sm text-gray-400", "Created" }
                                            p { class: "text-sm", "{format_timestamp(seal.created_at)}" }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    None => rsx! {}
                }
            }

            // Delete Confirmation Modal
            {
                let seal_opt = show_delete_confirm.read().clone();
                let mut close_modal = show_delete_confirm.clone();
                let mut ctx = wallet_ctx.clone();
                match seal_opt {
                    Some(seal) => rsx! {
                        div { class: "fixed inset-0 bg-black/50 flex items-center justify-center z-50",
                            div { class: "{card_class()} max-w-md w-full mx-4",
                                div { class: "p-6 space-y-4",
                                    div { class: "flex items-center gap-3",
                                        span { class: "text-2xl", "\u{26A0}\u{FE0F}" }
                                        h3 { class: "font-semibold text-lg", "Delete Seal?" }
                                    }
                                    p { class: "text-sm text-gray-400",
                                        "Are you sure you want to delete this seal? This action cannot be undone."
                                    }
                                    div { class: "bg-gray-800/50 rounded-lg p-3",
                                        p { class: "text-xs text-gray-500", "Seal Ref: {truncate_address(&seal.seal_ref, 20)}" }
                                        p { class: "text-xs text-gray-500", "Chain: {chain_name(&seal.chain)}" }
                                        p { class: "text-xs text-gray-500", "Status: {seal.status}" }
                                        p { class: "text-xs text-gray-500", "Right: {truncate_address(&seal.right_id, 12)}" }
                                    }
                                    div { class: "flex gap-3",
                                        button {
                                            onclick: move |_| close_modal.set(None),
                                            class: "flex-1 px-4 py-2 rounded-lg bg-gray-800 hover:bg-gray-700 text-sm font-medium transition-colors",
                                            "Cancel"
                                        }
                                        button {
                                            onclick: move |_| {
                                                ctx.remove_seal(&seal.seal_ref);
                                                close_modal.set(None);
                                            },
                                            class: "flex-1 px-4 py-2 rounded-lg bg-red-600 hover:bg-red-700 text-sm font-medium transition-colors",
                                            "Delete"
                                        }
                                    }
                                }
                            }
                        }
                    },
                    None => rsx! {}
                }
            }
        }
    }
}
