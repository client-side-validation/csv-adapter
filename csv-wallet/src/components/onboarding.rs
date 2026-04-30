//! Onboarding Flow Component
//!
//! First-time user onboarding with:
//! - Welcome screen
//! - Wallet setup guide
//! - Seal concept explanation
//! - Security best practices
//! - Interactive tutorial

use dioxus::prelude::*;
use crate::components::design_tokens::SealState;
use crate::components::seal_status::SealStatusBadge;
use crate::components::hash_display::HashDisplay;

/// Onboarding step definitions.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum OnboardingStep {
    Welcome,
    WhatIsSeal,
    WalletSetup,
    CreateFirstRight,
    SecurityTips,
    Complete,
}

impl OnboardingStep {
    fn title(&self) -> &'static str {
        match self {
            OnboardingStep::Welcome => "Welcome to CSV Wallet",
            OnboardingStep::WhatIsSeal => "What is a Seal?",
            OnboardingStep::WalletSetup => "Set Up Your Wallet",
            OnboardingStep::CreateFirstRight => "Create Your First Right",
            OnboardingStep::SecurityTips => "Security Tips",
            OnboardingStep::Complete => "You're Ready!",
        }
    }
    
    fn description(&self) -> &'static str {
        match self {
            OnboardingStep::Welcome => "CSV (Cross-Seal Validation) enables secure cross-chain asset transfers using cryptographic seals.",
            OnboardingStep::WhatIsSeal => "A seal is a unique digital fingerprint that represents ownership of an asset on a specific blockchain.",
            OnboardingStep::WalletSetup => "Connect or create a wallet to start managing your seals and rights.",
            OnboardingStep::CreateFirstRight => "Create your first right to experience how seals work in practice.",
            OnboardingStep::SecurityTips => "Follow these best practices to keep your seals and assets safe.",
            OnboardingStep::Complete => "You now understand the basics. Start exploring the CSV ecosystem!",
        }
    }
    
    fn progress(&self) -> (usize, usize) {
        match self {
            OnboardingStep::Welcome => (0, 5),
            OnboardingStep::WhatIsSeal => (1, 5),
            OnboardingStep::WalletSetup => (2, 5),
            OnboardingStep::CreateFirstRight => (3, 5),
            OnboardingStep::SecurityTips => (4, 5),
            OnboardingStep::Complete => (5, 5),
        }
    }
    
    fn next(&self) -> Option<OnboardingStep> {
        match self {
            OnboardingStep::Welcome => Some(OnboardingStep::WhatIsSeal),
            OnboardingStep::WhatIsSeal => Some(OnboardingStep::WalletSetup),
            OnboardingStep::WalletSetup => Some(OnboardingStep::CreateFirstRight),
            OnboardingStep::CreateFirstRight => Some(OnboardingStep::SecurityTips),
            OnboardingStep::SecurityTips => Some(OnboardingStep::Complete),
            OnboardingStep::Complete => None,
        }
    }
    
    fn prev(&self) -> Option<OnboardingStep> {
        match self {
            OnboardingStep::Welcome => None,
            OnboardingStep::WhatIsSeal => Some(OnboardingStep::Welcome),
            OnboardingStep::WalletSetup => Some(OnboardingStep::WhatIsSeal),
            OnboardingStep::CreateFirstRight => Some(OnboardingStep::WalletSetup),
            OnboardingStep::SecurityTips => Some(OnboardingStep::CreateFirstRight),
            OnboardingStep::Complete => Some(OnboardingStep::SecurityTips),
        }
    }
}

/// Props for the OnboardingFlow component.
#[derive(Props, Clone, PartialEq)]
pub struct OnboardingFlowProps {
    /// Called when onboarding is completed or skipped.
    pub on_complete: EventHandler<()>,
    /// Called when user wants to create a wallet.
    #[props(default)]
    pub on_create_wallet: Option<EventHandler<()>>,
    /// Called when user wants to import a wallet.
    #[props(default)]
    pub on_import_wallet: Option<EventHandler<()>>,
    /// Whether to allow skipping.
    #[props(default = true)]
    pub allow_skip: bool,
    /// Additional CSS classes.
    #[props(default)]
    pub class: String,
}

