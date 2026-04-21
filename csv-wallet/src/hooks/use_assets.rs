//! Asset state hook.

use dioxus::prelude::*;
use crate::assets::tracker::AssetRecord;
use csv_adapter_core::Chain;

/// Asset state.
#[derive(Clone, PartialEq)]
pub struct AssetState {
    pub assets: Vec<AssetRecord>,
    pub total_value_usd: f64,
    pub loading: bool,
}

/// Asset context.
#[derive(Clone, Copy)]
pub struct AssetContext {
    pub state: Signal<AssetState>,
}

impl AssetContext {
    pub fn add_asset(&mut self, asset: AssetRecord) {
        self.state.write().assets.push(asset);
    }

    pub fn update_value(&mut self, right_id: String, value: f64) {
        if let Some(asset) = self.state.write().assets.iter_mut()
            .find(|a| format!("{:x}", a.right_id.0) == right_id) 
        {
            asset.value = Some(value);
        }
    }

    pub fn get_assets_by_chain(&self, chain: Chain) -> Vec<AssetRecord> {
        self.state.read().assets.iter()
            .filter(|a| a.chain == chain)
            .cloned()
            .collect()
    }

    pub fn recalculate_total(&mut self) {
        self.state.write().total_value_usd = self.state.read().assets.iter()
            .filter_map(|a| a.value)
            .sum();
    }
}

/// Asset provider component.
#[component]
pub fn AssetProvider(children: Element) -> Element {
    let mut state = use_signal(|| AssetState {
        assets: Vec::new(),
        total_value_usd: 0.0,
        loading: false,
    });

    use_context_provider(|| AssetContext { state });
    
    rsx! { { children } }
}

/// Hook to access asset state.
pub fn use_assets() -> AssetContext {
    use_context::<AssetContext>()
}
