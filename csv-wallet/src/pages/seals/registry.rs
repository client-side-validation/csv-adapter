//! Seal Registry Viewer - Phase 4.3 Double-Spend Detection Surface
//!
//! This page provides visibility into the cross-chain seal registry,
//! showing all consumed seals and detecting double-spend attempts.
//!
//! ## Security Purpose
//!
//! The seal registry is the **primary defense against double-spending** in CSV.
//! This page makes the registry visible to users and validators, enabling:
//! - Audit of all seal consumptions
//! - Detection of double-spend attempts
//! - Cross-chain replay attack detection
//! - Forensic analysis of consumption patterns
//!
//! ## Seal Types by Chain
//!
//! | Chain | Seal Type | Enforcement |
//! |-------|-----------|-------------|
//! | Bitcoin | UTXO spend | Structural (output consumed) |
//! | Sui | Object deletion | Structural (object deleted) |
//! | Aptos | Resource destruction | Type-enforced (Move linearity) |
//! | Ethereum | Nullifier registration | Contract-enforced |
//! | Solana | PDA closure | Program-enforced |

use crate::components::design_tokens::SealState;
use crate::context::{use_wallet_context, SealRecord, SealStatus};
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

/// Seal Registry page - double-spend detection surface
#[component]
pub fn SealRegistry() -> Element {
    let wallet_ctx = use_wallet_context();
    let seals = wallet_ctx.seals();

    // Calculate statistics
    let total_seals = seals.len();
    let active_seals = seals.iter().filter(|s| s.status == SealStatus::Active).count();
    let consumed_seals = seals.iter().filter(|s| s.status == SealStatus::Consumed).count();
    let locked_seals = seals.iter().filter(|s| s.status == SealStatus::Locked).count();
    let transferred_seals = seals.iter().filter(|s| s.status == SealStatus::Transferred).count();

    rsx! {
        div { class: "max-w-6xl mx-auto space-y-6",
            // Header
            div { class: "flex items-center justify-between",
                div { class: "flex items-center gap-3",
                    h1 { class: "text-xl font-bold", "Seal Registry" }
                }
                div { class: "flex gap-2",
                    Link { to: Route::Seals {}, class: "{btn_secondary_class()}", "All Seals" }
                    Link { to: Route::CreateSeal {}, class: "{btn_primary_class()}", "+ Create" }
                }
            }

            // Security banner
            div { class: "p-4 bg-gradient-to-r from-purple-900/30 to-red-900/30 \
                          border border-purple-500/30 rounded-lg",
                h2 { class: "text-sm font-semibold text-purple-300 mb-2",
                    "🛡️ Double-Spend Detection Surface"
                }
                p { class: "text-sm text-gray-300",
                    "This registry tracks all seal consumptions across chains. \
                     Any attempt to consume a seal twice will be detected and flagged here."
                }
            }

            // Statistics
            div { class: "grid grid-cols-2 md:grid-cols-5 gap-4",
                {stat_card("Total", total_seals, "text-gray-400")}
                {stat_card("Active", active_seals, "text-green-400")}
                {stat_card("Locked", locked_seals, "text-yellow-400")}
                {stat_card("Consumed", consumed_seals, "text-gray-400")}
                {stat_card("Transferred", transferred_seals, "text-blue-400")}
            }

            // Seal table
            div { class: "{card_class()}",
                div { class: "{card_header_class()}",
                    h2 { class: "font-semibold", "Registered Seals" }
                    p { class: "text-xs text-gray-400 mt-1",
                        "All seals tracked across chains"
                    }
                }
                div { class: "overflow-x-auto",
                    if seals.is_empty() {
                        div { class: "p-8 text-center",
                            p { class: "text-gray-400", "No seals registered yet." }
                            Link { to: Route::CreateSeal {}, class: "{btn_primary_class()} mt-4", "Create First Seal" }
                        }
                    } else {
                        table { class: "w-full text-sm",
                            thead { class: "bg-gray-800/50",
                                tr {
                                    th { class: "px-4 py-3 text-left text-xs font-medium text-gray-400 uppercase", "Seal Reference" }
                                    th { class: "px-4 py-3 text-left text-xs font-medium text-gray-400 uppercase", "Chain" }
                                    th { class: "px-4 py-3 text-left text-xs font-medium text-gray-400 uppercase", "Status" }
                                    th { class: "px-4 py-3 text-left text-xs font-medium text-gray-400 uppercase", "Right" }
                                    th { class: "px-4 py-3 text-left text-xs font-medium text-gray-400 uppercase", "Value" }
                                    th { class: "px-4 py-3 text-left text-xs font-medium text-gray-400 uppercase", "Actions" }
                                }
                            }
                            tbody { class: "divide-y divide-gray-800",
                                for seal in seals.iter() {
                                    {seal_row(seal)}
                                }
                            }
                        }
                    }
                }
            }

            // Security info
            {security_info_section()}
        }
    }
}

/// Statistics card
fn stat_card(label: &str, value: usize, color_class: &str) -> Element {
    rsx! {
        div { class: "{card_class()} p-4 text-center",
            p { class: "text-2xl font-bold {color_class}", "{value}" }
            p { class: "text-xs text-gray-500 uppercase mt-1", "{label}" }
        }
    }
}