/// Main onboarding flow component.
#[allow(non_snake_case)]
pub fn OnboardingFlow(props: OnboardingFlowProps) -> Element {
    let mut current_step = use_signal(|| OnboardingStep::Welcome);
    let completed_steps = use_signal(|| std::collections::HashSet::<OnboardingStep>::new());
    
    let (progress, total) = current_step().progress();
    let progress_percent = (progress * 100) / total;
    
    let mut go_next = {
        let mut current = current_step.clone();
        let mut completed = completed_steps.clone();
        move || {
            completed.write().insert(current());
            if let Some(next) = current().next() {
                current.set(next);
            }
        }
    };
    
    let mut go_prev = {
        let mut current = current_step.clone();
        move || {
            if let Some(prev) = current().prev() {
                current.set(prev);
            }
        }
    };
    
    let complete = {
        let on_complete = props.on_complete.clone();
        move || {
            on_complete.call(());
        }
    };
    
    rsx! {
        div { class: "onboarding-flow {props.class}",
            // Progress bar
            div { class: "onboarding-progress",
                div { 
                    class: "onboarding-progress-bar",
                    style: format!("width: {}%", progress_percent),
                }
            }
            
            // Header with skip button
            div { class: "onboarding-header",
                div { class: "onboarding-brand",
                    span { class: "brand-icon", "🔒" }
                    span { class: "brand-name", "CSV Wallet" }
                }
                
                if props.allow_skip && current_step() != OnboardingStep::Complete {
                    button {
                        class: "onboarding-skip",
                        onclick: move |_| complete(),
                        "Skip Tour"
                    }
                }
            }
            
            // Step content
            div { class: "onboarding-content",
                match current_step() {
                    OnboardingStep::Welcome => rsx! {
                        WelcomeStep { on_next: go_next.clone() }
                    },
                    OnboardingStep::WhatIsSeal => rsx! {
                        WhatIsSealStep { on_next: go_next.clone() }
                    },
                    OnboardingStep::WalletSetup => rsx! {
                        WalletSetupStep {
                            on_next: go_next.clone(),
                            on_create_wallet: props.on_create_wallet.clone(),
                            on_import_wallet: props.on_import_wallet.clone(),
                        }
                    },
                    OnboardingStep::CreateFirstRight => rsx! {
                        CreateRightStep { on_next: go_next.clone() }
                    },
                    OnboardingStep::SecurityTips => rsx! {
                        SecurityTipsStep { on_next: go_next.clone() }
                    },
                    OnboardingStep::Complete => rsx! {
                        CompleteStep { on_finish: complete.clone() }
                    },
                }
            }
            
            // Navigation
            div { class: "onboarding-nav",
                if current_step().prev().is_some() {
                    button {
                        class: "onboarding-btn prev",
                        onclick: move |_| go_prev(),
                        "← Previous"
                    }
                } else {
                    div {} // Spacer
                }
                
                // Step indicators
                div { class: "onboarding-dots",
                    for step in [
                        OnboardingStep::Welcome,
                        OnboardingStep::WhatIsSeal,
                        OnboardingStep::WalletSetup,
                        OnboardingStep::CreateFirstRight,
                        OnboardingStep::SecurityTips,
                        OnboardingStep::Complete,
                    ] {
                        span {
                            class: "onboarding-dot",
                            class: if step == current_step() { "active" },
                            class: if completed_steps().contains(&step) { "completed" },
                            onclick: move |_| {
                                // Allow clicking dots to jump to completed steps
                                if completed_steps().contains(&step) || 
                                   current_step().prev() == Some(step) {
                                    current_step.set(step);
                                }
                            },
                        }
                    }
                }
                
                if let Some(_next) = current_step().next() {
                    button {
                        class: "onboarding-btn next",
                        onclick: move |_| go_next(),
                        "Next →"
                    }
                } else if current_step() == OnboardingStep::Complete {
                    button {
                        class: "onboarding-btn finish",
                        onclick: move |_| complete(),
                        "Get Started 🚀"
                    }
                }
            }
            
            // Step counter
            div { class: "onboarding-counter",
                "Step {progress + 1} of {total + 1}"
            }
        }
    }
}

/// Welcome step content.
#[derive(Props, Clone, PartialEq)]
struct WelcomeStepProps {
    on_next: EventHandler<()>,
}

