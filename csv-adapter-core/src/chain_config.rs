//! Chain configuration system for dynamic chain loading.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;

/// Chain-specific capabilities and features
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainCapabilities {
    /// Whether this chain supports NFTs
    pub supports_nfts: bool,
    /// Whether this chain supports smart contracts
    pub supports_smart_contracts: bool,
    /// Account model used by this chain
    pub account_model: AccountModel,
    /// Number of blocks needed for finality
    pub confirmation_blocks: u64,
    /// Maximum batch size for operations
    pub max_batch_size: usize,
    /// Supported networks for this chain
    pub supported_networks: Vec<String>,
    /// Whether chain supports cross-chain transfers
    pub supports_cross_chain: bool,
    /// Chain-specific features
    #[serde(default)]
    pub custom_features: HashMap<String, serde_json::Value>,
}

/// Account model types
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountModel {
    /// UTXO-based model (Bitcoin-like)
    UTXO,
    /// Account-based model (Ethereum-like)
    Account,
    /// Object-based model (Sui-like)
    Object,
    /// Hybrid model (mixed approaches)
    Hybrid,
}

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
    /// Chain capabilities
    pub capabilities: ChainCapabilities,
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
    /// Invalid configs are skipped with a warning rather than failing the entire operation
    pub fn load_from_directory(&mut self, config_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
        let entries = std::fs::read_dir(config_dir)?;
        
        for entry in entries {
            let entry = entry?;
            if entry.path().extension() == Some(std::ffi::OsStr::new("toml")) {
                let path = entry.path();
                let content = std::fs::read_to_string(&path)?;
                
                match toml::from_str::<ChainConfig>(&content) {
                    Ok(config) => {
                        let chain_id = config.chain_id.clone();
                        self.configs.insert(chain_id.clone(), config);
                        println!("Loaded chain config: {}", chain_id);
                    }
                    Err(e) => {
                        eprintln!("Warning: Failed to parse {}: {}", path.display(), e);
                        // Skip invalid config and continue
                    }
                }
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
            capabilities: ChainCapabilities {
                supports_nfts: true,
                supports_smart_contracts: true,
                account_model: AccountModel::Account,
                confirmation_blocks: 12,
                max_batch_size: 100,
                supported_networks: vec!["mainnet".to_string(), "testnet".to_string()],
                supports_cross_chain: true,
                custom_features: HashMap::new(),
            },
            custom_settings: HashMap::new(),
        };
        
        loader.configs.insert("test-chain".to_string(), config);
        
        let retrieved = loader.get_config("test-chain");
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().chain_id, "test-chain");
    }
}
