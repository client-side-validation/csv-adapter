//! Consume seal page.

use crate::context::{use_wallet_context, SealRecord};
use crate::pages::common::*;
use crate::routes::Route;
use csv_adapter_core::Chain;
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn ConsumeSeal(seal_ref: Option<String>) -> Element {
    let mut wallet_ctx = use_wallet_context();
    let seals = wallet_ctx.seals();

    // Get available (unconsumed) seals
    let available_seals: Vec<_> = seals.iter().filter(|s| !s.consumed).cloned().collect();

    // Initialize selected seal from URL parameter or first available
    let initial_seal_ref = seal_ref.clone().or_else(|| available_seals.first().map(|s| s.seal_ref.clone()));
    let mut selected_seal_ref = use_signal(|| initial_seal_ref.unwrap_or_default());

    // Get the currently selected seal by computing it from the signal
    let selected_seal_ref_read = selected_seal_ref.read();
    let selected_seal: Option<SealRecord> = seals.iter()
        .find(|s| s.seal_ref == *selected_seal_ref_read)
        .cloned();

    let mut result = use_signal(|| Option::<String>::None);
    let mut error = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Seals {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Consume Seal" }
            }

            div { class: "bg-yellow-900/30 border border-yellow-700/50 rounded-xl p-4",
                div { class: "flex items-center gap-2",
                    span { class: "text-yellow-400", "\u{26A0}\u{FE0F}" }
                    p { class: "text-yellow-300 font-medium", "Warning: Seal consumption is irreversible" }
                }
            }

            div { class: "{card_class()} p-6 space-y-5",
                if available_seals.is_empty() {
                    div { class: "bg-gray-800/50 rounded-lg p-4 text-center",
                        p { class: "text-gray-400", "No available seals to consume." }
                        p { class: "text-sm text-gray-500 mt-2", "Create a seal first from the Seals page." }
                    }
                } else {
                    {form_field("Select Seal", rsx! {
                        select {
                            class: "{select_class()}",
                            value: "{selected_seal_ref.read()}",
                            onchange: move |evt| {
                                selected_seal_ref.set(evt.value());
                                error.set(None);
                                result.set(None);
                            },
                            for seal in available_seals.iter() {
                                option {
                                    key: "{seal.seal_ref}",
                                    value: "{seal.seal_ref}",
                                    selected: seal.seal_ref == *selected_seal_ref.read(),
                                    "{chain_icon_emoji(&seal.chain)} {chain_name(&seal.chain)} - {truncate_address(&seal.seal_ref, 16)} (Value: {seal.value})"
                                }
                            }
                        }
                    })}

                    // Show selected seal details
                    {match selected_seal {
                        Some(seal) => rsx! {
                            div { class: "bg-gray-800/50 rounded-lg p-3 border border-gray-700",
                                p { class: "text-xs text-gray-400 mb-2", "Selected Seal Details:" }
                                div { class: "grid grid-cols-2 gap-2 text-xs",
                                    div { span { class: "text-gray-500", "Full Ref: " }, span { class: "font-mono text-gray-300 break-all", "{seal.seal_ref}" } }
                                    div { span { class: "text-gray-500", "Chain: " }, span { class: "{chain_badge_class(&seal.chain)}", "{chain_icon_emoji(&seal.chain)} {chain_name(&seal.chain)}" } }
                                    div { span { class: "text-gray-500", "Value: " }, span { class: "font-mono text-gray-300", "{seal.value}" } }
                                    div { span { class: "text-gray-500", "Status: " }, span { class: "text-green-300", "Available" } }
                                }
                            }
                        },
                        None => rsx! {}
                    }}
                }

                if let Some(e) = error.read().as_ref() {
                    div { class: "p-3 bg-red-900/30 border border-red-700/50 rounded-lg text-sm text-red-300", "{e}" }
                }

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        let seal_ref_val = selected_seal_ref.read().clone();
                        if seal_ref_val.is_empty() {
                            error.set(Some("Please select a seal to consume.".to_string()));
                            return;
                        }
                        if wallet_ctx.is_seal_consumed(&seal_ref_val) {
                            error.set(Some("Seal replay detected: this seal has already been consumed.".to_string()));
                        } else {
                            wallet_ctx.consume_seal(&seal_ref_val);
                            result.set(Some(format!("Seal {} consumed successfully.", truncate_address(&seal_ref_val, 12))));
                            // Refresh the list by clearing selection
                            selected_seal_ref.set(String::new());
                        }
                    },
                    disabled: available_seals.is_empty() || selected_seal_ref.read().is_empty(),
                    class: "w-full px-4 py-2.5 rounded-lg bg-red-600 hover:bg-red-700 text-sm font-medium transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
                    if available_seals.is_empty() { "No Available Seals" } else { "Consume Seal" }
                }
            }
        }
    }
}

