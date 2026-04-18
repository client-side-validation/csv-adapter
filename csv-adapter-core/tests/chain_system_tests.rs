//! Integration tests for the dynamic chain system.

use std::fs;
use tempfile::TempDir;

use csv_adapter_core::chain_config::ChainConfigLoader;
use csv_adapter_core::chain_discovery::ChainDiscovery;
use csv_adapter_core::chain_system::SimpleChainRegistry;

/// Test chain configuration loading
#[test]
fn test_chain_config_loader() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();
    
    // Create test configuration with proper structure
    let config_content = r#"
chain_id = "test-bitcoin"
chain_name = "Test Bitcoin"
default_network = "testnet"
rpc_endpoints = ["https://test-rpc.example.com"]
block_explorer_urls = ["https://test-explorer.example.com"]

[capabilities]
supports_nfts = true
supports_smart_contracts = false
account_model = "UTXO"
confirmation_blocks = 6
max_batch_size = 50
supported_networks = ["testnet", "mainnet"]
supports_cross_chain = false

[custom_settings]
test_key = "test_value"
"#;
    
    let config_file = config_dir.join("test-bitcoin.toml");
    fs::write(&config_file, config_content).unwrap();
    
    // Load configuration
    let mut loader = ChainConfigLoader::new();
    loader.load_from_directory(config_dir).unwrap();
    
    // Verify loaded configuration
    let config = loader.get_config("test-bitcoin").unwrap();
    assert_eq!(config.chain_id, "test-bitcoin");
    assert_eq!(config.chain_name, "Test Bitcoin");
    assert_eq!(config.default_network, "testnet");
    assert_eq!(config.rpc_endpoints.len(), 1);
    assert_eq!(config.rpc_endpoints[0], "https://test-rpc.example.com");
    
    // Verify capabilities
    assert_eq!(config.capabilities.supports_nfts, true);
    assert_eq!(config.capabilities.supports_smart_contracts, false);
    match config.capabilities.account_model {
        csv_adapter_core::chain_config::AccountModel::UTXO => {},
        _ => panic!("Expected UTXO account model"),
    }
    
    // Verify custom settings
    assert_eq!(config.custom_settings.get("test_key").unwrap().as_str().unwrap(), "test_value");
}

/// Test chain registry functionality
#[test]
fn test_chain_registry() {
    let mut registry = SimpleChainRegistry::new();
    
    // Register chains
    registry.register_chain("bitcoin".to_string(), "Bitcoin".to_string());
    registry.register_chain("ethereum".to_string(), "Ethereum".to_string());
    registry.register_chain("solana".to_string(), "Solana".to_string());
    
    // Test supported chains
    let supported = registry.supported_chains();
    assert_eq!(supported.len(), 3);
    assert!(supported.contains(&"bitcoin".to_string()));
    assert!(supported.contains(&"ethereum".to_string()));
    assert!(supported.contains(&"solana".to_string()));
    
    // Test chain info
    let bitcoin_info = registry.get_chain_info("bitcoin").unwrap();
    assert_eq!(bitcoin_info.chain_id, "bitcoin");
    assert_eq!(bitcoin_info.chain_name, "Bitcoin");
    assert!(bitcoin_info.supports_nfts);
    assert!(bitcoin_info.supports_smart_contracts);
    
    // Test NFT support
    assert!(registry.supports_nfts("bitcoin"));
    assert!(registry.supports_nfts("ethereum"));
    assert!(registry.supports_nfts("solana"));
    assert!(!registry.supports_nfts("nonexistent"));
}

