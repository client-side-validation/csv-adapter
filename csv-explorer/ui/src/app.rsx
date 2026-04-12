/// Main application component with routing.

use dioxus::prelude::*;

pub mod routes {
    use dioxus_router::prelude::*;
    use super::*;

    #[derive(Routable, Clone, PartialEq)]
    #[rustfmt::skip]
    pub enum Route {
        #[route("/")]
        Home {},
        #[route("/rights")]
        RightsList {},
        #[route("/rights/:id")]
        RightDetail { id: String },
        #[route("/transfers")]
        TransfersList {},
        #[route("/transfers/:id")]
        TransferDetail { id: String },
        #[route("/seals")]
        SealsList {},
        #[route("/seals/:id")]
        SealDetail { id: String },
        #[route("/contracts")]
        ContractsList {},
        #[route("/stats")]
        Stats {},
        #[route("/chains")]
        Chains {},
        #[route("/wallet")]
        Wallet {},
    }
}

/// Root application component.
#[component]
pub fn App() -> Element {
    rsx! {
        document::Stylesheet { href: "/assets/tailwind.css" }
        document::Stylesheet { href: "/assets/styles.css" }

        div { class: "min-h-screen bg-gray-950 text-gray-100",
            // Header
            header { class: "border-b border-gray-800 bg-gray-900/80 backdrop-blur-sm sticky top-0 z-50",
                div { class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8",
                    div { class: "flex items-center justify-between h-16",
                        // Logo and nav
                        div { class: "flex items-center gap-8",
                            Link { to: routes::Route::Home {},
                                span { class: "text-xl font-bold bg-gradient-to-r from-blue-400 to-purple-500 bg-clip-text text-transparent",
                                    "CSV Explorer"
                                }
                            }
                            nav { class: "hidden md:flex gap-6",
                                NavLink { to: routes::Route::RightsList {}, class: "nav-link", "Rights" }
                                NavLink { to: routes::Route::TransfersList {}, class: "nav-link", "Transfers" }
                                NavLink { to: routes::Route::SealsList {}, class: "nav-link", "Seals" }
                                NavLink { to: routes::Route::ContractsList {}, class: "nav-link", "Contracts" }
                                NavLink { to: routes::Route::Stats {}, class: "nav-link", "Stats" }
                                NavLink { to: routes::Route::Chains {}, class: "nav-link", "Chains" }
                            }
                        }
                        // Right side: search and wallet
                        div { class: "flex items-center gap-4",
                            components::search::GlobalSearch {}
                            Link { to: routes::Route::Wallet {},
                                button { class: "px-4 py-2 rounded-lg bg-blue-600 hover:bg-blue-700 text-sm font-medium transition-colors",
                                    "Connect Wallet"
                                }
                            }
                        }
                    }
                }
            }

            // Main content
            main { class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8",
                Outlet::<routes::Route> {}
            }

            // Footer
            footer { class: "border-t border-gray-800 mt-auto",
                div { class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-6",
                    div { class: "flex items-center justify-between text-sm text-gray-500",
                        span { "CSV Explorer v0.1.0" }
                        div { class: "flex items-center gap-4",
                            span { class: "flex items-center gap-2",
                                span { class: "w-2 h-2 rounded-full bg-green-500" }
                                "All chains synced"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Navigation link with active state styling.
#[component]
fn NavLink(to: routes::Route, class: String, children: Element) -> Element {
    rsx! {
        Link {
            to,
            class: "{class} text-gray-300 hover:text-white transition-colors",
            {children}
        }
    }
}
