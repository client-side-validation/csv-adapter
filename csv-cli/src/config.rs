//! Configuration management — chains, wallets, RPC endpoints
//!
//! Uses unified storage types from csv-adapter-store for compatibility with csv-wallet.

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

// Re-export unified types from csv-adapter-store
pub use csv_adapter_store::state::{
    Chain, ChainConfig, FaucetConfig, Network, WalletAccount,
};

/// CSV Wallet exported JSON format (legacy, for migration from csv-wallet < 0.4)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CsvWalletData {
    accounts: Vec<CsvAccount>,
    selected_account_id: Option<String>,
}

/// CSV Wallet account entry (legacy format)
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CsvAccount {
    id: String,
    chain: String,
    name: String,
    private_key: String,
    address: String,
}

impl CsvWalletData {
    /// Load from csv-wallet JSON file (legacy format)
    fn load_from_file(path: &str) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let data: CsvWalletData = serde_json::from_str(&content)?;
        Ok(data)
    }

    /// Find account by chain name (case-insensitive)
    fn find_account(&self, chain: &str) -> Option<&CsvAccount> {
        self.accounts.iter().find(|a| a.chain.eq_ignore_ascii_case(chain))
    }
}

/// Global cache for csv-wallet configs loaded at runtime
static CSV_WALLET_CACHE: OnceLock<Mutex<HashMap<Chain, LegacyWalletConfig>>> = OnceLock::new();

fn get_csv_wallet_cache() -> &'static Mutex<HashMap<Chain, LegacyWalletConfig>> {
    CSV_WALLET_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Legacy wallet config for backwards compatibility (maps to unified WalletAccount)
#[derive(Debug, Clone)]
pub(crate) struct LegacyWalletConfig {
    pub private_key: Option<String>,
    pub xpub: Option<String>,
    pub mnemonic: Option<String>,
    pub mnemonic_passphrase: Option<String>,
    pub derivation_path: Option<String>,
}

/// Parse chain from string (for clap)
pub fn parse_chain(s: &str) -> anyhow::Result<Chain> {
    match s.to_lowercase().as_str() {
        "bitcoin" => Ok(Chain::Bitcoin),
        "ethereum" => Ok(Chain::Ethereum),
        "sui" => Ok(Chain::Sui),
        "aptos" => Ok(Chain::Aptos),
        "solana" => Ok(Chain::Solana),
        _ => Err(anyhow::anyhow!("Unknown chain: {}", s)),
    }
}

/// Parse network from string (for clap)
pub fn parse_network(s: &str) -> anyhow::Result<Network> {
    match s.to_lowercase().as_str() {
        "dev" => Ok(Network::Dev),
        "test" => Ok(Network::Test),
        "main" => Ok(Network::Main),
        _ => Err(anyhow::anyhow!("Unknown network: {}", s)),
    }
}

/// Full CLI configuration using unified storage types
/// 
/// Note: New code should use UnifiedStorage from csv_adapter_store::state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Chain configurations
    #[serde(default)]
    pub chains: HashMap<Chain, ChainConfig>,
    /// Legacy wallet configurations (per chain) - migrated to unified.accounts
    #[serde(default)]
    pub wallets: HashMap<Chain, LegacyWalletConfigToml>,
    /// Faucet configurations
    #[serde(default)]
    pub faucets: HashMap<Chain, FaucetConfig>,
    /// Data directory for state persistence
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
}

/// Legacy wallet config for TOML parsing (will be migrated to unified WalletAccount)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LegacyWalletConfigToml {
    pub private_key: Option<String>,
    pub xpub: Option<String>,
    pub mnemonic: Option<String>,
    pub mnemonic_passphrase: Option<String>,
    pub derivation_path: Option<String>,
}

impl From<LegacyWalletConfigToml> for LegacyWalletConfig {
    fn from(cfg: LegacyWalletConfigToml) -> Self {
        Self {
            private_key: cfg.private_key,
            xpub: cfg.xpub,
            mnemonic: cfg.mnemonic,
            mnemonic_passphrase: cfg.mnemonic_passphrase,
            derivation_path: cfg.derivation_path,
        }
    }
}

fn default_data_dir() -> String {
    "~/.csv/data".to_string()
}

