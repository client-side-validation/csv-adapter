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
