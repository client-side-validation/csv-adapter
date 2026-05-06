//! Proof Bundle Viewer - Phase 4.1 Competitive Advantage
//!
//! This page displays a ProofBundle as a first-class UI object.
//! Users can see:
//! - Source chain anchor (tx hash, block height, finality status)
//! - Inclusion proof (Merkle branch, root)
//! - Finality proof (confirmations, checkpoint)
//! - Seal reference consumed (real chain-native ID)
//! - Commitment hash
//! - Sanad ID before and after
//!
//! Competitive advantage: This proof can be exported and verified OFFLINE
//! without trusting any server. Traditional bridges give you a receipt;
//! CSV gives you a cryptographic proof.

use crate::context::{use_wallet_context, ProofRecord, ProofStatus};
use crate::pages::common::*;
use crate::routes::Route;
use dioxus::prelude::*;

/// Proof Bundle detail page - shows all components of a proof
#[component]
pub fn ProofBundlePage(id: String) -> Element {
    let wallet_ctx = use_wallet_context();
    // Find proof by seal_ref (which is used as the proof ID)
    let proof = wallet_ctx.proof_for_seal(&id);
    let mut show_export_modal = use_signal(|| false);
    let export_format = use_signal(|| ExportFormat::Json);

    rsx! {
        div { class: "max-w-4xl mx-auto space-y-6",
            // Header
            div { class: "flex items-center justify-between",
                div { class: "flex items-center gap-3",
                    Link { to: Route::Proofs {}, class: "{btn_secondary_class()}", "← Back" }
                    h1 { class: "text-xl font-bold", "Proof Bundle" }
                }
                if proof.is_some() {
                    button {
                        class: "{btn_primary_class()}",
                        onclick: move |_| show_export_modal.set(true),
                        "📥 Export"
                    }
                }
            }

            if let Some(proof) = proof {
                // Proof bundle overview
                {proof_overview_section(&proof)}

                // Chain anchor details
                {anchor_section(&proof)}

                // Seal reference (the consumed seal)
                {seal_section(&proof)}

                // Inclusion proof
                {inclusion_proof_section(&proof)}

                // Finality proof
                {finality_section(&proof)}

                // Verification actions
                {verification_actions(&proof)}
            } else {
                div { class: "{card_class()} p-6",
                    p { class: "text-gray-400", "Proof not found." }
                }
            }

            // Export modal
            if show_export_modal() {
                {export_modal(&id, show_export_modal, export_format)}
            }
        }
    }
}

/// Proof bundle overview section
fn proof_overview_section(proof: &ProofRecord) -> Element {
    let status_color = match proof.status {
        ProofStatus::Verified => "var(--proof-valid)",
        ProofStatus::Invalid => "var(--proof-invalid)",
        ProofStatus::Pending => "var(--proof-pending)",
        ProofStatus::Generated => "var(--proof-unverified)",
    };

    rsx! {
        div { class: "{card_class()} p-6",
            h2 { class: "text-lg font-semibold mb-4", "Proof Overview" }

            div { class: "grid grid-cols-2 gap-4",
                div { class: "space-y-1",
                    label { class: "text-xs text-gray-500 uppercase", "Proof ID" }
                    p { class: "font-mono text-sm", "{&proof.seal_ref}" }
                }
                div { class: "space-y-1",
                    label { class: "text-xs text-gray-500 uppercase", "Chain" }
                    p { class: "font-medium", "{proof.chain.to_string()}" }
                }
                div { class: "space-y-1",
                    label { class: "text-xs text-gray-500 uppercase", "Sanad ID" }
                    p { class: "font-mono text-sm", "{&proof.sanad_id}" }
                }
                div { class: "space-y-1",
                    label { class: "text-xs text-gray-500 uppercase", "Status" }
                    p { class: "font-medium flex items-center gap-2",
                        span { style: "color: {status_color}", "●" }
                        "{proof.status.to_string()}"
                    }
                }
            }
        }
    }
}