impl Default for Config {
    fn default() -> Self {
        let mut chains = HashMap::new();

        // Bitcoin Signet (default dev network)
        chains.insert(
            Chain::Bitcoin,
            ChainConfig {
                rpc_url: "https://mempool.space/signet/api/".to_string(),
                network: Network::Test,
                contract_address: None, // UTXO-native, no contract
                chain_id: None,
                finality_depth: 6,
                default_fee: Some(10), // 10 sat/vB
            },
        );

        // Ethereum Sepolia
        chains.insert(
            Chain::Ethereum,
            ChainConfig {
                rpc_url: "https://ethereum-sepolia-rpc.publicnode.com".to_string(),
                network: Network::Test,
                contract_address: None, // Not deployed yet
                chain_id: Some(11155111),
                finality_depth: 15,
                default_fee: Some(20_000_000_000), // 20 gwei
            },
        );

        // Sui Testnet
        chains.insert(
            Chain::Sui,
            ChainConfig {
                rpc_url: "https://fullnode.testnet.sui.io:443".to_string(),
                network: Network::Test,
                contract_address: None, // Not deployed yet
                chain_id: None,
                finality_depth: 1, // Checkpoint certified
                default_fee: Some(1000),
            },
        );

        // Aptos Testnet
        chains.insert(
            Chain::Aptos,
            ChainConfig {
                rpc_url: "https://fullnode.testnet.aptoslabs.com/v1".to_string(),
                network: Network::Test,
                contract_address: None, // Not deployed yet
                chain_id: None,
                finality_depth: 1, // HotStuff consensus
                default_fee: Some(100),
            },
        );

        // Solana Devnet
        chains.insert(
            Chain::Solana,
            ChainConfig {
                rpc_url: "https://api.devnet.solana.com".to_string(),
                network: Network::Test,
                contract_address: None, // Not deployed yet
                chain_id: None,
                finality_depth: 32,      // Solana finality
                default_fee: Some(5000), // 5000 lamports
            },
        );

        let wallets = HashMap::new();
        let mut faucets = HashMap::new();

        // Bitcoin faucet
        faucets.insert(
            Chain::Bitcoin,
            FaucetConfig {
                url: "https://signet.bc-2.jp".to_string(),
                amount: Some(100_000), // 100k sats
            },
        );

        // Sui faucet
        faucets.insert(
            Chain::Sui,
            FaucetConfig {
                url: "https://faucet.testnet.sui.io/v1/gas".to_string(),
                amount: Some(10_000_000_000), // 10 SUI
            },
        );

        // Aptos faucet
        faucets.insert(
            Chain::Aptos,
            FaucetConfig {
                url: "https://faucet.testnet.aptoslabs.com".to_string(),
                amount: Some(100_000_000), // 1 APT
            },
        );

        // Ethereum faucet
        faucets.insert(
            Chain::Ethereum,
            FaucetConfig {
                url: "https://sepoliafaucet.com".to_string(),
                amount: Some(100_000_000_000_000_000), // 0.1 ETH
            },
        );

        // Solana faucet
        faucets.insert(
            Chain::Solana,
            FaucetConfig {
                url: "https://faucet.devnet.solana.com".to_string(),
                amount: Some(1_000_000_000), // 1 SOL
            },
        );

        Self {
            chains,
            wallets,
            faucets,
            data_dir: "~/.csv/data".to_string(),
        }
    }
}

#[allow(dead_code)]
impl Config {
    /// Get default network
    pub fn network(&self) -> Network {
        Network::Dev
    }

    /// Load configuration from file or return defaults
    pub fn load(path: Option<&str>) -> anyhow::Result<Self> {
        let path = match path {
            Some(p) => expand_path(p),
            None => expand_path("~/.csv/config.toml"),
        };

        if Path::new(&path).exists() {
            let content = std::fs::read_to_string(&path)?;
            let mut config: Config = toml::from_str(&content)?;

            // Merge missing chains and faucets from defaults
            let defaults = Config::default();
            for (chain, chain_config) in defaults.chains {
                config.chains.entry(chain).or_insert(chain_config);
            }
            for (chain, faucet_config) in defaults.faucets {
                config.faucets.entry(chain).or_insert(faucet_config);
            }

            Ok(config)
        } else {
            // Create default config
            let config = Config::default();
            // Ensure directory exists
            if let Some(parent) = Path::new(&path).parent() {
                std::fs::create_dir_all(parent)?;
            }
            // Write default config
            let toml_content = toml::to_string_pretty(&config)?;
            std::fs::write(&path, toml_content)?;
            eprintln!("Created default config at {}", path);
            Ok(config)
        }
    }

