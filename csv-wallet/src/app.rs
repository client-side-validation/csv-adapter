//! Root application component.
//!
//! Sets up the router, global providers, and application layout.

use dioxus::prelude::*;
use dioxus_router::*;

use crate::routes::Route;
use crate::hooks::{use_wallet, use_network, use_seals, use_assets, use_balance, use_wallet_connection};
use crate::components::{Header, Sidebar};

/// Root application component.
#[component]
pub fn App() -> Element {
    rsx! {
        // Global providers
        wallet::WalletProvider {}
        network::NetworkProvider {}
        seals::SealProvider {}
        assets::AssetProvider {}
        balance::BalanceProvider {}
        wallet_connection::WalletConnectionProvider {}

        // Router
        Router::<Route> {}
    }
}

/// Layout component wrapping all pages with header and sidebar.
#[component]
pub fn Layout() -> Element {
    rsx! {
        div { class: "min-h-screen bg-gray-950 text-gray-100 flex",
            // Sidebar navigation
            Sidebar {}

            // Main content area
            div { class: "flex-1 flex flex-col",
                // Header with network selector
                Header {}

                // Page content
                main { class: "flex-1 p-6 overflow-auto",
                    Outlet::<Route> {}
                }
            }
        }
    }
}
