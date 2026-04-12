/// Rights list page with filtering, sorting, and pagination.

use dioxus::prelude::*;

use crate::{components, routes};

#[component]
pub fn RightsList() -> Element {
    let chain_filter = use_signal(|| String::new());
    let status_filter = use_signal(|| String::new());
    let search_query = use_signal(|| String::new());
    let page = use_signal(|| 1u64);

    let rights = use_resource(move || async move {
        fetch_rights(&chain_filter.read(), &status_filter.read(), page.read()).await
    });

    rsx! {
        div { class: "space-y-6",
            // Header
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold", "Rights" }
                span { class: "text-gray-400 text-sm",
                    {rights.with(|r| match r.as_ref() {
                        Some(records) => format!("{} rights found", records.len()),
                        None => "Loading...".to_string(),
                    })}
                }
            }

            // Filters
            div { class: "bg-gray-900 rounded-xl border border-gray-800 p-4",
                div { class: "flex flex-wrap gap-4",
                    // Search
                    div { class: "flex-1 min-w-[200px]",
                        input {
                            r#type: "text",
                            value: "{search_query.read()}",
                            oninput: move |evt| search_query.set(evt.value()),
                            placeholder: "Search by ID or owner...",
                            class: "w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500"
                        }
                    }
                    // Chain filter
                    select {
                        class: "bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm",
                        option { value: "", "All Chains" }
                        option { value: "bitcoin", "Bitcoin" }
                        option { value: "ethereum", "Ethereum" }
                        option { value: "sui", "Sui" }
                        option { value: "aptos", "Aptos" }
                        option { value: "solana", "Solana" }
                    }
                    // Status filter
                    select {
                        class: "bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm",
                        option { value: "", "All Statuses" }
                        option { value: "active", "Active" }
                        option { value: "spent", "Spent" }
                        option { value: "pending", "Pending" }
                    }
                }
            }

            // Table
            div { class: "bg-gray-900 rounded-xl border border-gray-800 overflow-hidden",
                table { class: "w-full",
                    thead {
                        tr { class: "border-b border-gray-800 text-left text-sm text-gray-400",
                            th { class: "px-6 py-3 font-medium", "Right ID" }
                            th { class: "px-6 py-3 font-medium", "Chain" }
                            th { class: "px-6 py-3 font-medium", "Owner" }
                            th { class: "px-6 py-3 font-medium", "Status" }
                            th { class: "px-6 py-3 font-medium", "Created" }
                            th { class: "px-6 py-3 font-medium", "Transfers" }
                        }
                    }
                    tbody { class: "divide-y divide-gray-800",
                        {rights.with(|r| match r.as_ref() {
                            Some(records) => rsx! {
                                {records.iter().map(|right| rsx! {
                                    RightRow {
                                        key: "{right.id}",
                                        id: right.id.clone(),
                                        chain: right.chain.clone(),
                                        owner: right.owner.clone(),
                                        status: right.status.to_string(),
                                        created_at: right.created_at.to_rfc3339(),
                                        transfer_count: right.transfer_count,
                                    }
                                }).collect::<Vec<Element>>()}
                            },
                            None => rsx! {
                                tr {
                                    td { col_span: 6, class: "px-6 py-12 text-center text-gray-500",
                                        "Loading rights..."
                                    }
                                }
                            },
                        })}
                    }
                }
            }

            // Pagination
            div { class: "flex items-center justify-between",
                button {
                    onclick: move |_| { if *page.read() > 1 { page.set(page.read() - 1); } },
                    disabled: *page.read() <= 1,
                    class: "px-4 py-2 rounded-lg bg-gray-800 hover:bg-gray-700 disabled:opacity-50 disabled:cursor-not-allowed transition-colors"
                    "Previous"
                }
                span { class: "text-gray-400", "Page {page.read()}" }
                button {
                    onclick: move |_| page.set(page.read() + 1),
                    class: "px-4 py-2 rounded-lg bg-gray-800 hover:bg-gray-700 transition-colors"
                    "Next"
                }
            }
        }
    }
}

#[component]
fn RightRow(id: String, chain: String, owner: String, status: String, created_at: String, transfer_count: u64) -> Element {
    rsx! {
        tr { class: "hover:bg-gray-800/50 transition-colors",
            td { class: "px-6 py-4",
                Link {
                    to: routes::Route::RightDetail { id: id.clone() },
                    class: "font-mono text-sm text-blue-400 hover:text-blue-300"
                    "{id}"
                }
            }
            td { class: "px-6 py-4",
                components::chain_badge::ChainBadge { chain }
            }
            td { class: "px-6 py-4 font-mono text-sm text-gray-300",
                "{owner}"
            }
            td { class: "px-6 py-4",
                components::status_badge::StatusBadge { status }
            }
            td { class: "px-6 py-4 text-sm text-gray-400",
                "{created_at}"
            }
            td { class: "px-6 py-4 text-sm text-gray-300",
                "{transfer_count}"
            }
        }
    }
}

async fn fetch_rights(_chain: &str, _status: &str, _page: u64) -> Option<Vec<shared::RightRecord>> {
    // In production, fetch from API
    None
}