#[allow(non_snake_case)]
fn WelcomeStep(props: WelcomeStepProps) -> Element {
    rsx! {
        div { class: "onboarding-step welcome",
            div { class: "welcome-icon", "🔐" }
            
            h2 { class: "step-title", "Welcome to CSV Wallet" }
            
            p { class: "step-description",
                "CSV (Cross-Seal Validation) is a revolutionary protocol for secure, "
                "verifiable cross-chain asset transfers using cryptographic seals."
            }
            
            div { class: "feature-highlights",
                div { class: "feature-item",
                    span { class: "feature-icon", "🔗" }
                    h4 { "Cross-Chain" }
                    p { "Transfer assets between any supported blockchains" }
                }
                div { class: "feature-item",
                    span { class: "feature-icon", "🛡️" }
                    h4 { "Secure by Design" }
                    p { "Cryptographic proofs ensure your assets are safe" }
                }
                div { class: "feature-item",
                    span { class: "feature-icon", "⚡" }
                    h4 { "Fast & Efficient" }
                    p { "Optimized for speed without compromising security" }
                }
            }
            
            button {
                class: "onboarding-btn primary",
                onclick: move |_| props.on_next.call(()),
                "Start Tutorial →"
            }
        }
    }
}

/// What is a Seal explanation.
#[derive(Props, Clone, PartialEq)]
struct WhatIsSealStepProps {
    on_next: EventHandler<()>,
}

#[allow(non_snake_case)]
fn WhatIsSealStep(_props: WhatIsSealStepProps) -> Element {
    rsx! {
        div { class: "onboarding-step seal-intro",
            h2 { class: "step-title", "What is a Seal?" }
            
            p { class: "step-description",
                "A seal is a unique digital fingerprint that represents ownership "
                "of an asset on a specific blockchain. Think of it like a tamper-proof "
                "sticker that can only be used once."
            }
            
            // Visual seal example
            div { class: "seal-demo",
                div { class: "seal-card",
                    div { class: "seal-header",
                        span { class: "seal-badge-icon", "🏷️" }
                        span { class: "seal-id",
                            HashDisplay {
                                value: "seal_a1b2c3d4e5f6".to_string(),
                                prefix_len: 8,
                                suffix_len: 4,
                                show_copy: false,
                            }
                        }
                    }
                    div { class: "seal-body",
                        div { class: "seal-asset",
                            span { class: "asset-label", "Asset: " }
                            span { class: "asset-value", "1.5 ETH" }
                        }
                        div { class: "seal-owner",
                            span { class: "owner-label", "Owner: " }
                            span { class: "owner-value",
                                HashDisplay {
                                    value: "0x1234...5678".to_string(),
                                    prefix_len: 6,
                                    suffix_len: 4,
                                    show_copy: false,
                                }
                            }
                        }
                        div { class: "seal-status",
                            SealStatusBadge {
                                state: SealState::Active,
                                show_label: true,
                            }
                        }
                    }
                }
            }
            
            // Seal lifecycle
            div { class: "seal-lifecycle-intro",
                h4 { "Seal Lifecycle" }
                div { class: "lifecycle-steps",
                    div { class: "lifecycle-step",
                        div { class: "step-num", "1" }
                        span { "Created" }
                    }
                    div { class: "lifecycle-arrow", "→" }
                    div { class: "lifecycle-step",
                        div { class: "step-num", "2" }
                        span { "Locked" }
                    }
                    div { class: "lifecycle-arrow", "→" }
                    div { class: "lifecycle-step",
                        div { class: "step-num", "3" }
                        span { "Consumed" }
                    }
                }
            }
            
            p { class: "step-note",
                "Once a seal is consumed, it cannot be used again. "
                "This prevents double-spending across chains."
            }
        }
    }
}

/// Wallet setup step.
#[derive(Props, Clone, PartialEq)]
struct WalletSetupStepProps {
    on_next: EventHandler<()>,
    on_create_wallet: Option<EventHandler<()>>,
    on_import_wallet: Option<EventHandler<()>>,
}

