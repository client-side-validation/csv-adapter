# Scalable Chain Architecture

This document outlines a scalable architecture for supporting unlimited blockchain chains in the CSV adapter ecosystem.

## 🎯 Objectives

- Support 100+ chains without core code changes
- Enable community contributions for new chains
- Maintain backward compatibility with existing implementations
- Reduce development complexity for new chain integrations

## 🏗️ Architecture Overview

### Current Limitations

The current monolithic architecture has several scalability issues:

1. **Hardcoded Chain Knowledge**: Core builder must know about every chain at compile time
2. **Feature Flag Explosion**: Each chain requires separate feature flags
3. **Tight Coupling**: Chain-specific logic scattered throughout codebase
4. **Manual Integration**: Adding chains requires modifying multiple core components

### Proposed Solution: Plugin Architecture

Move to a **dynamic, plugin-based architecture** where chains are discovered and loaded at runtime.

## 🔧 Core Components

### 1. Chain Adapter Interface

```rust
/// Standard interface for all chain adapters
pub trait ChainAdapter: Send + Sync {
    /// Unique identifier for this chain
    fn chain_id(&self) -> &'static str;
    
    /// Human-readable name for this chain
    fn chain_name(&self) -> &'static str;
    
    /// Create RPC client for this chain
    fn create_client(&self, config: &ChainConfig) -> Result<Box<dyn RpcClient>, ChainError>;
    
    /// Create wallet for this chain
    fn create_wallet(&self, config: &WalletConfig) -> Result<Box<dyn Wallet>, ChainError>;
    
    /// Get chain-specific capabilities
    fn capabilities(&self) -> ChainCapabilities;
    
    /// Validate chain-specific configuration
    fn validate_config(&self, config: &ChainConfig) -> Result<(), ChainError>;
}
```

### 2. Chain Capabilities

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainCapabilities {
    pub supports_nfts: bool,
    pub supports_smart_contracts: bool,
    pub supports_account_model: AccountModel,
    pub confirmation_blocks: u64,
    pub max_batch_size: usize,
    pub supported_networks: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AccountModel {
    UTXO,      // Bitcoin-like
    Account,    // Ethereum-like  
    Object,     // Sui-like
    Hybrid,     // Mixed models
}
```

### 3. Dynamic Chain Registry

```rust
/// Registry for managing chain adapters
pub struct ChainRegistry {
    adapters: HashMap<String, Box<dyn ChainAdapter>>,
    capabilities: HashMap<String, ChainCapabilities>,
}

impl ChainRegistry {
    pub fn new() -> Self {
        Self {
            adapters: HashMap::new(),
            capabilities: HashMap::new(),
        }
    }
    
    /// Register a new chain adapter
    pub fn register_adapter(&mut self, adapter: Box<dyn ChainAdapter>) {
        let chain_id = adapter.chain_id();
        let capabilities = adapter.capabilities();
        
        self.adapters.insert(chain_id.to_string(), adapter);
        self.capabilities.insert(chain_id.to_string(), capabilities);
    }
    
    /// Get adapter by chain ID
    pub fn get_adapter(&self, chain_id: &str) -> Option<&dyn ChainAdapter>> {
        self.adapters.get(chain_id)
    }
    
    /// Get all supported chain IDs
    pub fn supported_chains(&self) -> Vec<&str> {
        self.adapters.keys().map(|k| k.as_str()).collect()
    }
    
    /// Get capabilities for a chain
    pub fn get_capabilities(&self, chain_id: &str) -> Option<&ChainCapabilities> {
        self.capabilities.get(chain_id)
    }
    
    /// Find chains by capability
    pub fn find_chains_with_capability(&self, capability: impl Fn(&ChainCapabilities) -> bool) -> Vec<&str> {
        self.adapters
            .iter()
            .filter(|(_, cap)| capability(cap))
            .map(|(id, _)| id.as_str())
            .collect()
    }
}
```

### 4. Configuration System

```rust
/// Chain-agnostic configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub chain_id: String,
    pub chain_name: String,
    pub default_network: String,
    pub rpc_endpoints: Vec<String>,
    pub program_id: Option<String>,
    pub block_explorer_urls: Vec<String>,
    pub capabilities: ChainCapabilities,
    pub custom_settings: HashMap<String, serde_json::Value>,
}

/// Configuration loader for dynamic chain discovery
pub struct ChainConfigLoader {
    chains_dir: PathBuf,
    configs: HashMap<String, ChainConfig>,
}

impl ChainConfigLoader {
    pub fn new(chains_dir: impl AsRef<Path>) -> Self {
        Self {
            chains_dir: chains_dir.as_ref().to_path_buf(),
            configs: HashMap::new(),
        }
    }
    
