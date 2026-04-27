//! Seal Lifecycle Visualizer
//!
//! Interactive visualization of seal states and transitions:
//! - Timeline view of seal history
//! - State transition diagram
//! - Transaction details at each step
//! - Cross-chain transfer tracking

use dioxus::prelude::*;
use crate::components::{
    design_tokens::SealState,
    hash_display::{HashDisplay, TxHashDisplay},
    seal_status::{SealLifecycle, SealStatusBadge, Size, StateTransition},
};

/// Seal event for the timeline.
#[derive(Clone, PartialEq)]
pub struct SealEvent {
    pub id: String,
    pub state: SealState,
    pub timestamp: u64,
    pub tx_hash: Option<String>,
    pub description: String,
    pub chain: Option<String>,
    pub block_height: Option<u64>,
    pub confirmations: Option<u32>,
}

/// Cross-chain transfer segment.
#[derive(Clone, PartialEq)]
pub struct TransferSegment {
    pub from_chain: String,
    pub to_chain: String,
    pub status: TransferStatus,
    pub source_tx: String,
    pub dest_tx: Option<String>,
    pub start_time: u64,
    pub completion_time: Option<u64>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TransferStatus {
    Pending,
    SourceConfirmed,
    ProofGenerated,
    DestPending,
    Completed,
    Failed,
}

impl TransferStatus {
    fn label(&self) -> &'static str {
        match self {
            TransferStatus::Pending => "Initiated",
            TransferStatus::SourceConfirmed => "Source Confirmed",
            TransferStatus::ProofGenerated => "Proof Ready",
            TransferStatus::DestPending => "Destination Pending",
            TransferStatus::Completed => "Completed",
            TransferStatus::Failed => "Failed",
        }
    }
    
    fn seal_state(&self) -> SealState {
        match self {
            TransferStatus::Pending => SealState::Pending,
            TransferStatus::SourceConfirmed => SealState::Locked,
            TransferStatus::ProofGenerated => SealState::Locked,
            TransferStatus::DestPending => SealState::Pending,
            TransferStatus::Completed => SealState::Consumed,
            TransferStatus::Failed => SealState::Error,
        }
    }
}

/// Props for the seal visualizer.
#[derive(Props, Clone, PartialEq)]
pub struct SealVisualizerProps {
    /// Seal ID.
    pub seal_id: String,
    /// Current state.
    pub current_state: SealState,
    /// Event history.
    pub events: Vec<SealEvent>,
    /// Optional cross-chain transfer info.
    #[props(default)]
    pub transfer: Option<TransferSegment>,
    /// Additional CSS classes.
    #[props(default)]
    pub class: String,
}

