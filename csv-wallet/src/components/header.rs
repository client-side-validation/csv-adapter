/// Header component with chain/network selectors and wallet info.

use dioxus::prelude::*;
use crate::context::{use_wallet_context, truncate_address};
use crate::routes::Route;
use crate::components::{Dropdown, ChainDisplay, NetworkDisplay, all_chain_displays, all_network_displays};

/// Header component.
#[component]
pub fn Header(
    sidebar_open: bool,
    on_sidebar_toggle: EventHandler<()>,
) -> Element {
    let wallet_ctx = use_wallet_context();
    let selected_chain = wallet_ctx.selected_chain();
    let selected_network = wallet_ctx.selected_network();
    let active_addr = wallet_ctx.address_for_chain(selected_chain);

    rsx! {
        header { class: "bg-gray-900/80 backdrop-blur-sm sticky top-0 z-50 border-b border-gray-800",
            div { class: "px-4 sm:px-6 lg:px-8",
                div { class: "flex items-center justify-between h-16",
                    // Left: sidebar toggle and breadcrumb
                    div { class: "flex items-center gap-4",
                        button {
                            onclick: move |_| on_sidebar_toggle.call(()),
                            class: "text-gray-400 hover:text-white p-2 rounded hover:bg-gray-800",
                            if sidebar_open { "\u{25C0}" } else { "\u{25B6}" }
                        }
                        span { class: "text-sm text-gray-400",
                            "CSV Wallet"
                            span { class: "text-gray-600", " / " }
                            span { class: "text-gray-200 font-medium", "Wallet" }
                        }
                    }

                    // Right: chain selector, network selector, wallet info
                    div { class: "flex items-center gap-3",
                        // Chain selector
                        div { class: "flex items-center gap-2",
                            span { class: "text-xs text-gray-400", "Chain:" }
                            div { class: "w-40",
                                Dropdown {
                                    options: all_chain_displays(),
                                    selected: ChainDisplay(selected_chain),
                                    on_change: {
                                        let mut ctx = wallet_ctx.clone();
                                        move |cd: ChainDisplay| {
                                            ctx.set_selected_chain(cd.0);
                                        }
                                    },
                                }
                            }
                        }

                        // Network selector
                        div { class: "flex items-center gap-2",
                            span { class: "text-xs text-gray-400", "Network:" }
                            div { class: "w-28",
                                Dropdown {
                                    options: all_network_displays(),
                                    selected: NetworkDisplay(selected_network),
                                    on_change: {
                                        let mut ctx = wallet_ctx.clone();
                                        move |nd: NetworkDisplay| {
                                            ctx.set_selected_network(nd.0);
                                        }
                                    },
                                }
                            }
                        }

                        // Divider
                        div { class: "w-px h-6 bg-gray-700" }

                        // Active address
                        if let Some(addr) = active_addr {
                            div { class: "flex items-center gap-2",
                                div { class: "w-2 h-2 rounded-full bg-green-500" }
                                span { class: "font-mono text-xs text-gray-300", "{truncate_address(&addr, 4)}" }
                            }
                        }

                        // Export / Settings links
                        div { class: "flex items-center gap-1",
                            Link { to: Route::ExportWallet {}, class: "p-2 text-gray-400 hover:text-white hover:bg-gray-800 rounded-lg transition-colors", title: "Export Wallet", "\u{1F4E4}" }
                            Link { to: Route::Settings {}, class: "p-2 text-gray-400 hover:text-white hover:bg-gray-800 rounded-lg transition-colors", title: "Settings", "\u{2699}\u{FE0F}" }
                        }
                    }
                }
            }
        }
    }
}