    /// Load all chain configurations from directory
    pub fn load_all(&mut self) -> Result<(), ConfigError> {
        let entries = fs::read_dir(&self.chains_dir)
            .map_err(|e| ConfigError::Io(e.to_string()))?;
            
        for entry in entries {
            let entry = entry
                .map_err(|e| ConfigError::Io(e.to_string()))?;
                
            if entry.path().extension() == Some("toml") {
                let content = fs::read_to_string(entry.path())
                    .map_err(|e| ConfigError::Io(e.to_string()))?;
                    
                let config: ChainConfig = toml::from_str(&content)
                    .map_err(|e| ConfigError::Parse(e.to_string()))?;
                    
                self.configs.insert(config.chain_id.clone(), config);
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
}
```

## 🚀 Implementation Strategy

### Phase 1: Core Refactoring (Week 1-2)

#### 1.1 Extract Chain Interface
- Define `ChainAdapter` trait based on existing adapter patterns
- Create `ChainCapabilities` struct
- Implement capability detection

#### 1.2 Create Registry System
- Implement `ChainRegistry` with dynamic registration
- Add configuration loading from filesystem
- Add chain discovery and validation

#### 1.3 Update Core Builder
- Modify `CsvClientBuilder` to use `ChainRegistry`
- Replace hardcoded chain methods with registry lookups
- Maintain backward compatibility during transition

### Phase 2: Adapter Migration (Week 3-4)

#### 2.1 Create Adapter Factory
```rust
pub struct ChainAdapterFactory;

impl ChainAdapterFactory {
    pub fn create_adapter(config: &ChainConfig) -> Result<Box<dyn ChainAdapter>, ChainError> {
        match config.chain_id.as_str() {
            "bitcoin" => Ok(Box::new(BitcoinAdapter::new(config))),
            "ethereum" => Ok(Box::new(EthereumAdapter::new(config))),
            "solana" => Ok(Box::new(SolanaAdapter::new(config))),
            // ... existing adapters
            _ => Err(ChainError::UnsupportedChain(config.chain_id.clone())),
        }
    }
}
```

#### 2.2 Migrate Existing Adapters
- Refactor Bitcoin adapter to implement `ChainAdapter`
- Refactor Ethereum adapter to implement `ChainAdapter`
- Refactor Solana adapter to implement `ChainAdapter`
- Maintain existing functionality during migration

#### 2.3 Dynamic Loading
- Load all chain configurations from `chains/` directory
- Register adapters dynamically based on available configs
- Validate adapter capabilities before registration

### Phase 3: Developer Experience (Week 5-6)

#### 3.1 Tooling and Templates

```bash
# Generate new chain adapter
cargo generate-chain-adapter --chain-id polygon --name "Polygon"

# Create chain config template
cargo create-chain-config --template ethereum --output chains/polygon.toml

# Validate chain configuration
cargo validate-chain-config chains/polygon.toml
```

#### 3.2 Documentation Structure
```
docs/
├── architecture/
│   ├── SCALABLE_CHAIN_ARCHITECTURE.md
│   └── MIGRATION_GUIDE.md
├── development/
│   ├── CHAIN_ADAPTER_DEVELOPMENT.md
│   ├── CONFIGURATION_REFERENCE.md
│   └── TEMPLATES.md
└── examples/
    ├── bitcoin-adapter-example/
    ├── ethereum-adapter-example/
    └── solana-adapter-example/
```

## 🔄 Migration Benefits

### For Developers
- **Standardized Interface**: Consistent API across all chains
- **Faster Development**: Templates and tooling for new chains
- **Better Testing**: Isolated adapter testing
- **Documentation**: Comprehensive guides and examples

### For Users
- **More Chains**: Access to community-developed chain adapters
- **Faster Updates**: Independent adapter updates
- **Better Support**: Chain-specific optimizations and features

### For Maintainers
- **Reduced Maintenance**: Decoupled chain implementations
- **Easier Reviews**: Standardized adapter interface
- **Community Growth**: Lower barrier to contributions

## 📋 Implementation Timeline

| Week | Milestone | Deliverables |
|-------|------------|-------------|
| 1 | Core Refactoring | ChainAdapter trait, ChainRegistry, ConfigLoader |
| 2 | Builder Updates | Dynamic CsvClientBuilder, registry integration |
| 3 | Adapter Migration | Migrate Bitcoin, Ethereum, Solana to new architecture |
| 4 | Tooling | CLI tools, templates, validation |
| 5 | Documentation | Complete developer documentation |
| 6 | Testing | Comprehensive test suite for new architecture |

## 🎯 Success Metrics

- **Chain Addition Time**: < 1 day (config only, no code changes)
- **Developer Onboarding**: < 30 minutes (templates + documentation)
- **Adapter Development**: 1-2 weeks (for complex chains)
- **Backward Compatibility**: 100% maintained during migration

---

*Last updated: April 2026*
