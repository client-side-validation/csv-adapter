//! Seal Status Badge Component
//!
//! Visual indicators for seal states with:
//! - Color-coded status badges
//! - Pulse animation for pending states
//! - State transition indicators
//! - Countdown/timer for time-sensitive states

use dioxus::prelude::*;
use crate::components::design_tokens::SealState;

/// Props for the SealStatusBadge component.
#[derive(Props, Clone, PartialEq)]
pub struct SealStatusBadgeProps {
    /// Current seal state.
    pub state: SealState,
    /// Show pulse animation (default: true for pending).
    #[props(default = None)]
    pub pulse: Option<bool>,
    /// Size variant.
    #[props(default = Size::Medium)]
    pub size: Size,
    /// Show state label text.
    #[props(default = true)]
    pub show_label: bool,
    /// Additional CSS classes.
    #[props(default)]
    pub class: String,
    /// Optional block/time info for pending seals.
    #[props(default)]
    pub confirmation_info: Option<ConfirmationInfo>,
}

/// Size variants for the badge.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Size {
    #[default]
    Small,
    Medium,
    Large,
}

impl Size {
    fn class(&self) -> &'static str {
        match self {
            Size::Small => "seal-badge-sm",
            Size::Medium => "seal-badge-md",
            Size::Large => "seal-badge-lg",
        }
    }
}

/// Confirmation progress information.
#[derive(Clone, PartialEq)]
pub struct ConfirmationInfo {
    /// Current confirmation count.
    pub current: u32,
    /// Required confirmations.
    pub required: u32,
    /// Estimated time remaining (seconds).
    pub eta_seconds: Option<u32>,
}

/// Display a seal status badge with visual indicators.
pub fn SealStatusBadge(props: SealStatusBadgeProps) -> Element {
    let state_class = match props.state {
        SealState::Active => "seal-active",
        SealState::Pending => "seal-pending",
        SealState::Consumed => "seal-consumed",
        SealState::Locked => "seal-locked",
        SealState::Error => "seal-error",
    };
    
    let should_pulse = props.pulse.unwrap_or(props.state == SealState::Pending);
    let pulse_class = if should_pulse { "seal-pulse" } else { "" };
    let size_class = props.size.class();
    
    let confirmation_text = props.confirmation_info.as_ref().map(|info| {
        if let Some(eta) = info.eta_seconds {
            format!("{} / {} conf (~{}s)", info.current, info.required, eta)
        } else {
            format!("{} / {} conf", info.current, info.required)
        }
    });
    
    rsx! {
        div {
            class: "seal-status-badge {state_class} {pulse_class} {size_class} {props.class}",
            
            // Status dot
            span { class: "seal-status-dot" }
            
            // State label
            if props.show_label {
                span { class: "seal-status-label", "{props.state.label()}" }
            }
            
            // Confirmation progress (for pending)
            if let Some(text) = confirmation_text {
                if props.state == SealState::Pending {
                    span { class: "seal-confirmations", "({text})" }
                }
            }
        }
    }
}

/// Seal lifecycle timeline component.
#[derive(Props, Clone, PartialEq)]
pub struct SealLifecycleProps {
    /// Current state.
    pub current_state: SealState,
    /// Historical state transitions with timestamps.
    pub transitions: Vec<StateTransition>,
    /// Show future states.
    #[props(default = true)]
    pub show_future: bool,
    /// Additional CSS classes.
    #[props(default)]
    pub class: String,
}

/// A state transition event.
#[derive(Clone, PartialEq)]
pub struct StateTransition {
    pub state: SealState,
    pub timestamp: u64,
    pub tx_hash: Option<String>,
    pub note: Option<String>,
}

/// Display seal lifecycle as a timeline.
pub fn SealLifecycle(props: SealLifecycleProps) -> Element {
    let all_states = vec![
        SealState::Active,
        SealState::Pending,
        SealState::Locked,
        SealState::Consumed,
    ];
    
    let current_index = all_states.iter()
        .position(|s| *s == props.current_state)
        .unwrap_or(0);
    
    rsx! {
        div { class: "seal-lifecycle {props.class}",
            h4 { class: "seal-lifecycle-title", "Seal Lifecycle" }
            
            div { class: "seal-timeline",
                for (i, state) in all_states.iter().enumerate() {
                    div {
                        class: "seal-timeline-step",
                        class: if i < current_index { "completed" },
                        class: if i == current_index { "current" },
                        class: if i > current_index { "future" },
                        
                        // Step indicator
                        div { class: "seal-step-dot" }
                        
                        // Step label
                        span { class: "seal-step-label", "{state.label()}" }
                        
                        // Timestamp if available
                        if let Some(transition) = props.transitions.iter().find(|t| t.state == *state) {
                            span { 
                                class: "seal-step-time",
                                "{format_timestamp(transition.timestamp)}"
                            }
                        }
                    }
                }
            }
            
            // Current state details
            div { class: "seal-current-state",
                SealStatusBadge {
                    state: props.current_state,
                    size: Size::Large,
                    show_label: true,
                }
            }
        }
    }
}

/// Seal state indicator for lists/tables.
#[derive(Props, Clone, PartialEq)]
pub struct SealIndicatorProps {
    pub state: SealState,
    #[props(default = false)]
    pub compact: bool,
    #[props(default)]
    pub class: String,
}

/// Compact seal state indicator (just dot + optional text).
pub fn SealIndicator(props: SealIndicatorProps) -> Element {
    let class = if props.compact {
        "seal-indicator-compact"
    } else {
        "seal-indicator"
    };
    
    rsx! {
        span {
            class: "{class} {seal_state_class(props.state)} {props.class}",
            span { class: "seal-dot" }
            if !props.compact {
                span { class: "seal-text", "{props.state.label()}" }
            }
        }
    }
}

fn seal_state_class(state: SealState) -> &'static str {
    match state {
        SealState::Active => "seal-active",
        SealState::Pending => "seal-pending",
        SealState::Consumed => "seal-consumed",
        SealState::Locked => "seal-locked",
        SealState::Error => "seal-error",
    }
}

fn format_timestamp(timestamp: u64) -> String {
    // Simple formatting - in production, use a proper time library
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
