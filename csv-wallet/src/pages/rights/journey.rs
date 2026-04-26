//! Right Journey page - shows the complete lifecycle of a Right.
//!
//! This page visualizes the flow: Right → Seal → Proof → Destination Right
//! helping users understand the relationship between these concepts.

use crate::context::{use_wallet_context, TrackedRight, SealRecord, ProofRecord, SealStatus, ProofStatus, RightStatus};
use crate::pages::common::*;
use crate::routes::Route;
use csv_adapter_core::Chain;
use dioxus::prelude::*;

/// Right Journey page - visualizes the complete right lifecycle
#[component]
pub fn RightJourney(id: String) -> Element {
    let wallet_ctx = use_wallet_context();
    let right = wallet_ctx.get_right(&id);
    let seal = wallet_ctx.seal_for_right(&id);
    let proofs = wallet_ctx.proofs_for_right(&id);
    
    // Find destination right if this was transferred
    let dest_right = if let Some(ref r) = right {
        if r.status == RightStatus::Transferred {
            // Look for a right that might be the destination
            wallet_ctx.rights().into_iter().find(|other| {
                other.id != r.id && other.value == r.value && other.status == RightStatus::Active
            })
        } else {
            None
        }
    } else {
        None
    };

    rsx! {
        div { class: "max-w-4xl mx-auto space-y-6",
            // Header
            div { class: "flex items-center gap-3",
                Link { to: Route::Rights {}, class: "{btn_secondary_class()}", "\u{2190} Back" }
                h1 { class: "text-xl font-bold", "Right Journey" }
            }
            
            if right.is_none() {
                div { class: "{card_class()} p-6",
                    p { class: "text-gray-400", "Right not found." }
                }
            } else {
                // Flow visualization
                {flow_visualization(&right, &seal, &proofs, &dest_right)}
                
                // Detailed sections
                if let Some(ref r) = right {
                    {right_details_section(r)}
                }
                if let Some(ref s) = seal {
                    {seal_details_section(s)}
                }
                if !proofs.is_empty() {
                    {proofs_section(&proofs)}
                }
                if let Some(ref d) = dest_right {
                    {destination_section(d)}
                }
            }
        }
    }
}

/// Visual flow diagram showing the lifecycle
fn flow_visualization(
    right: &Option<TrackedRight>,
    seal: &Option<SealRecord>,
    proofs: &[ProofRecord],
    dest_right: &Option<TrackedRight>,
) -> Element {
    let right_active = right.is_some();
    let seal_active = seal.is_some();
    let proof_active = !proofs.is_empty();
    let dest_active = dest_right.is_some();
    
    // Determine status colors
    let right_color = if right_active { "bg-blue-500" } else { "bg-gray-600" };
    let seal_color = if seal_active { 
        match seal.as_ref().unwrap().status {
            SealStatus::Active => "bg-yellow-500",
            SealStatus::Locked => "bg-orange-500",
            SealStatus::Consumed => "bg-gray-600",
            SealStatus::Transferred => "bg-green-500",
        }
    } else { "bg-gray-600" };
    let proof_color = if proof_active {
        if proofs.iter().any(|p| p.status == ProofStatus::Verified) {
            "bg-green-500"
        } else {
            "bg-yellow-500"
        }
    } else { "bg-gray-600" };
    let dest_color = if dest_active { "bg-green-500" } else { "bg-gray-600" };
    
    rsx! {
        div { class: "{card_class()} p-6",
            h2 { class: "text-lg font-semibold mb-6", "Lifecycle Flow" }
            
            div { class: "flex flex-col md:flex-row items-center justify-between gap-4",
                // Step 1: Right
                div { class: "flex flex-col items-center gap-2",
                    div { class: "w-16 h-16 rounded-full {right_color} flex items-center justify-center text-2xl",
                        "\u{1F48E}"
                    }
                    span { class: "text-sm font-medium", "Right" }
                    if let Some(ref r) = right {
                        span { class: "text-xs text-gray-400", "{truncate_address(&r.id, 6)}" }
                    }
                }
                
                // Arrow
                div { class: "flex flex-col items-center",
                    span { class: "text-2xl text-gray-500", "\u{2192}" }
                    if seal_active {
                        span { class: "text-xs text-blue-400", "Lock" }
                    }
                }
                
                // Step 2: Seal
                div { class: "flex flex-col items-center gap-2",
                    div { class: "w-16 h-16 rounded-full {seal_color} flex items-center justify-center text-2xl",
                        "\u{1F512}"
                    }
                    span { class: "text-sm font-medium", "Seal" }
                    if let Some(ref s) = seal {
                        span { class: "text-xs text-gray-400", "{s.status}" }
                    } else {
                        span { class: "text-xs text-gray-500", "Not created" }
                    }
                }
                
                // Arrow
                div { class: "flex flex-col items-center",
                    span { class: "text-2xl text-gray-500", "\u{2192}" }
                    if proof_active {
                        span { class: "text-xs text-blue-400", "Prove" }
                    }
                }
                
                // Step 3: Proof
                div { class: "flex flex-col items-center gap-2",
                    div { class: "w-16 h-16 rounded-full {proof_color} flex items-center justify-center text-2xl",
                        "\u{1F4C4}"
                    }
                    span { class: "text-sm font-medium", "Proof" }
                    if !proofs.is_empty() {
                        let verified_count = proofs.iter().filter(|p| p.status == ProofStatus::Verified).count();
                        span { class: "text-xs text-gray-400", "{verified_count}/{proofs.len()} verified" }
                    } else {
                        span { class: "text-xs text-gray-500", "Not generated" }
                    }
                }
                
                // Arrow
                div { class: "flex flex-col items-center",
                    span { class: "text-2xl text-gray-500", "\u{2192}" }
                    if dest_active {
                        span { class: "text-xs text-blue-400", "Mint" }
                    }
                }
                
                // Step 4: Destination Right
                div { class: "flex flex-col items-center gap-2",
                    div { class: "w-16 h-16 rounded-full {dest_color} flex items-center justify-center text-2xl",
                        "\u{1F48E}"
                    }
                    span { class: "text-sm font-medium", "Destination" }
                    if dest_active {
                        span { class: "text-xs text-gray-400", "Complete" }
                    } else {
                        span { class: "text-xs text-gray-500", "Pending" }
                    }
                }
            }
            
            // Status summary
            div { class: "mt-6 p-4 bg-gray-800/50 rounded-lg",
                p { class: "text-sm text-gray-300",
                    {flow_status_text(right, seal, proofs, dest_right)}
                }
            }
        }
    }
}

