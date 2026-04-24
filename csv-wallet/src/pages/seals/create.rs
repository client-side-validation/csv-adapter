//! Create seal page.

use crate::context::{generate_id, use_wallet_context, SealRecord};
use crate::pages::common::*;
use crate::routes::Route;
use csv_adapter_core::Chain;
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn CreateSeal() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut selected_chain = use_signal(|| Chain::Bitcoin);
    let mut value = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Seals {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Create Seal" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Chain", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<Chain>() { selected_chain.set(c); }
                }, *selected_chain.read()))}

                {form_field("Value (optional)", rsx! {
                    input {
                        value: "{value.read()}",
                        oninput: move |evt| { value.set(evt.value()); },
                        class: "{input_mono_class()}",
                        placeholder: "e.g., 1000"
                    }
                })}

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300 font-mono text-sm break-all", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        let seal_ref = generate_id();
                        let val: u64 = value.read().parse().unwrap_or(0);
                        wallet_ctx.add_seal(SealRecord {
                            seal_ref: seal_ref.clone(),
                            chain: *selected_chain.read(),
                            value: val,
                            consumed: false,
                            created_at: 0,
                        });
                        result.set(Some(format!("Seal created on {} (ref: {})", chain_name(&selected_chain.read()), truncate_address(&seal_ref, 16))));
                    },
                    class: "{btn_full_primary_class()}",
                    "Create Seal"
                }
            }
        }
    }
}