/// Anchor section - shows the on-chain commitment
fn anchor_section(proof: &ProofRecord) -> Element {
    rsx! {
        div { class: "{card_class()} p-6",
            h2 { class: "text-lg font-semibold mb-4 flex items-center gap-2",
                "🔗 "
                "Chain Anchor"
            }

            div { class: "space-y-4",
                div { class: "p-3 bg-gray-800/50 rounded-lg",
                    p { class: "text-xs text-gray-500 mb-1", "Source Chain" }
                    p { class: "font-medium", "{proof.chain.to_string()}" }
                }

                if let Some(ref tx_hash) = proof.verification_tx_hash {
                    div { class: "p-3 bg-gray-800/50 rounded-lg",
                        p { class: "text-xs text-gray-500 mb-1", "Transaction Hash" }
                        p { class: "font-mono text-sm break-all", "{tx_hash}" }
                    }
                }

                if let Some(ref target) = proof.target_chain {
                    div { class: "p-3 bg-gray-800/50 rounded-lg",
                        p { class: "text-xs text-gray-500 mb-1", "Target Chain" }
                        p { class: "font-medium", "{target.to_string()}" }
                    }
                }

                // Competitive advantage callout
                div { class: "p-3 bg-blue-900/20 border border-blue-500/30 rounded-lg",
                    p { class: "text-sm text-blue-300",
                        "✓ This proof is anchored on-chain. Anyone can verify it without trusting a server."
                    }
                }
            }
        }
    }
}

/// Seal section - shows the consumed seal reference
fn seal_section(proof: &ProofRecord) -> Element {
    rsx! {
        div { class: "{card_class()} p-6",
            h2 { class: "text-lg font-semibold mb-4 flex items-center gap-2",
                "🔒 "
                "Seal Reference"
            }

            div { class: "space-y-4",
                div { class: "p-3 bg-gray-800/50 rounded-lg",
                    p { class: "text-xs text-gray-500 mb-1", "Seal ID (Chain-Native)" }
                    p { class: "font-mono text-sm break-all", "{&proof.seal_ref}" }
                }

                div { class: "p-3 bg-gray-800/50 rounded-lg",
                    p { class: "text-xs text-gray-500 mb-1", "Chain" }
                    p { class: "font-medium", "{proof.chain.to_string()}" }
                }

                // Single-use seal explanation
                div { class: "p-3 bg-purple-900/20 border border-purple-500/30 rounded-lg",
                    p { class: "text-sm text-purple-300",
                        "✓ This seal was consumed when the proof was generated. \
                         Single-use seals prevent double-spends cryptographically."
                    }
                }
            }
        }
    }
}