fn flow_status_text(
    right: &Option<TrackedRight>,
    seal: &Option<SealRecord>,
    proofs: &[ProofRecord],
    dest_right: &Option<TrackedRight>,
) -> String {
    if let Some(ref r) = right {
        match r.status {
            RightStatus::Active => {
                if seal.is_none() {
                    "This Right is active and can be locked for cross-chain transfer.".to_string()
                } else if proofs.is_empty() {
                    "Right is locked. Generate a proof to verify it on another chain.".to_string()
                } else if dest_right.is_none() {
                    "Proof generated. Ready to mint on destination chain.".to_string()
                } else {
                    "Cross-chain transfer complete!".to_string()
                }
            }
            RightStatus::Transferred => "This Right has been transferred to another chain.".to_string(),
            RightStatus::Consumed => "This Right has been consumed.".to_string(),
        }
    } else {
        "Right not found.".to_string()
    }
}

/// Right details section
fn right_details_section(right: &TrackedRight) -> Element {
    rsx! {
        div { class: "{card_class()}",
            div { class: "{card_header_class()}",
                h2 { class: "font-semibold", "\u{1F48E} Right Details" }
            }
            div { class: "p-4 space-y-4",
                div { class: "grid grid-cols-2 gap-4",
                    div {
                        p { class: "text-xs text-gray-500", "Right ID" }
                        p { class: "text-sm font-mono break-all", "{&right.id}" }
                    }
                    div {
                        p { class: "text-xs text-gray-500", "Chain" }
                        p { class: "text-sm", span { class: "{chain_badge_class(&right.chain)}", "{chain_icon_emoji(&right.chain)} {chain_name(&right.chain)}" } }
                    }
                    div {
                        p { class: "text-xs text-gray-500", "Value" }
                        p { class: "text-sm font-mono", "{right.value}" }
                    }
                    div {
                        p { class: "text-xs text-gray-500", "Status" }
                        p { class: "text-sm",
                            span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {right_status_class(&right.status)}",
                                "{right.status}"
                            }
                        }
                    }
                    div {
                        p { class: "text-xs text-gray-500", "Owner" }
                        p { class: "text-sm font-mono", "{truncate_address(&right.owner, 12)}" }
                    }
                }
            }
        }
    }
}

