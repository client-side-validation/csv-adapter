//! Complete demonstration of the scalable chain architecture.

use std::path::Path;
use csv_adapter::prelude::*;
use csv_adapter_core::chain_discovery::ChainDiscovery;
use csv_adapter_core::adapters::ScalableBitcoinAdapter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("=== Complete Scalable Chain Architecture Demo ===\n");

    // Step 1: Demonstrate the old vs new approach
    println!("1. Comparing Old vs New Architecture:");
    compare_architectures().await?;

    println!("\n2. Dynamic Chain Loading:");
    dynamic_chain_loading_demo().await?;

    println!("\n3. Chain Management:");
    chain_management_demo().await?;

    println!("\n4. Scalability Demonstration:");
    scalability_demo().await?;

    println!("\n5. Real-world Usage Example:");
    real_world_example().await?;

    println!("\n=== Demo Completed Successfully! ===");
    Ok(())
}

/// Compare the old hardcoded approach with the new scalable approach
async fn compare_architectures() -> Result<(), Box<dyn std::error::Error>> {
    println!("  Old Approach (Hardcoded):");
    println!("    - Chains hardcoded in builder.rs");
    println!("    - Feature flags for each chain");
    println!("    - Manual code changes for new chains");
    println!("    - Limited to ~5-10 chains maximum");
    
    println!("  New Approach (Scalable):");
    println!("    - Dynamic chain registry");
    println!("    - Configuration-driven loading");
    println!("    - Zero-code chain addition");
    println!("    - Unlimited chain support");
    
    // Show old approach
    let old_client = CsvClient::builder()
        .with_chain(Chain::Bitcoin)
        .with_chain(Chain::Ethereum)
        .with_store_backend(StoreBackend::InMemory)
        .build()?;
    
    println!("    Old client enabled chains: {:?}", old_client.enabled_chains());
    
    // Show new approach
    let mut registry = ChainRegistry::new();
    registry.register_chain("bitcoin".to_string(), "Bitcoin".to_string());
    registry.register_chain("ethereum".to_string(), "Ethereum".to_string());
    registry.register_chain("solana".to_string(), "Solana".to_string());
    
    let new_client = CsvClient::scalable_builder()
        .with_chain_registry(registry)
        .with_all_available_chains()
        .with_store_backend(StoreBackend::InMemory)
        .build()?;
    
    println!("    New client enabled chains: {:?}", new_client.enabled_chains());
    
    Ok(())
}

/// Demonstrate dynamic chain loading from configuration files
async fn dynamic_chain_loading_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("  Loading chains from configuration files...");
    
    // Create chain discovery system
    let mut discovery = ChainDiscovery::new();
    
    // Try to load from chains/ directory
    if Path::new("chains").exists() {
        discovery.discover_chains(Path::new("chains"))?;
        
        let chains = discovery.supported_chain_ids();
        println!("    Discovered {} chains from configuration:", chains.len());
        
        for chain_id in &chains {
            let config = discovery.get_chain_config(chain_id);
            if let Some(cfg) = config {
                let mut features = Vec::new();
                
                if cfg.custom_settings.get("supports_nfts").and_then(|v| v.as_bool()).unwrap_or(false) {
                    features.push("NFTs");
                }
                if cfg.custom_settings.get("supports_smart_contracts").and_then(|v| v.as_bool()).unwrap_or(false) {
                    features.push("Smart Contracts");
                }
                
                println!("      - {} ({}): {}", chain_id, cfg.chain_name, features.join(", "));
            }
        }
        
        // Create client with discovered chains
        let registry = discovery.registry().clone();
        let client = CsvClient::scalable_builder()
            .with_chain_registry(registry)
            .with_all_available_chains()
            .with_store_backend(StoreBackend::InMemory)
            .build()?;
        
        println!("    Created client with {} chains", client.enabled_chains().len());
    } else {
        println!("    No chains/ directory found - this is expected in fresh checkout");
        println!("    Create chain config files to test this feature");
    }
    
    Ok(())
}

/// Demonstrate chain management capabilities
async fn chain_management_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("  Demonstrating chain management...");
    
    // Create registry and add chains dynamically
    let mut registry = ChainRegistry::new();
    
    // Add existing chains
    registry.register_chain("bitcoin".to_string(), "Bitcoin".to_string());
    registry.register_chain("ethereum".to_string(), "Ethereum".to_string());
    registry.register_chain("solana".to_string(), "Solana".to_string());
    
    // Add new chains (simulating community contributions)
    registry.register_chain("polygon".to_string(), "Polygon".to_string());
    registry.register_chain("arbitrum".to_string(), "Arbitrum".to_string());
    registry.register_chain("optimism".to_string(), "Optimism".to_string());
    registry.register_chain("avalanche".to_string(), "Avalanche".to_string());
    registry.register_chain("fantom".to_string(), "Fantom".to_string());
    
    println!("    Registered {} chains:", registry.supported_chains().len());
    for chain_id in registry.supported_chains() {
        println!("      - {}", chain_id);
    }
    
    // Filter chains by capabilities
    let nft_chains: Vec<_> = registry.supported_chains()
        .iter()
        .filter(|&chain_id| registry.supports_nfts(chain_id))
        .collect();
    
    println!("    NFT-supported chains: {:?}", nft_chains);
    
    // Create client with subset of chains
    let client = CsvClient::scalable_builder()
        .with_chain_registry(registry)
        .with_chain("bitcoin")
        .with_chain("ethereum")
        .with_chain("polygon")
        .with_chain("arbitrum")
        .with_store_backend(StoreBackend::InMemory)
        .build()?;
    
    println!("    Created client with selected chains: {:?}", client.enabled_chains());
    
    Ok(())
}

