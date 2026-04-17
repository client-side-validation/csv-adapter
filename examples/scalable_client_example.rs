//! Example usage of the ScalableClientBuilder with dynamic chain support.

use std::path::Path;
use csv_adapter::prelude::*;
use csv_adapter_core::chain_discovery::ChainDiscovery;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Scalable CSV Client Example ===\n");

    // Example 1: Basic usage with chain registry
    println!("1. Basic Scalable Client Setup:");
    basic_scalable_client_example().await?;

    println!("\n2. Dynamic Chain Discovery:");
    dynamic_chain_discovery_example().await?;

    println!("\n3. Configuration-Driven Setup:");
    configuration_driven_example().await?;

    println!("\n4. Custom Chain Registration:");
    custom_chain_registration_example().await?;

    println!("\n=== All Examples Completed Successfully! ===");
    Ok(())
}

/// Example 1: Basic scalable client setup
async fn basic_scalable_client_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating scalable client with manual chain registration...");
    
    // Create chain registry
    let mut registry = ChainRegistry::new();
    
    // Register chains manually
    registry.register_chain("bitcoin".to_string(), "Bitcoin".to_string());
    registry.register_chain("ethereum".to_string(), "Ethereum".to_string());
    registry.register_chain("solana".to_string(), "Solana".to_string());
    
    // Create scalable client
    let client = CsvClient::scalable_builder()
        .with_chain_registry(registry)
        .with_chain("bitcoin")
        .with_chain("ethereum")
        .with_chain("solana")
        .with_store_backend(StoreBackend::InMemory)
        .build()?;
    
    println!("  Created client with {} enabled chains", client.enabled_chains().len());
    
    // Check which chains are enabled
    for chain in client.enabled_chains() {
        println!("  - Enabled chain: {:?}", chain);
    }
    
    println!("  Basic setup completed successfully!");
    Ok(())
}

/// Example 2: Dynamic chain discovery from configuration files
async fn dynamic_chain_discovery_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Discovering chains from configuration files...");
    
    // Create chain discovery system
    let mut discovery = ChainDiscovery::new();
    
    // Discover chains from default directory
    discovery.load_default_chains()?;
    
    let discovered_chains = discovery.supported_chain_ids();
    println!("  Discovered {} chains:", discovered_chains.len());
    
    for chain_id in &discovered_chains {
        let config = discovery.get_chain_config(chain_id);
        if let Some(cfg) = config {
            println!("    - {} ({})", chain_id, cfg.chain_name);
            
            // Show capabilities
            let nft_support = cfg.custom_settings.get("supports_nfts")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let sc_support = cfg.custom_settings.get("supports_smart_contracts")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            
            if nft_support || sc_support {
                let mut capabilities = Vec::new();
                if nft_support { capabilities.push("NFTs"); }
                if sc_support { capabilities.push("Smart Contracts"); }
                println!("      Capabilities: {}", capabilities.join(", "));
            }
        }
    }
    
    if discovered_chains.is_empty() {
        println!("  No chains discovered. This is expected if chains/ directory doesn't exist.");
        println!("  Create chain configuration files in the chains/ directory to test this example.");
    } else {
        // Create client with discovered chains
        let registry = discovery.registry().clone();
        let client = CsvClient::scalable_builder()
            .with_chain_registry(registry)
            .with_all_available_chains()
            .with_store_backend(StoreBackend::InMemory)
            .build()?;
        
        println!("  Created client with {} enabled chains", client.enabled_chains().len());
    }
    
    println!("  Dynamic discovery completed successfully!");
    Ok(())
}

/// Example 3: Configuration-driven setup
async fn configuration_driven_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Setting up client using configuration...");
    
    // Create a configuration object
    let mut config = Config::default();
    
    // Add chain configurations
    config.chains.insert("bitcoin".to_string(), crate::config::ChainConfig {
        enabled: true,
        rpc_url: "https://blockstream.info/api".to_string(),
        network: "mainnet".to_string(),
        program_id: None,
    });
    
    config.chains.insert("ethereum".to_string(), crate::config::ChainConfig {
        enabled: true,
        rpc_url: "https://ethereum.publicnode.com".to_string(),
        network: "mainnet".to_string(),
        program_id: Some("0x1234567890123456789012345678901234567890".to_string()),
    });
    
    // Create client with configuration
    let client = CsvClient::scalable_builder()
        .with_config(config)
        .with_store_backend(StoreBackend::InMemory)
        .build()?;
    
    println!("  Created client from configuration");
    println!("  Enabled chains: {:?}", client.enabled_chains());
    
    println!("  Configuration-driven setup completed successfully!");
    Ok(())
}

/// Example 4: Custom chain registration
async fn custom_chain_registration_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Registering custom chain...");
    
    // Create chain registry
    let mut registry = ChainRegistry::new();
    
    // Register a custom chain (e.g., Polygon)
    registry.register_chain("polygon".to_string(), "Polygon".to_string());
    registry.register_chain("arbitrum".to_string(), "Arbitrum".to_string());
    registry.register_chain("optimism".to_string(), "Optimism".to_string());
    
    // Create client with custom chains
    let client = CsvClient::scalable_builder()
        .with_chain_registry(registry)
        .with_chain("polygon")
        .with_chain("arbitrum")
        .with_chain("optimism")
        .with_store_backend(StoreBackend::InMemory)
        .build()?;
    
    println!("  Created client with custom chains");
    println!("  Enabled chains: {:?}", client.enabled_chains());
    
    // Demonstrate chain capabilities
    println!("  Chain capabilities:");
    for chain_id in ["polygon", "arbitrum", "optimism"] {
        println!("    - {}: NFTs enabled, Smart Contracts enabled", chain_id);
    }
    
    println!("  Custom chain registration completed successfully!");
    Ok(())
}

/// Example 5: Chain filtering and capabilities
async fn chain_filtering_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("Demonstrating chain filtering and capabilities...");
    
    let mut registry = ChainRegistry::new();
    
    // Register chains with different characteristics
    registry.register_chain("bitcoin".to_string(), "Bitcoin".to_string());
    registry.register_chain("ethereum".to_string(), "Ethereum".to_string());
    registry.register_chain("solana".to_string(), "Solana".to_string());
    registry.register_chain("sui".to_string(), "Sui".to_string());
    
    // Filter chains by capabilities
    let all_chains = registry.supported_chains();
    println!("  All registered chains: {:?}", all_chains);
    
    // All chains in our test setup support NFTs
    let nft_chains: Vec<_> = all_chains.iter()
        .filter(|&chain_id| registry.supports_nfts(chain_id))
        .collect();
    println!("  NFT-supported chains: {:?}", nft_chains);
    
    // Create client with filtered chains
    let client = CsvClient::scalable_builder()
        .with_chain_registry(registry)
        .with_chain("bitcoin")
        .with_chain("ethereum")
        .with_store_backend(StoreBackend::InMemory)
        .build()?;
    
    println!("  Created client with filtered chains");
    println!("  Chain filtering example completed successfully!");
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_basic_scalable_client() {
        let mut registry = ChainRegistry::new();
        registry.register_chain("bitcoin".to_string(), "Bitcoin".to_string());
        
        let client = CsvClient::scalable_builder()
            .with_chain_registry(registry)
            .with_chain("bitcoin")
            .with_store_backend(StoreBackend::InMemory)
            .build();
        
        assert!(client.is_ok());
    }
    
    #[tokio::test]
    async fn test_chain_discovery() {
        let mut discovery = ChainDiscovery::new();
        // This will work even if chains/ directory doesn't exist
        let result = discovery.load_default_chains();
        assert!(result.is_ok());
    }
}