/// Inclusion proof section
fn inclusion_proof_section(proof: &ProofRecord) -> Element {
    rsx! {
        div { class: "{card_class()} p-6",
            h2 { class: "text-lg font-semibold mb-4 flex items-center gap-2",
                "🌳 "
                "Inclusion Proof"
            }

            if let Some(ref data) = proof.data {
                div { class: "space-y-4",
                    {match data {
                        crate::context::ProofData::Merkle { root, path, leaf_index } => rsx! {
                            div { class: "p-3 bg-gray-800/50 rounded-lg",
                                p { class: "text-xs text-gray-500 mb-1", "Merkle Root" }
                                p { class: "font-mono text-sm break-all", "{root}" }
                            }
                            div { class: "p-3 bg-gray-800/50 rounded-lg",
                                p { class: "text-xs text-gray-500 mb-1", "Proof Path Depth" }
                                p { class: "font-medium", "{path.len()} siblings" }
                            }
                            div { class: "p-3 bg-gray-800/50 rounded-lg",
                                p { class: "text-xs text-gray-500 mb-1", "Leaf Index" }
                                p { class: "font-medium", "{leaf_index}" }
                            }
                        },
                        crate::context::ProofData::Mpt { root, .. } => rsx! {
                            div { class: "p-3 bg-gray-800/50 rounded-lg",
                                p { class: "text-xs text-gray-500 mb-1", "MPT Root" }
                                p { class: "font-mono text-sm break-all", "{root}" }
                            }
                        },
                        crate::context::ProofData::Checkpoint { sequence, digest, .. } => rsx! {
                            div { class: "p-3 bg-gray-800/50 rounded-lg",
                                p { class: "text-xs text-gray-500 mb-1", "Checkpoint" }
                                p { class: "font-medium", "#{sequence}" }
                                p { class: "font-mono text-xs break-all mt-1", "{digest}" }
                            }
                        },
                        crate::context::ProofData::Ledger { version, .. } => rsx! {
                            div { class: "p-3 bg-gray-800/50 rounded-lg",
                                p { class: "text-xs text-gray-500 mb-1", "Ledger Version" }
                                p { class: "font-medium", "{version}" }
                            }
                        },
                        crate::context::ProofData::Solana { slot, .. } => rsx! {
                            div { class: "p-3 bg-gray-800/50 rounded-lg",
                                p { class: "text-xs text-gray-500 mb-1", "Slot" }
                                p { class: "font-medium", "{slot}" }
                            }
                        },
                        crate::context::ProofData::Zk { proof_system, block_height, .. } => rsx! {
                            div { class: "p-3 bg-gray-800/50 rounded-lg",
                                p { class: "text-xs text-gray-500 mb-1", "ZK Proof System" }
                                p { class: "font-medium", "{proof_system}" }
                            }
                            div { class: "p-3 bg-gray-800/50 rounded-lg",
                                p { class: "text-xs text-gray-500 mb-1", "Block Height" }
                                p { class: "font-medium", "{block_height}" }
                            }
                        },
                    }}

                    // Inclusion proof explanation
                    div { class: "p-3 bg-green-900/20 border border-green-500/30 rounded-lg",
                        p { class: "text-sm text-green-300",
                            "✓ This inclusion proof cryptographically verifies the commitment \
                             was included in a specific block. No RPC call needed for verification."
                        }
                    }
                }
            } else {
                p { class: "text-gray-400", "No inclusion proof data available." }
            }
        }
    }
}

/// Finality section
fn finality_section(proof: &ProofRecord) -> Element {
    rsx! {
        div { class: "{card_class()} p-6",
            h2 { class: "text-lg font-semibold mb-4 flex items-center gap-2",
                "✓ "
                "Finality"
            }

            div { class: "space-y-4",
                div { class: "p-3 bg-gray-800/50 rounded-lg",
                    p { class: "text-xs text-gray-500 mb-1", "Status" }
                    p { class: "font-medium flex items-center gap-2",
                        if proof.status == ProofStatus::Verified {
                            span { class: "text-green-500", "●" }
                            "Finalized"
                        } else if proof.status == ProofStatus::Pending {
                            span { class: "text-yellow-500", "●" }
                            "Awaiting Finality"
                        } else {
                            span { class: "text-gray-500", "●" }
                            "{proof.status.to_string()}"
                        }
                    }
                }

                if let Some(verified_at) = proof.verified_at {
                    div { class: "p-3 bg-gray-800/50 rounded-lg",
                        p { class: "text-xs text-gray-500 mb-1", "Verified At" }
                        p { class: "font-mono text-sm", "{verified_at}" }
                    }
                }

                // Finality explanation
                div { class: "p-3 bg-green-900/20 border border-green-500/30 rounded-lg",
                    p { class: "text-sm text-green-300",
                        "✓ Finality proofs ensure the commitment cannot be reverted. \
                         This is a cryptographic guarantee, not a trust assumption."
                    }
                }
            }
        }
    }
}

/// Verification actions section
fn verification_actions(_proof: &ProofRecord) -> Element {
    rsx! {
        div { class: "{card_class()} p-6",
            h2 { class: "text-lg font-semibold mb-4", "Verification" }

            div { class: "space-y-4",
                p { class: "text-sm text-gray-400",
                    "This proof bundle can be verified completely offline. \
                     No RPC calls to any blockchain are required."
                }

                div { class: "flex gap-3",
                    Link {
                        to: Route::VerifyProof {},
                        class: "{btn_primary_class()}",
                        "Verify This Proof"
                    }
                    Link {
                        to: Route::ValidateProof {},
                        class: "{btn_secondary_class()}",
                        "Validate Offline"
                    }
                }

                // CSV advantage callout
                div { class: "mt-4 p-4 bg-gradient-to-r from-blue-900/30 to-purple-900/30 \
                              border border-blue-500/30 rounded-lg",
                    h3 { class: "text-sm font-semibold text-blue-300 mb-2",
                        "CSV Competitive Advantage"
                    }
                    p { class: "text-sm text-gray-300",
                        "Traditional bridges require you to trust their servers. \
                         With CSV, anyone holding this proof bundle can verify it \
                         cryptographically—no trusted party needed."
                    }
                }
            }
        }
    }
}

