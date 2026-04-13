//! CSV Wallet — Standalone Multi-Chain Wallet with Dioxus UI

#![warn(missing_docs)]

use dioxus::prelude::*;

mod routes;
mod context;
mod wallet_core;
mod storage;
mod pages;
mod components;

use routes::Route;
use context::WalletProvider;
use components::{Sidebar, Header};

fn main() {
    console_error_panic_hook::set_once();
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        // Global stylesheet
        document::Link { rel: "stylesheet", href: "/style.css" }

        WalletProvider {}
    }
}

// ===== Layout =====
#[component]
pub fn Layout() -> Element {
    let mut sidebar_open = use_signal(|| true);

    rsx! {
        div { class: "min-h-screen bg-gray-950 text-gray-100 flex",
            Sidebar { sidebar_open: *sidebar_open.read() }

            // Main content area
            div { class: "flex-1 flex flex-col min-w-0",
                Header {
                    sidebar_open: *sidebar_open.read(),
                    on_sidebar_toggle: move |_| {
                        let open = *sidebar_open.read();
                        sidebar_open.set(!open);
                    },
                }

                // Page content with fade-in transition
                main { class: "flex-1 px-4 sm:px-6 lg:px-8 py-6 overflow-auto",
                    div { class: "page-enter",
                        Outlet::<Route> {}
                    }
                }
            }
        }
    }
}

// ===== Auth Layout =====
#[component]
pub fn AuthLayout() -> Element {
    rsx! {
        div { class: "min-h-screen bg-gray-950 text-gray-100",
            div { class: "absolute inset-0 bg-gradient-to-br from-gray-950 via-gray-900 to-gray-950" }
            div { class: "relative flex items-center justify-center min-h-screen p-4",
                div { class: "w-full max-w-lg",
                    div { class: "text-center mb-8",
                        h1 { class: "text-3xl font-bold bg-gradient-to-r from-blue-400 to-purple-500 bg-clip-text text-transparent",
                            "\u{1F510} CSV Wallet"
                        }
                        p { class: "mt-2 text-gray-400", "Multi-chain wallet for Client-Side Validation" }
                    }
                    // Page content with fade-in transition
                    div { class: "page-enter",
                        Outlet::<Route> {}
                    }
                }
            }
        }
    }
}
