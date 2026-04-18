//! Simplified chain system for dynamic chain support.

use std::collections::HashMap;

/// Simplified chain registry for basic chain information
/// 
/// Note: For full adapter storage with trait objects, use `ChainRegistry` from `chain_adapter`.
#[derive(Clone)]
pub struct SimpleChainRegistry {
    adapters: HashMap<String, ChainInfo>,
}

/// Basic chain information
#[derive(Debug, Clone)]
pub struct ChainInfo {
    /// Unique identifier for the chain
    pub chain_id: String,
    /// Human-readable name of the chain
    pub chain_name: String,
    /// Whether the chain supports NFTs
    pub supports_nfts: bool,
    /// Whether the chain supports smart contracts
    pub supports_smart_contracts: bool,
}

impl SimpleChainRegistry {
    /// Create new registry
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }
    
    /// Register a chain
    pub fn register_chain(&mut self, chain_id: String, chain_name: String) {
        let info = ChainInfo {
            chain_id: chain_id.clone(),
            chain_name,
            supports_nfts: true,
            supports_smart_contracts: true,
        };
        
        self.adapters.insert(chain_id, info);
    }
    
    /// Get all supported chains
    pub fn supported_chains(&self) -> Vec<String> {
        self.adapters.keys().cloned().collect()
    }
    
    /// Get chain info
    pub fn get_chain_info(&self, chain_id: &str) -> Option<&ChainInfo> {
        self.adapters.get(chain_id)
    }
    
    /// Check if chain supports NFTs
    pub fn supports_nfts(&self, chain_id: &str) -> bool {
        self.adapters
            .get(chain_id)
            .map(|info| info.supports_nfts)
            .unwrap_or(false)
    }
}

impl Default for SimpleChainRegistry {
    fn default() -> Self {
        Self::new()
    }
}
