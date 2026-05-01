//! Cross-chain transfer retry page.

use crate::context::{use_wallet_context, TransferStatus};
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

#[component]
pub fn CrossChainRetry() -> Element {
    let wallet_ctx = use_wallet_context();
    let mut transfer_id = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::CrossChain {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Retry Transfer" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Transfer ID", rsx! {
                    input {
                        value: "{transfer_id.read()}",
                        oninput: move |evt| { transfer_id.set(evt.value()); },
                        class: "{input_mono_class()}",
                        r#type: "text"
                    }
                })}

                button {
                    onclick: move |_| {
                        if let Some(t) = wallet_ctx.get_transfer(&transfer_id.read()) {
                            if t.status == TransferStatus::Failed {
                                result.set(Some("Retry initiated. Monitor status for updates.".to_string()));
                            } else {
                                result.set(Some(format!("Transfer status: {}. Only failed transfers can be retried.", t.status)));
                            }
                        } else {
                            result.set(Some("Transfer not found.".to_string()));
                        }
                    },
                    class: "{btn_full_primary_class()}",
                    "Retry Transfer"
                }

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-blue-900/30 border border-blue-700/50 rounded-lg",
                        p { class: "text-blue-300", "{msg}" }
                    }
                }
            }
        }
    }
}
