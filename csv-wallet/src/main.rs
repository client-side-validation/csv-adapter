//! CSV Wallet — Standalone Multi-Chain Wallet with Dioxus UI

#![warn(missing_docs)]

use dioxus::prelude::*;
use dioxus_router::*;

mod routes;
mod context;
mod wallet_core;
mod storage;
mod pages;

use routes::Route;
use context::{WalletProvider, use_wallet_context, Network, truncate_address, chain_badge_class, chain_icon_emoji, chain_name};
use csv_adapter_core::Chain;

fn main() {
    console_error_panic_hook::set_once();
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! { WalletProvider {} }
}

// ===== Sidebar Section Helper =====
fn sidebar_section(title: &str, children: Element) -> Element {
    rsx! {
        div { class: "mb-4",
            h3 { class: "px-3 mb-2 text-[10px] uppercase tracking-wider text-gray-500 font-semibold", "{title}" }
            {children}
        }
    }
}

fn sidebar_link(to: Route, icon: &str, label: &str) -> Element {
    rsx! {
        Link {
            to,
            class: "flex items-center gap-2.5 px-3 py-2 text-sm text-gray-400 hover:text-white hover:bg-gray-800 rounded-lg transition-colors",
            span { class: "text-sm", "{icon}" }
            span { "{label}" }
        }
    }
}

