//! Seal state hook.

use dioxus::prelude::*;
use csv_store::state::domain::{SealRecord, SealStatus};
use csv_core::ChainId;

/// Seal state.
#[derive(Clone, PartialEq)]
pub struct SealState {
    pub seals: Vec<SealRecord>,
    pub loading: bool,
}

/// Seal context.
#[derive(Clone, Copy)]
pub struct SealContext {
    pub state: Signal<SealState>,
}

impl SealContext {
    pub fn add_seal(&mut self, seal: SealRecord) {
        self.state.write().seals.push(seal);
    }

    pub fn update_seal(&mut self, seal_ref: &str, status: SealStatus) {
        if let Some(seal) = self.state.write().seals.iter_mut().find(|s| s.seal_ref == seal_ref) {
            seal.status = status;
        }
    }

    pub fn get_seals_by_chain(&self, chain: ChainId) -> Vec<SealRecord> {
        self.state.read().seals.iter()
            .filter(|s| s.chain == chain)
            .cloned()
            .collect()
    }
}

/// Seal provider component.
#[component]
pub fn SealProvider(children: Element) -> Element {
    let mut state = use_signal(|| SealState {
        seals: Vec::new(),
        loading: false,
    });

    use_context_provider(|| SealContext { state });
    
    rsx! { { children } }
}

/// Hook to access seal state.
pub fn use_seals() -> SealContext {
    use_context::<SealContext>()
}
