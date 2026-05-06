//! Cross-chain transfer detail page.

use crate::context::use_wallet_context;
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

#[component]
pub fn TransferDetail(id: String) -> Element {
    let wallet_ctx = use_wallet_context();

    // Find the transfer by ID
    let transfer = wallet_ctx.get_transfer(&id);

    let Some(t) = transfer else {
        return rsx! {
            div { class: "max-w-4xl mx-auto space-y-6",
                div { class: "flex items-center gap-3",
                    Link { to: Route::CrossChain {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                    h1 { class: "text-xl font-bold", "Transfer Not Found" }
                }
                div { class: "{card_class()} p-6",
                    p { class: "text-gray-400", "The requested transfer could not be found." }
                }
            }
        };
    };

    rsx! {
        div { class: "max-w-4xl mx-auto space-y-6",
            // Header
            div { class: "flex items-center gap-3",
                Link { to: Route::CrossChain {}, class: "{btn_secondary_class()}", "\u{2190} Back to Transfers" }
                h1 { class: "text-xl font-bold", "Transfer Details" }
            }

            // Transfer Overview Card
            div { class: "{card_class()} p-6",
                div { class: "flex items-center justify-between mb-4",
                    h2 { class: "text-lg font-semibold", "Overview" }
                    span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {transfer_status_class(&t.status)}",
                        "{t.status}"
                    }
                }

                div { class: "space-y-4",
                    // Transfer ID
                    div { class: "flex justify-between items-start",
                        span { class: "text-sm text-gray-400", "Transfer ID" }
                        div { class: "text-sanad",
                            p { class: "font-mono text-sm text-gray-200 break-all", "{t.id}" }
                        }
                    }

                    // Chain direction
                    div { class: "flex justify-between items-center",
                        span { class: "text-sm text-gray-400", "Direction" }
                        div { class: "flex items-center gap-2",
                            span { class: "{chain_badge_class(&t.from_chain)}",
                                "{chain_icon_emoji(&t.from_chain)} {chain_name(&t.from_chain)}"
                            }
                            span { class: "text-gray-500", "\u{2192}" }
                            span { class: "{chain_badge_class(&t.to_chain)}",
                                "{chain_icon_emoji(&t.to_chain)} {chain_name(&t.to_chain)}"
                            }
                        }
                    }

                    // Sanad ID
                    div { class: "flex justify-between items-start",
                        span { class: "text-sm text-gray-400", "Sanad ID" }
                        p { class: "font-mono text-sm text-gray-200 break-all max-w-md", "{t.sanad_id}" }
                    }

                    // Destination Owner
                    div { class: "flex justify-between items-start",
                        span { class: "text-sm text-gray-400", "Destination Owner" }
                        p { class: "font-mono text-sm text-gray-200 break-all max-w-md", "{t.dest_owner}" }
                    }

                    // Created At
                    div { class: "flex justify-between items-center",
                        span { class: "text-sm text-gray-400", "Created" }
                        span { class: "text-sm text-gray-300", "{format_timestamp(t.created_at)}" }
                    }
                }
            }

            // Source Chain (Lock) Details
            div { class: "{card_class()} p-6",
                div { class: "flex items-center gap-2 mb-4",
                    span { class: "{chain_badge_class(&t.from_chain)}",
                        "{chain_icon_emoji(&t.from_chain)}"
                    }
                    h2 { class: "text-lg font-semibold", "Source Chain (Lock)" }
                }

                div { class: "space-y-3",
                    // Transaction Hash with explorer link
                    div { class: "flex justify-between items-start",
                        span { class: "text-sm text-gray-400", "Transaction Hash" }
                        div { class: "text-sanad",
                            if let Some(ref hash) = t.source_tx_hash {
                                p { class: "font-mono text-sm text-gray-200 break-all max-w-md", "{hash}" }
                                if let Some(url) = wallet_ctx.get_explorer_url(t.from_chain, hash) {
                                    a {
                                        href: "{url}",
                                        target: "_blank",
                                        class: "text-xs text-blue-400 hover:text-blue-300",
                                        "View on Explorer \u{2197}"
                                    }
                                }
                            } else {
                                span { class: "text-sm text-gray-500", "N/A" }
                            }
                        }
                    }

                    // Contract Address
                    div { class: "flex justify-between items-start",
                        span { class: "text-sm text-gray-400", "Contract Address" }
                        if let Some(ref addr) = t.source_contract {
                            p { class: "font-mono text-sm text-gray-200 break-all max-w-md", "{addr}" }
                        } else {
                            span { class: "text-sm text-gray-500", "N/A" }
                        }
                    }

                    // Fee
                    div { class: "flex justify-between items-center",
                        span { class: "text-sm text-gray-400", "Fee" }
                        if let Some(ref fee) = t.source_fee {
                            span { class: "text-sm text-gray-300", "{fee}" }
                        } else {
                            span { class: "text-sm text-gray-500", "-" }
                        }
                    }
                }
            }

            // Destination Chain (Mint) Details
            div { class: "{card_class()} p-6",
                div { class: "flex items-center gap-2 mb-4",
                    span { class: "{chain_badge_class(&t.to_chain)}",
                        "{chain_icon_emoji(&t.to_chain)}"
                    }
                    h2 { class: "text-lg font-semibold", "Destination Chain (Mint)" }
                }

                div { class: "space-y-3",
                    // Transaction Hash with explorer link
                    div { class: "flex justify-between items-start",
                        span { class: "text-sm text-gray-400", "Transaction Hash" }
                        div { class: "text-sanad",
                            if let Some(ref hash) = t.dest_tx_hash {
                                p { class: "font-mono text-sm text-gray-200 break-all max-w-md", "{hash}" }
                                if let Some(url) = wallet_ctx.get_explorer_url(t.to_chain, hash) {
                                    a {
                                        href: "{url}",
                                        target: "_blank",
                                        class: "text-xs text-blue-400 hover:text-blue-300",
                                        "View on Explorer \u{2197}"
                                    }
                                }
                            } else {
                                span { class: "text-sm text-gray-500", "N/A" }
                            }
                        }
                    }

                    // Contract Address
                    div { class: "flex justify-between items-start",
                        span { class: "text-sm text-gray-400", "Contract Address" }
                        if let Some(ref addr) = t.dest_contract {
                            p { class: "font-mono text-sm text-gray-200 break-all max-w-md", "{addr}" }
                        } else {
                            span { class: "text-sm text-gray-500", "N/A" }
                        }
                    }

                    // Fee
                    div { class: "flex justify-between items-center",
                        span { class: "text-sm text-gray-400", "Fee" }
                        if let Some(ref fee) = t.dest_fee {
                            span { class: "text-sm text-gray-300", "{fee}" }
                        } else {
                            span { class: "text-sm text-gray-500", "-" }
                        }
                    }
                }
            }
        }
    }
}

/// Format timestamp for display
fn format_timestamp(timestamp: u64) -> String {
    let dt = js_sys::Date::new(&wasm_bindgen::JsValue::from_f64(timestamp as f64 * 1000.0));
    dt.to_locale_string("en-US", &js_sys::Object::new()).into()
}
