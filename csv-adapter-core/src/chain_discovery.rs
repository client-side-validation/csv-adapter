//! Chain discovery and automatic configuration loading.

use std::path::Path;
use std::collections::HashMap;

use super::chain_config::{ChainConfig, ChainConfigLoader};
use super::chain_system::ChainRegistry;

/// Chain discovery system for automatic chain loading
pub struct ChainDiscovery {
    config_loader: ChainConfigLoader,
    registry: ChainRegistry,
}

impl ChainDiscovery {
    /// Create new chain discovery system
    pub fn new() -> Self {
        Self {
            config_loader: ChainConfigLoader::new(),
            registry: ChainRegistry::new(),
        }
    }
    
    /// Discover and load all chains from the chains directory
    pub fn discover_chains(&mut self, chains_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        println!("Discovering chains from: {}", chains_dir.display());
        
        // Load all chain configurations
        self.config_loader.load_from_directory(chains_dir)?;
        
        // Register all discovered chains
        for (chain_id, config) in self.config_loader.all_configs() {
            println!("Registering chain: {} ({})", chain_id, config.chain_name);
            self.registry.register_chain(chain_id.clone(), config.chain_name.clone());
        }
        
        let discovered_count = self.registry.supported_chains().len();
        println!("Successfully discovered {} chains", discovered_count);
        
        Ok(())
    }
    
    /// Get the chain registry
    pub fn registry(&self) -> &ChainRegistry {
        &self.registry
    }
    
    /// Get the chain registry (mutable)
    pub fn registry_mut(&mut self) -> &mut ChainRegistry {
        &mut self.registry
    }
    
    /// Get configuration for a specific chain
    pub fn get_chain_config(&self, chain_id: &str) -> Option<&ChainConfig> {
        self.config_loader.get_config(chain_id)
    }
    
    /// Get all chain configurations
    pub fn all_chain_configs(&self) -> &HashMap<String, ChainConfig> {
        self.config_loader.all_configs()
    }
    
    /// Get supported chain IDs
    pub fn supported_chain_ids(&self) -> Vec<String> {
        self.registry.supported_chains().into_iter().map(|s| s.to_string()).collect()
    }
    
    /// Check if a chain supports NFTs
    pub fn supports_nfts(&self, chain_id: &str) -> bool {
        self.registry.supports_nfts(chain_id)
    }
    
    /// Get chains that support NFTs
    pub fn nft_supported_chains(&self) -> Vec<String> {
        self.registry.supported_chains()
            .into_iter()
            .filter(|chain_id| self.registry.supports_nfts(chain_id))
            .map(|s| s.to_string())
            .collect()
    }
    
    /// Load chains from default directory
    pub fn load_default_chains(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let chains_dir = Path::new("chains");
        if chains_dir.exists() {
            self.discover_chains(chains_dir)
        } else {
            println!("Default chains directory not found, no chains loaded");
            Ok(())
        }
    }
}

impl Default for ChainDiscovery {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;
    
    #[test]
    fn test_chain_discovery() {
        let temp_dir = TempDir::new().unwrap();
        let chains_dir = temp_dir.path();
        
        // Create a test chain config
        let test_config = r#"
chain_id = "test-chain"
chain_name = "Test Chain"
default_network = "testnet"
rpc_endpoints = ["https://test-rpc.example.com"]
program_id = null
block_explorer_urls = ["https://test-explorer.example.com"]

[custom_settings]
supports_nfts = true
supports_smart_contracts = false
"#;
        
        let config_path = chains_dir.join("test-chain.toml");
        fs::write(&config_path, test_config).unwrap();
        
        let mut discovery = ChainDiscovery::new();
        discovery.discover_chains(chains_dir).unwrap();
        
        let supported_chains = discovery.supported_chain_ids();
        assert_eq!(supported_chains.len(), 1);
        assert_eq!(supported_chains[0], "test-chain");
        
        let config = discovery.get_chain_config("test-chain");
        assert!(config.is_some());
        assert_eq!(config.unwrap().chain_name, "Test Chain");
    }
}
