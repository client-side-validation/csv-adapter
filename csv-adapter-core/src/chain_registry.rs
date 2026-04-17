//! Chain registry for dynamic chain management.

use std::collections::HashMap;

/// Registry for managing chain adapters
pub struct ChainRegistry {
    adapters: HashMap<String, Box<dyn super::ChainAdapter>>,
}

impl ChainRegistry {
    /// Create new empty registry
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
        }
    }
    
    /// Register a new chain adapter
    pub fn register_adapter(&mut self, adapter: Box<dyn super::ChainAdapter>) {
        let chain_id = adapter.chain_id();
        self.adapters.insert(chain_id.to_string(), adapter);
    }
    
    /// Get adapter by chain ID
    pub fn get_adapter(&self, chain_id: &str) -> Option<&dyn super::ChainAdapter>> {
        self.adapters.get(chain_id)
    }
    
    /// Get all supported chain IDs
    pub fn supported_chains(&self) -> Vec<&str> {
        self.adapters.keys().map(|k| k.as_str()).collect()
    }
}

impl Default for ChainRegistry {
    fn default() -> Self {
        Self::new()
    }
}
