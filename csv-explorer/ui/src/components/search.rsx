/// Global search component for the header.

use dioxus::prelude::*;

use crate::routes;

/// Global search input with autocomplete suggestions.
#[component]
pub fn GlobalSearch() -> Element {
    let query = use_signal(|| String::new());
    let show_results = use_signal(|| false);
    let suggestions = use_resource(move || async move {
        let q = query.read().clone();
        if q.len() < 3 {
            return Vec::new();
        }
        fetch_suggestions(&q).await
    });

    rsx! {
        div { class: "relative",
            div { class: "relative",
                span { class: "absolute left-3 top-1/2 -translate-y-1/2 text-gray-500", "🔍" }
                input {
                    r#type: "text",
                    value: "{query.read()}",
                    oninput: move |evt| {
                        query.set(evt.value());
                        show_results.set(!evt.value().is_empty());
                    },
                    onfocus: move |_| show_results.set(true),
                    onblur: move |_| {
                        // Delay to allow click events
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                        show_results.set(false);
                    },
                    placeholder: "Search...",
                    class: "w-64 bg-gray-800 border border-gray-700 rounded-lg pl-10 pr-4 py-2 text-sm text-gray-100 focus:outline-none focus:ring-2 focus:ring-blue-500"
                }
                // Keyboard shortcut hint
                span { class: "absolute right-3 top-1/2 -translate-y-1/2 text-xs text-gray-600",
                    "⌘K"
                }
            }

            // Suggestions dropdown
            if *show_results.read() && query.read().len() >= 3 {
                div { class: "absolute top-full mt-2 w-96 bg-gray-900 border border-gray-700 rounded-lg shadow-xl overflow-hidden z-50",
                    if let Some(Some(suggestions_list)) = suggestions.value() {
                        if suggestions_list.is_empty() {
                            div { class: "px-4 py-3 text-sm text-gray-500",
                                "No results found for \"{query.read()}\""
                            }
                        } else {
                            div { class: "divide-y divide-gray-800",
                                {suggestions_list.iter().map(|s| rsx! {
                                    Link {
                                        key: "{s.id}",
                                        to: s.route.clone(),
                                        class: "px-4 py-3 hover:bg-gray-800 transition-colors flex items-center justify-between",
                                        onclick: move |_| {
                                            query.set(String::new());
                                            show_results.set(false);
                                        },
                                        div {
                                            div { class: "text-sm text-gray-200 font-mono", "{s.id}" }
                                            div { class: "text-xs text-gray-500", "{s.type_}" }
                                        }
                                        span { class: "text-xs text-gray-500", "{s.chain}" }
                                    }
                                }).collect::<Vec<Element>>()}
                            }
                        }
                    } else {
                        div { class: "px-4 py-3 text-sm text-gray-500 animate-pulse",
                            "Searching..."
                        }
                    }
                }
            }
        }
    }
}

/// A search suggestion result.
#[derive(Clone)]
struct SearchSuggestion {
    id: String,
    type_: String,
    chain: String,
    route: routes::Route,
}

async fn fetch_suggestions(_query: &str) -> Vec<SearchSuggestion> {
    // In production, query the API for suggestions
    Vec::new()
}
