/// Transfers list page with filtering and pagination.

use dioxus::prelude::*;

use crate::{components, routes};

#[component]
pub fn TransfersList() -> Element {
    let transfers = use_resource(move || async move {
        fetch_transfers().await
    });

    rsx! {
        div { class: "space-y-6",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold", "Cross-Chain Transfers" }
                span { class: "text-gray-400 text-sm",
                    {transfers.with(|t| match t.as_ref() {
                        Some(records) => format!("{} transfers found", records.len()),
                        None => "Loading...".to_string(),
                    })}
                }
            }

            // Filters
            div { class: "bg-gray-900 rounded-xl border border-gray-800 p-4",
                div { class: "flex flex-wrap gap-4",
                    select { class: "bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm",
                        option { value: "", "All Source Chains" }
                        option { value: "bitcoin", "Bitcoin" }
                        option { value: "ethereum", "Ethereum" }
                        option { value: "sui", "Sui" }
                        option { value: "aptos", "Aptos" }
                        option { value: "solana", "Solana" }
                    }
                    select { class: "bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm",
                        option { value: "", "All Destination Chains" }
                        option { value: "bitcoin", "Bitcoin" }
                        option { value: "ethereum", "Ethereum" }
                        option { value: "sui", "Sui" }
                        option { value: "aptos", "Aptos" }
                        option { value: "solana", "Solana" }
                    }
                    select { class: "bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm",
                        option { value: "", "All Statuses" }
                        option { value: "pending", "Pending" }
                        option { value: "in_progress", "In Progress" }
                        option { value: "completed", "Completed" }
                        option { value: "failed", "Failed" }
                    }
                }
            }

            // Table
            div { class: "bg-gray-900 rounded-xl border border-gray-800 overflow-hidden",
                table { class: "w-full",
                    thead {
                        tr { class: "border-b border-gray-800 text-left text-sm text-gray-400",
                            th { class: "px-6 py-3 font-medium", "Transfer ID" }
                            th { class: "px-6 py-3 font-medium", "Route" }
                            th { class: "px-6 py-3 font-medium", "Right ID" }
                            th { class: "px-6 py-3 font-medium", "Status" }
                            th { class: "px-6 py-3 font-medium", "Lock TX" }
                            th { class: "px-6 py-3 font-medium", "Created" }
                        }
                    }
                    tbody { class: "divide-y divide-gray-800",
                        {transfers.with(|t| match t.as_ref() {
                            Some(records) => rsx! {
                                {records.iter().map(|transfer| rsx! {
                                    TransferRow {
                                        key: "{transfer.id}",
                                        id: transfer.id.clone(),
                                        from_chain: transfer.from_chain.clone(),
                                        to_chain: transfer.to_chain.clone(),
                                        right_id: transfer.right_id.clone(),
                                        status: transfer.status.to_string(),
                                        lock_tx: transfer.lock_tx.clone(),
                                        created_at: transfer.created_at.to_rfc3339(),
                                    }
                                }).collect::<Vec<Element>>()}
                            },
                            None => rsx! {
                                tr {
                                    td { col_span: 6, class: "px-6 py-12 text-center text-gray-500",
                                        "Loading transfers..."
                                    }
                                }
                            },
                        })}
                    }
                }
            }
        }
    }
}

#[component]
fn TransferRow(id: String, from_chain: String, to_chain: String, right_id: String, status: String, lock_tx: String, created_at: String) -> Element {
    rsx! {
        tr { class: "hover:bg-gray-800/50 transition-colors",
            td { class: "px-6 py-4",
                Link { to: routes::Route::TransferDetail { id: id.clone() },
                    class: "font-mono text-sm text-blue-400 hover:text-blue-300"
                    "{id}"
                }
            }
            td { class: "px-6 py-4",
                div { class: "flex items-center gap-2",
                    components::chain_badge::ChainBadge { chain: from_chain.clone() }
                    span { class: "text-gray-500", "→" }
                    components::chain_badge::ChainBadge { chain: to_chain.clone() }
                }
            }
            td { class: "px-6 py-4 font-mono text-sm text-gray-300",
                "{right_id}"
            }
            td { class: "px-6 py-4",
                components::status_badge::StatusBadge { status }
            }
            td { class: "px-6 py-4 font-mono text-sm text-gray-400",
                "{lock_tx}"
            }
            td { class: "px-6 py-4 text-sm text-gray-400",
                "{created_at}"
            }
        }
    }
}

async fn fetch_transfers() -> Option<Vec<shared::TransferRecord>> {
    None
}
