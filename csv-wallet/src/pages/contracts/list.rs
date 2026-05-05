//! Contracts list page.

use crate::context::{use_wallet_context, DeployedContract};
use crate::pages::common::*;
use crate::pages::contracts::ContractDetailModal;
use crate::routes::Route;
use csv_adapter_core::Chain;
use dioxus::prelude::*;

#[component]
pub fn Contracts() -> Element {
    let wallet_ctx = use_wallet_context();
    let accounts = wallet_ctx.accounts();

    // State for contract discovery
    let mut discovering = use_signal(|| false);
    let mut discovered_count = use_signal(|| 0usize);
    let accounts_for_discovery = accounts.clone();

    // Use global selected contract from context
    let selected_contract_for_modal = use_signal(|| None::<DeployedContract>);
    // Clone for use in closures
    let accounts_empty = accounts.is_empty();
    let mut selected_contract_modal_clone = selected_contract_for_modal.clone();
    let wallet_ctx_for_discovery = wallet_ctx.clone();

    rsx! {
        div { class: "space-y-6",
            div { class: "flex items-center justify-between",
                h1 { class: "text-2xl font-bold", "Contracts" }
                div { class: "flex gap-2",
                    button {
                        onclick: move |_| {
                            if discovering() {
                                return;
                            }
                            discovering.set(true);
                            discovered_count.set(0);
                            let accounts_clone = accounts_for_discovery.clone();
                            let mut wallet_ctx_clone = wallet_ctx_for_discovery.clone();
                            wasm_bindgen_futures::spawn_local(async move {
                                use crate::services::chain_api::ChainConfig;
                                use csv_adapter::prelude::NetworkType;
                                use crate::services::transaction_builder::discover_contracts;

                                let mut total_found = 0;

                                for account in &accounts_clone {
                                    if matches!(account.chain, Chain::Bitcoin) {
                                        continue;
                                    }

                                    let config = ChainConfig::for_chain(account.chain, NetworkType::Testnet);

                                    match discover_contracts(account.chain, &account.address, &config.api_url, None).await {
                                        Ok(contracts) => {
                                            for c in contracts {
                                                let c_addr = c.contract_address.clone();
                                                // Use a deterministic tx_hash based on address
                                                let tx_hash = format!("discovered_{}_{}", account.chain, &c_addr[..20.min(c_addr.len())]);
                                                let contract = DeployedContract {
                                                    chain: account.chain,
                                                    address: c_addr,
                                                    tx_hash,
                                                    deployed_at: js_sys::Date::now() as u64 / 1000,
                                                };
                                                web_sys::console::log_1(&format!("Discovered contract: {:?}", contract).into());
                                                wallet_ctx_clone.add_contract(contract);
                                                total_found += 1;
                                            }
                                        }
                                        Err(e) => {
                                            web_sys::console::warn_1(&format!("Discovery failed for {:?}: {:?}", account.chain, e).into());
                                        }
                                    }
                                }

                                let _ = discovered_count.try_write().map(|mut s| *s = total_found);
                                let _ = discovering.try_write().map(|mut s| *s = false);
                            });
                        },
                        disabled: *discovering.read() || accounts_empty,
                        class: if *discovering.read() {
                            "px-3 py-1.5 rounded-lg text-sm font-medium bg-gray-700 text-gray-400 cursor-not-allowed"
                        } else {
                            "px-3 py-1.5 rounded-lg text-sm font-medium bg-indigo-600 hover:bg-indigo-700 text-white transition-colors"
                        },
                        if *discovering.read() {
                            span { class: "flex items-center gap-2",
                                span { class: "animate-spin", "\u{27F3}" }
                                "Discovering..."
                            }
                        } else {
                            span { "\u{1F50D} Discover from Chain" }
                        }
                    }
                    Link { to: Route::AddContract {}, class: "{btn_secondary_class()}", "+ Add Existing" }
                    Link { to: Route::DeployContract {}, class: "{btn_primary_class()}", "+ Deploy New" }
                }
            }

            if *discovered_count.read() > 0 {
                div { class: "bg-green-900/30 border border-green-700/50 rounded-lg p-3",
                    p { class: "text-sm text-green-300",
                        "\u{2713} Discovered and added {discovered_count} contract(s) from chain"
                    }
                }
            }

            if wallet_ctx.contracts().is_empty() {
                {empty_state("\u{1F4DC}", "No contracts deployed", "Deploy contracts or discover from chain to enable cross-chain functionality.")}
            } else {
                div { class: "{table_class()}",
                    div { class: "{card_header_class()}",
                        h2 { class: "font-semibold text-sm", "Deployed Contracts ({wallet_ctx.contracts().len()})" }
                    }
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "text-left text-gray-400 border-b border-gray-800",
                                    th { class: "px-4 py-2 font-medium", "Chain" }
                                    th { class: "px-4 py-2 font-medium", "Address" }
                                    th { class: "px-4 py-2 font-medium", "TX Hash" }
                                    th { class: "px-4 py-2 font-medium", "Action" }
                                }
                            }
                            tbody { class: "divide-y divide-gray-800",
                                {wallet_ctx.contracts().into_iter().enumerate().map(|(idx, c)| {
                                    let c_for_click = c.clone();
                                    let key = format!("{}-{}-{}", idx, c.chain, c.address);
                                    rsx! {
                                        tr {
                                            key: "{key}",
                                            class: "hover:bg-gray-800/50 transition-colors cursor-pointer",
                                            onclick: move |_| selected_contract_modal_clone.set(Some(c_for_click.clone())),
                                            td { class: "px-4 py-3", span { class: "{chain_badge_class(&c.chain)}", "{chain_icon_emoji(&c.chain)} {chain_name(&c.chain)}" } }
                                            td { class: "px-4 py-3 font-mono text-xs", "{truncate_address(&c.address, 8)}" }
                                            td { class: "px-4 py-3 font-mono text-xs", "{truncate_address(&c.tx_hash, 8)}" }
                                            td { class: "px-4 py-3",
                                                span { class: "text-xs text-blue-400 hover:text-blue-300", "Click for details \u{2192}" }
                                            }
                                        }
                                    }
                                })}
                            }
                        }
                    }
                }
            }

            // Contract Detail Modal
            {
                let contract_opt = selected_contract_for_modal.read().clone();
                let mut selected_contract_modal_close = selected_contract_for_modal.clone();
                let mut selected_contract_modal_close2 = selected_contract_for_modal.clone();
                let mut wallet_ctx_clone = wallet_ctx.clone();
                match contract_opt {
                    Some(contract) => {
                        let contract_clone2 = contract.clone();
                        rsx! {
                            ContractDetailModal {
                                contract: contract,
                                on_close: move |_| selected_contract_modal_close.set(None),
                                on_use_in_transfer: move |_| {
                                    // Set global selected contract and navigate to cross-chain transfer
                                    wallet_ctx_clone.set_selected_contract(Some(contract_clone2.clone()));
                                    selected_contract_modal_close2.set(None);
                                },
                            }
                        }
                    }
                    None => rsx! {}
                }
            }
        }
    }
}
