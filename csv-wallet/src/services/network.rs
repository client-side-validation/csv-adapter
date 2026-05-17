//! Network configuration service.

use csv_core::ChainId;

/// Network type (testnet or mainnet).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum NetworkType {
    /// Testnet
    Testnet,
    /// Mainnet
    Mainnet,
}

impl NetworkType {
    /// Check if this is a testnet.
    pub fn is_testnet(&self) -> bool {
        matches!(self, Self::Testnet)
    }
}

/// Network configuration for a chain.
pub struct NetworkConfig {
    networks: std::collections::HashMap<ChainId, NetworkType>,
}

impl NetworkConfig {
    /// Create default network configuration.
    pub fn new() -> Self {
        let mut networks = std::collections::HashMap::new();
        networks.insert(ChainId::new("bitcoin"), NetworkType::Testnet);
        networks.insert(ChainId::new("ethereum"), NetworkType::Testnet);
        networks.insert(ChainId::new("sui"), NetworkType::Testnet);
        networks.insert(ChainId::new("aptos"), NetworkType::Testnet);
        networks.insert(ChainId::new("solana"), NetworkType::Testnet);
        Self { networks }
    }

    /// Get network for a chain.
    pub fn get_network(&self, chain: ChainId) -> NetworkType {
        self.networks
            .get(&chain)
            .copied()
            .unwrap_or(NetworkType::Testnet)
    }

    /// Set network for a chain.
    pub fn set_network(&mut self, chain: ChainId, network: NetworkType) {
        self.networks.insert(chain, network);
    }

    /// Check if chain is on testnet.
    pub fn is_testnet(&self, chain: ChainId) -> bool {
        self.get_network(chain).is_testnet()
    }
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self::new()
    }
}
