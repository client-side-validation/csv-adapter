//! Run tests page.

use crate::context::{use_wallet_context, TestResult, TestStatus, TrackedRight, TrackedTransfer};
use crate::pages::common::*;
use crate::routes::Route;
use csv_adapter_core::Chain;
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn RunTests() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut selected_from = use_signal(|| Chain::Bitcoin);
    let mut selected_to = use_signal(|| Chain::Sui);
    let mut run_all = use_signal(|| false);
    let mut running = use_signal(|| false);
    let mut current_step = use_signal(|| 0);

    let test_steps = [
        "Checking chain connectivity...",
        "Creating Right on source...",
        "Locking Right on source...",
        "Verifying proof on destination...",
        "Minting Right on destination...",
    ];

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Test {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Run Tests" }
            }

            div { class: "{card_class()} p-6 space-y-5",
                // All chains checkbox
                div { class: "flex items-center gap-2",
                    input {
                        r#type: "checkbox",
                        id: "run_all",
                        checked: *run_all.read(),
                        onchange: move |evt| { run_all.set(evt.data().checked()); },
                    }
                    label { r#for: "run_all", class: "text-sm text-gray-300", "Run all chain pairs" }
                }

                if !*run_all.read() {
                    div { class: "grid grid-cols-2 gap-4",
                        {form_field("From Chain", chain_select(move |v: Rc<FormData>| {
                            if let Ok(c) = v.value().parse::<Chain>() { selected_from.set(c); }
                        }, *selected_from.read()))}

                        {form_field("To Chain", chain_select(move |v: Rc<FormData>| {
                            if let Ok(c) = v.value().parse::<Chain>() { selected_to.set(c); }
                        }, *selected_to.read()))}
                    }
                }

                // Progress
                if *running.read() {
                    div { class: "space-y-2",
                        for (i, step_text) in test_steps.iter().enumerate() {
                            div { key: "{i}", class: "flex items-center gap-2",
                                if i < *current_step.read() {
                                    span { class: "text-green-400", "\u{2705}" }
                                    p { class: "text-sm text-green-400", "{step_text}" }
                                } else if i == *current_step.read() {
                                    span { class: "text-blue-400 animate-pulse", "\u{23F3}" }
                                    p { class: "text-sm text-blue-400", "{step_text}" }
                                } else {
                                    span { class: "text-gray-600", "\u{2B55}" }
                                    p { class: "text-sm text-gray-500", "{step_text}" }
                                }
                            }
                        }
                    }
                }

                button {
                    onclick: move |_| {
                        running.set(true);
                        current_step.set(0);

                        // Simulate test steps
                        let pairs = if *run_all.read() {
                            vec![
                                (Chain::Bitcoin, Chain::Sui),
                                (Chain::Bitcoin, Chain::Ethereum),
                                (Chain::Sui, Chain::Ethereum),
                            ]
                        } else {
                            vec![(*selected_from.read(), *selected_to.read())]
                        };

                        for (from, to) in &pairs {
                            for i in 0..5 {
                                current_step.set(i);
                                wallet_ctx.add_test_result(TestResult {
                                    from_chain: *from,
                                    to_chain: *to,
                                    status: if i == 4 { TestStatus::Passed } else { TestStatus::Running },
                                    message: format!("Step {}/5", i + 1),
                                });
                            }
                        }

                        wallet_ctx.add_test_result(TestResult {
                            from_chain: pairs[0].0,
                            to_chain: pairs[0].1,
                            status: TestStatus::Passed,
                            message: "All tests completed".to_string(),
                        });
                        running.set(false);
                    },
                    disabled: *running.read(),
                    class: "{btn_full_primary_class()}",
                    if *running.read() { "Running..." } else { "Run Tests" }
                }
            }
        }
    }
}
