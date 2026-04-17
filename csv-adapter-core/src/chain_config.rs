//! Chain configuration system for dynamic chain loading.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

use super::Chain;

/// Chain-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// Unique identifier for this chain
    pub chain_id: String,
    /// Human-readable name for this chain
    pub chain_name: String,
    /// Default network to use
    pub default_network: String,
    /// List of RPC endpoints
    pub rpc_endpoints: Vec<String>,
    /// CSV program ID for this chain
    pub program_id: Option<String>,
    /// Block explorer URLs
    pub block_explorer_urls: Vec<String>,
    /// Chain-specific settings
    pub custom_settings: HashMap<String, serde_json::Value>,
}

/// Configuration loader for dynamic chain discovery
pub struct ChainConfigLoader {
    configs: HashMap<String, ChainConfig>,
}

impl ChainConfigLoader {
    /// Create new loader
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }
    
    /// Load all chain configurations from directory
    pub fn load_from_directory(&mut self, config_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let entries = std::fs::read_dir(config_dir)?;
        
        for entry in entries {
            let entry = entry?;
            if entry.path().extension() == Some("toml") {
                let content = std::fs::read_to_string(entry.path())?;
                let config: ChainConfig = toml::from_str(&content)
                    .map_err(|e| format!("Failed to parse {}: {}", entry.path().display(), e))?;
                    
                self.configs.insert(config.chain_id.clone(), config);
                println!("Loaded chain config: {}", config.chain_id);
            }
        }
        
        Ok(())
    }
    
    /// Get configuration for specific chain
    pub fn get_config(&self, chain_id: &str) -> Option<&ChainConfig> {
        self.configs.get(chain_id)
    }
    
    /// Get all loaded configurations
    pub fn all_configs(&self) -> &HashMap<String, ChainConfig> {
        &self.configs
    }
    
    /// Get all supported chain IDs
    pub fn supported_chain_ids(&self) -> Vec<String> {
        self.configs.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_chain_config_loader() {
        let mut loader = ChainConfigLoader::new();
        
        let config = ChainConfig {
            chain_id: "test-chain".to_string(),
            chain_name: "Test Chain".to_string(),
            default_network: "testnet".to_string(),
            rpc_endpoints: vec!["https://test-rpc.example.com".to_string()],
            program_id: Some("TestProgram11111111111111111111111111111".to_string()),
            block_explorer_urls: vec!["https://test-explorer.example.com".to_string()],
            custom_settings: HashMap::new(),
        };
        
        loader.configs.insert("test-chain".to_string(), config);
        
        let retrieved = loader.get_config("test-chain");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().chain_id, "test-chain");
    }
}
