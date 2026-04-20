//! Chain registry integration for CLI
//!
//! This module provides integration between the CLI's Chain enum
//! and the scalable ChainAdapter registry from csv-adapter-core.

#![allow(dead_code)]

use csv_adapter_core::{AdapterFactory, ChainAdapter, ChainCapabilities};
use crate::config::Chain;

/// Get the chain ID string for a Chain enum variant
pub fn chain_id(chain: &Chain) -> &'static str {
    match chain {
        Chain::Bitcoin => "bitcoin",
        Chain::Ethereum => "ethereum",
        Chain::Sui => "sui",
        Chain::Aptos => "aptos",
        Chain::Solana => "solana",
    }
}

/// Get chain adapter for a Chain enum variant
pub fn get_adapter(chain: &Chain) -> Option<Box<dyn ChainAdapter>> {
    let factory = AdapterFactory::new();
    factory.create_adapter(chain_id(chain))
}

/// Get chain capabilities for a Chain enum variant
pub fn get_capabilities(chain: &Chain) -> Option<ChainCapabilities> {
    get_adapter(chain).map(|a| a.capabilities())
}

/// Check if a chain supports a specific capability
pub fn supports_capability(chain: &Chain, check: impl Fn(&ChainCapabilities) -> bool) -> bool {
    get_capabilities(chain).map(|c| check(&c)).unwrap_or(false)
}

/// Check if chain supports NFTs
pub fn supports_nfts(chain: &Chain) -> bool {
    supports_capability(chain, |c| c.supports_nfts)
}

/// Check if chain supports smart contracts
pub fn supports_smart_contracts(chain: &Chain) -> bool {
    supports_capability(chain, |c| c.supports_smart_contracts)
}

/// Get all supported chains from registry
pub fn supported_chains() -> Vec<String> {
    let factory = AdapterFactory::new();
    factory.supported_chains().iter().map(|s| s.to_string()).collect()
}

/// Chain metadata for UI display
pub struct ChainMetadata {
    pub chain_id: &'static str,
    pub chain_name: &'static str,
    pub color_hex: &'static str,
    pub icon_emoji: &'static str,
}

/// Get chain metadata for UI
pub fn get_metadata(chain: &Chain) -> ChainMetadata {
    match chain {
        Chain::Bitcoin => ChainMetadata {
            chain_id: "bitcoin",
            chain_name: "Bitcoin",
            color_hex: "#F7931A",
            icon_emoji: "\u{1F7E0}",
        },
        Chain::Ethereum => ChainMetadata {
            chain_id: "ethereum",
            chain_name: "Ethereum",
            color_hex: "#627EEA",
            icon_emoji: "\u{1F537}",
        },
        Chain::Sui => ChainMetadata {
            chain_id: "sui",
            chain_name: "Sui",
            color_hex: "#06BDFF",
            icon_emoji: "\u{1F30A}",
        },
        Chain::Aptos => ChainMetadata {
            chain_id: "aptos",
            chain_name: "Aptos",
            color_hex: "#2DD8A3",
            icon_emoji: "\u{1F7E2}",
        },
        Chain::Solana => ChainMetadata {
            chain_id: "solana",
            chain_name: "Solana",
            color_hex: "#9945FF",
            icon_emoji: "\u{25C8}",
        },
    }
}

/// Get badge class for chain
pub fn get_badge_class(chain: &Chain) -> &'static str {
    match chain {
        Chain::Bitcoin => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-orange-400 bg-orange-500/20 border border-orange-500/30",
        Chain::Ethereum => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-blue-400 bg-blue-500/20 border border-blue-500/30",
        Chain::Sui => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-cyan-400 bg-cyan-500/20 border border-cyan-500/30",
        Chain::Aptos => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-emerald-400 bg-emerald-500/20 border border-emerald-500/30",
        Chain::Solana => "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-purple-400 bg-purple-500/20 border border-purple-500/30",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chain_id() {
        assert_eq!(chain_id(&Chain::Bitcoin), "bitcoin");
        assert_eq!(chain_id(&Chain::Ethereum), "ethereum");
        assert_eq!(chain_id(&Chain::Solana), "solana");
        assert_eq!(chain_id(&Chain::Sui), "sui");
        assert_eq!(chain_id(&Chain::Aptos), "aptos");
    }

    #[test]
    fn test_get_adapter() {
        assert!(get_adapter(&Chain::Bitcoin).is_some());
        assert!(get_adapter(&Chain::Ethereum).is_some());
        assert!(get_adapter(&Chain::Solana).is_some());
        assert!(get_adapter(&Chain::Sui).is_some());
        assert!(get_adapter(&Chain::Aptos).is_some());
    }

    #[test]
    fn test_get_capabilities() {
        let bitcoin_caps = get_capabilities(&Chain::Bitcoin).unwrap();
        assert!(!bitcoin_caps.supports_smart_contracts);

        let ethereum_caps = get_capabilities(&Chain::Ethereum).unwrap();
        assert!(ethereum_caps.supports_smart_contracts);
    }

    #[test]
    fn test_supports_nfts() {
        assert!(supports_nfts(&Chain::Bitcoin));
        assert!(supports_nfts(&Chain::Ethereum));
        assert!(supports_nfts(&Chain::Solana));
    }

    #[test]
    fn test_supported_chains() {
        let chains = supported_chains();
        assert!(chains.iter().any(|c| c == "bitcoin"));
        assert!(chains.iter().any(|c| c == "ethereum"));
        assert!(chains.iter().any(|c| c == "solana"));
        assert!(chains.iter().any(|c| c == "sui"));
        assert!(chains.iter().any(|c| c == "aptos"));
    }
}