/// Test chain discovery system
#[test]
fn test_chain_discovery() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();
    
    // Create multiple chain configurations
    let chains = vec![
        ("bitcoin.toml", r#"
chain_id = "bitcoin"
chain_name = "Bitcoin"
default_network = "mainnet"
rpc_endpoints = ["https://bitcoin-rpc.example.com"]
block_explorer_urls = ["https://bitcoin-explorer.example.com"]

[capabilities]
supports_nfts = true
supports_smart_contracts = false
account_model = "UTXO"
confirmation_blocks = 6
max_batch_size = 50
supported_networks = ["mainnet", "testnet"]
supports_cross_chain = false

[custom_settings]
network_type = "bitcoin"
"#),
        ("ethereum.toml", r#"
chain_id = "ethereum"
chain_name = "Ethereum"
default_network = "mainnet"
rpc_endpoints = ["https://ethereum-rpc.example.com"]
program_id = "0x1234567890123456789012345678901234567890"
block_explorer_urls = ["https://ethereum-explorer.example.com"]

[capabilities]
supports_nfts = true
supports_smart_contracts = true
account_model = "Account"
confirmation_blocks = 12
max_batch_size = 100
supported_networks = ["mainnet", "sepolia"]
supports_cross_chain = true

[custom_settings]
network_type = "ethereum"
"#),
        ("solana.toml", r#"
chain_id = "solana"
chain_name = "Solana"
default_network = "mainnet"
rpc_endpoints = ["https://solana-rpc.example.com"]
program_id = "CsvProgramSolana11111111111111111111111111111"
block_explorer_urls = ["https://solana-explorer.example.com"]

[capabilities]
supports_nfts = true
supports_smart_contracts = true
account_model = "Account"
confirmation_blocks = 32
max_batch_size = 200
supported_networks = ["mainnet", "devnet"]
supports_cross_chain = true

[custom_settings]
network_type = "solana"
"#),
    ];
    
    // Write configuration files
    for (filename, content) in chains {
        let config_file = config_dir.join(filename);
        fs::write(&config_file, content).unwrap();
    }
    
    // Test discovery
    let mut discovery = ChainDiscovery::new();
    discovery.discover_chains(config_dir).unwrap();
    
    // Verify discovered chains
    let discovered = discovery.supported_chain_ids();
    assert_eq!(discovered.len(), 3);
    assert!(discovered.contains(&"bitcoin".to_string()));
    assert!(discovered.contains(&"ethereum".to_string()));
    assert!(discovered.contains(&"solana".to_string()));
    
    // Test chain configurations
    let bitcoin_config = discovery.get_chain_config("bitcoin").unwrap();
    assert_eq!(bitcoin_config.chain_name, "Bitcoin");
    match bitcoin_config.capabilities.account_model {
        csv_adapter_core::chain_config::AccountModel::UTXO => {},
        _ => panic!("Expected UTXO account model for Bitcoin"),
    }
    
    let ethereum_config = discovery.get_chain_config("ethereum").unwrap();
    assert_eq!(ethereum_config.chain_name, "Ethereum");
    assert_eq!(ethereum_config.program_id.as_ref().unwrap(), "0x1234567890123456789012345678901234567890");
    
    let solana_config = discovery.get_chain_config("solana").unwrap();
    assert_eq!(solana_config.chain_name, "Solana");
    assert_eq!(solana_config.program_id.as_ref().unwrap(), "CsvProgramSolana11111111111111111111111111111");
    
    // Test NFT support filtering
    let nft_chains = discovery.nft_supported_chains();
    assert_eq!(nft_chains.len(), 3); // All test chains support NFTs
}

/// Test error handling for invalid configurations
#[test]
fn test_invalid_configuration_handling() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();
    
    // Create invalid configuration (missing required fields)
    let invalid_config = r#"
chain_id = "invalid-chain"
# Missing chain_name, rpc_endpoints, etc.
"#;
    
    let config_file = config_dir.join("invalid.toml");
    fs::write(&config_file, invalid_config).unwrap();
    
    // Test that loader handles invalid configurations gracefully
    let mut loader = ChainConfigLoader::new();
    let result = loader.load_from_directory(config_dir);
    
    // Should succeed but not load the invalid config
    assert!(result.is_ok());
    assert!(loader.get_config("invalid-chain").is_none());
}

/// Test chain capability detection
#[test]
fn test_chain_capability_detection() {
    let mut registry = SimpleChainRegistry::new();
    
    // Register chains with different capabilities
    registry.register_chain("utxo-chain".to_string(), "UTHO Chain".to_string());
    registry.register_chain("account-chain".to_string(), "Account Chain".to_string());
    registry.register_chain("object-chain".to_string(), "Object Chain".to_string());
    
    // All chains support NFTs and smart contracts by default in our test setup
    assert!(registry.supports_nfts("utxo-chain"));
    assert!(registry.supports_nfts("account-chain"));
    assert!(registry.supports_nfts("object-chain"));
    
    // Test chain info retrieval
    let utxo_info = registry.get_chain_info("utxo-chain").unwrap();
    assert_eq!(utxo_info.chain_id, "utxo-chain");
    assert!(utxo_info.supports_nfts);
    assert!(utxo_info.supports_smart_contracts);
}

/// Test configuration file validation
#[test]
fn test_configuration_validation() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();
    
    // Create valid configuration
    let valid_config = r#"
chain_id = "valid-chain"
chain_name = "Valid Chain"
default_network = "mainnet"
rpc_endpoints = ["https://rpc.example.com"]
block_explorer_urls = ["https://explorer.example.com"]

[capabilities]
supports_nfts = true
supports_smart_contracts = true
account_model = "Account"
confirmation_blocks = 6
max_batch_size = 100
supported_networks = ["mainnet", "testnet"]
supports_cross_chain = false

[custom_settings]
test_key = "test_value"
"#;
    
    let config_file = config_dir.join("valid.toml");
    fs::write(&config_file, valid_config).unwrap();
    
    // Test loading valid configuration
    let mut loader = ChainConfigLoader::new();
    loader.load_from_directory(config_dir).unwrap();
    
    let config = loader.get_config("valid-chain").unwrap();
    
    // Validate all required fields
    assert!(!config.chain_id.is_empty());
    assert!(!config.chain_name.is_empty());
    assert!(!config.rpc_endpoints.is_empty());
    assert!(!config.block_explorer_urls.is_empty());
    
    // Validate capabilities
    assert_eq!(config.capabilities.supports_nfts, true);
    assert_eq!(config.capabilities.supports_smart_contracts, true);
    assert_eq!(config.capabilities.confirmation_blocks, 6);
    assert_eq!(config.capabilities.max_batch_size, 100);
    
    // Validate custom settings
    assert!(config.custom_settings.contains_key("test_key"));
}

