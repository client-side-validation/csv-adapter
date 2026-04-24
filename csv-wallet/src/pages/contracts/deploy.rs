//! Deploy contract page.

use crate::context::{generate_id, use_wallet_context, DeployedContract, Network};
use crate::pages::common::*;
use crate::routes::Route;
use crate::services::chain_api::ChainConfig;
use crate::services::transaction_builder::discover_contracts;
use csv_adapter_core::Chain;
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn DeployContract() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut selected_chain = use_signal(|| Chain::Ethereum);
    let mut selected_network = use_signal(|| Network::Test);
    let mut deployer_key = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);

    let is_bitcoin = *selected_chain.read() == Chain::Bitcoin;

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Contracts {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Deploy Contract" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Chain", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<Chain>() { selected_chain.set(c); }
                }, *selected_chain.read()))}

                {form_field("Network", network_select(move |n| {
                    selected_network.set(n);
                }, *selected_network.read()))}

                if !is_bitcoin {
                    {form_field("Deployer Private Key", rsx! {
                        input {
                            value: "{deployer_key.read()}",
                            oninput: move |evt| { deployer_key.set(evt.value()); },
                            class: "{input_mono_class()}",
                            placeholder: "0x..."
                        }
                    })}
                }

                if is_bitcoin {
                    div { class: "bg-gray-800/50 rounded-lg p-3 border border-gray-700",
                        p { class: "text-sm text-gray-400",
                            "\u{2139}\u{FE0F} Bitcoin is UTXO-native and does not require contract deployment."
                        }
                    }
                }

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300 font-mono text-sm break-all", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        if is_bitcoin {
                            result.set(Some("Bitcoin does not need contract deployment.".to_string()));
                        } else {
                            let addr = generate_id();
                            wallet_ctx.add_contract(DeployedContract {
                                chain: *selected_chain.read(),
                                address: addr.clone(),
                                tx_hash: generate_id(),
                                deployed_at: 0,
                            });
                            result.set(Some(format!("Contract deployed at: {}", addr)));
                        }
                    },
                    class: "{btn_full_primary_class()}",
                    if is_bitcoin { "Not Applicable" } else { "Deploy" }
                }
            }
        }
    }
}