/// Seal details section
fn seal_details_section(seal: &SealRecord) -> Element {
    let status_class = match seal.status {
        SealStatus::Active => "text-yellow-400 bg-yellow-500/20",
        SealStatus::Locked => "text-orange-400 bg-orange-500/20",
        SealStatus::Consumed => "text-gray-400 bg-gray-500/20",
        SealStatus::Transferred => "text-green-400 bg-green-500/20",
    };
    
    rsx! {
        div { class: "{card_class()}",
            div { class: "{card_header_class()}",
                h2 { class: "font-semibold", "\u{1F512} Seal Details" }
            }
            div { class: "p-4 space-y-4",
                div { class: "grid grid-cols-2 gap-4",
                    div {
                        p { class: "text-xs text-gray-500", "Seal Reference" }
                        p { class: "text-sm font-mono break-all", "{&seal.seal_ref}" }
                    }
                    div {
                        p { class: "text-xs text-gray-500", "Status" }
                        p { class: "text-sm",
                            span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {status_class}",
                                "{seal.status}"
                            }
                        }
                    }
                    div {
                        p { class: "text-xs text-gray-500", "Protects Right" }
                        p { class: "text-sm font-mono", "{truncate_address(&seal.right_id, 12)}" }
                    }
                    div {
                        p { class: "text-xs text-gray-500", "Value Locked" }
                        p { class: "text-sm font-mono", "{seal.value}" }
                    }
                }
                
                if let Some(ref content) = seal.content {
                    div { class: "border-t border-gray-700 pt-4",
                        p { class: "text-xs text-gray-500 mb-2", "Cryptographic Content" }
                        div { class: "space-y-2",
                            div {
                                p { class: "text-xs text-gray-600", "Content Hash" }
                                p { class: "text-xs font-mono break-all", "{&content.content_hash}" }
                            }
                            div {
                                p { class: "text-xs text-gray-600", "Owner" }
                                p { class: "text-xs font-mono", "{truncate_address(&content.owner, 12)}" }
                            }
                            if let Some(block) = content.block_number {
                                div {
                                    p { class: "text-xs text-gray-600", "Block Number" }
                                    p { class: "text-xs font-mono", "{block}" }
                                }
                            }
                            if let Some(ref tx) = content.lock_tx_hash {
                                div {
                                    p { class: "text-xs text-gray-600", "Lock Transaction" }
                                    p { class: "text-xs font-mono break-all", "{tx}" }
                                }
                            }
                        }
                    }
                }
                
                if let Some(ref proof_ref) = seal.proof_ref {
                    div { class: "border-t border-gray-700 pt-4",
                        p { class: "text-xs text-gray-500", "Linked Proof" }
                        p { class: "text-xs font-mono", "{truncate_address(proof_ref, 12)}" }
                    }
                }
            }
        }
    }
}

/// Proofs section
fn proofs_section(proofs: &[ProofRecord]) -> Element {
    rsx! {
        div { class: "{card_class()}",
            div { class: "{card_header_class()}",
                h2 { class: "font-semibold", "\u{1F4C4} Proofs ({proofs.len()})" }
            }
            div { class: "p-4 space-y-4",
                for proof in proofs.iter() {
                    {proof_card(proof)}
                }
            }
        }
    }
}

fn proof_card(proof: &ProofRecord) -> Element {
    let status_class = match proof.status {
        ProofStatus::Generated => "text-yellow-400 bg-yellow-500/20",
        ProofStatus::Pending => "text-blue-400 bg-blue-500/20",
        ProofStatus::Verified => "text-green-400 bg-green-500/20",
        ProofStatus::Invalid => "text-red-400 bg-red-500/20",
    };
    
    rsx! {
        div { class: "border border-gray-700 rounded-lg p-4",
            div { class: "flex items-center justify-between mb-3",
                h3 { class: "font-medium", "Proof: {proof.proof_type}" }
                span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {status_class}",
                    "{proof.status}"
                }
            }
            
            div { class: "grid grid-cols-2 gap-3 text-sm",
                div {
                    p { class: "text-xs text-gray-500", "Source Chain" }
                    p { class: "font-mono", "{chain_name(&proof.chain)}" }
                }
                if let Some(target) = proof.target_chain {
                    div {
                        p { class: "text-xs text-gray-500", "Target Chain" }
                        p { class: "font-mono", "{chain_name(&target)}" }
                    }
                }
                div {
                    p { class: "text-xs text-gray-500", "For Seal" }
                    p { class: "font-mono", "{truncate_address(&proof.seal_ref, 8)}" }
                }
                div {
                    p { class: "text-xs text-gray-500", "Generated" }
                    p { class: "font-mono", "{format_timestamp(proof.generated_at)}" }
                }
            }
            
            if let Some(ref data) = proof.data {
                div { class: "mt-3 pt-3 border-t border-gray-700",
                    p { class: "text-xs text-gray-500 mb-2", "Cryptographic Data" }
                    {proof_data_display(data)}
                }
            }
            
            if let Some(ref tx) = proof.verification_tx_hash {
                div { class: "mt-3 pt-3 border-t border-gray-700",
                    p { class: "text-xs text-gray-500", "Verification TX" }
                    p { class: "text-xs font-mono break-all", "{tx}" }
                }
            }
        }
    }
}

