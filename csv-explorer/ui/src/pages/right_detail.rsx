/// Right detail page showing all fields, transfer history, and seal info.

use dioxus::prelude::*;

use crate::{components, routes};

#[component]
pub fn RightDetail(id: String) -> Element {
    let right = use_resource(move || async move {
        fetch_right(&id).await
    });

    let transfers = use_resource(move || async move {
        fetch_transfers_for_right(&id).await
    });

    rsx! {
        div { class: "space-y-6",
            // Breadcrumb
            nav { class: "text-sm text-gray-400",
                Link { to: routes::Route::Home {}, class: "hover:text-white", "Home" }
                span { " / " }
                Link { to: routes::Route::RightsList {}, class: "hover:text-white", "Rights" }
                span { " / " }
                span { class: "text-gray-200 font-mono text-xs", "{id}" }
            }

            // Loading state
            if let Some(Some(right_data)) = right.value() {
                div {
                    // Header
                    div { class: "flex items-center justify-between",
                        h1 { class: "text-2xl font-bold", "Right Detail" }
                        components::status_badge::StatusBadge { status: right_data.status.to_string() }
                    }

                    // Main info grid
                    div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                        InfoCard { title: "Right ID", value: right_data.id.clone(), copyable: true }
                        InfoCard { title: "Chain", value: right_data.chain.clone() }
                        InfoCard { title: "Seal Reference", value: right_data.seal_ref.clone(), copyable: true }
                        InfoCard { title: "Commitment", value: right_data.commitment.clone(), copyable: true }
                        InfoCard { title: "Owner", value: right_data.owner.clone(), copyable: true }
                        InfoCard { title: "Created At", value: right_data.created_at.to_rfc3339() }
                        InfoCard { title: "Creation TX", value: right_data.created_tx.clone(), copyable: true }
                        InfoCard { title: "Transfer Count", value: right_data.transfer_count.to_string() }
                        if let Some(last_transfer) = right_data.last_transfer_at {
                            InfoCard { title: "Last Transfer", value: last_transfer.to_rfc3339() }
                        }
                    }

                    // Metadata
                    if let Some(metadata) = &right_data.metadata {
                        div { class: "bg-gray-900 rounded-xl border border-gray-800 p-6",
                            h2 { class: "text-lg font-semibold mb-4", "Metadata" }
                            pre { class: "text-sm text-gray-300 overflow-x-auto",
                                "{serde_json::to_string_pretty(metadata).unwrap_or_default()}"
                            }
                        }
                    }

                    // Transfer history
                    div {
                        h2 { class: "text-lg font-semibold mb-4", "Transfer History" }
                        if let Some(Some(transfer_list)) = transfers.value() {
                            if transfer_list.is_empty() {
                                div { class: "bg-gray-900 rounded-xl border border-gray-800 p-8 text-center text-gray-500",
                                    "No transfers for this right"
                                }
                            } else {
                                div { class: "bg-gray-900 rounded-xl border border-gray-800 overflow-hidden",
                                    div { class: "divide-y divide-gray-800",
                                        {transfer_list.iter().map(|t| rsx! {
                                            TransferHistoryRow {
                                                key: "{t.id}",
                                                id: t.id.clone(),
                                                from_chain: t.from_chain.clone(),
                                                to_chain: t.to_chain.clone(),
                                                status: t.status.to_string(),
                                                created_at: t.created_at.to_rfc3339(),
                                            }
                                        }).collect::<Vec<Element>>()}
                                    }
                                }
                            }
                        } else {
                            div { class: "bg-gray-900 rounded-xl border border-gray-800 p-8 text-center text-gray-500",
                                "Loading transfers..."
                            }
                        }
                    }

                    // Raw JSON
                    div { class: "bg-gray-900 rounded-xl border border-gray-800 p-6",
                        h2 { class: "text-lg font-semibold mb-4", "Raw Data" }
                        pre { class: "text-sm text-gray-300 overflow-x-auto",
                            "{serde_json::to_string_pretty(&right_data).unwrap_or_default()}"
                        }
                    }
                }
            } else {
                div { class: "bg-gray-900 rounded-xl border border-gray-800 p-12 text-center",
                    div { class: "animate-pulse space-y-4",
                        div { class: "h-8 bg-gray-800 rounded w-48 mx-auto" }
                        div { class: "h-4 bg-gray-800 rounded w-96 mx-auto" }
                    }
                }
            }
        }
    }
}

#[component]
fn InfoCard(title: String, value: String, copyable: Option<bool>) -> Element {
    rsx! {
        div { class: "bg-gray-900 rounded-xl border border-gray-800 p-4",
            div { class: "text-sm text-gray-400 mb-1", "{title}" }
            div { class: "flex items-center gap-2",
                span { class: "font-mono text-sm text-gray-200 break-all", "{value}" }
                if copyable.unwrap_or(false) {
                    button { class: "text-gray-500 hover:text-gray-300",
                        "⧉"
                    }
                }
            }
        }
    }
}

#[component]
fn TransferHistoryRow(id: String, from_chain: String, to_chain: String, status: String, created_at: String) -> Element {
    rsx! {
        div { class: "px-6 py-4 flex items-center justify-between hover:bg-gray-800/50 transition-colors",
            div { class: "flex items-center gap-3",
                components::chain_badge::ChainBadge { chain: from_chain.clone() }
                span { class: "text-gray-500", "→" }
                components::chain_badge::ChainBadge { chain: to_chain.clone() }
            }
            div { class: "flex items-center gap-4",
                components::status_badge::StatusBadge { status }
                Link { to: routes::Route::TransferDetail { id: id.clone() },
                    span { class: "text-blue-400 hover:text-blue-300 text-sm", "View" }
                }
            }
        }
    }
}

async fn fetch_right(_id: &str) -> Option<shared::RightRecord> {
    None
}

async fn fetch_transfers_for_right(_id: &str) -> Option<Vec<shared::TransferRecord>> {
    None
}
