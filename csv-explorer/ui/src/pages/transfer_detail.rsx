/// Transfer detail page with full info and timeline.

use dioxus::prelude::*;

use crate::{components, routes};

#[component]
pub fn TransferDetail(id: String) -> Element {
    let transfer = use_resource(move || async move {
        fetch_transfer(&id).await
    });

    rsx! {
        div { class: "space-y-6",
            // Breadcrumb
            nav { class: "text-sm text-gray-400",
                Link { to: routes::Route::Home {}, class: "hover:text-white", "Home" }
                span { " / " }
                Link { to: routes::Route::TransfersList {}, class: "hover:text-white", "Transfers" }
                span { " / " }
                span { class: "text-gray-200 font-mono text-xs", "{id}" }
            }

            if let Some(Some(t)) = transfer.value() {
                div {
                    // Header
                    div { class: "flex items-center justify-between",
                        h1 { class: "text-2xl font-bold", "Transfer Detail" }
                        components::status_badge::StatusBadge { status: t.status.to_string() }
                    }

                    // Route visualization
                    div { class: "bg-gray-900 rounded-xl border border-gray-800 p-6",
                        div { class: "flex items-center justify-center gap-4 py-4",
                            div { class: "text-center",
                                components::chain_badge::ChainBadge { chain: t.from_chain.clone() }
                                div { class: "text-sm text-gray-400 mt-2", "Source" }
                            }
                            span { class: "text-2xl text-blue-400", "→" }
                            div { class: "text-center",
                                components::chain_badge::ChainBadge { chain: t.to_chain.clone() }
                                div { class: "text-sm text-gray-400 mt-2", "Destination" }
                            }
                        }
                    }

                    // Details grid
                    div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                        DetailCard { title: "Transfer ID", value: t.id.clone(), copyable: true }
                        DetailCard { title: "Right ID", value: t.right_id.clone(), copyable: true }
                        DetailCard { title: "From Owner", value: t.from_owner.clone(), copyable: true }
                        DetailCard { title: "To Owner", value: t.to_owner.clone(), copyable: true }
                        DetailCard { title: "Lock TX", value: t.lock_tx.clone(), copyable: true }
                        if let Some(ref mint) = t.mint_tx {
                            DetailCard { title: "Mint TX", value: mint.clone(), copyable: true }
                        }
                        if let Some(ref proof) = t.proof_ref {
                            DetailCard { title: "Proof Reference", value: proof.clone(), copyable: true }
                        }
                        DetailCard { title: "Created At", value: t.created_at.to_rfc3339() }
                        if let Some(completed) = t.completed_at {
                            DetailCard { title: "Completed At", value: completed.to_rfc3339() }
                        }
                        if let Some(duration) = t.duration_ms {
                            DetailCard { title: "Duration", value: format!("{}ms", duration) }
                        }
                    }

                    // Timeline
                    div { class: "bg-gray-900 rounded-xl border border-gray-800 p-6",
                        h2 { class: "text-lg font-semibold mb-6", "Transfer Timeline" }
                        components::timeline::TransferTimeline {
                            status: t.status.to_string(),
                            created_at: t.created_at.to_rfc3339(),
                            completed_at: t.completed_at.map(|dt| dt.to_rfc3339()),
                        }
                    }

                    // Raw JSON
                    div { class: "bg-gray-900 rounded-xl border border-gray-800 p-6",
                        h2 { class: "text-lg font-semibold mb-4", "Raw Data" }
                        pre { class: "text-sm text-gray-300 overflow-x-auto",
                            "{serde_json::to_string_pretty(&t).unwrap_or_default()}"
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
fn DetailCard(title: String, value: String, copyable: Option<bool>) -> Element {
    rsx! {
        div { class: "bg-gray-900 rounded-xl border border-gray-800 p-4",
            div { class: "text-sm text-gray-400 mb-1", "{title}" }
            div { class: "flex items-center gap-2",
                span { class: "font-mono text-sm text-gray-200 break-all", "{value}" }
                if copyable.unwrap_or(false) {
                    button { class: "text-gray-500 hover:text-gray-300", "⧉" }
                }
            }
        }
    }
}

async fn fetch_transfer(_id: &str) -> Option<shared::TransferRecord> {
    None
}
