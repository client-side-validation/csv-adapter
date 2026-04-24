//! Consume seal page.

use crate::context::{use_wallet_context, SealRecord};
use crate::pages::common::*;
use crate::routes::Route;
use csv_adapter_core::Chain;
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn ConsumeSeal() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut selected_chain = use_signal(|| Chain::Bitcoin);
    let mut seal_ref = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);
    let mut error = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Seals {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Consume Seal" }
            }

            div { class: "bg-yellow-900/30 border border-yellow-700/50 rounded-xl p-4",
                div { class: "flex items-center gap-2",
                    span { class: "text-yellow-400", "\u{26A0}\u{FE0F}" }
                    p { class: "text-yellow-300 font-medium", "Warning: Seal consumption is irreversible" }
                }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Chain", chain_select(move |v: Rc<FormData>| {
                    if let Ok(c) = v.value().parse::<Chain>() { selected_chain.set(c); }
                }, *selected_chain.read()))}

                {form_field("Seal Reference (hex)", rsx! {
                    input {
                        value: "{seal_ref.read()}",
                        oninput: move |evt| { seal_ref.set(evt.value()); error.set(None); },
                        class: "{input_mono_class()}",
                        placeholder: "0x..."
                    }
                })}

                if let Some(e) = error.read().as_ref() {
                    div { class: "p-3 bg-red-900/30 border border-red-700/50 rounded-lg text-sm text-red-300", "{e}" }
                }

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        if wallet_ctx.is_seal_consumed(&seal_ref.read()) {
                            error.set(Some("Seal replay detected: this seal has already been consumed.".to_string()));
                        } else {
                            let val: u64 = 0;
                            wallet_ctx.add_seal(SealRecord {
                                seal_ref: seal_ref.read().clone(),
                                chain: *selected_chain.read(),
                                value: val,
                                consumed: true,
                                created_at: 0,
                            });
                            result.set(Some("Seal consumed successfully.".to_string()));
                        }
                    },
                    class: "w-full px-4 py-2.5 rounded-lg bg-red-600 hover:bg-red-700 text-sm font-medium transition-colors",
                    "Consume Seal"
                }
            }
        }
    }
}