/// Export format options
#[derive(Clone, Copy, PartialEq)]
enum ExportFormat {
    Json,
    Base64,
    Hex,
}

impl ExportFormat {
    fn label(&self) -> &'static str {
        match self {
            ExportFormat::Json => "JSON",
            ExportFormat::Base64 => "Base64",
            ExportFormat::Hex => "Hex",
        }
    }
}

/// Export modal component
fn export_modal(
    _proof_id: &str,
    mut show: Signal<bool>,
    mut format: Signal<ExportFormat>,
) -> Element {
    let format_value = format();

    rsx! {
        div { class: "modal-backdrop fixed inset-0 bg-black/70 z-50 flex items-center justify-center",
            onclick: move |_| show.set(false),

            div { class: "modal-content bg-gray-900 rounded-xl max-w-md w-full mx-4 overflow-hidden",
                onclick: |e| e.stop_propagation(),

                // Header
                div { class: "px-6 py-4 border-b border-gray-800",
                    h3 { class: "text-lg font-semibold", "Export Proof Bundle" }
                    p { class: "text-sm text-gray-400 mt-1",
                        "Share this proof with counterparties for verification"
                    }
                }

                // Format selection
                div { class: "p-6 space-y-4",
                    p { class: "text-sm text-gray-400", "Choose export format:" }

                    div { class: "space-y-2",
                        for fmt in [ExportFormat::Json, ExportFormat::Base64, ExportFormat::Hex] {
                            label { class: "flex items-center gap-3 p-3 bg-gray-800 rounded-lg cursor-pointer \
                                          hover:bg-gray-700 transition-colors",
                                input {
                                    r#type: "radio",
                                    name: "export-format",
                                    checked: format_value == fmt,
                                    onchange: move |_| format.set(fmt),
                                }
                                span { "{fmt.label()}" }
                            }
                        }
                    }

                    // QR code placeholder (simplified - static pattern)
                    div { class: "p-4 bg-white rounded-lg flex flex-col items-center",
                        p { class: "text-gray-900 text-xs mb-2", "QR Code (Proof Hash)" }
                        div { class: "w-32 h-32 bg-gray-900 grid grid-cols-5 gap-0.5 p-2",
                            // Static QR pattern for display purposes
                            div { class: "bg-black" }
                            div { class: "bg-white" }
                            div { class: "bg-black" }
                            div { class: "bg-black" }
                            div { class: "bg-black" }
                            div { class: "bg-white" }
                            div { class: "bg-black" }
                            div { class: "bg-white" }
                            div { class: "bg-black" }
                            div { class: "bg-white" }
                            div { class: "bg-black" }
                            div { class: "bg-white" }
                            div { class: "bg-white" }
                            div { class: "bg-black" }
                            div { class: "bg-white" }
                            div { class: "bg-black" }
                            div { class: "bg-white" }
                            div { class: "bg-black" }
                            div { class: "bg-black" }
                            div { class: "bg-white" }
                            div { class: "bg-black" }
                            div { class: "bg-white" }
                            div { class: "bg-black" }
                            div { class: "bg-white" }
                            div { class: "bg-black" }
                        }
                        p { class: "text-gray-500 text-xs mt-2 text-center",
                            "Scan to verify offline"
                        }
                    }
                }

                // Actions
                div { class: "px-6 py-4 border-t border-gray-800 flex gap-3",
                    button {
                        class: "flex-1 {btn_primary_class()}",
                        onclick: move |_| {
                            // TODO: Implement actual export
                            show.set(false);
                        },
                        "📋 Copy to Clipboard"
                    }
                    button {
                        class: "flex-1 {btn_secondary_class()}",
                        onclick: move |_| show.set(false),
                        "Cancel"
                    }
                }
            }
        }
    }
}
