//! Consume sanad page.

use crate::context::{use_wallet_context, SanadStatus, TrackedSanad};
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

#[component]
pub fn ConsumeSanad() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut sanad_id = use_signal(String::new);
    let mut result = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Sanads {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Consume Sanad" }
            }

            div { class: "bg-yellow-900/30 border border-yellow-700/50 rounded-xl p-4",
                div { class: "flex items-center gap-2",
                    span { class: "text-yellow-400", "\u{26A0}\u{FE0F}" }
                    p { class: "text-yellow-300 font-medium", "Warning: This action is irreversible" }
                }
                p { class: "text-sm text-yellow-400/80 mt-1", "Consuming a Sanad will permanently destroy it." }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Sanad ID", rsx! {
                    input {
                        value: "{sanad_id.read()}",
                        oninput: move |evt| { sanad_id.set(evt.value()); },
                        class: "{input_mono_class()}",
                        r#type: "text"
                    }
                })}

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        if let Some(sanad) = wallet_ctx.get_sanad(&sanad_id.read()) {
                            // Update status to consumed
                            wallet_ctx.add_sanad(TrackedSanad {
                                status: SanadStatus::Consumed,
                                ..sanad
                            });
                            result.set(Some("Sanad consumed successfully.".to_string()));
                        } else {
                            result.set(Some("Sanad not found.".to_string()));
                        }
                    },
                    class: "w-full px-4 py-2.5 rounded-lg bg-red-600 hover:bg-red-700 text-sm font-medium transition-colors",
                    "Consume Sanad"
                }
            }
        }
    }
}
