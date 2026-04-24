//! Verify seal page.

use crate::context::use_wallet_context;
use crate::pages::common::*;
use crate::routes::Route;
use csv_adapter_core::Chain;
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn VerifySeal() -> Element {
    let wallet_ctx = use_wallet_context();
    let mut selected_chain = use_signal(|| Chain::Bitcoin);
    let mut seal_ref = use_signal(String::new);
    let mut result = use_signal(|| Option::<bool>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Seals {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Verify Seal" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Chain", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<Chain>() { selected_chain.set(c); }
                }, *selected_chain.read()))}

                {form_field("Seal Reference (hex)", rsx! {
                    input {
                        value: "{seal_ref.read()}",
                        oninput: move |evt| { seal_ref.set(evt.value()); result.set(None); },
                        class: "{input_mono_class()}",
                        placeholder: "0x..."
                    }
                })}

                if let Some(consumed) = result.read().as_ref() {
                    div { class: "p-4 {seal_consumed_class(*consumed)} border rounded-lg",
                        p { class: "{seal_consumed_text_class(*consumed)}",
                            if *consumed { "Seal is CONSUMED" } else { "Seal is UNCONSUMED (available)" }
                        }
                    }
                }

                button {
                    onclick: move |_| {
                        result.set(Some(wallet_ctx.is_seal_consumed(&seal_ref.read())));
                    },
                    class: "{btn_full_primary_class()}",
                    "Verify Seal"
                }
            }
        }
    }
}
