//! Add contract page.

use crate::context::{generate_id, use_wallet_context, DeployedContract};
use crate::pages::common::*;
use crate::routes::Route;
use csv_adapter_core::Chain;
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn AddContract() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut selected_chain = use_signal(|| Chain::Sui);
    let mut contract_address = use_signal(String::new);
    let mut tx_hash = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);
    let mut error = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Contracts {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Add Existing Contract" }
            }

            div { class: "bg-blue-900/30 border border-blue-700/50 rounded-lg p-4",
                p { class: "text-sm text-blue-300",
                    "Use this form to add a contract that was already deployed (e.g., via csv-cli)."
                }
                p { class: "text-xs text-blue-400 mt-1",
                    "Example Sui package: 0xa972ca52e0c69118471a755aee0efd89993649b6d1f32a4fc9e186c1458694c2"
                }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Chain", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<Chain>() {
                        selected_chain.set(c);
                        error.set(None);
                    }
                }, *selected_chain.read()))}

                {form_field("Contract Address / Package ID", rsx! {
                    input {
                        value: "{contract_address.read()}",
                        oninput: move |evt| { contract_address.set(evt.value()); error.set(None); },
                        class: "{input_mono_class()}",
                        r#type: "text"
                    }
                })}

                {form_field("Transaction Hash (optional)", rsx! {
                    input {
                        value: "{tx_hash.read()}",
                        oninput: move |evt| { tx_hash.set(evt.value()); },
                        class: "{input_mono_class()}",
                        r#type: "text"
                    }
                })}

                if let Some(e) = error.read().as_ref() {
                    div { class: "p-4 bg-red-900/30 border border-red-700/50 rounded-lg",
                        p { class: "text-red-300 text-sm", "{e}" }
                    }
                }

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300 font-mono text-sm", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        let addr = contract_address.read().trim().to_string();
                        if addr.is_empty() {
                            error.set(Some("Contract address is required".to_string()));
                            return;
                        }
                        // Only require 0x prefix for non-Solana chains (Solana uses base58)
                        let chain = *selected_chain.read();
                        if chain != Chain::Solana && !addr.starts_with("0x") {
                            error.set(Some("Address must start with 0x".to_string()));
                            return;
                        }

                        let tx = tx_hash.read().trim().to_string();
                        let tx = if tx.is_empty() { generate_id() } else { tx };

                        wallet_ctx.add_contract(DeployedContract {
                            chain: *selected_chain.read(),
                            address: addr.clone(),
                            tx_hash: tx,
                            deployed_at: js_sys::Date::now() as u64 / 1000,
                        });

                        result.set(Some(format!("Contract added for {:?}", *selected_chain.read())));
                        contract_address.set(String::new());
                        tx_hash.set(String::new());
                    },
                    disabled: contract_address.read().is_empty(),
                    class: "{btn_full_primary_class()}",
                    "Add Contract"
                }
            }
        }
    }
}
