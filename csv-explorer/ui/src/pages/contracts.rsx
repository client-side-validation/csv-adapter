/// Contracts list page.

use dioxus::prelude::*;

use crate::{components, routes};

#[component]
pub fn ContractsList() -> Element {
    let contracts = use_resource(move || async move {
        fetch_contracts().await
    });

    rsx! {
        div { class: "space-y-6",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold", "CSV Contracts" }
                span { class: "text-gray-400 text-sm",
                    {contracts.with(|c| match c.as_ref() {
                        Some(records) => format!("{} contracts found", records.len()),
                        None => "Loading...".to_string(),
                    })}
                }
            }

            // Table
            div { class: "bg-gray-900 rounded-xl border border-gray-800 overflow-hidden",
                table { class: "w-full",
                    thead {
                        tr { class: "border-b border-gray-800 text-left text-sm text-gray-400",
                            th { class: "px-6 py-3 font-medium", "Contract ID" }
                            th { class: "px-6 py-3 font-medium", "Chain" }
                            th { class: "px-6 py-3 font-medium", "Type" }
                            th { class: "px-6 py-3 font-medium", "Address" }
                            th { class: "px-6 py-3 font-medium", "Version" }
                            th { class: "px-6 py-3 font-medium", "Status" }
                        }
                    }
                    tbody { class: "divide-y divide-gray-800",
                        {contracts.with(|c| match c.as_ref() {
                            Some(records) => rsx! {
                                {records.iter().map(|contract| rsx! {
                                    ContractRow {
                                        key: "{contract.id}",
                                        id: contract.id.clone(),
                                        chain: contract.chain.clone(),
                                        contract_type: contract.contract_type.to_string(),
                                        address: contract.address.clone(),
                                        version: contract.version.clone(),
                                        status: contract.status.to_string(),
                                    }
                                }).collect::<Vec<Element>>()}
                            },
                            None => rsx! {
                                tr {
                                    td { col_span: 6, class: "px-6 py-12 text-center text-gray-500",
                                        "Loading contracts..."
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
fn ContractRow(id: String, chain: String, contract_type: String, address: String, version: String, status: String) -> Element {
    rsx! {
        tr { class: "hover:bg-gray-800/50 transition-colors",
            td { class: "px-6 py-4 font-mono text-sm text-blue-400",
                "{id}"
            }
            td { class: "px-6 py-4",
                components::chain_badge::ChainBadge { chain }
            }
            td { class: "px-6 py-4 text-sm text-gray-300",
                "{contract_type}"
            }
            td { class: "px-6 py-4 font-mono text-sm text-gray-300",
                "{address}"
            }
            td { class: "px-6 py-4 text-sm text-gray-400",
                "{version}"
            }
            td { class: "px-6 py-4",
                components::status_badge::StatusBadge { status }
            }
        }
    }
}

async fn fetch_contracts() -> Option<Vec<shared::CsvContract>> {
    None
}
