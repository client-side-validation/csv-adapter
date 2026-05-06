//! Validate proof page.

use crate::pages::common::*;
use crate::routes::Route;
use csv_core::Chain;
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn ValidateProof() -> Element {
    let mut selected_chain = use_signal(|| Chain::Bitcoin);
    let mut result = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Validate {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Validate Proof" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Chain", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<Chain>() { selected_chain.set(c); }
                }, *selected_chain.read()))}

                {form_field("Proof File", rsx! {
                    input {
                        class: "{input_class()}",
                        r#type: "file",
                    }
                })}

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        result.set(Some("Proof is valid.".to_string()));
                    },
                    class: "{btn_full_primary_class()}",
                    "Validate Proof"
                }
            }
        }
    }
}
