//! Cross-chain transfer status page.

use crate::context::{use_wallet_context, TrackedTransfer};
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

#[component]
pub fn CrossChainStatus() -> Element {
    let wallet_ctx = use_wallet_context();
    let mut transfer_id = use_signal(String::new);
    let mut result = use_signal(|| Option::<TrackedTransfer>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::CrossChain {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Transfer Status" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Transfer ID", rsx! {
                    input {
                        value: "{transfer_id.read()}",
                        oninput: move |evt| { transfer_id.set(evt.value()); },
                        class: "{input_mono_class()}",
                        placeholder: "0x..."
                    }
                })}

                button {
                    onclick: move |_| {
                        result.set(wallet_ctx.get_transfer(&transfer_id.read()));
                    },
                    class: "{btn_full_primary_class()}",
                    "Check Status"
                }

                if let Some(t) = result.read().as_ref() {
                    div { class: "space-y-3",
                        div { class: "bg-gray-800/50 rounded-lg p-4 border border-gray-700 space-y-2",
                            div { class: "flex justify-between",
                                span { class: "text-sm text-gray-400", "Source" }
                                span { class: "{chain_badge_class(&t.from_chain)}", "{chain_icon_emoji(&t.from_chain)} {chain_name(&t.from_chain)}" }
                            }
                            div { class: "flex justify-between",
                                span { class: "text-sm text-gray-400", "Destination" }
                                span { class: "{chain_badge_class(&t.to_chain)}", "{chain_icon_emoji(&t.to_chain)} {chain_name(&t.to_chain)}" }
                            }
                            div { class: "flex justify-between",
                                span { class: "text-sm text-gray-400", "Status" }
                                span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {transfer_status_class(&t.status)}",
                                    "{t.status}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