fn proof_data_display(data: &crate::context::ProofData) -> Element {
    match data {
        crate::context::ProofData::Merkle { root, path, leaf_index } => {
            rsx! {
                div { class: "space-y-1 text-xs",
                    p { span { class: "text-gray-500", "Root: " }, span { class: "font-mono", "{truncate_address(root, 16)}" } }
                    p { span { class: "text-gray-500", "Path length: " }, "{path.len()} nodes" }
                    p { span { class: "text-gray-500", "Leaf index: " }, "{leaf_index}" }
                }
            }
        }
        crate::context::ProofData::Mpt { root, account_proof, storage_proof } => {
            rsx! {
                div { class: "space-y-1 text-xs",
                    p { span { class: "text-gray-500", "State Root: " }, span { class: "font-mono", "{truncate_address(root, 16)}" } }
                    p { span { class: "text-gray-500", "Account proof: " }, "{account_proof.len()} nodes" }
                    p { span { class: "text-gray-500", "Storage proof: " }, "{storage_proof.len()} nodes" }
                }
            }
        }
        crate::context::ProofData::Checkpoint { sequence, digest, signatures } => {
            rsx! {
                div { class: "space-y-1 text-xs",
                    p { span { class: "text-gray-500", "Sequence: " }, "{sequence}" }
                    p { span { class: "text-gray-500", "Digest: " }, span { class: "font-mono", "{truncate_address(digest, 16)}" } }
                    p { span { class: "text-gray-500", "Signatures: " }, "{signatures.len()} validators" }
                }
            }
        }
        crate::context::ProofData::Ledger { version, proof } => {
            rsx! {
                div { class: "space-y-1 text-xs",
                    p { span { class: "text-gray-500", "Version: " }, "{version}" }
                    p { span { class: "text-gray-500", "Proof: " }, span { class: "font-mono", "{truncate_address(proof, 16)}" } }
                }
            }
        }
        crate::context::ProofData::Solana { slot, bank_hash, merkle_proof } => {
            rsx! {
                div { class: "space-y-1 text-xs",
                    p { span { class: "text-gray-500", "Slot: " }, "{slot}" }
                    p { span { class: "text-gray-500", "Bank Hash: " }, span { class: "font-mono", "{truncate_address(bank_hash, 16)}" } }
                    p { span { class: "text-gray-500", "Merkle proof: " }, "{merkle_proof.len()} nodes" }
                }
            }
        }
    }
}

/// Destination right section
fn destination_section(right: &TrackedRight) -> Element {
    rsx! {
        div { class: "{card_class()} border-green-500/30",
            div { class: "{card_header_class()} bg-green-900/20",
                h2 { class: "font-semibold text-green-400", "\u{2705} Destination Right" }
            }
            div { class: "p-4 space-y-4",
                p { class: "text-sm text-gray-300", 
                    "The Right has been successfully minted on the destination chain."
                }
                div { class: "grid grid-cols-2 gap-4",
                    div {
                        p { class: "text-xs text-gray-500", "Right ID" }
                        p { class: "text-sm font-mono break-all", "{&right.id}" }
                    }
                    div {
                        p { class: "text-xs text-gray-500", "Chain" }
                        p { class: "text-sm", span { class: "{chain_badge_class(&right.chain)}", "{chain_icon_emoji(&right.chain)} {chain_name(&right.chain)}" } }
                    }
                    div {
                        p { class: "text-xs text-gray-500", "Value" }
                        p { class: "text-sm font-mono", "{right.value}" }
                    }
                    div {
                        p { class: "text-xs text-gray-500", "Status" }
                        p { class: "text-sm",
                            span { class: "inline-flex items-center px-2 py-0.5 rounded text-xs font-medium {right_status_class(&right.status)}",
                                "{right.status}"
                            }
                        }
                    }
                }
            }
        }
    }
}
