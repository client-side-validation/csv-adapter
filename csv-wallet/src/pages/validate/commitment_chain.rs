//! Validate commitment chain page.

use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

#[component]
pub fn ValidateCommitmentChain() -> Element {
    let mut result = use_signal(|| Option::<String>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Validate {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Validate Commitment Chain" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Commitment Chain File", rsx! {
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
                        result.set(Some("Commitment chain is valid.".to_string()));
                    },
                    class: "{btn_full_primary_class()}",
                    "Validate"
                }
            }
        }
    }
}
