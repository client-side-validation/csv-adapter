//! Sidebar navigation component with Mode Switcher.
//!
//! Three distinct user personas: User Mode, Developer Mode, Validator Mode

use crate::routes::Route;
use dioxus::prelude::*;

/// Wallet user modes - defines the navigation structure
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum WalletMode {
    /// End users holding tokenized sanads. Simple navigation.
    #[default]
    User,
    /// Developers building on CSV protocol. Advanced tools exposed.
    Developer,
    /// Counterparties verifying proof bundles. Validation tools.
    Validator,
}

impl WalletMode {
    pub fn label(&self) -> &'static str {
        match self {
            WalletMode::User => "User",
            WalletMode::Developer => "Developer",
            WalletMode::Validator => "Validator",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            WalletMode::User => "\u{1F465}",      // 👤
            WalletMode::Developer => "\u{1F4BB}", // 💻
            WalletMode::Validator => "\u{1F50D}", // 🔍
        }
    }
}

/// Sidebar section helper.
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

/// Mode switcher component - allows switching between User/Developer/Validator modes
#[component]
fn ModeSwitcher(mode: Signal<WalletMode>) -> Element {
    let current = mode();

    rsx! {
        div { class: "px-3 py-2",
            div { class: "flex bg-gray-800 rounded-lg p-1",
                // User Mode
                button {
                    class: if current == WalletMode::User {
                        "flex-1 px-2 py-1 text-xs font-medium bg-blue-600 text-white rounded transition-colors"
                    } else {
                        "flex-1 px-2 py-1 text-xs font-medium text-gray-400 hover:text-white transition-colors"
                    },
                    onclick: move |_| mode.set(WalletMode::User),
                    span { "{WalletMode::User.icon()}" }
                    span { " User" }
                }
                // Developer Mode
                button {
                    class: if current == WalletMode::Developer {
                        "flex-1 px-2 py-1 text-xs font-medium bg-purple-600 text-white rounded transition-colors"
                    } else {
                        "flex-1 px-2 py-1 text-xs font-medium text-gray-400 hover:text-white transition-colors"
                    },
                    onclick: move |_| mode.set(WalletMode::Developer),
                    span { "{WalletMode::Developer.icon()}" }
                    span { " Dev" }
                }
                // Validator Mode
                button {
                    class: if current == WalletMode::Validator {
                        "flex-1 px-2 py-1 text-xs font-medium bg-green-600 text-white rounded transition-colors"
                    } else {
                        "flex-1 px-2 py-1 text-xs font-medium text-gray-400 hover:text-white transition-colors"
                    },
                    onclick: move |_| mode.set(WalletMode::Validator),
                    span { "{WalletMode::Validator.icon()}" }
                    span { " Valid" }
                }
            }
        }
    }
}

/// Sidebar navigation component with mode-based sections.
#[component]
pub fn Sidebar(sidebar_open: bool) -> Element {
    let open = sidebar_open;
    let mode = use_signal(WalletMode::default);

    rsx! {
        aside {
            class: if open {
                "w-64 bg-gray-900 border-r border-gray-800 flex-shrink-0 flex flex-col h-screen sticky top-0 overflow-y-auto"
            } else {
                "w-16 bg-gray-900 border-r border-gray-800 flex-shrink-0 flex flex-col h-screen sticky top-0 overflow-y-auto"
            },

            // Sidebar header
            div { class: "px-4 py-4 border-b border-gray-800 flex items-center gap-3",
                Link { to: Route::Dashboard {}, class: "flex items-center gap-2",
                    span { class: "text-lg", "\u{1F510}" }
                    if open {
                        span { class: "text-lg font-bold bg-gradient-to-r from-blue-400 to-purple-500 bg-clip-text text-transparent whitespace-nowrap", "CSV Wallet" }
                    }
                }
            }

            // Mode Switcher (only when expanded)
            if open {
                ModeSwitcher { mode }
            }

            // Nav sections - conditionally rendered based on mode
            if open {
                nav { class: "flex-1 py-3 px-2 overflow-y-auto",
                    // Mode-specific navigation sections
                    {user_mode_nav(mode())}
                    {developer_mode_nav(mode())}
                    {validator_mode_nav(mode())}

                    // Common sections for all modes
                    div { class: "border-t border-gray-800 my-3" }

                    {sidebar_section("Wallet & Settings", rsx! {
                        {sidebar_link(Route::WalletPage {}, "\u{1F510}", "Wallet")}
                        {sidebar_link(Route::Settings {}, "\u{2699}\u{FE0F}", "Settings")}
                    })}
                }
            } else {
                // Collapsed sidebar - icons only (common links)
                nav { class: "flex-1 py-3 px-2 flex flex-col items-center gap-1",
                    Link { to: Route::Dashboard {}, class: "p-2 text-gray-400 hover:text-white hover:bg-gray-800 rounded-lg transition-colors", title: "Dashboard", "\u{1F4CA}" }
                    Link { to: Route::Sanads {}, class: "p-2 text-gray-400 hover:text-white hover:bg-gray-800 rounded-lg transition-colors", title: "Sanads", "\u{1F48E}" }
                    Link { to: Route::WalletPage {}, class: "p-2 text-gray-400 hover:text-white hover:bg-gray-800 rounded-lg transition-colors", title: "Wallet", "\u{1F4B3}" }
                    Link { to: Route::Settings {}, class: "p-2 text-gray-400 hover:text-white hover:bg-gray-800 rounded-lg transition-colors", title: "Settings", "\u{2699}\u{FE0F}" }
                }
            }
        }
    }
}

