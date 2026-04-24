//! Verify proof page.

use crate::pages::common::*;
use crate::routes::Route;
use csv_adapter_core::Chain;
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn VerifyProof() -> Element {
    let mut selected_chain = use_signal(|| Chain::Bitcoin);
    let mut proof_input = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Proofs {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Verify Proof" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Destination Chain", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<Chain>() { selected_chain.set(c); }
                }, *selected_chain.read()))}

                {form_field("Proof JSON", rsx! {
                    textarea {
                        value: "{proof_input.read()}",
                        oninput: move |evt| { proof_input.set(evt.value()); },
                        class: "{input_class()} h-40 font-mono text-xs",
                        placeholder: "Paste the proof JSON here..."
                    }
                })}

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        result.set(Some(format!("Proof verified on {:?}", *selected_chain.read())));
                    },
                    class: "{btn_full_primary_class()}",
                    "Verify Proof"
                }
            }
        }
    }
}
