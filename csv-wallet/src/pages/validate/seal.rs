//! Validate seal page.

use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

pub fn ValidateSeal() -> Element {
    let mut seal_ref = use_signal(String::new);
    let mut result = use_signal(|| Option::<bool>::None);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Validate {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Validate Seal" }
            }

            div { class: "{card_class()} p-6 space-y-5",
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
                            if *consumed { "Consumed (double-spend if reused)" } else { "Unconsumed (available)" }
                        }
                    }
                }

                button {
                    onclick: move |_| {
                        result.set(Some(!seal_ref.read().is_empty() && seal_ref.read().len() > 4));
                    },
                    class: "{btn_full_primary_class()}",
                    "Validate Seal"
                }
            }
        }
    }
}

