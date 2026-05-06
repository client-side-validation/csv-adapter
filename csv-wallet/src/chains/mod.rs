//! Chain-specific integrations.

use csv_core::{AdapterFactory, Chain};

pub mod aptos;
pub mod bitcoin;
pub mod ethereum;
pub mod solana;
pub mod sui;

/// Chains currently supported by the plug-and-play adapter factory and wallet UI.
pub fn supported_wallet_chains() -> Vec<Chain> {
    let factory = AdapterFactory::new();
    Chain::all()
        .iter()
        .copied()
        .filter(|chain| factory.is_supported(chain.id()))
        .collect()
}
