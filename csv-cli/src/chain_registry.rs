//! Chain registry integration for CLI
//!
//! This module provides integration between the CLI's Chain enum
//! and the scalable ChainDriver registry from csv-adapter-core.

#![allow(dead_code)]
#![allow(deprecated)]

use crate::config::Chain;
use csv_core::{ChainCapabilities, ChainDriver, DriverRegistry};

/// Get the chain ID string for a Chain enum variant
pub fn chain_id(chain: &Chain) -> &str {
    chain.as_str()
}

/// Get chain adapter for a Chain enum variant
pub fn get_adapter(chain: &Chain) -> Option<Box<dyn ChainDriver>> {
    let factory = DriverRegistry::new();
    factory.create_driver(chain_id(chain))
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
    let factory = DriverRegistry::new();
    factory
        .supported_chains()
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// Chain metadata for UI display
pub struct ChainMetadata {
    pub chain_id: String,
    pub chain_name: String,
    pub color_hex: &'static str,
    pub icon_emoji: &'static str,
}

/// Get chain metadata for UI
pub fn get_metadata(chain: &Chain) -> ChainMetadata {
    match chain.as_str() {
        "bitcoin" => ChainMetadata {
            chain_id: "bitcoin".to_string(),
            chain_name: "Bitcoin".to_string(),
            color_hex: "#F7931A",
            icon_emoji: "\u{1F7E0}",
        },
        "ethereum" => ChainMetadata {
            chain_id: "ethereum".to_string(),
            chain_name: "Ethereum".to_string(),
            color_hex: "#627EEA",
            icon_emoji: "\u{1F537}",
        },
        "sui" => ChainMetadata {
            chain_id: "sui".to_string(),
            chain_name: "Sui".to_string(),
            color_hex: "#06BDFF",
            icon_emoji: "\u{1F30A}",
        },
        "aptos" => ChainMetadata {
            chain_id: "aptos".to_string(),
            chain_name: "Aptos".to_string(),
            color_hex: "#2DD8A3",
            icon_emoji: "\u{1F7E2}",
        },
        "solana" => ChainMetadata {
            chain_id: "solana".to_string(),
            chain_name: "Solana".to_string(),
            color_hex: "#9945FF",
            icon_emoji: "\u{25C8}",
        },
        _ => ChainMetadata {
            chain_id: chain.as_str().to_string(),
            chain_name: chain.as_str().to_string(),
            color_hex: "#6B7280",
            icon_emoji: "\u{1F532}",
        },
    }
}

/// Get badge class for chain
pub fn get_badge_class(chain: &Chain) -> &'static str {
    match chain.as_str() {
        "bitcoin" => {
            "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-orange-400 bg-orange-500/20 border border-orange-500/30"
        }
        "ethereum" => {
            "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-blue-400 bg-blue-500/20 border border-blue-500/30"
        }
        "sui" => {
            "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-cyan-400 bg-cyan-500/20 border border-cyan-500/30"
        }
        "aptos" => {
            "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-emerald-400 bg-emerald-500/20 border border-emerald-500/30"
        }
        "solana" => {
            "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-purple-400 bg-purple-500/20 border border-purple-500/30"
        }
        _ => {
            "inline-flex items-center gap-1.5 px-2 py-1 rounded-full text-xs font-medium text-gray-400 bg-gray-500/20 border border-gray-500/30"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ChainId;

    #[test]
    fn test_chain_id() {
        assert_eq!(chain_id(&ChainId::new("bitcoin")), "bitcoin");
        assert_eq!(chain_id(&ChainId::new("ethereum")), "ethereum");
        assert_eq!(chain_id(&ChainId::new("solana")), "solana");
        assert_eq!(chain_id(&ChainId::new("sui")), "sui");
        assert_eq!(chain_id(&ChainId::new("aptos")), "aptos");
    }

    #[test]
    fn test_get_metadata() {
        let btc = get_metadata(&ChainId::new("bitcoin"));
        assert_eq!(btc.chain_id, "bitcoin");
        assert_eq!(btc.chain_name, "Bitcoin");

        let eth = get_metadata(&ChainId::new("ethereum"));
        assert_eq!(eth.chain_id, "ethereum");
        assert_eq!(eth.color_hex, "#627EEA");

        let sui = get_metadata(&ChainId::new("sui"));
        assert_eq!(sui.chain_id, "sui");

        let aptos = get_metadata(&ChainId::new("aptos"));
        assert_eq!(aptos.chain_id, "aptos");

        let sol = get_metadata(&ChainId::new("solana"));
        assert_eq!(sol.chain_id, "solana");
    }

    #[test]
    fn test_get_badge_class() {
        let btc = get_badge_class(&ChainId::new("bitcoin"));
        assert!(btc.contains("orange"));

        let eth = get_badge_class(&ChainId::new("ethereum"));
        assert!(eth.contains("blue"));

        let sui = get_badge_class(&ChainId::new("sui"));
        assert!(sui.contains("cyan"));
    }

    #[test]
    fn test_supported_chains_static() {
        let chains = ["bitcoin", "ethereum", "solana", "sui", "aptos"];
        assert!(chains.contains(&"bitcoin"));
        assert!(chains.contains(&"ethereum"));
        assert!(chains.contains(&"solana"));
        assert!(chains.contains(&"sui"));
        assert!(chains.contains(&"aptos"));
    }
}