/// User Mode navigation - simple, focused on assets and transfers
fn user_mode_nav(mode: WalletMode) -> Element {
    if mode != WalletMode::User {
        return rsx! {};
    }

    rsx! {
        {sidebar_section("Portfolio", rsx! {
            {sidebar_link(Route::WalletPage {}, "\u{1F4B0}", "My Assets")}
            {sidebar_link(Route::Dashboard {}, "\u{1F4CA}", "Overview")}
        })}

        {sidebar_section("My Sanads", rsx! {
            {sidebar_link(Route::Sanads {}, "\u{1F48E}", "All Sanads")}
            {sidebar_link(Route::CreateSanad {}, "\u{2795}", "Create Sanad")}
        })}

        {sidebar_section("Send / Receive", rsx! {
            {sidebar_link(Route::TransferSanad {}, "\u{1F4E4}", "Send Sanad")}
            {sidebar_link(Route::CrossChainTransfer {}, "\u{1F504}", "Cross-Chain")}
        })}

        {sidebar_section("History", rsx! {
            {sidebar_link(Route::Transactions {}, "\u{1F4B8}", "Transactions")}
            {sidebar_link(Route::CrossChain {}, "\u{21C4}", "Cross-Chain Transfers")}
        })}
    }
}

/// Developer Mode navigation - advanced tools and raw data
fn developer_mode_nav(mode: WalletMode) -> Element {
    if mode != WalletMode::Developer {
        return rsx! {};
    }

    rsx! {
        {sidebar_section("Overview", rsx! {
            {sidebar_link(Route::Dashboard {}, "\u{1F4CA}", "Dashboard")}
            {sidebar_link(Route::Transactions {}, "\u{1F4B8}", "Transactions")}
        })}

        {sidebar_section("Sanads & Assets", rsx! {
            {sidebar_link(Route::Sanads {}, "\u{1F48E}", "All Sanads")}
            {sidebar_link(Route::CreateSanad {}, "\u{2795}", "Create Sanad")}
            {sidebar_link(Route::TransferSanad {}, "\u{27A1}", "Transfer Sanad")}
            {sidebar_link(Route::ConsumeSanad {}, "\u{1F525}", "Consume Sanad")}
        })}

        {sidebar_section("Cross-Chain Transfer", rsx! {
            {sidebar_link(Route::CrossChain {}, "\u{21C4}", "All Transfers")}
            {sidebar_link(Route::CrossChainTransfer {}, "\u{2795}", "New Transfer")}
            {sidebar_link(Route::CrossChainStatus {}, "\u{1F50D}", "Status")}
        })}

        {sidebar_section("Advanced", rsx! {
            {sidebar_link(Route::Seals {}, "\u{1F512}", "Seals")}
            {sidebar_link(Route::SealRegistry {}, "\u{1F4D7}", "Seal Registry")}
            {sidebar_link(Route::Proofs {}, "\u{1F4C4}", "Proofs")}
            {sidebar_link(Route::Contracts {}, "\u{1F4DC}", "Contracts")}
        })}

        {sidebar_section("Tools", rsx! {
            {sidebar_link(Route::Test {}, "\u{1F9EA}", "Tests")}
            {sidebar_link(Route::Validate {}, "\u{2705}", "Validate")}
        })}
    }
}

/// Validator Mode navigation - verification and proof checking
fn validator_mode_nav(mode: WalletMode) -> Element {
    if mode != WalletMode::Validator {
        return rsx! {};
    }

    rsx! {
        {sidebar_section("Overview", rsx! {
            {sidebar_link(Route::Dashboard {}, "\u{1F4CA}", "Dashboard")}
        })}

        {sidebar_section("Verification", rsx! {
            {sidebar_link(Route::Proofs {}, "\u{1F4C4}", "Verify Proof")}
            {sidebar_link(Route::CrossChainStatus {}, "\u{1F50D}", "Verify Transfer")}
            {sidebar_link(Route::Seals {}, "\u{1F512}", "Check Seal")}
        })}

        {sidebar_section("Validation", rsx! {
            {sidebar_link(Route::Validate {}, "\u{2705}", "Validate Consignment")}
            {sidebar_link(Route::Proofs {}, "\u{1F4CB}", "Proof Inspector")}
        })}

        {sidebar_section("Cross-Chain", rsx! {
            {sidebar_link(Route::CrossChain {}, "\u{21C4}", "All Transfers")}
        })}
    }
}