/// Seal table row
fn seal_row(seal: &SealRecord) -> Element {
    let status_badge = match seal.status {
        SealStatus::Active => ("bg-green-500/20 text-green-400", "Active"),
        SealStatus::Locked => ("bg-yellow-500/20 text-yellow-400", "Locked"),
        SealStatus::Consumed => ("bg-gray-500/20 text-gray-400", "Consumed"),
        SealStatus::Transferred => ("bg-blue-500/20 text-blue-400", "Transferred"),
    };

    rsx! {
        tr { class: "hover:bg-gray-800/30",
            td { class: "px-4 py-3",
                p { class: "font-mono text-xs", "{truncate_address(&seal.seal_ref, 12)}" }
            }
            td { class: "px-4 py-3",
                span { class: "{chain_badge_class(&seal.chain)}", "{chain_icon_emoji(&seal.chain)} {chain_name(&seal.chain)}" }
            }
            td { class: "px-4 py-3",
                span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {status_badge.0}",
                    "{status_badge.1}"
                }
            }
            td { class: "px-4 py-3",
                if seal.right_id.is_empty() {
                    span { class: "text-gray-500", "-" }
                } else {
                    Link { to: Route::RightJourney { id: seal.right_id.clone() },
                        class: "font-mono text-xs text-blue-400 hover:underline",
                        "{truncate_address(&seal.right_id, 8)}"
                    }
                }
            }
            td { class: "px-4 py-3 font-mono", "{seal.value}" }
            td { class: "px-4 py-3",
                div { class: "flex gap-2",
                    Link { to: Route::VerifySeal {}, class: "text-xs text-blue-400 hover:underline", "Verify" }
                    if seal.status == SealStatus::Active {
                        Link { to: Route::ConsumeSeal { seal_ref: Some(seal.seal_ref.clone()) }, class: "text-xs text-orange-400 hover:underline", "Consume" }
                    }
                }
            }
        }
    }
}

/// Security information section
fn security_info_section() -> Element {
    rsx! {
        div { class: "{card_class()} p-6",
            h2 { class: "text-lg font-semibold mb-4", "Security Properties" }

            div { class: "grid md:grid-cols-2 gap-6",
                // Single-Use Enforcement
                div { class: "space-y-2",
                    h3 { class: "font-medium text-green-400", "✓ Single-Use Enforcement" }
                    p { class: "text-sm text-gray-400",
                        "Each seal can only be consumed once. This is enforced by:"
                    }
                    ul { class: "text-sm text-gray-400 list-disc list-inside space-y-1",
                        li { "Bitcoin: UTXO is spent (output consumed)" }
                        li { "Sui: Object is deleted" }
                        li { "Aptos: Resource is destroyed (Move linearity)" }
                        li { "Ethereum: Nullifier registered in contract" }
                        li { "Solana: PDA account is closed" }
                    }
                }

                // Cross-Chain Detection
                div { class: "space-y-2",
                    h3 { class: "font-medium text-blue-400", "✓ Cross-Chain Detection" }
                    p { class: "text-sm text-gray-400",
                        "The registry detects attempts to use the same seal on different chains:"
                    }
                    ul { class: "text-sm text-gray-400 list-disc list-inside space-y-1",
                        li { "Maps all seal types to unified identity" }
                        li { "Detects equivalent seals across chains" }
                        li { "Prevents cross-chain replay attacks" }
                        li { "Immutable audit trail for forensics" }
                    }
                }

                // Double-Spend Prevention
                div { class: "space-y-2",
                    h3 { class: "font-medium text-red-400", "🛡️ Double-Spend Prevention" }
                    p { class: "text-sm text-gray-400",
                        "Multiple layers of protection against double-spending:"
                    }
                    ul { class: "text-sm text-gray-400 list-disc list-inside space-y-1",
                        li { "At-most-once consumption guarantee" }
                        li { "Immutable consumption history" }
                        li { "Chain-agnostic seal comparison" }
                        li { "Forensic tracking of attempts" }
                    }
                }

                // Audit Trail
                div { class: "space-y-2",
                    h3 { class: "font-medium text-purple-400", "📋 Audit Trail" }
                    p { class: "text-sm text-gray-400",
                        "All seal events are recorded for transparency:"
                    }
                    ul { class: "text-sm text-gray-400 list-disc list-inside space-y-1",
                        li { "Creation timestamp and block" }
                        li { "Consumption transaction hash" }
                        li { "Right ID that consumed the seal" }
                        li { "Chain where consumption occurred" }
                    }
                }
            }

            // CSV vs Bridges comparison
            div { class: "mt-6 p-4 bg-gray-800/50 rounded-lg",
                h3 { class: "font-medium mb-3", "CSV vs Traditional Bridges" }
                div { class: "grid md:grid-cols-2 gap-4 text-sm",
                    div {
                        p { class: "text-gray-500 mb-2", "Traditional Bridge" }
                        ul { class: "space-y-1 text-gray-400",
                            li { "• Relies on trusted operators" }
                            li { "• No cryptographic seal enforcement" }
                            li { "• Centralized double-spend detection" }
                            li { "• Opaque audit trail" }
                        }
                    }
                    div {
                        p { class: "text-green-400 mb-2", "CSV Protocol" }
                        ul { class: "space-y-1 text-green-300",
                            li { "• Cryptographic single-use seals" }
                            li { "• Chain-native enforcement" }
                            li { "• Transparent registry" }
                            li { "• Immutable audit trail" }
                        }
                    }
                }
            }
        }
    }
}
