//! Show sanad details page.

use crate::context::use_wallet_context;
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

#[component]
pub fn ShowSanad(id: String) -> Element {
    let mut wallet_ctx = use_wallet_context();
    let sanad = wallet_ctx.get_sanad(&id);
    let mut deleted = use_signal(|| false);

    rsx! {
        div { class: "max-w-2xl space-y-6",
            div { class: "flex items-center gap-3",
                Link { to: Route::Sanads {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Sanad Details" }
            }

            if deleted() {
                div { class: "{card_class()} p-6",
                    p { class: "text-green-400", "Sanad deleted successfully." }
                    Link { to: Route::Sanads {}, class: "{btn_secondary_class()} mt-4 inline-block", "Return to Sanads" }
                }
            } else if let Some(sanad) = sanad {
                div { class: "{card_class()} overflow-hidden",
                    div { class: "{card_header_class()}",
                        h2 { class: "font-semibold text-sm", "Sanad Information" }
                    }
                    div { class: "p-6 space-y-4",
                        div {
                            p { class: "text-sm text-gray-400 mb-1", "Sanad ID" }
                            p { class: "font-mono text-sm text-gray-200 break-all", "{sanad.id}" }
                        }
                        div {
                            p { class: "text-sm text-gray-400 mb-1", "Chain" }
                            span { class: "{chain_badge_class(&sanad.chain)}", "{chain_icon_emoji(&sanad.chain)} {chain_name(&sanad.chain)}" }
                        }
                        div {
                            p { class: "text-sm text-gray-400 mb-1", "Value" }
                            p { class: "font-mono text-sm", "{sanad.value}" }
                        }
                        div {
                            p { class: "text-sm text-gray-400 mb-1", "Status" }
                            span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {sanad_status_class(&sanad.status)}",
                                "{sanad.status}"
                            }
                        }
                        div {
                            p { class: "text-sm text-gray-400 mb-1", "Owner" }
                            p { class: "font-mono text-sm text-gray-300 break-all", "{sanad.owner}" }
                        }
                    }
                    div { class: "p-6 border-t border-gray-800",
                        div { class: "flex gap-2",
                            button {
                                onclick: move |_| {
                                    wallet_ctx.remove_sanad(&sanad.id);
                                    deleted.set(true);
                                },
                                class: "px-4 py-2 rounded bg-red-600 hover:bg-red-700 text-sm font-medium transition-colors",
                                "Delete Sanad"
                            }
                        }
                    }
                }
            } else {
                div { class: "{card_class()} p-6",
                    p { class: "text-gray-400", "Sanad not found in local state." }
                    p { class: "text-sm text-gray-500 mt-1", "Enter the Sanad ID above to look it up." }
                }
            }
        }
    }
}
