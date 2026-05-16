//! UI components module.
#![allow(unused_imports)] // Intentional re-exports for public API

pub mod card;
pub mod chain_display;
pub mod design_tokens;
pub mod dropdown;
pub mod hash_display;
pub mod header;
pub mod onboarding;
pub mod proof_view;
pub mod seal_status;
pub mod seal_view;
pub mod sidebar;

pub use card::Card;
pub use chain_display::{ChainDisplay, NetworkDisplay, all_chain_displays, all_network_displays};
pub use design_tokens::{SealState, inject_design_tokens, seal_state_class};
pub use dropdown::Dropdown;
pub use hash_display::{AddressDisplay, HashDisplay, TxHashDisplay, shorten_hash};
pub use header::Header;
pub use onboarding::{OnboardingChecklist, OnboardingFlow, OnboardingStep};
pub use proof_view::{CrossChainProof, ProofInspector, ProofStatus, ValidatorSignature};
pub use seal_status::{SealIndicator, SealLifecycle, SealStatusBadge};
pub use seal_view::{SealEvent, SealVisualizer, TransferSegment, TransferStatus};
pub use sidebar::Sidebar;
