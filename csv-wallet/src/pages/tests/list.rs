//! Tests list page.

use crate::context::{use_wallet_context, TestResult, TestStatus};
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

fn stat_card(label: &str, value: &str, icon: &str) -> Element {
    rsx! {
        div { class: "{card_class()} p-5",
            div { class: "flex items-center justify-between",
                div {
                    p { class: "text-xs text-gray-400", "{label}" }
                    p { class: "text-xl font-bold", "{value}" }
                }
                span { class: "text-2xl", "{icon}" }
            }
        }
    }
}

// ===== Test Pages =====
#[component]
pub fn Test() -> Element {
    let wallet_ctx = use_wallet_context();
    let results = wallet_ctx.test_results();
    let passed = results
        .iter()
        .filter(|r| r.status == TestStatus::Passed)
        .count();
    let failed = results
        .iter()
        .filter(|r| r.status == TestStatus::Failed)
        .count();

    rsx! {
        div { class: "space-y-6",
            h1 { class: "text-2xl font-bold", "Tests" }

            if !results.is_empty() {
                div { class: "grid grid-cols-1 sm:grid-cols-3 gap-4",
                    {stat_card("Total", &results.len().to_string(), "\u{1F9EA}")}
                    {stat_card("Passed", &passed.to_string(), "\u{2705}")}
                    {stat_card("Failed", &failed.to_string(), "\u{274C}")}
                }
            }

            div { class: "grid grid-cols-1 sm:grid-cols-2 gap-4",
                Link { to: Route::RunTests {}, class: "{card_class()} p-6 hover:bg-gray-800/50 transition-colors block",
                    div { class: "flex items-center gap-3", span { class: "text-2xl", "\u{25B6}\u{FE0F}" }, div { h3 { class: "font-semibold", "Run Tests" } p { class: "text-sm text-gray-400", "Run end-to-end chain tests" } } }
                }
                Link { to: Route::RunScenario {}, class: "{card_class()} p-6 hover:bg-gray-800/50 transition-colors block",
                    div { class: "flex items-center gap-3", span { class: "text-2xl", "\u{1F3AC}" }, div { h3 { class: "font-semibold", "Run Scenario" } p { class: "text-sm text-gray-400", "Run specific test scenarios" } } }
                }
            }

            if !results.is_empty() {
                div { class: "{table_class()}",
                    div { class: "{card_header_class()}",
                        h2 { class: "font-semibold text-sm", "Test Results" }
                    }
                    div { class: "overflow-x-auto",
                        table { class: "w-full text-sm",
                            thead {
                                tr { class: "text-left text-gray-400 border-b border-gray-800",
                                    th { class: "px-4 py-2 font-medium", "From" }
                                    th { class: "px-4 py-2 font-medium", "To" }
                                    th { class: "px-4 py-2 font-medium", "Status" }
                                    th { class: "px-4 py-2 font-medium", "Message" }
                                }
                            }
                            tbody { class: "divide-y divide-gray-800",
                                for r in results {
                                    tr { class: "hover:bg-gray-800/50 transition-colors",
                                        td { class: "px-4 py-3", span { class: "{chain_badge_class(&r.from_chain)}", "{chain_icon_emoji(&r.from_chain)}" } }
                                        td { class: "px-4 py-3", span { class: "{chain_badge_class(&r.to_chain)}", "{chain_icon_emoji(&r.to_chain)}" } }
                                        td { class: "px-4 py-3",
                                            span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {test_status_class(&r.status)}",
                                                "{r.status}"
                                            }
                                        }
                                        td { class: "px-4 py-3 text-xs text-gray-400", "{r.message}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
