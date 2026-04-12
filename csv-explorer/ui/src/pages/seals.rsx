/// Seals list page with filtering and pagination.

use dioxus::prelude::*;

use crate::{components, routes};

#[component]
pub fn SealsList() -> Element {
    let seals = use_resource(move || async move {
        fetch_seals().await
    });

    rsx! {
        div { class: "space-y-6",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold", "Seals" }
                span { class: "text-gray-400 text-sm",
                    {seals.with(|s| match s.as_ref() {
                        Some(records) => format!("{} seals found", records.len()),
                        None => "Loading...".to_string(),
                    })}
                }
            }

            // Filters
            div { class: "bg-gray-900 rounded-xl border border-gray-800 p-4",
                div { class: "flex flex-wrap gap-4",
                    select { class: "bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm",
                        option { value: "", "All Chains" }
                        option { value: "bitcoin", "Bitcoin" }
                        option { value: "ethereum", "Ethereum" }
                        option { value: "sui", "Sui" }
                        option { value: "aptos", "Aptos" }
                        option { value: "solana", "Solana" }
                    }
                    select { class: "bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm",
                        option { value: "", "All Types" }
                        option { value: "utxo", "UTXO" }
                        option { value: "object", "Object" }
                        option { value: "resource", "Resource" }
                        option { value: "nullifier", "Nullifier" }
                        option { value: "account", "Account" }
                    }
                    select { class: "bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm",
                        option { value: "", "All Statuses" }
                        option { value: "available", "Available" }
                        option { value: "consumed", "Consumed" }
                    }
                }
            }

            // Table
            div { class: "bg-gray-900 rounded-xl border border-gray-800 overflow-hidden",
                table { class: "w-full",
                    thead {
                        tr { class: "border-b border-gray-800 text-left text-sm text-gray-400",
                            th { class: "px-6 py-3 font-medium", "Seal ID" }
                            th { class: "px-6 py-3 font-medium", "Chain" }
                            th { class: "px-6 py-3 font-medium", "Type" }
                            th { class: "px-6 py-3 font-medium", "Status" }
                            th { class: "px-6 py-3 font-medium", "Right ID" }
                            th { class: "px-6 py-3 font-medium", "Block" }
                        }
                    }
                    tbody { class: "divide-y divide-gray-800",
                        {seals.with(|s| match s.as_ref() {
                            Some(records) => rsx! {
                                {records.iter().map(|seal| rsx! {
                                    SealRow {
                                        key: "{seal.id}",
                                        id: seal.id.clone(),
                                        chain: seal.chain.clone(),
                                        seal_type: seal.seal_type.to_string(),
                                        status: seal.status.to_string(),
                                        right_id: seal.right_id.clone().unwrap_or_else(|| "—".to_string()),
                                        block_height: seal.block_height,
                                    }
                                }).collect::<Vec<Element>>()}
                            },
                            None => rsx! {
                                tr {
                                    td { col_span: 6, class: "px-6 py-12 text-center text-gray-500",
                                        "Loading seals..."
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
fn SealRow(id: String, chain: String, seal_type: String, status: String, right_id: String, block_height: u64) -> Element {
    rsx! {
        tr { class: "hover:bg-gray-800/50 transition-colors",
            td { class: "px-6 py-4",
                Link { to: routes::Route::SealDetail { id: id.clone() },
                    class: "font-mono text-sm text-blue-400 hover:text-blue-300"
                    "{id}"
                }
            }
            td { class: "px-6 py-4",
                components::chain_badge::ChainBadge { chain }
            }
            td { class: "px-6 py-4",
                span { class: "px-2 py-1 rounded-full text-xs font-medium bg-gray-800 text-gray-300",
                    "{seal_type}"
                }
            }
            td { class: "px-6 py-4",
                components::status_badge::StatusBadge { status }
            }
            td { class: "px-6 py-4 font-mono text-sm text-gray-300",
                {if right_id == "—" {
                    rsx! { span { class: "text-gray-600", "—" } }
                } else {
                    rsx! {
                        Link { to: routes::Route::RightDetail { id: right_id.clone() },
                            class: "text-blue-400 hover:text-blue-300"
                            "{right_id}"
                        }
                    }
                }}
            }
            td { class: "px-6 py-4 text-sm text-gray-400",
                "{block_height}"
            }
        }
    }
}

async fn fetch_seals() -> Option<Vec<shared::SealRecord>> {
    None
}
