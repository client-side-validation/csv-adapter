/// Sidebar navigation component.

use dioxus::prelude::*;
use crate::routes::Route;

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

/// Sidebar navigation component.
#[component]
pub fn Sidebar(sidebar_open: bool) -> Element {
    let open = sidebar_open;

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

            // Nav sections
            if open {
                nav { class: "flex-1 py-3 px-2 overflow-y-auto",
                    {sidebar_section("Overview", rsx! {
                        {sidebar_link(Route::Dashboard {}, "\u{1F4CA}", "Dashboard")}
                    })}

                    {sidebar_section("Rights", rsx! {
                        {sidebar_link(Route::Rights {}, "\u{1F48E}", "All Rights")}
                        {sidebar_link(Route::CreateRight {}, "\u{2795}", "Create Right")}
                        {sidebar_link(Route::TransferRight {}, "\u{27A1}", "Transfer Right")}
                        {sidebar_link(Route::ConsumeRight {}, "\u{1F525}", "Consume Right")}
                    })}

                    {sidebar_section("Proofs", rsx! {
                        {sidebar_link(Route::Proofs {}, "\u{1F4C4}", "All Proofs")}
                        {sidebar_link(Route::GenerateProof {}, "\u{2795}", "Generate Proof")}
                        {sidebar_link(Route::VerifyProof {}, "\u{2705}", "Verify Proof")}
                        {sidebar_link(Route::VerifyCrossChainProof {}, "\u{1F504}", "Verify Cross-Chain")}
                    })}

                    {sidebar_section("Cross-Chain", rsx! {
                        {sidebar_link(Route::CrossChain {}, "\u{21C4}", "All Transfers")}
                        {sidebar_link(Route::CrossChainTransfer {}, "\u{2795}", "New Transfer")}
                        {sidebar_link(Route::CrossChainStatus {}, "\u{1F50D}", "Status")}
                    })}

                    {sidebar_section("Contracts", rsx! {
                        {sidebar_link(Route::Contracts {}, "\u{1F4DC}", "All Contracts")}
                        {sidebar_link(Route::DeployContract {}, "\u{2795}", "Deploy")}
                    })}

                    {sidebar_section("Seals", rsx! {
                        {sidebar_link(Route::Seals {}, "\u{1F512}", "All Seals")}
                        {sidebar_link(Route::CreateSeal {}, "\u{2795}", "Create Seal")}
                        {sidebar_link(Route::ConsumeSeal {}, "\u{1F525}", "Consume Seal")}
                    })}

                    {sidebar_section("Test & Validate", rsx! {
                        {sidebar_link(Route::Test {}, "\u{1F9EA}", "Tests")}
                        {sidebar_link(Route::Validate {}, "\u{2705}", "Validate")}
                    })}

                    div { class: "border-t border-gray-800 my-3" }

                    {sidebar_section("Wallet & Settings", rsx! {
                        {sidebar_link(Route::WalletPage {}, "\u{1F510}", "Wallet")}
                        {sidebar_link(Route::Settings {}, "\u{2699}\u{FE0F}", "Settings")}
                    })}
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
    }
}