#[allow(non_snake_case)]
fn WalletSetupStep(props: WalletSetupStepProps) -> Element {
    rsx! {
        div { class: "onboarding-step wallet-setup",
            h2 { class: "step-title", "Set Up Your Wallet" }
            
            p { class: "step-description",
                "To start using CSV, you'll need a wallet. "
                "This will generate your cryptographic keys for signing transactions."
            }
            
            div { class: "wallet-options",
                div { class: "wallet-option",
                    span { class: "option-icon", "🆕" }
                    h4 { "Create New Wallet" }
                    p { "Generate a fresh wallet with a new mnemonic phrase" }
                    button {
                        class: "wallet-btn",
                        onclick: move |_| {
                            if let Some(ref handler) = props.on_create_wallet {
                                handler.call(());
                            }
                            props.on_next.call(());
                        },
                        "Create Wallet"
                    }
                }
                
                div { class: "wallet-option",
                    span { class: "option-icon", "📥" }
                    h4 { "Import Existing" }
                    p { "Restore from mnemonic phrase or private key" }
                    button {
                        class: "wallet-btn secondary",
                        onclick: move |_| {
                            if let Some(ref handler) = props.on_import_wallet {
                                handler.call(());
                            }
                            props.on_next.call(());
                        },
                        "Import Wallet"
                    }
                }
            }
            
            div { class: "wallet-note",
                p { "🔐 Your keys are stored securely and never leave your device." }
            }
        }
    }
}

/// Create first right step.
#[derive(Props, Clone, PartialEq)]
struct CreateRightStepProps {
    on_next: EventHandler<()>,
}

#[allow(non_snake_case)]
fn CreateRightStep(props: CreateRightStepProps) -> Element {
    rsx! {
        div { class: "onboarding-step create-right",
            h2 { class: "step-title", "Create Your First Right" }
            
            p { class: "step-description",
                "A Right represents ownership of an asset. "
                "When you create a right, a seal is automatically generated to protect it."
            }
            
            // Right creation flow
            div { class: "right-flow",
                div { class: "flow-step",
                    div { class: "flow-icon", "📝" }
                    span { class: "flow-label", "Define Right" }
                    p { "Specify asset, amount, and conditions" }
                }
                div { class: "flow-arrow", "↓" }
                div { class: "flow-step",
                    div { class: "flow-icon", "🏷️" }
                    span { class: "flow-label", "Seal Created" }
                    p { "Cryptographic seal is generated" }
                }
                div { class: "flow-arrow", "↓" }
                div { class: "flow-step",
                    div { class: "flow-icon", "✓" }
                    span { class: "flow-label", "Anchored" }
                    p { "Right is recorded on blockchain" }
                }
            }
            
            div { class: "right-example",
                h4 { "Example Right" }
                div { class: "example-card",
                    div { class: "example-row",
                        span { class: "example-label", "Right: " }
                        span { class: "example-value", "Transfer 1.5 ETH to Bob" }
                    }
                    div { class: "example-row",
                        span { class: "example-label", "Condition: " }
                        span { class: "example-value", "After 24 hours" }
                    }
                    div { class: "example-row",
                        span { class: "example-label", "Seal: " }
                        span { class: "example-value seal-ref",
                            HashDisplay {
                                value: "seal_abc123".to_string(),
                                prefix_len: 6,
                                suffix_len: 4,
                                show_copy: false,
                            }
                        }
                    }
                }
            }
            
            button {
                class: "onboarding-btn",
                onclick: move |_| props.on_next.call(()),
                "Continue →"
            }
        }
    }
}

/// Security tips step.
#[derive(Props, Clone, PartialEq)]
struct SecurityTipsStepProps {
    on_next: EventHandler<()>,
}

#[allow(non_snake_case)]
fn SecurityTipsStep(props: SecurityTipsStepProps) -> Element {
    rsx! {
        div { class: "onboarding-step security-tips",
            h2 { class: "step-title", "Security Tips" }
            
            p { class: "step-description",
                "Follow these best practices to keep your seals and assets safe."
            }
            
            div { class: "tips-list",
                div { class: "tip-item",
                    span { class: "tip-icon", "🔐" }
                    div { class: "tip-content",
                        h4 { "Backup Your Keys" }
                        p { "Write down your mnemonic phrase and store it in a secure, offline location." }
                    }
                }
                
                div { class: "tip-item",
                    span { class: "tip-icon", "🚫" }
                    div { class: "tip-content",
                        h4 { "Never Share Private Keys" }
                        p { "Your private keys grant full access to your assets. Never share them with anyone." }
                    }
                }
                
                div { class: "tip-item",
                    span { class: "tip-icon", "✓" }
                    div { class: "tip-content",
                        h4 { "Verify Before Confirming" }
                        p { "Always double-check transaction details before signing." }
                    }
                }
                
                div { class: "tip-item",
                    span { class: "tip-icon", "📊" }
                    div { class: "tip-content",
                        h4 { "Monitor Your Seals" }
                        p { "Regularly check the status of your seals in the dashboard." }
                    }
                }
            }
            
            div { class: "security-note",
                p { "💡 CSV seals are cryptographically secure, but your keys are your responsibility." }
            }
            
            button {
                class: "onboarding-btn",
                onclick: move |_| props.on_next.call(()),
                "Finish →"
            }
        }
    }
}