    /// Get chain configuration
    pub fn chain(&self, chain: &Chain) -> anyhow::Result<&ChainConfig> {
        self.chains
            .get(chain)
            .ok_or_else(|| anyhow::anyhow!("Chain {} not configured", chain))
    }

    /// Get wallet configuration (legacy - use unified storage instead)
    /// First checks config.toml, then falls back to ~/.csv/wallet/csv-wallet.json
    #[deprecated(since = "0.4.0", note = "Use unified storage WalletConfig instead")]
    pub fn wallet(&self, chain: &Chain) -> Option<LegacyWalletConfig> {
        // First check config.toml wallets
        if let Some(wallet) = self.wallets.get(chain) {
            return Some(wallet.clone().into());
        }

        // Fall back to csv-wallet exported JSON (legacy format)
        let csv_wallet_path = expand_path("~/.csv/wallet/csv-wallet.json");
        if let Ok(csv_wallet) = CsvWalletData::load_from_file(&csv_wallet_path) {
            if let Some(account) = csv_wallet.find_account(&chain.to_string()) {
                // Create a LegacyWalletConfig from the CSV account
                return get_cached_wallet_config(chain, account);
            }
        }

        None
    }
    
    /// Get unified wallet account for a chain (preferred method)
    pub fn wallet_account(&self, chain: &Chain) -> Option<WalletAccount> {
        // First try to load from unified storage
        if let Ok(unified) = crate::state::UnifiedStateManager::load() {
            if let Some(account) = unified.storage.get_account(chain) {
                return Some(account.clone());
            }
        }
        
        // Fall back to legacy config.toml
        if let Some(legacy) = self.wallets.get(chain) {
            return Some(WalletAccount {
                id: format!("{}-legacy", chain),
                chain: chain.clone(),
                name: format!("{} Legacy", chain),
                address: String::new(), // Will be derived from private key
                private_key: legacy.private_key.clone(),
                xpub: legacy.xpub.clone(),
                derivation_path: legacy.derivation_path.clone(),
                keystore_ref: None,
            });
        }
        
        None
    }

    /// Get faucet configuration
    pub fn faucet(&self, chain: &Chain) -> Option<&FaucetConfig> {
        self.faucets.get(chain)
    }

    /// Set chain configuration
    pub fn set_chain(&mut self, chain: Chain, config: ChainConfig) {
        self.chains.insert(chain, config);
    }

    /// Set wallet configuration (legacy TOML format)
    pub fn set_wallet(&mut self, chain: Chain, config: LegacyWalletConfigToml) {
        self.wallets.insert(chain, config);
    }
    
    /// Set unified wallet account
    pub fn set_wallet_account(&mut self, _chain: Chain, account: WalletAccount) -> anyhow::Result<()> {
        // Also update unified storage
        let mut unified = crate::state::UnifiedStateManager::load()?;
        unified.storage.set_account(account);
        unified.save()?;
        Ok(())
    }
}

/// Get cached wallet config from csv-wallet data (internal helper, legacy format)
fn get_cached_wallet_config(chain: &Chain, account: &CsvAccount) -> Option<LegacyWalletConfig> {
    let cache = get_csv_wallet_cache();
    let mut cache = cache.lock().ok()?;

    // Insert if not exists
    cache.entry(chain.clone()).or_insert_with(|| LegacyWalletConfig {
        private_key: Some(account.private_key.clone()),
        xpub: None,
        mnemonic: None,
        mnemonic_passphrase: None,
        derivation_path: None,
    });

    cache.get(chain).cloned()
}

