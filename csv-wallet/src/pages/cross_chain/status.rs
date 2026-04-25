//! Cross-chain transfer status page.

use crate::context::{use_wallet_context, TrackedTransfer};
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

#[component]
pub fn CrossChainStatus() -> Element {
    let wallet_ctx = use_wallet_context();
    let transfers: Vec<_> = wallet_ctx.transfers();

    // Initialize with first transfer if available
    let initial_transfer_id = transfers.first().map(|t| t.id.clone()).unwrap_or_default();
    let mut selected_transfer_id = use_signal(|| initial_transfer_id);

    // Get the currently selected transfer by computing it from the signal
    let selected_transfer_id_read = selected_transfer_id.read();
    let selected_transfer: Option<TrackedTransfer> = transfers.iter()
        .find(|t| t.id == *selected_transfer_id_read)
        .cloned();

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::CrossChain {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Transfer Status" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                if transfers.is_empty() {
                    div { class: "bg-gray-800/50 rounded-lg p-4 text-center",
                        p { class: "text-gray-400", "No transfers found." }
                        p { class: "text-sm text-gray-500 mt-2", "Execute a cross-chain transfer first." }
                    }
                } else {
                    {form_field("Select Transfer", rsx! {
                        select {
                            class: "{select_class()}",
                            value: "{selected_transfer_id.read()}",
                            onchange: move |evt| {
                                selected_transfer_id.set(evt.value());
                            },
                            for transfer in transfers.iter() {
                                option {
                                    key: "{transfer.id}",
                                    value: "{transfer.id}",
                                    selected: transfer.id == *selected_transfer_id.read(),
                                    "{chain_icon_emoji(&transfer.from_chain)} {chain_name(&transfer.from_chain)} \u{2192} {chain_icon_emoji(&transfer.to_chain)} {chain_name(&transfer.to_chain)} - {truncate_address(&transfer.id, 12)}"
                                }
                            }
                        }
                    })}
                }

                // Display selected transfer details
                {match selected_transfer {
                    Some(t) => rsx! {
                        div { class: "space-y-3",
                            div { class: "bg-gray-800/50 rounded-lg p-4 border border-gray-700 space-y-3",
                                div { class: "flex justify-between",
                                    span { class: "text-sm text-gray-400", "Transfer ID" }
                                    span { class: "text-sm font-mono text-gray-300", "{truncate_address(&t.id, 16)}" }
                                }
                                div { class: "flex justify-between",
                                    span { class: "text-sm text-gray-400", "Source" }
                                    span { class: "{chain_badge_class(&t.from_chain)}", "{chain_icon_emoji(&t.from_chain)} {chain_name(&t.from_chain)}" }
                                }
                                div { class: "flex justify-between",
                                    span { class: "text-sm text-gray-400", "Destination" }
                                    span { class: "{chain_badge_class(&t.to_chain)}", "{chain_icon_emoji(&t.to_chain)} {chain_name(&t.to_chain)}" }
                                }
                                div { class: "flex justify-between",
                                    span { class: "text-sm text-gray-400", "Right ID" }
                                    span { class: "text-sm font-mono text-gray-300", "{truncate_address(&t.right_id, 12)}" }
                                }
                                div { class: "flex justify-between",
                                    span { class: "text-sm text-gray-400", "Destination Owner" }
                                    span { class: "text-sm font-mono text-gray-300", "{truncate_address(&t.dest_owner, 12)}" }
                                }
                                div { class: "flex justify-between",
                                    span { class: "text-sm text-gray-400", "Status" }
                                    span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {transfer_status_class(&t.status)}",
                                        "{t.status}"
                                    }
                                }
                                if t.created_at > 0 {
                                    div { class: "flex justify-between",
                                        span { class: "text-sm text-gray-400", "Created" }
                                        span { class: "text-sm text-gray-300", "{format_timestamp(t.created_at)}" }
                                    }
                                }
                                if let Some(ref source_tx) = t.source_tx_hash {
                                    div { class: "flex justify-between",
                                        span { class: "text-sm text-gray-400", "Source TX" }
                                        span { class: "text-sm font-mono text-blue-400", "{truncate_address(source_tx, 12)}" }
                                    }
                                }
                                if let Some(ref dest_tx) = t.dest_tx_hash {
                                    div { class: "flex justify-between",
                                        span { class: "text-sm text-gray-400", "Destination TX" }
                                        span { class: "text-sm font-mono text-blue-400", "{truncate_address(dest_tx, 12)}" }
                                    }
                                }
                            }
                        }
                    },
                    None => if !transfers.is_empty() {
                        rsx! {
                            div { class: "bg-gray-800/50 rounded-lg p-4 text-center",
                                p { class: "text-gray-400", "Select a transfer to view details." }
                            }
                        }
                    } else {
                        rsx! {}
                    }
                }}
            }
        }
    }
}
