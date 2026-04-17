//! Bitcoin chain adapter implementation for the new interface.

use super::super::chain_system::ChainInfo;

/// Bitcoin chain adapter for the new scalable system
pub struct BitcoinAdapter {
    chain_id: &'static str,
    chain_name: &'static str,
}

impl BitcoinAdapter {
    /// Create new Bitcoin adapter
    pub fn new() -> Self {
        Self {
            chain_id: "bitcoin",
            chain_name: "Bitcoin",
        }
    }
    
    /// Get Bitcoin-specific chain info
    pub fn chain_info() -> ChainInfo {
        ChainInfo {
            chain_id: "bitcoin".to_string(),
            chain_name: "Bitcoin".to_string(),
            supports_nfts: true,
            supports_smart_contracts: false,
        }
    }
    
    /// Get Bitcoin network configuration
    pub fn network_config(network: &str) -> BitcoinNetworkConfig {
        match network {
            "mainnet" => BitcoinNetworkConfig::Mainnet,
            "testnet" => BitcoinNetworkConfig::Testnet,
            "regtest" => BitcoinNetworkConfig::Regtest,
            _ => BitcoinNetworkConfig::Mainnet,
        }
    }
}

/// Bitcoin network configuration
#[derive(Debug, Clone)]
pub enum BitcoinNetworkConfig {
    Mainnet,
    Testnet,
    Regtest,
}

impl BitcoinNetworkConfig {
    /// Get the default RPC endpoint for this network
    pub fn default_rpc_endpoint(&self) -> &'static str {
        match self {
            Self::Mainnet => "https://blockstream.info/api",
            Self::Testnet => "https://blockstream.info/testnet/api",
            Self::Regtest => "http://localhost:8332",
        }
    }
    
    /// Get the default block explorer URL for this network
    pub fn default_block_explorer(&self) -> &'static str {
        match self {
            Self::Mainnet => "https://blockstream.info",
            Self::Testnet => "https://blockstream.info/testnet",
            Self::Regtest => "http://localhost:3000",
        }
    }
    
    /// Get the confirmation requirement for this network
    pub fn confirmations_required(&self) -> u32 {
        match self {
            Self::Mainnet => 6,
            Self::Testnet => 3,
            Self::Regtest => 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_bitcoin_adapter() {
        let adapter = BitcoinAdapter::new();
        assert_eq!(adapter.chain_id, "bitcoin");
        assert_eq!(adapter.chain_name, "Bitcoin");
    }
    
    #[test]
    fn test_bitcoin_chain_info() {
        let info = BitcoinAdapter::chain_info();
        assert_eq!(info.chain_id, "bitcoin");
        assert_eq!(info.chain_name, "Bitcoin");
        assert!(info.supports_nfts);
        assert!(!info.supports_smart_contracts);
    }
    
    #[test]
    fn test_bitcoin_network_config() {
        let mainnet = BitcoinNetworkConfig::Mainnet;
        assert_eq!(mainnet.default_rpc_endpoint(), "https://blockstream.info/api");
        assert_eq!(mainnet.confirmations_required(), 6);
        
        let testnet = BitcoinNetworkConfig::Testnet;
        assert_eq!(testnet.default_rpc_endpoint(), "https://blockstream.info/testnet/api");
        assert_eq!(testnet.confirmations_required(), 3);
        
        let regtest = BitcoinNetworkConfig::Regtest;
        assert_eq!(regtest.default_rpc_endpoint(), "http://localhost:8332");
        assert_eq!(regtest.confirmations_required(), 1);
    }
}
