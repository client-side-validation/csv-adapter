//! Network state hook.

use dioxus::prelude::*;
use csv_adapter_core::Chain;
use crate::services::NetworkType;

/// Network state.
#[derive(Clone, PartialEq)]
pub struct NetworkState {
    pub networks: std::collections::HashMap<Chain, NetworkType>,
}

/// Network context.
pub struct NetworkContext {
    pub state: Signal<NetworkState>,
}

impl NetworkContext {
    pub fn get_network(&self, chain: Chain) -> NetworkType {
        self.state.read().networks.get(&chain).copied()
            .unwrap_or(NetworkType::Testnet)
    }

    pub fn set_network(&mut self, chain: Chain, network: NetworkType) {
        self.state.write().networks.insert(chain, network);
    }

    pub fn is_testnet(&self, chain: Chain) -> bool {
        self.get_network(chain).is_testnet()
    }
}

/// Network provider component.
#[component]
pub fn NetworkProvider() -> Element {
    let mut state = use_signal(|| NetworkState {
        networks: [
            (Chain::Bitcoin, NetworkType::Testnet),
            (Chain::Ethereum, NetworkType::Testnet),
            (Chain::Sui, NetworkType::Testnet),
            (Chain::Aptos, NetworkType::Testnet),
            (Chain::Solana, NetworkType::Testnet),
        ].into_iter().collect(),
    });

    use_context_provider(|| NetworkContext { state });
    
    rsx! {
        Outlet {}
    }
}

/// Hook to access network state.
pub fn use_network() -> NetworkContext {
    use_context::<NetworkContext>().unwrap()
}