/// Expand ~ to home directory
fn expand_path(path: &str) -> String {
    if let Some(stripped) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped).to_string_lossy().to_string();
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert!(
            !config.chains.is_empty(),
            "Default config should have chains"
        );
        assert!(
            config.wallets.is_empty(),
            "Default config should have empty wallets"
        );
        assert!(
            !config.faucets.is_empty(),
            "Default config should have faucets"
        );
        assert_eq!(config.data_dir, "~/.csv/data");
    }

    #[test]
    fn test_config_with_missing_chains_field() {
        // This was the original bug - config with only wallet section
        let toml_content = r#"
[wallet]
mnemonic = "word1 word2 word3"
network = "dev"

[wallet.bitcoin]
address = "bcrt1test"
"#;
        let config: Result<Config, _> = toml::from_str(toml_content);
        assert!(
            config.is_ok(),
            "Should parse config without chains field: {:?}",
            config.err()
        );

        let config = config.unwrap();
        assert!(
            config.chains.is_empty(),
            "Chains should be empty when not specified"
        );
        assert!(
            config.wallets.is_empty(),
            "Wallets HashMap not populated from [wallet]"
        );
    }

    #[test]
    fn test_config_with_empty_file() {
        let toml_content = "";
        let config: Result<Config, _> = toml::from_str(toml_content);
        assert!(
            config.is_ok(),
            "Should parse empty config: {:?}",
            config.err()
        );

        let config = config.unwrap();
        assert!(config.chains.is_empty());
        assert!(config.wallets.is_empty());
        assert!(config.faucets.is_empty());
        assert_eq!(config.data_dir, "~/.csv/data");
    }

    #[test]
    fn test_config_with_partial_chains() {
        let toml_content = r#"
[chains.bitcoin]
rpc_url = "https://bitcoin.example.com"
network = "test"
finality_depth = 6

[chains.ethereum]
rpc_url = "https://ethereum.example.com"
network = "main"
finality_depth = 12
chain_id = 1
"#;
        let config: Result<Config, _> = toml::from_str(toml_content);
        assert!(
            config.is_ok(),
            "Should parse config with partial chains: {:?}",
            config.err()
        );

        let config = config.unwrap();
        assert_eq!(config.chains.len(), 2);
        assert!(config.chains.contains_key(&Chain::Bitcoin));
        assert!(config.chains.contains_key(&Chain::Ethereum));
    }

    #[test]
    fn test_config_with_wallets() {
        let toml_content = r#"
[wallets.bitcoin]
mnemonic = "word1 word2"
derivation_path = "m/84'/0'/0'/0/0"

[wallets.ethereum]
private_key = "0xabc123"
"#;
        let config: Result<Config, _> = toml::from_str(toml_content);
        assert!(
            config.is_ok(),
            "Should parse config with wallets: {:?}",
            config.err()
        );

        let config = config.unwrap();
        assert_eq!(config.wallets.len(), 2);
        assert!(config.wallets.contains_key(&Chain::Bitcoin));
        assert!(config.wallets.contains_key(&Chain::Ethereum));
    }

    #[test]
    fn test_config_roundtrip_serialization() {
        let original = Config::default();
        let toml_str = toml::to_string_pretty(&original).expect("Should serialize");
        let deserialized: Config = toml::from_str(&toml_str).expect("Should deserialize");

        assert_eq!(original.chains.len(), deserialized.chains.len());
        assert_eq!(original.faucets.len(), deserialized.faucets.len());
        assert_eq!(original.data_dir, deserialized.data_dir);
    }

    #[test]
    fn test_config_load_creates_default_when_missing() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("nonexistent_config.toml");

        let config = Config::load(Some(config_path.to_str().unwrap()));
        assert!(
            config.is_ok(),
            "Should create default config when file missing: {:?}",
            config.err()
        );

        // Verify file was created
        assert!(config_path.exists(), "Config file should be created");
    }

    #[test]
    fn test_config_load_from_existing_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("test_config.toml");

        // Create a config file with valid structure
        // data_dir must be at top level, not inside any table
        let toml_content = r#"data_dir = "/custom/data/dir"