/// Complete step.
#[derive(Props, Clone, PartialEq)]
struct CompleteStepProps {
    on_finish: EventHandler<()>,
}

#[allow(non_snake_case)]
fn CompleteStep(props: CompleteStepProps) -> Element {
    rsx! {
        div { class: "onboarding-step complete",
            div { class: "complete-icon", "🎉" }
            
            h2 { class: "step-title", "You're Ready!" }
            
            p { class: "step-description",
                "You've learned the fundamentals of CSV. "
                "Start by creating your first right or exploring the dashboard."
            }
            
            div { class: "next-actions",
                div { class: "action-card",
                    span { class: "action-icon", "➕" }
                    h4 { "Create a Right" }
                    p { "Define ownership for your first asset" }
                }
                div { class: "action-card",
                    span { class: "action-icon", "🔗" }
                    h4 { "Transfer Cross-Chain" }
                    p { "Move assets between blockchains" }
                }
                div { class: "action-card",
                    span { class: "action-icon", "🔍" }
                    h4 { "Explore Dashboard" }
                    p { "View your seals and rights" }
                }
            }
            
            button {
                class: "onboarding-btn primary finish",
                onclick: move |_| props.on_finish.call(()),
                "Get Started 🚀"
            }
            
            p { class: "help-link",
                "Need help? Visit the "
                a { href: "#", "documentation" }
                " or join our "
                a { href: "#", "community" }
                "."
            }
        }
    }
}

/// Compact onboarding checklist for dashboard.
#[derive(Props, Clone, PartialEq)]
pub struct OnboardingChecklistProps {
    /// Items completed.
    pub completed: Vec<String>,
    /// Callback when item is clicked.
    pub on_action: EventHandler<String>,
    /// Whether to show the checklist.
    #[props(default = true)]
    pub visible: bool,
    /// Callback to dismiss.
    pub on_dismiss: EventHandler<()>,
}

/// Quick checklist for remaining onboarding tasks.
#[allow(non_snake_case)]
pub fn OnboardingChecklist(props: OnboardingChecklistProps) -> Element {
    if !props.visible {
        return rsx! {};
    }
    
    let items = vec![
        ("wallet", "Set up your wallet", "🔐"),
        ("right", "Create your first right", "📝"),
        ("transfer", "Try a cross-chain transfer", "🔗"),
        ("seal", "View seal lifecycle", "🏷️"),
    ];
    
    use std::rc::Rc;
    let completed = Rc::new(props.completed.clone());
    let completed_count = completed.len();
    let total = items.len();
    let progress = (completed_count * 100) / total;
    let on_action = props.on_action.clone();
    let on_dismiss = props.on_dismiss.clone();
    
    rsx! {
        div { class: "onboarding-checklist",
            div { class: "checklist-header",
                h4 { "Getting Started" }
                button {
                    class: "dismiss-btn",
                    onclick: move |_| on_dismiss.call(()),
                    "×"
                }
            }
            
            div { class: "checklist-progress",
                div { 
                    class: "progress-bar",
                    style: format!("width: {}%", progress),
                }
                span { class: "progress-text", "{completed_count}/{total}" }
            }
            
            div { class: "checklist-items",
                for (id, label, icon) in items {
                    div {
                        class: "checklist-item",
                        class: if completed.contains(&id.to_string()) { "completed" },
                        onclick: {
                            let completed = completed.clone();
                            let on_action = on_action.clone();
                            let id = id.to_string();
                            move |_| {
                                if !completed.contains(&id) {
                                    on_action.call(id.clone());
                                }
                            }
                        },
                        span { class: "item-icon", "{icon}" }
                        span { class: "item-label", "{label}" }
                        if completed.contains(&id.to_string()) {
                            span { class: "item-check", "✓" }
                        } else {
                            span { class: "item-arrow", "→" }
                        }
                    }
                }
            }
        }
    }
}
