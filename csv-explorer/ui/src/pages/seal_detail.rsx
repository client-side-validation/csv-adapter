/// Seal detail page.

use dioxus::prelude::*;

use crate::{components, routes};

#[component]
pub fn SealDetail(id: String) -> Element {
    let seal = use_resource(move || async move {
        fetch_seal(&id).await
    });

    rsx! {
        div { class: "space-y-6",
            // Breadcrumb
            nav { class: "text-sm text-gray-400",
                Link { to: routes::Route::Home {}, class: "hover:text-white", "Home" }
                span { " / " }
                Link { to: routes::Route::SealsList {}, class: "hover:text-white", "Seals" }
                span { " / " }
                span { class: "text-gray-200 font-mono text-xs", "{id}" }
            }

            if let Some(Some(s)) = seal.value() {
                div {
                    // Header
                    div { class: "flex items-center justify-between",
                        h1 { class: "text-2xl font-bold", "Seal Detail" }
                        components::status_badge::StatusBadge { status: s.status.to_string() }
                    }

                    // Details grid
                    div { class: "grid grid-cols-1 md:grid-cols-2 gap-6",
                        DetailCard { title: "Seal ID", value: s.id.clone(), copyable: true }
                        DetailCard { title: "Chain", value: s.chain.clone() }
                        DetailCard { title: "Seal Type", value: s.seal_type.to_string() }
                        DetailCard { title: "Seal Reference", value: s.seal_ref.clone(), copyable: true }
                        DetailCard { title: "Block Height", value: s.block_height.to_string() }
                        if let Some(ref right_id) = s.right_id {
                            DetailCard { title: "Linked Right", value: right_id.clone(), copyable: true }
                        }
                        if let Some(consumed_at) = s.consumed_at {
                            DetailCard { title: "Consumed At", value: consumed_at.to_rfc3339() }
                        }
                        if let Some(ref consumed_tx) = s.consumed_tx {
                            DetailCard { title: "Consumed TX", value: consumed_tx.clone(), copyable: true }
                        }
                    }

                    // Linked right
                    if let Some(ref right_id) = s.right_id {
                        div { class: "bg-gray-900 rounded-xl border border-gray-800 p-6",
                            h2 { class: "text-lg font-semibold mb-4", "Linked Right" }
                            Link {
                                to: routes::Route::RightDetail { id: right_id.clone() },
                                class: "font-mono text-sm text-blue-400 hover:text-blue-300"
                                "{right_id}"
                            }
                        }
                    }

                    // Raw JSON
                    div { class: "bg-gray-900 rounded-xl border border-gray-800 p-6",
                        h2 { class: "text-lg font-semibold mb-4", "Raw Data" }
                        pre { class: "text-sm text-gray-300 overflow-x-auto",
                            "{serde_json::to_string_pretty(&s).unwrap_or_default()}"
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

async fn fetch_seal(_id: &str) -> Option<shared::SealRecord> {
    None
}
