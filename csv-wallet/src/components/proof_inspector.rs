//! Proof Inspector Component
//!
//! Visualizes and verifies cross-chain proofs:
//! - Proof structure breakdown
//! - Merkle path visualization
//! - Verification status
//! - Chain signatures display
//! - Raw proof data viewer

use dioxus::prelude::*;
use crate::components::hash_display::{HashDisplay, shorten_hash};

/// Cross-chain proof data structure.
#[derive(Clone, PartialEq)]
pub struct CrossChainProof {
    /// Unique proof identifier.
    pub proof_id: String,
    /// Source chain where the lock occurred.
    pub source_chain: String,
    /// Target chain for the transfer.
    pub target_chain: String,
    /// Source transaction hash.
    pub source_tx_hash: String,
    /// Target transaction hash (if completed).
    pub target_tx_hash: Option<String>,
    /// Block height on source chain.
    pub source_block_height: u64,
    /// Timestamp of proof generation.
    pub timestamp: u64,
    /// Merkle root of the anchor.
    pub merkle_root: String,
    /// Merkle path (hashes from leaf to root).
    pub merkle_path: Vec<String>,
    /// Leaf hash (commitment).
    pub leaf_hash: String,
    /// Validator signatures.
    pub signatures: Vec<ValidatorSignature>,
    /// Raw proof data (BCS encoded).
    pub raw_data: Vec<u8>,
    /// Verification status.
    pub status: ProofStatus,
}

/// Proof verification status.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ProofStatus {
    Pending,
    Verified,
    Failed,
    Expired,
}

impl ProofStatus {
    fn label(&self) -> &'static str {
        match self {
            ProofStatus::Pending => "Pending Verification",
            ProofStatus::Verified => "Verified",
            ProofStatus::Failed => "Verification Failed",
            ProofStatus::Expired => "Expired",
        }
    }
    
    fn color(&self) -> &'static str {
        match self {
            ProofStatus::Pending => "var(--color-warning-500)",
            ProofStatus::Verified => "var(--color-success-500)",
            ProofStatus::Failed => "var(--color-error-500)",
            ProofStatus::Expired => "var(--color-gray-500)",
        }
    }
    
    fn icon(&self) -> &'static str {
        match self {
            ProofStatus::Pending => "⏳",
            ProofStatus::Verified => "✓",
            ProofStatus::Failed => "✗",
            ProofStatus::Expired => "⚠",
        }
    }
}

/// Validator signature.
#[derive(Clone, PartialEq)]
pub struct ValidatorSignature {
    /// Validator address/ID.
    pub validator_id: String,
    /// Signature data.
    pub signature: String,
    /// Whether the signature is valid.
    pub is_valid: bool,
    /// Timestamp of signature.
    pub timestamp: u64,
}

/// Props for the ProofInspector component.
#[derive(Props, Clone, PartialEq)]
pub struct ProofInspectorProps {
    /// The proof to inspect.
    pub proof: CrossChainProof,
    /// Whether to allow verification trigger.
    #[props(default = true)]
    pub allow_verify: bool,
    /// Callback when verify is clicked.
    #[props(default)]
    pub on_verify: Option<EventHandler<()>>,
    /// Additional CSS classes.
    #[props(default)]
    pub class: String,
}

