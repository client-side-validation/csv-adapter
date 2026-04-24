//! Run scenario page.

use crate::context::TestResult;
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;


#[component]
pub fn RunScenario() -> Element {
    let mut selected_scenario = use_signal(|| String::from("double_spend"));
    let mut result = use_signal(|| Option::<String>::None);

    let scenarios = [
        ("double_spend", "Double Spend Detection"),
        ("invalid_proof", "Invalid Proof Rejection"),
        ("ownership_transfer", "Ownership Transfer"),
    ];

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Test {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Run Scenario" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                {form_field("Scenario", rsx! {
                    select {
                        class: "{select_class()}",
                        value: "{selected_scenario.read()}",
                        onchange: move |evt| { selected_scenario.set(evt.value()); },
                        for (id, label) in scenarios {
                            option { value: "{id}", "{label}" }
                        }
                    }
                })}

                if let Some(msg) = result.read().as_ref() {
                    div { class: "p-4 bg-green-900/30 border border-green-700/50 rounded-lg",
                        p { class: "text-green-300", "{msg}" }
                    }
                }

                button {
                    onclick: move |_| {
                        result.set(Some(format!("Scenario '{}' completed successfully.", selected_scenario.read())));
                    },
                    class: "{btn_full_primary_class()}",
                    "Run Scenario"
                }
            }
        }
    }
}