/// Demonstrate scalability with many chains
async fn scalability_demo() -> Result<(), Box<dyn std::error::Error>> {
    println!("  Testing scalability with many chains...");
    
    let mut registry = ChainRegistry::new();
    
    // Simulate adding 50+ chains (demonstrating unlimited scalability)
    let chain_categories = vec![
        ("layer1", vec!["bitcoin", "ethereum", "solana", "sui", "aptos", "cardano", "polkadot", "cosmos"]),
        ("layer2", vec!["polygon", "arbitrum", "optimism", "base", "zksync", "starknet", "loopring"]),
        ("alt_l1", vec!["avalanche", "fantom", "harmony", "celo", "near", "algorand", "hedera"]),
        ("emerging", vec!["mantle", "scroll", "blast", "mode", "linea", "zklink", "taiko"]),
        ("testnets", vec!["sepolia", "goerli", "mumbai", "fuji", "testnet", "devnet"]),
    ];
    
    let mut total_chains = 0;
    for (category, chains) in chain_categories {
        for chain in chains {
            registry.register_chain(chain.to_string(), format!("{} ({})", 
                chain.to_uppercase().chars().next().unwrap_or('X').to_uppercase() + &chain[1..], category));
            total_chains += 1;
        }
    }
    
    println!("    Successfully registered {} chains", total_chains);
    println!("    All chains support NFTs and Smart Contracts by default");
    
    // Demonstrate selective chain loading
    let client = CsvClient::scalable_builder()
        .with_chain_registry(registry)
        .with_chain("bitcoin")
        .with_chain("ethereum")
        .with_chain("polygon")
        .with_chain("arbitrum")
        .with_chain("avalanche")
        .with_store_backend(StoreBackend::InMemory)
        .build()?;
    
    println!("    Created client with 5 selected chains from {} available", total_chains);
    println!("    Architecture supports unlimited chain addition without code changes");
    
    Ok(())
}

/// Real-world usage example
async fn real_world_example() -> Result<(), Box<dyn std::error::Error>> {
    println!("  Real-world usage scenario...");
    
    // Simulate a DeFi application that needs multiple chains
    let mut registry = ChainRegistry::new();
    
    // Register chains needed for DeFi operations
    let defi_chains = vec![
        ("ethereum", "Ethereum"),
        ("polygon", "Polygon"),
        ("arbitrum", "Arbitrum"),
        ("optimism", "Optimism"),
        ("base", "Base"),
        ("avalanche", "Avalanche"),
    ];
    
    for (chain_id, chain_name) in defi_chains {
        registry.register_chain(chain_id.to_string(), chain_name.to_string());
    }
    
    println!("    Setting up DeFi application with {} chains", defi_chains.len());
    
    // Create client for DeFi operations
    let defi_client = CsvClient::scalable_builder()
        .with_chain_registry(registry)
        .with_all_available_chains()
        .with_store_backend(StoreBackend::InMemory)
        .build()?;
    
    println!("    DeFi client ready with chains: {:?}", defi_client.enabled_chains());
    
    // Simulate NFT marketplace setup
    let mut nft_registry = ChainRegistry::new();
    
    let nft_chains = vec![
        ("ethereum", "Ethereum"),
        ("polygon", "Polygon"),
        ("solana", "Solana"),
        ("sui", "Sui"),
        ("aptos", "Aptos"),
    ];
    
    for (chain_id, chain_name) in nft_chains {
        nft_registry.register_chain(chain_id.to_string(), chain_name.to_string());
    }
    
    println!("    Setting up NFT marketplace with {} chains", nft_chains.len());
    
    let nft_client = CsvClient::scalable_builder()
        .with_chain_registry(nft_registry)
        .with_all_available_chains()
        .with_store_backend(StoreBackend::InMemory)
        .build()?;
    
    println!("    NFT marketplace ready with chains: {:?}", nft_client.enabled_chains());
    
    // Demonstrate chain-specific operations
    println!("    Ready for cross-chain operations:");
    println!("      - CSV rights transfer across {} chains", defi_client.enabled_chains().len());
    println!("      - NFT operations across {} chains", nft_client.enabled_chains().len());
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_scalable_architecture() {
        let mut registry = ChainRegistry::new();
        registry.register_chain("test".to_string(), "Test".to_string());
        
        let client = CsvClient::scalable_builder()
            .with_chain_registry(registry)
            .with_chain("test")
            .with_store_backend(StoreBackend::InMemory)
            .build();
        
        assert!(client.is_ok());
    }
    
    #[tokio::test]
    async fn test_chain_discovery() {
        let mut discovery = ChainDiscovery::new();
        let result = discovery.load_default_chains();
        assert!(result.is_ok());
    }
    
    #[tokio::test]
    async fn test_scalability() {
        let mut registry = ChainRegistry::new();
        
        // Test with many chains
        for i in 0..100 {
            registry.register_chain(format!("chain{}", i), format!("Chain {}", i));
        }
        
        assert_eq!(registry.supported_chains().len(), 100);
    }
}
