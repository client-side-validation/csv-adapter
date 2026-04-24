//! Transfer right page.

use crate::context::{use_wallet_context, RightStatus};
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

#[component]
pub fn TransferRight() -> Element {
    let wallet_ctx = use_wallet_context();
    let rights = wallet_ctx.rights();
    // Filter only active rights
    let active_rights: Vec<_> = rights.into_iter().filter(|r| r.status == RightStatus::Active).collect();
    let has_active_rights = !active_rights.is_empty();
    // Clone for use in closure
    let active_rights_for_closure = active_rights.clone();
    
    let mut selected_right_index = use_signal(|| 0usize);
    let mut to_address = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Rights {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Transfer Right" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Available Rights", rsx! {
                    if active_rights.is_empty() {
                        p { class: "text-sm text-red-400", "No active rights available. Create a right first." }
                    } else {
                        select {
                            class: "{input_mono_class()}",
                            onchange: move |evt| {
                                if let Ok(idx) = evt.value().parse::<usize>() {
                                    selected_right_index.set(idx);
                                }
                            },
                            for (idx, right) in active_rights.iter().enumerate() {
                                option { key: "{idx}", value: idx.to_string(), selected: idx == *selected_right_index.read(),
                                    {format!("{} - {} - Value: {} - {}",
                                        &right.id[..12.min(right.id.len())],
                                        chain_name(&right.chain),
                                        right.value,
                                        right.status
                                    )}
                                }
                            }
                        }
                    }
                })}
                
                // Show selected right details
                if let Some(right) = active_rights.get(*selected_right_index.read()) {
                    div { class: "bg-gray-800/50 rounded-lg p-3 border border-gray-700",
                        p { class: "text-xs text-gray-400 mb-2", "Selected Right Details:" }
                        div { class: "grid grid-cols-2 gap-2 text-xs",
                            div { span { class: "text-gray-500", "ID: " }, span { class: "font-mono text-gray-300", "{truncate_address(&right.id, 10)}" } }
                            div { span { class: "text-gray-500", "Chain: " }, span { class: "{chain_badge_class(&right.chain)}", "{chain_icon_emoji(&right.chain)} {chain_name(&right.chain)}" } }
                            div { span { class: "text-gray-500", "Value: " }, span { class: "font-mono text-gray-300", "{right.value}" } }
                            div { span { class: "text-gray-500", "Status: " }, span { class: "{right_status_class(&right.status)}", "{right.status}" } }
                            div { span { class: "text-gray-500", "Owner: " }, span { class: "font-mono text-gray-300", "{truncate_address(&right.owner, 8)}" } }
                        }
                    }
                }

                {form_field("New Owner Address", rsx! {
                    input {
                        value: "{to_address.read()}",
                        oninput: move |evt| { to_address.set(evt.value()); },
                        class: "{input_mono_class()}",
                        placeholder: "Recipient address"
                    }
                })}

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        if let Some(right) = active_rights_for_closure.get(*selected_right_index.read()) {
                            result.set(Some(format!("Transfer initiated for Right {} on {}. For cross-chain transfers, use the Cross-Chain section.", 
                                truncate_address(&right.id, 8),
                                chain_name(&right.chain)
                            )));
                        }
                    },
                    disabled: !has_active_rights,
                    class: if !has_active_rights { "{btn_full_primary_class()} opacity-50 cursor-not-allowed" } else { "{btn_full_primary_class()}" },
                    if !has_active_rights { "No Rights Available" } else { "Transfer" }
                }
            }
        }
    }
}