// ===== Layout =====
#[component]
pub fn Layout() -> Element {
    let wallet_ctx = use_wallet_context();
    let mut sidebar_open = use_signal(|| true);
    let addrs = wallet_ctx.addresses();
    let selected_chain = wallet_ctx.selected_chain();
    let selected_network = wallet_ctx.selected_network();
    let active_addr = wallet_ctx.address_for_chain(selected_chain);

    rsx! {
        div { class: "min-h-screen bg-gray-950 text-gray-100 flex",
            // Sidebar
            aside {
                class: if *sidebar_open.read() {
                    "w-64 bg-gray-900 border-r border-gray-800 flex-shrink-0 flex flex-col h-screen sticky top-0 overflow-y-auto"
                } else {
                    "w-16 bg-gray-900 border-r border-gray-800 flex-shrink-0 flex flex-col h-screen sticky top-0 overflow-y-auto"
                },

                // Sidebar header
                div { class: "px-4 py-4 border-b border-gray-800 flex items-center gap-3",
                    Link { to: Route::Dashboard {}, class: "flex items-center gap-2",
                        span { class: "text-lg", "\u{1F510}" }
                        if *sidebar_open.read() {
                            span { class: "text-lg font-bold bg-gradient-to-r from-blue-400 to-purple-500 bg-clip-text text-transparent whitespace-nowrap", "CSV Wallet" }
                        }
                    }
                    button {
                        onclick: move |_| { sidebar_open.set(!*sidebar_open.read()); },
                        class: "ml-auto text-gray-400 hover:text-white p-1 rounded hover:bg-gray-800",
                        if *sidebar_open.read() { "\u{25C0}" } else { "\u{25B6}" }
                    }
                }

                // Nav sections
                if *sidebar_open.read() {
                    nav { class: "flex-1 py-3 px-2 overflow-y-auto",
                        // Main sections
                        sidebar_section("Overview", rsx! {
                            sidebar_link(Route::Dashboard {}, "\u{1F4CA}", "Dashboard")
                        })

                        sidebar_section("Rights", rsx! {
                            sidebar_link(Route::Rights {}, "\u{1F48E}", "All Rights")
                            sidebar_link(Route::CreateRight {}, "\u{2795}", "Create Right")
                            sidebar_link(Route::ShowRight { id: String::new() }, "\u{1F441}", "Show Right")
                            sidebar_link(Route::TransferRight {}, "\u{27A1}", "Transfer Right")
                            sidebar_link(Route::ConsumeRight {}, "\u{1F525}", "Consume Right")
                        })

                        sidebar_section("Proofs", rsx! {
                            sidebar_link(Route::Proofs {}, "\u{1F4C4}", "All Proofs")
                            sidebar_link(Route::GenerateProof {}, "\u{2795}", "Generate Proof")
                            sidebar_link(Route::VerifyProof {}, "\u{2705}", "Verify Proof")
                            sidebar_link(Route::VerifyCrossChainProof {}, "\u{1F504}", "Verify Cross-Chain")
                        })

                        sidebar_section("Cross-Chain", rsx! {
                            sidebar_link(Route::CrossChain {}, "\u{21C4}", "All Transfers")
                            sidebar_link(Route::CrossChainTransfer {}, "\u{2795}", "New Transfer")
                            sidebar_link(Route::CrossChainStatus {}, "\u{1F50D}", "Status")
                            sidebar_link(Route::CrossChainRetry {}, "\u{267B}", "Retry")
                        })

                        sidebar_section("Contracts", rsx! {
                            sidebar_link(Route::Contracts {}, "\u{1F4DC}", "All Contracts")
                            sidebar_link(Route::DeployContract {}, "\u{2795}", "Deploy")
                            sidebar_link(Route::ContractStatus {}, "\u{1F4A0}", "Status")
                        })

                        sidebar_section("Seals", rsx! {
                            sidebar_link(Route::Seals {}, "\u{1F512}", "All Seals")
                            sidebar_link(Route::CreateSeal {}, "\u{2795}", "Create Seal")
                            sidebar_link(Route::ConsumeSeal {}, "\u{1F525}", "Consume Seal")
                            sidebar_link(Route::VerifySeal {}, "\u{2705}", "Verify Seal")
                        })

                        sidebar_section("Test", rsx! {
                            sidebar_link(Route::Test {}, "\u{1F9EA}", "Overview")
                            sidebar_link(Route::RunTests {}, "\u{25B6}\u{FE0F}", "Run Tests")
                            sidebar_link(Route::RunScenario {}, "\u{1F3AC}", "Run Scenario")
                        })

                        sidebar_section("Validate", rsx! {
                            sidebar_link(Route::Validate {}, "\u{2705}", "Overview")
                            sidebar_link(Route::ValidateConsignment {}, "\u{1F4C3}", "Consignment")
                            sidebar_link(Route::ValidateProof {}, "\u{1F50D}", "Proof")
                            sidebar_link(Route::ValidateSeal {}, "\u{1F512}", "Seal")
                            sidebar_link(Route::ValidateCommitmentChain {}, "\u{1F517}", "Commitment Chain")
                        })

                        // Divider
                        div { class: "border-t border-gray-800 my-3" }

                        sidebar_section("Wallet", rsx! {
                            sidebar_link(Route::GenerateWallet {}, "\u{2795}", "Generate")
                            sidebar_link(Route::ImportWalletPage {}, "\u{1F4E5}", "Import")
                            sidebar_link(Route::ExportWallet {}, "\u{1F4E4}", "Export")
                            sidebar_link(Route::ListWallets {}, "\u{1F4CB}", "List")
                        })

                        sidebar_section("", rsx! {
                            sidebar_link(Route::Settings {}, "\u{2699}\u{FE0F}", "Settings")
                        })
                    }
                } else {
                    // Collapsed sidebar - icons only
                    nav { class: "flex-1 py-3 px-2 flex flex-col items-center gap-1",
                        Link { to: Route::Dashboard {}, class: "p-2 text-gray-400 hover:text-white hover:bg-gray-800 rounded-lg transition-colors", title: "Dashboard", "\u{1F4CA}" }
                        Link { to: Route::Rights {}, class: "p-2 text-gray-400 hover:text-white hover:bg-gray-800 rounded-lg transition-colors", title: "Rights", "\u{1F48E}" }
                        Link { to: Route::Seals {}, class: "p-2 text-gray-400 hover:text-white hover:bg-gray-800 rounded-lg transition-colors", title: "Seals", "\u{1F512}" }
                        Link { to: Route::CrossChain {}, class: "p-2 text-gray-400 hover:text-white hover:bg-gray-800 rounded-lg transition-colors", title: "Cross-Chain", "\u{21C4}" }
                        Link { to: Route::Settings {}, class: "p-2 text-gray-400 hover:text-white hover:bg-gray-800 rounded-lg transition-colors", title: "Settings", "\u{2699}\u{FE0F}" }
                    }
                }
            }

            // Main content area
            div { class: "flex-1 flex flex-col min-w-0",
                // Header with chain/network selectors
                header { class: "bg-gray-900/80 backdrop-blur-sm sticky top-0 z-50 border-b border-gray-800",
                    div { class: "px-4 sm:px-6 lg:px-8",
                        div { class: "flex items-center justify-between h-16",
                            // Left: breadcrumb
                            div { class: "flex items-center gap-4",
                                span { class: "text-sm text-gray-400",
                                    "CSV Wallet"
                                    span { class: "text-gray-600", " / " }
                                    span { class: "text-gray-200 font-medium",
                                        // Dynamic breadcrumb based on route would go here
                                        "Wallet"
                                    }
                                }
                            }

                            // Right: chain selector, network selector, wallet info
                            div { class: "flex items-center gap-3",
                                // Chain selector
                                div { class: "flex items-center gap-2",
                                    span { class: "text-xs text-gray-400", "Chain:" }
                                    ChainSelector {}
                                }

                                // Network selector
                                div { class: "flex items-center gap-2",
                                    span { class: "text-xs text-gray-400", "Network:" }
                                    NetworkSelector {}
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

                // Page content
                main { class: "flex-1 px-4 sm:px-6 lg:px-8 py-6 overflow-auto",
                    Outlet::<Route> {}
                }
            }
        }
    }
}

// ===== Chain Selector Component =====
#[component]
fn ChainSelector() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let current = wallet_ctx.selected_chain();

    rsx! {
        select {
            class: "bg-gray-800 border border-gray-700 rounded-lg px-2 py-1 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
            value: "{current}",
            onchange: move |evt| {
                if let Ok(c) = evt.value().parse::<Chain>() {
                    wallet_ctx.set_selected_chain(c);
                }
            },
            for chain in [Chain::Bitcoin, Chain::Ethereum, Chain::Sui, Chain::Aptos] {
                option {
                    value: "{chain}",
                    selected: chain == current,
                    "{chain_icon_emoji(&chain)} {chain_name(&chain)}"
                }
            }
        }
    }
}

// ===== Network Selector Component =====
#[component]
fn NetworkSelector() -> Element {
    let mut wallet_ctx = use_wallet_context();
    let current = wallet_ctx.selected_network();

    rsx! {
        select {
            class: "bg-gray-800 border border-gray-700 rounded-lg px-2 py-1 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500",
            value: "{current}",
            onchange: move |evt| {
                let n = match evt.value().as_str() {
                    "dev" => Network::Dev,
                    "main" => Network::Main,
                    _ => Network::Test,
                };
                wallet_ctx.set_selected_network(n);
            },
            for network in [Network::Dev, Network::Test, Network::Main] {
                option {
                    value: "{network}",
                    selected: network == current,
                    "{network}"
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
                    Outlet::<Route> {}
                }
            }
        }
    }
}