/// Test multiple configuration loading
#[test]
fn test_multiple_configuration_loading() {
    let temp_dir = TempDir::new().unwrap();
    let config_dir = temp_dir.path();
    
    // Create multiple configuration files
    let configs = vec![
        ("chain1.toml", r#"
chain_id = "chain1"
chain_name = "Chain 1"
default_network = "mainnet"
rpc_endpoints = ["https://chain1-rpc.example.com"]
block_explorer_urls = ["https://chain1-explorer.example.com"]

[capabilities]
supports_nfts = true
supports_smart_contracts = false
account_model = "UTXO"
confirmation_blocks = 6
max_batch_size = 50
supported_networks = ["mainnet"]
supports_cross_chain = false

[custom_settings]
chain_type = "chain1"
"#),
        ("chain2.toml", r#"
chain_id = "chain2"
chain_name = "Chain 2"
default_network = "testnet"
rpc_endpoints = ["https://chain2-rpc.example.com"]
program_id = "0x1234567890123456789012345678901234567890"
block_explorer_urls = ["https://chain2-explorer.example.com"]

[capabilities]
supports_nfts = false
supports_smart_contracts = true
account_model = "Account"
confirmation_blocks = 12
max_batch_size = 100
supported_networks = ["testnet"]
supports_cross_chain = true

[custom_settings]
chain_type = "chain2"
"#),
    ];
    
    // Write all configuration files
    for (filename, content) in configs {
        let config_file = config_dir.join(filename);
        fs::write(&config_file, content).unwrap();
    }
    
    // Load all configurations
    let mut loader = ChainConfigLoader::new();
    loader.load_from_directory(config_dir).unwrap();
    
    // Verify all configurations were loaded
    assert_eq!(loader.all_configs().len(), 2);
    assert!(loader.get_config("chain1").is_some());
    assert!(loader.get_config("chain2").is_some());
    
    // Verify individual configurations
    let chain1_config = loader.get_config("chain1").unwrap();
    assert_eq!(chain1_config.chain_name, "Chain 1");
    assert_eq!(chain1_config.default_network, "mainnet");
    assert_eq!(chain1_config.capabilities.supports_nfts, true);
    
    let chain2_config = loader.get_config("chain2").unwrap();
    assert_eq!(chain2_config.chain_name, "Chain 2");
    assert_eq!(chain2_config.default_network, "testnet");
    assert_eq!(chain2_config.capabilities.supports_nfts, false);
}
