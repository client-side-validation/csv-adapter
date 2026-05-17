//! Network state hook.

use crate::chains::supported_wallet_chains;
use crate::services::network::NetworkType;
use csv_core::ChainId;
use dioxus::prelude::*;

/// Network state.
#[derive(Clone, PartialEq)]
pub struct NetworkState {
    pub networks: std::collections::HashMap<ChainId, NetworkType>,
}

/// Network context.
#[derive(Clone)]
pub struct NetworkContext {
    pub state: Signal<NetworkState>,
}

impl NetworkContext {
    pub fn get_network(&self, chain: ChainId) -> NetworkType {
        self.state
            .read()
            .networks
            .get(&chain)
            .copied()
            .unwrap_or(NetworkType::Testnet)
    }

    pub fn set_network(&mut self, chain: ChainId, network: NetworkType) {
        self.state.write().networks.insert(chain, network);
    }

    pub fn is_testnet(&self, chain: ChainId) -> bool {
        self.get_network(chain).is_testnet()
    }
}

/// Network provider component.
#[component]
pub fn NetworkProvider(children: Element) -> Element {
    let state = use_signal(|| NetworkState {
        networks: supported_wallet_chains()
            .into_iter()
            .map(|chain| (chain, NetworkType::Testnet))
            .collect(),
    });

    use_context_provider(|| NetworkContext { state });

    rsx! { { children } }
}

/// Hook to access network state.
pub fn use_network() -> NetworkContext {
    use_context::<NetworkContext>()
}
