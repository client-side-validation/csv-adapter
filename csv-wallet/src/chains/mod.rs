//! ChainId-specific integrations.

use csv_core::ChainId;

pub mod aptos;
pub mod bitcoin;
pub mod ethereum;
pub mod solana;
pub mod sui;

/// Chains currently supported by the wallet UI.
pub fn supported_wallet_chains() -> Vec<ChainId> {
    vec![
        ChainId::new("bitcoin"),
        ChainId::new("ethereum"),
        ChainId::new("sui"),
        ChainId::new("aptos"),
        ChainId::new("solana"),
    ]
}
