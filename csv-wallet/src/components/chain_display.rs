//! Display wrapper types for Chain and Network to work with Dropdown component.

use csv_adapter_core::Chain;
use crate::context::Network;

/// Display wrapper for Chain with emoji and name.
pub struct ChainDisplay(pub Chain);

impl std::fmt::Display for ChainDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Chain::Bitcoin => write!(f, "\u{1F7E0} Bitcoin"),
            Chain::Ethereum => write!(f, "\u{1F537} Ethereum"),
            Chain::Sui => write!(f, "\u{1F30A} Sui"),
            Chain::Aptos => write!(f, "\u{1F7E2} Aptos"),
            _ => write!(f, "Unknown"),
        }
    }
}

impl PartialEq for ChainDisplay {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Clone for ChainDisplay {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

/// Display wrapper for Network.
pub struct NetworkDisplay(pub Network);

impl std::fmt::Display for NetworkDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Network::Dev => write!(f, "Dev"),
            Network::Test => write!(f, "Test"),
            Network::Main => write!(f, "Main"),
        }
    }
}

impl PartialEq for NetworkDisplay {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl Clone for NetworkDisplay {
    fn clone(&self) -> Self {
        Self(self.0)
    }
}

/// Helper to get all chain display options.
pub fn all_chain_displays() -> Vec<ChainDisplay> {
    vec![
        ChainDisplay(Chain::Bitcoin),
        ChainDisplay(Chain::Ethereum),
        ChainDisplay(Chain::Sui),
        ChainDisplay(Chain::Aptos),
    ]
}

/// Helper to get all network display options.
pub fn all_network_displays() -> Vec<NetworkDisplay> {
    vec![
        NetworkDisplay(Network::Dev),
        NetworkDisplay(Network::Test),
        NetworkDisplay(Network::Main),
    ]
}
