//! Simplified chain system for dynamic chain support.

use std::collections::HashMap;

/// Chain adapter registry for dynamic chain management
pub struct ChainRegistry {
    adapters: HashMap<String, ChainInfo>,
}

/// Basic chain information
#[derive(Debug, Clone)]
pub struct ChainInfo {
    pub chain_id: String,
    pub chain_name: String,
    pub supports_nfts: bool,
    pub supports_smart_contracts: bool,
}

impl ChainRegistry {
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

impl Default for ChainRegistry {
    fn default() -> Self {
        Self::new()
    }
}
