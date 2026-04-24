//! Show right details page.

use crate::context::{use_wallet_context};
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

#[component]
pub fn ShowRight(id: String) -> Element {
    let mut wallet_ctx = use_wallet_context();
    let right = wallet_ctx.get_right(&id);
    let mut deleted = use_signal(|| false);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Rights {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Right Details" }
            }

            if deleted() {
                div { class: "{card_class()} p-6",
                    p { class: "text-green-400", "Right deleted successfully." }
                    Link { to: Route::Rights {}, class: "{btn_secondary_class()} mt-4 inline-block", "Return to Rights" }
                }
            } else if let Some(right) = right {
                div { class: "{card_class()} overflow-hidden",
                    div { class: "{card_header_class()}",
                        h2 { class: "font-semibold text-sm", "Right Information" }
                    }
                    div { class: "p-6 space-y-4",
                        div {
                            p { class: "text-sm text-gray-400 mb-1", "Right ID" }
                            p { class: "font-mono text-sm text-gray-200 break-all", "{right.id}" }
                        }
                        div {
                            p { class: "text-sm text-gray-400 mb-1", "Chain" }
                            span { class: "{chain_badge_class(&right.chain)}", "{chain_icon_emoji(&right.chain)} {chain_name(&right.chain)}" }
                        }
                        div {
                            p { class: "text-sm text-gray-400 mb-1", "Value" }
                            p { class: "font-mono text-sm", "{right.value}" }
                        }
                        div {
                            p { class: "text-sm text-gray-400 mb-1", "Status" }
                            span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {right_status_class(&right.status)}",
                                "{right.status}"
                            }
                        }
                        div {
                            p { class: "text-sm text-gray-400 mb-1", "Owner" }
                            p { class: "font-mono text-sm text-gray-300 break-all", "{right.owner}" }
                        }
                    }
                    div { class: "p-6 border-t border-gray-800",
                        div { class: "flex gap-2",
                            button {
                                onclick: move |_| {
                                    wallet_ctx.remove_right(&right.id);
                                    deleted.set(true);
                                },
                                class: "px-4 py-2 rounded bg-red-600 hover:bg-red-700 text-sm font-medium transition-colors",
                                "Delete Right"
                            }
                        }
                    }
                }
            } else {
                div { class: "{card_class()} p-6",
                    p { class: "text-gray-400", "Right not found in local state." }
                    p { class: "text-sm text-gray-500 mt-1", "Enter the Right ID above to look it up." }
                }
            }
        }
    }
}