/// Interactive seal lifecycle visualizer.
pub fn SealVisualizer(props: SealVisualizerProps) -> Element {
    let selected_event = use_signal(|| None::<usize>);
    let view_mode = use_signal(|| ViewMode::Timeline);
    
    rsx! {
        div { class: "seal-visualizer {props.class}",
            // Header
            div { class: "seal-viz-header",
                h3 { class: "seal-viz-title",
                    "Seal "
                    HashDisplay {
                        value: props.seal_id.clone(),
                        prefix_len: 8,
                        suffix_len: 8,
                        show_copy: true,
                    }
                }
                
                // View mode toggle
                div { class: "seal-viz-modes",
                    button {
                        class: if *view_mode.read() == ViewMode::Timeline { "active" } else { "" },
                        onclick: move |_| view_mode.set(ViewMode::Timeline),
                        "Timeline"
                    }
                    button {
                        class: if *view_mode.read() == ViewMode::Diagram { "active" } else { "" },
                        onclick: move |_| view_mode.set(ViewMode::Diagram),
                        "Diagram"
                    }
                    if props.transfer.is_some() {
                        button {
                            class: if *view_mode.read() == ViewMode::Transfer { "active" } else { "" },
                            onclick: move |_| view_mode.set(ViewMode::Transfer),
                            "Transfer"
                        }
                    }
                }
            }
            
            // Current status
            div { class: "seal-viz-status",
                SealStatusBadge {
                    state: props.current_state,
                    size: Size::Large,
                    show_label: true,
                }
            }
            
            // Content based on view mode
            match *view_mode.read() {
                ViewMode::Timeline => rsx! {
                    SealTimelineView {
                        events: props.events.clone(),
                        selected_event: selected_event,
                    }
                },
                ViewMode::Diagram => rsx! {
                    SealDiagramView {
                        current_state: props.current_state,
                        events: props.events.clone(),
                    }
                },
                ViewMode::Transfer => rsx! {
                    if let Some(ref transfer) = props.transfer {
                        TransferView { transfer: transfer.clone() }
                    }
                },
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Timeline,
    Diagram,
    Transfer,
}

/// Timeline view of seal events.
#[derive(Props, Clone, PartialEq)]
struct SealTimelineViewProps {
    events: Vec<SealEvent>,
    selected_event: Signal<Option<usize>>,
}

fn SealTimelineView(props: SealTimelineViewProps) -> Element {
    rsx! {
        div { class: "seal-timeline-view",
            div { class: "seal-timeline-list",
                for (i, event) in props.events.iter().enumerate().rev() {
                    div {
                        class: "seal-timeline-item",
                        class: if props.selected_event() == Some(i) { "selected" },
                        onclick: move |_| props.selected_event.set(Some(i)),
                        
                        // Timeline connector
                        div { class: "seal-timeline-connector" }
                        
                        // Event dot
                        div {
                            class: "seal-timeline-dot",
                            style: format!("background-color: {}", event.state.dot_color()),
                        }
                        
                        // Event content
                        div { class: "seal-timeline-content",
                            div { class: "seal-timeline-header",
                                span { class: "seal-timeline-state",
                                    SealStatusBadge {
                                        state: event.state,
                                        size: Size::Small,
                                        show_label: true,
                                    }
                                }
                                span { class: "seal-timeline-time",
                                    "{format_time(event.timestamp)}"
                                }
                            }
                            
                            p { class: "seal-timeline-desc", "{event.description}" }
                            
                            if let Some(ref tx) = event.tx_hash {
                                div { class: "seal-timeline-tx",
                                    span { class: "label", "Transaction: " }
                                    TxHashDisplay {
                                        tx_hash: tx.clone(),
                                        chain: event.chain.clone().unwrap_or_default(),
                                        explorer_url: None,
                                    }
                                }
                            }
                            
                            if let Some(height) = event.block_height {
                                div { class: "seal-timeline-block",
                                    span { class: "label", "Block: " }
                                    "{height}"
                                    if let Some(conf) = event.confirmations {
                                        " ({conf} confirmations)"
                                    }
                                }
                            }
                        }
                    }
                }
            }
            
            // Event detail panel
            if let Some(idx) = props.selected_event() {
                if let Some(event) = props.events.get(idx) {
                    div { class: "seal-event-detail",
                        h4 { "Event Details" }
                        div { class: "detail-row",
                            span { class: "detail-label", "ID: " }
                            span { class: "detail-value", "{event.id}" }
                        }
                        div { class: "detail-row",
                            span { class: "detail-label", "State: " }
                            span { class: "detail-value",
                                SealStatusBadge {
                                    state: event.state,
                                    size: Size::Medium,
                                    show_label: true,
                                }
                            }
                        }
                        div { class: "detail-row",
                            span { class: "detail-label", "Time: " }
                            span { class: "detail-value", "{format_datetime(event.timestamp)}" }
                        }
                        if let Some(ref chain) = event.chain {
                            div { class: "detail-row",
                                span { class: "detail-label", "Chain: " }
                                span { class: "detail-value", "{chain}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// State diagram view.
#[derive(Props, Clone, PartialEq)]
struct SealDiagramViewProps {
    current_state: SealState,
    events: Vec<SealEvent>,
}

fn SealDiagramView(props: SealDiagramViewProps) -> Element {
    let states = vec![
        (SealState::Active, "Created", 0),
        (SealState::Pending, "Pending", 1),
        (SealState::Locked, "Locked", 2),
        (SealState::Consumed, "Consumed", 3),
    ];
    
    let current_idx = states.iter()
        .position(|(s, _, _)| *s == props.current_state)
        .unwrap_or(0);
    
    rsx! {
        div { class: "seal-diagram-view",
            div { class: "seal-state-flow",
                for (i, (state, label, _)) in states.iter().enumerate() {
                    div {
                        class: "seal-state-node",
                        class: if i == current_idx { "current" },
                        class: if i < current_idx { "completed" },
                        class: if i > current_idx { "future" },
                        
                        div {
                            class: "seal-node-circle",
                            style: format!("border-color: {}", state.dot_color()),
                            if i < current_idx {
                                // Checkmark for completed
                                "✓"
                            } else {
                                "{i + 1}"
                            }
                        }
                        span { class: "seal-node-label", "{label}" }
                        
                        // Connection line (except for last)
                        if i < states.len() - 1 {
                            div {
                                class: "seal-node-connector",
                                class: if i < current_idx { "completed" },
                            }
                        }
                    }
                }
            }
            
            // State descriptions
            div { class: "seal-state-descriptions",
                div { class: "state-desc-item",
                    strong { "Active: " }
                    "Seal is created and available for use"
                }
                div { class: "state-desc-item",
                    strong { "Pending: " }
                    "Transaction submitted, awaiting confirmation"
                }
                div { class: "state-desc-item",
                    strong { "Locked: " }
                    "Seal locked in cross-chain transfer"
                }
                div { class: "state-desc-item",
                    strong { "Consumed: " }
                    "Seal has been consumed and is no longer valid"
                }
            }
        }
    }
}

/// Cross-chain transfer view.
#[derive(Props, Clone, PartialEq)]
struct TransferViewProps {
    transfer: TransferSegment,
}

fn TransferView(props: TransferViewProps) -> Element {
    rsx! {
        div { class: "seal-transfer-view",
            h4 { class: "transfer-title", "Cross-Chain Transfer" }
            
            div { class: "transfer-flow",
                // Source chain
                div { class: "transfer-chain source",
                    div { class: "chain-badge", "{props.transfer.from_chain}" }
                    div { class: "chain-tx",
                        TxHashDisplay {
                            tx_hash: props.transfer.source_tx.clone(),
                            chain: props.transfer.from_chain.clone(),
                            explorer_url: None,
                        }
                    }
                }
                
                // Transfer progress
                div { class: "transfer-progress",
                    div { class: "progress-line" }
                    div {
                        class: "progress-status",
                        style: format!("color: {}", props.transfer.status.seal_state().dot_color()),
                        "{props.transfer.status.label()}"
                    }
                    
                    // Progress steps
                    div { class: "progress-steps",
                        for step in transfer_steps(&props.transfer) {
                            div {
                                class: "progress-step",
                                class: if step.completed { "completed" },
                                class: if step.current { "current" },
                                div { class: "step-dot" }
                                span { class: "step-label", "{step.label}" }
                            }
                        }
                    }
                }
                
                // Destination chain
                div { class: "transfer-chain dest",
                    div { class: "chain-badge", "{props.transfer.to_chain}" }
                    if let Some(ref tx) = props.transfer.dest_tx {
                        div { class: "chain-tx",
                            TxHashDisplay {
                                tx_hash: tx.clone(),
                                chain: props.transfer.to_chain.clone(),
                                explorer_url: None,
                            }
                        }
                    } else {
                        div { class: "chain-pending", "Awaiting..." }
                    }
                }
            }
            
            // Transfer metadata
            div { class: "transfer-meta",
                div { class: "meta-item",
                    span { class: "meta-label", "Started: " }
                    span { class: "meta-value", "{format_datetime(props.transfer.start_time)}" }
                }
                if let Some(time) = props.transfer.completion_time {
                    div { class: "meta-item",
                        span { class: "meta-label", "Completed: " }
                        span { class: "meta-value", "{format_datetime(time)}" }
                    }
                }
            }
        }
    }
}

struct TransferStep {
    label: &'static str,
    completed: bool,
    current: bool,
}

fn transfer_steps(transfer: &TransferSegment) -> Vec<TransferStep> {
    let status_order = [
        TransferStatus::Pending,
        TransferStatus::SourceConfirmed,
        TransferStatus::ProofGenerated,
        TransferStatus::DestPending,
        TransferStatus::Completed,
    ];
    
    let current_idx = status_order.iter()
        .position(|s| *s == transfer.status)
        .unwrap_or(0);
    
    vec![
        TransferStep { label: "Initiate", completed: current_idx > 0, current: current_idx == 0 },
        TransferStep { label: "Confirm", completed: current_idx > 1, current: current_idx == 1 },
        TransferStep { label: "Generate Proof", completed: current_idx > 2, current: current_idx == 2 },
        TransferStep { label: "Verify", completed: current_idx > 3, current: current_idx == 3 },
        TransferStep { label: "Complete", completed: current_idx > 4, current: current_idx == 4 },
    ]
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

fn format_datetime(timestamp: u64) -> String {
    // In production, use chrono or similar
    format!("{}", timestamp)
}
