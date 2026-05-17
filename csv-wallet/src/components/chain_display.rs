//! Display wrapper types for ChainId and Network to work with Dropdown component.

use crate::chains::supported_wallet_chains;
use crate::context::Network;
use csv_core::ChainId;

/// Display wrapper for ChainId with emoji and name.
pub struct ChainDisplay(pub ChainId);

impl std::fmt::Display for ChainDisplay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.as_str() {
            "bitcoin" => write!(f, "\u{1F7E0} Bitcoin"),
            "ethereum" => write!(f, "\u{1F537} Ethereum"),
            "sui" => write!(f, "\u{1F30A} Sui"),
            "aptos" => write!(f, "\u{1F7E2} Aptos"),
            "solana" => write!(f, "\u{2600} Solana"),
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
        Self(self.0.clone())
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
    supported_wallet_chains()
        .into_iter()
        .map(ChainDisplay)
        .collect()
}

/// Helper to get all network display options.
pub fn all_network_displays() -> Vec<NetworkDisplay> {
    vec![
        NetworkDisplay(Network::Dev),
        NetworkDisplay(Network::Test),
        NetworkDisplay(Network::Main),
    ]
}