[chains.bitcoin]
rpc_url = "https://custom.bitcoin.com"
network = "test"
finality_depth = 3
"#;
        let mut file = std::fs::File::create(&config_path).unwrap();
        file.write_all(toml_content.as_bytes()).unwrap();
        drop(file);

        // Test direct TOML parsing first
        let parsed: Result<Config, _> = toml::from_str(toml_content);
        assert!(
            parsed.is_ok(),
            "Direct TOML parse failed: {:?}",
            parsed.err()
        );
        let direct = parsed.unwrap();
        assert_eq!(
            direct.data_dir, "/custom/data/dir",
            "Direct parse: data_dir mismatch"
        );

        // Test loading from file
        let config = Config::load(Some(config_path.to_str().unwrap()));
        assert!(
            config.is_ok(),
            "Should load existing config: {:?}",
            config.err()
        );

        let config = config.unwrap();
        // Config now merges missing chains from defaults, so we expect 5 chains total
        assert_eq!(
            config.chains.len(),
            5,
            "Expected 5 chains (1 from file + 4 merged defaults), got {}",
            config.chains.len()
        );
        assert!(
            config.chains.contains_key(&Chain::Bitcoin),
            "Should have Bitcoin chain"
        );
        assert!(
            config.chains.contains_key(&Chain::Ethereum),
            "Should have Ethereum chain (merged)"
        );
        assert!(
            config.chains.contains_key(&Chain::Sui),
            "Should have Sui chain (merged)"
        );
        assert!(
            config.chains.contains_key(&Chain::Aptos),
            "Should have Aptos chain (merged)"
        );
        assert!(
            config.chains.contains_key(&Chain::Solana),
            "Should have Solana chain (merged)"
        );
        assert_eq!(
            config.data_dir, "/custom/data/dir",
            "Loaded config: data_dir should be from file"
        );
    }

    #[test]
    fn test_config_chain_accessor() {
        let config = Config::default();

        // Should get existing chain
        let bitcoin = config.chain(&Chain::Bitcoin);
        assert!(bitcoin.is_ok());

        // Should error for non-existent chain in default config (all chains exist)
        // But if we create empty config:
        let empty_config = Config {
            chains: HashMap::new(),
            wallets: HashMap::new(),
            faucets: HashMap::new(),
            data_dir: "~/.csv/data".to_string(),
        };
        let missing = empty_config.chain(&Chain::Bitcoin);
        assert!(missing.is_err());
    }

    #[test]
    fn test_expand_path() {
        let expanded = expand_path("~/.csv/config.toml");
        assert!(!expanded.starts_with('~'), "Path should be expanded");

        let absolute = expand_path("/absolute/path/config.toml");
        assert_eq!(absolute, "/absolute/path/config.toml");
    }

    #[test]
    fn test_invalid_toml_errors_gracefully() {
        let invalid_toml = r#"
[chains.bitcoin
rpc_url = "missing bracket"
"#;
        let config: Result<Config, _> = toml::from_str(invalid_toml);
        assert!(config.is_err(), "Should error on invalid TOML");
    }

    #[test]
    fn test_config_set_chain_and_wallet() {
        let mut config = Config::default();

        // Set a new chain config
        let new_chain = ChainConfig {
            rpc_url: "https://new.chain.com".to_string(),
            network: Network::Test,
            contract_address: None,
            chain_id: Some(12345),
            finality_depth: 10,
            default_fee: Some(1000),
        };
        config.set_chain(Chain::Solana, new_chain);

        assert!(config.chains.contains_key(&Chain::Solana));

        // Set a wallet
        let wallet = WalletConfig {
            private_key: Some("key".to_string()),
            xpub: None,
            mnemonic: None,
            mnemonic_passphrase: None,
            derivation_path: None,
        };
        config.set_wallet(Chain::Bitcoin, wallet);

        assert!(config.wallets.contains_key(&Chain::Bitcoin));
    }

    #[test]
    fn test_network_display() {
        assert_eq!(Network::Dev.to_string(), "dev");
        assert_eq!(Network::Test.to_string(), "test");
        assert_eq!(Network::Main.to_string(), "main");
    }

    #[test]
    fn test_chain_display() {
        assert_eq!(Chain::Bitcoin.to_string(), "bitcoin");
        assert_eq!(Chain::Ethereum.to_string(), "ethereum");
        assert_eq!(Chain::Sui.to_string(), "sui");
        assert_eq!(Chain::Aptos.to_string(), "aptos");
        assert_eq!(Chain::Solana.to_string(), "solana");
    }

    #[test]
    fn test_wallet_and_faucet_accessors() {
        let config = Config::default();

        // Wallet accessor returns None for non-existent
        assert!(config.wallet(&Chain::Bitcoin).is_none());

        // Faucet accessor returns Some for existing
        assert!(config.faucet(&Chain::Bitcoin).is_some());

        // Faucet accessor returns Some for Solana (now configured by default)
        assert!(config.faucet(&Chain::Solana).is_some());
    }
}
