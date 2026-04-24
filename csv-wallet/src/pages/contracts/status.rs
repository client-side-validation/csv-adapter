//! Contract status page.

use crate::context::use_wallet_context;
use crate::pages::common::*;
use crate::routes::Route;
use csv_adapter_core::Chain;
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn ContractStatus() -> Element {
    let wallet_ctx = use_wallet_context();
    let mut selected_chain = use_signal(|| Chain::Ethereum);

    let contracts = wallet_ctx.contracts_for_chain(*selected_chain.read());

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Contracts {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Contract Status" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Chain", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<Chain>() { selected_chain.set(c); }
                }, *selected_chain.read()))}

                if contracts.is_empty() {
                    div { class: "bg-gray-800/50 rounded-lg p-4 border border-gray-700 text-center",
                        p { class: "text-gray-400", "No contracts deployed on {chain_name(&selected_chain.read())}" }
                        Link { to: Route::DeployContract {}, class: "text-blue-400 hover:text-blue-300 text-sm mt-1 inline-block", "Deploy now \u{2192}" }
                    }
                } else {
                    for c in contracts {
                        div { class: "bg-gray-800/50 rounded-lg p-4 border border-gray-700 space-y-2",
                            div { class: "flex justify-between",
                                span { class: "text-sm text-gray-400", "Address" }
                                p { class: "font-mono text-sm text-gray-200", "{truncate_address(&c.address, 10)}" }
                            }
                            div { class: "flex justify-between",
                                span { class: "text-sm text-gray-400", "TX Hash" }
                                p { class: "font-mono text-sm text-gray-200", "{truncate_address(&c.tx_hash, 10)}" }
                            }
                        }
                    }
                }
            }
        }
    }
}
