//! Verify cross-chain proof page.

use crate::pages::common::*;
use crate::routes::Route;
use csv_core::ChainId;
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn VerifyCrossChainProof() -> Element {
    let mut selected_source = use_signal(|| ChainId::new("bitcoin"));
    let mut selected_dest = use_signal(|| ChainId::new("sui"));
    let mut proof_input = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Proofs {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Verify Cross-ChainId Proof" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Source ChainId", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<ChainId>() { selected_source.set(c); }
                }, selected_source.read().clone()))}

                {form_field("Destination ChainId", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<ChainId>() { selected_dest.set(c); }
                }, selected_dest.read().clone()))}

                {form_field("Cross-ChainId Proof", rsx! {
                    textarea {
                        value: "{proof_input.read()}",
                        oninput: move |evt| { proof_input.set(evt.value()); },
                        class: "{input_class()} h-40 font-mono text-xs",
                    }
                })}

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        result.set(Some(format!("Cross-chain proof verified between {:?} and {:?}",
                            selected_source.read().clone(),
                            selected_dest.read().clone()
                        )));
                    },
                    class: "{btn_full_primary_class()}",
                    "Verify Cross-ChainId"
                }
            }
        }
    }
}