/// Interactive proof inspector component.
#[allow(non_snake_case)]
pub fn ProofInspector(props: ProofInspectorProps) -> Element {
    let mut active_tab = use_signal(|| Tab::Overview);
    let expanded_nodes = use_signal(|| std::collections::HashSet::<usize>::new());
    
    rsx! {
        div { class: "proof-inspector {props.class}",
            // Header with status
            div { class: "proof-header",
                div { class: "proof-title-section",
                    h3 { class: "proof-title", "Cross-Chain Proof" }
                    span { 
                        class: "proof-status-badge",
                        style: format!("color: {}; border-color: {}", 
                            props.proof.status.color(), 
                            props.proof.status.color()),
                        "{props.proof.status.icon()} {props.proof.status.label()}"
                    }
                }
                
                div { class: "proof-meta",
                    div { class: "meta-item",
                        span { class: "meta-label", "Proof ID: " }
                        HashDisplay {
                            value: props.proof.proof_id.clone(),
                            prefix_len: 12,
                            suffix_len: 8,
                            show_copy: true,
                        }
                    }
                    div { class: "meta-item",
                        span { class: "meta-label", "Generated: " }
                        span { class: "meta-value", "{format_time(props.proof.timestamp)}" }
                    }
                }
            }
            
            // Tab navigation
            div { class: "proof-tabs",
                for tab in [Tab::Overview, Tab::MerkleTree, Tab::Signatures, Tab::Raw] {
                    button {
                        class: if *active_tab.read() == tab { "active" } else { "" },
                        onclick: move |_| active_tab.set(tab),
                        "{tab.label()}"
                    }
                }
            }
            
            // Tab content
            div { class: "proof-content",
                match *active_tab.read() {
                    Tab::Overview => rsx! {
                        ProofOverview { 
                            proof: props.proof.clone(),
                            on_verify: props.on_verify.clone(),
                            allow_verify: props.allow_verify,
                        }
                    },
                    Tab::MerkleTree => rsx! {
                        MerkleTreeView { 
                            proof: props.proof.clone(),
                            expanded_nodes: expanded_nodes,
                        }
                    },
                    Tab::Signatures => rsx! {
                        SignaturesView { 
                            signatures: props.proof.signatures.clone(),
                        }
                    },
                    Tab::Raw => rsx! {
                        RawDataView { 
                            raw_data: props.proof.raw_data.clone(),
                        }
                    },
                }
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Overview,
    MerkleTree,
    Signatures,
    Raw,
}

impl Tab {
    fn label(&self) -> &'static str {
        match self {
            Tab::Overview => "Overview",
            Tab::MerkleTree => "Merkle Tree",
            Tab::Signatures => "Signatures",
            Tab::Raw => "Raw Data",
        }
    }
}

/// Proof overview tab.
#[derive(Props, Clone, PartialEq)]
struct ProofOverviewProps {
    proof: CrossChainProof,
    allow_verify: bool,
    on_verify: Option<EventHandler<()>>,
}

#[allow(non_snake_case)]
fn ProofOverview(props: ProofOverviewProps) -> Element {
    rsx! {
        div { class: "proof-overview",
            // Chain flow visualization
            div { class: "proof-chain-flow",
                div { class: "chain-box source",
                    div { class: "chain-label", "Source" }
                    div { class: "chain-name", "{props.proof.source_chain}" }
                    div { class: "chain-tx",
                        span { "TX: " }
                        HashDisplay {
                            value: props.proof.source_tx_hash.clone(),
                            prefix_len: 8,
                            suffix_len: 8,
                            show_copy: true,
                        }
                    }
                    div { class: "chain-block", "Block #{props.proof.source_block_height}" }
                }
                
                div { class: "flow-arrow", "→" }
                
                div { class: "chain-box target",
                    div { class: "chain-label", "Target" }
                    div { class: "chain-name", "{props.proof.target_chain}" }
                    if let Some(ref tx) = props.proof.target_tx_hash {
                        div { class: "chain-tx",
                            span { "TX: " }
                            HashDisplay {
                                value: tx.clone(),
                                prefix_len: 8,
                                suffix_len: 8,
                                show_copy: true,
                            }
                        }
                    } else {
                        div { class: "chain-pending", "Pending..." }
                    }
                }
            }
            
            // Key proof data
            div { class: "proof-data-grid",
                div { class: "data-card",
                    h5 { "Leaf Hash" }
                    HashDisplay {
                        value: props.proof.leaf_hash.clone(),
                        prefix_len: 16,
                        suffix_len: 16,
                        show_copy: true,
                        class: "monospace".to_string(),
                    }
                }
                
                div { class: "data-card",
                    h5 { "Merkle Root" }
                    HashDisplay {
                        value: props.proof.merkle_root.clone(),
                        prefix_len: 16,
                        suffix_len: 16,
                        show_copy: true,
                        class: "monospace".to_string(),
                    }
                }
                
                div { class: "data-card",
                    h5 { "Path Depth" }
                    div { class: "path-stat", "{props.proof.merkle_path.len()} levels" }
                    div { class: "path-desc", "Merkle path from leaf to root" }
                }
                
                div { class: "data-card",
                    h5 { "Signatures" }
                    div { class: "sig-stat", 
                        "{props.proof.signatures.iter().filter(|s| s.is_valid).count()} / {props.proof.signatures.len()} valid"
                    }
                    div { class: "sig-desc", "Validator attestations" }
                }
            }
            
            // Verify button
            if props.allow_verify && props.proof.status == ProofStatus::Pending {
                div { class: "proof-actions",
                    button {
                        class: "verify-btn",
                        onclick: move |_| {
                            if let Some(ref handler) = props.on_verify {
                                handler.call(());
                            }
                        },
                        "Verify Proof"
                    }
                }
            }
        }
    }
}

/// Merkle tree visualization.
#[derive(Props, Clone, PartialEq)]
struct MerkleTreeViewProps {
    proof: CrossChainProof,
    expanded_nodes: Signal<std::collections::HashSet<usize>>,
}

#[allow(non_snake_case)]
fn MerkleTreeView(mut props: MerkleTreeViewProps) -> Element {
    let total_levels = props.proof.merkle_path.len() + 1;
    
    rsx! {
        div { class: "merkle-tree-view",
            h4 { "Merkle Path Visualization" }
            
            p { class: "merkle-desc",
                "The Merkle path proves that the leaf (seal commitment) is included in the anchor root. 
                Each level combines the current hash with a sibling hash to produce the parent hash."
            }
            
            // Tree structure (drawn bottom-up)
            div { class: "merkle-tree",
                // Leaf node at bottom
                div { class: "merkle-level",
                    div { class: "merkle-node leaf",
                        span { class: "node-label", "Leaf (Your Seal)" }
                        HashDisplay {
                            value: props.proof.leaf_hash.clone(),
                            prefix_len: 12,
                            suffix_len: 8,
                            show_copy: true,
                        }
                    }
                }
                
                // Path levels
                for (i, sibling_hash) in props.proof.merkle_path.iter().enumerate() {
                    div { class: "merkle-level",
                        // Connector
                        div { class: "merkle-connector" }
                        
                        // Sibling node
                        div { class: "merkle-sibling",
                            span { class: "sibling-label", "Sibling {i + 1}" }
                            HashDisplay {
                                value: sibling_hash.clone(),
                                prefix_len: 8,
                                suffix_len: 4,
                                show_copy: true,
                            }
                        }
                        
                        // Hash operation
                        div { class: "merkle-op", "⊕" }
                        
                        // Result node
                        div { class: "merkle-node",
                            span { class: "node-label", 
                                if i == props.proof.merkle_path.len() - 1 {
                                    "Root"
                                } else {
                                    "Level {i + 1}"
                                }
                            }
                            // Show computed hash at each level
                            button {
                                class: "expand-btn",
                                onclick: move |_| {
                                    let mut expanded = props.expanded_nodes.cloned();
                                    if expanded.contains(&i) {
                                        expanded.remove(&i);
                                    } else {
                                        expanded.insert(i);
                                    }
                                    props.expanded_nodes.set(expanded);
                                },
                                if (props.expanded_nodes)().contains(&i) {
                                    "▼"
                                } else {
                                    "▶"
                                }
                            }
                        }
                        
                        if (props.expanded_nodes)().contains(&i) {
                            div { class: "merkle-expanded",
                                p { "Hash(parent) = Hash(Hash(child) ⊕ Hash(sibling))" }
                                p { class: "hash-formula",
                                    "SHA-256 concatenation with sibling at level {i}"
                                }
                            }
                        }
                    }
                }
                
                // Root at top
                div { class: "merkle-level root",
                    div { class: "merkle-connector root" }
                    div { class: "merkle-node root",
                        span { class: "node-label", "Merkle Root (Anchor)" }
                        HashDisplay {
                            value: props.proof.merkle_root.clone(),
                            prefix_len: 16,
                            suffix_len: 16,
                            show_copy: true,
                            class: "root-hash".to_string(),
                        }
                    }
                }
            }
            
            // Verification equation
            div { class: "merkle-verification",
                h5 { "Verification Formula" }
                code { class: "verify-formula",
                    "verify(leaf_hash, merkle_path, expected_root) == true"
                }
            }
        }
    }
}

/// Signatures list view.
#[derive(Props, Clone, PartialEq)]
struct SignaturesViewProps {
    signatures: Vec<ValidatorSignature>,
}

#[allow(non_snake_case)]
fn SignaturesView(props: SignaturesViewProps) -> Element {
    let valid_count = props.signatures.iter().filter(|s| s.is_valid).count();
    let threshold = (props.signatures.len() as f32 * 0.66).ceil() as usize;
    
    rsx! {
        div { class: "signatures-view",
            h4 { "Validator Signatures" }
            
            div { class: "sig-summary",
                div { class: "sig-progress",
                    div { 
                        class: "sig-progress-bar",
                        style: format!("width: {}%", (valid_count * 100) / props.signatures.len()),
                    }
                }
                div { class: "sig-count",
                    "{valid_count} / {props.signatures.len()} valid (threshold: {threshold})"
                }
            }
            
            div { class: "signatures-list",
                for sig in props.signatures.iter() {
                    div {
                        class: "signature-item",
                        class: if sig.is_valid { "valid" } else { "invalid" },
                        
                        div { class: "sig-header",
                            span { class: "sig-status",
                                if sig.is_valid { "✓ Valid" } else { "✗ Invalid" }
                            }
                            span { class: "sig-time", "{format_time(sig.timestamp)}" }
                        }
                        
                        div { class: "sig-validator",
                            span { class: "sig-label", "Validator: " }
                            HashDisplay {
                                value: sig.validator_id.clone(),
                                prefix_len: 8,
                                suffix_len: 8,
                                show_copy: false,
                            }
                        }
                        
                        div { class: "sig-data",
                            span { class: "sig-label", "Signature: " }
                            span { class: "sig-value monospace",
                                "{shorten_hash(&sig.signature, 16, 16)}"
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Raw data viewer.
#[derive(Props, Clone, PartialEq)]
struct RawDataViewProps {
    raw_data: Vec<u8>,
}

#[allow(non_snake_case)]
fn RawDataView(props: RawDataViewProps) -> Element {
    let hex_string = props.raw_data
        .chunks(16)
        .map(|chunk| {
            chunk.iter()
                .map(|b| format!("{:02x}", b))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .collect::<Vec<_>>()
        .join("\n");
    
    let base64_string = base64::encode(&props.raw_data);
    
    rsx! {
        div { class: "raw-data-view",
            h4 { "Raw Proof Data" }
            
            div { class: "raw-stats",
                div { class: "stat", "Size: {props.raw_data.len()} bytes" }
                div { class: "stat", "Format: BCS (Binary Canonical Serialization)" }
            }
            
            div { class: "raw-formats",
                div { class: "format-section",
                    h5 { "Hexadecimal" }
                    pre { class: "raw-hex", "{hex_string}" }
                    button {
                        class: "copy-btn",
                        onclick: move |_| {
                            // Copy hex to clipboard
                        },
                        "Copy Hex"
                    }
                }
                
                div { class: "format-section",
                    h5 { "Base64" }
                    pre { class: "raw-base64", "{base64_string}" }
                    button {
                        class: "copy-btn",
                        onclick: move |_| {
                            // Copy base64 to clipboard
                        },
                        "Copy Base64"
                    }
                }
            }
            
            div { class: "raw-note",
                p { "This is the raw proof data that can be submitted to the target chain's verification contract." }
            }
        }
    }
}

fn format_time(timestamp: u64) -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    
    let diff = now.saturating_sub(timestamp);
    
    if diff < 60 {
        format!("{}s ago", diff)
    } else if diff < 3600 {
        format!("{}m ago", diff / 60)
    } else if diff < 86400 {
        format!("{}h ago", diff / 3600)
    } else {
        format!("{}d ago", diff / 86400)
    }
}

// Simple base64 encoder for raw data display
mod base64 {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    
    pub fn encode(data: &[u8]) -> String {
        let mut result = String::new();
        let chunks = data.chunks(3);
        
        for chunk in chunks {
            let b1 = chunk[0];
            let b2 = chunk.get(1).copied().unwrap_or(0);
            let b3 = chunk.get(2).copied().unwrap_or(0);
            
            let n = (b1 as u32) << 16 | (b2 as u32) << 8 | b3 as u32;
            
            result.push(ALPHABET[(n >> 18) as usize] as char);
            result.push(ALPHABET[((n >> 12) & 0x3f) as usize] as char);
            
            if chunk.len() > 1 {
                result.push(ALPHABET[((n >> 6) & 0x3f) as usize] as char);
            } else {
                result.push('=');
            }
            
            if chunk.len() > 2 {
                result.push(ALPHABET[(n & 0x3f) as usize] as char);
            } else {
                result.push('=');
            }
        }
        
        result
    }
}
