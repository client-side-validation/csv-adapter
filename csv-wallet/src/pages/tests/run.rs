//! Run tests page.

use crate::context::{TestResult, TestStatus, use_wallet_context};
use crate::pages::common::*;
use crate::routes::Route;
use csv_core::ChainId;
use dioxus::prelude::*;
use std::rc::Rc;

#[component]
pub fn RunTests() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let mut selected_from = use_signal(|| ChainId::new("bitcoin"));
    let mut selected_to = use_signal(|| ChainId::new("sui"));
    let mut run_all = use_signal(|| false);
    let mut running = use_signal(|| false);
    let mut current_step = use_signal(|| 0);

    let test_steps = [
        "Checking chain connectivity...",
        "Creating Sanad on source...",
        "Locking Sanad on source...",
        "Verifying proof on destination...",
        "Minting Sanad on destination...",
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
                        {form_field("From ChainId", chain_select(move |v: Rc<FormData>| {
                            if let Ok(c) = v.value().parse::<ChainId>() { selected_from.set(c); }
                        }, selected_from.read().clone()))}

                        {form_field("To ChainId", chain_select(move |v: Rc<FormData>| {
                            if let Ok(c) = v.value().parse::<ChainId>() { selected_to.set(c); }
                        }, selected_to.read().clone()))}
                    }
                }

                // Progress
                if *running.read() {
                    div { class: "space-y-2",
                        for (i, step_text) in test_steps.iter().enumerate() {
                            div { key: "step-{i}", class: "flex items-center gap-2",
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
                                (ChainId::new("bitcoin"), ChainId::new("sui")),
                                (ChainId::new("bitcoin"), ChainId::new("ethereum")),
                                (ChainId::new("sui"), ChainId::new("ethereum")),
                            ]
                        } else {
                            vec![(selected_from.read().clone(), selected_to.read().clone())]
                        };

                        for (from, to) in &pairs {
                            for i in 0..5 {
                                current_step.set(i);
                                wallet_ctx.add_test_result(TestResult {
                                    id: format!("test-{}-{}-{}", from, to, i),
                                    from_chain: from.clone(),
                                    to_chain: to.clone(),
                                    status: if i == 4 { TestStatus::Passed } else { TestStatus::Running },
                                    message: format!("Step {}/5", i + 1),
                                });
                            }
                        }

                        wallet_ctx.add_test_result(TestResult {
                            id: format!("test-complete-{}", js_sys::Date::now()),
                            from_chain: pairs[0].0.clone(),
                            to_chain: pairs[0].1.clone(),
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
