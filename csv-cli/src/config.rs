//! Configuration management — chains, wallets, RPC endpoints

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

/// Network environment
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Dev,
    Test,
    Main,
}

impl std::fmt::Display for Network {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Network::Dev => write!(f, "dev"),
            Network::Test => write!(f, "test"),
            Network::Main => write!(f, "main"),
        }
    }
}

/// Supported chains
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Chain {
    Bitcoin,
    Ethereum,
    Sui,
    Aptos,
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Chain::Bitcoin => write!(f, "bitcoin"),
            Chain::Ethereum => write!(f, "ethereum"),
            Chain::Sui => write!(f, "sui"),
            Chain::Aptos => write!(f, "aptos"),
        }
    }
}

/// Chain-specific configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    /// RPC endpoint URL
    pub rpc_url: String,
    /// Network environment
    pub network: Network,
    /// Contract/package address (if deployed)
    pub contract_address: Option<String>,
    /// Chain ID (for EVM chains) or magic bytes (Bitcoin)
    pub chain_id: Option<u64>,
    /// Finality depth (confirmations required)
    pub finality_depth: u64,
    /// Default gas price / fee rate
    pub default_fee: Option<u64>,
}

/// Wallet configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    /// Private key (hex) — for signing
    pub private_key: Option<String>,
    /// Extended public key — for address derivation
    pub xpub: Option<String>,
    /// Mnemonic phrase (space-separated)
    pub mnemonic: Option<String>,
    /// Mnemonic passphrase (optional)
    pub mnemonic_passphrase: Option<String>,
    /// BIP-44/86 derivation path
    pub derivation_path: Option<String>,
}

/// Faucet configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaucetConfig {
    /// Faucet endpoint URL
    pub url: String,
    /// Amount to request (chain-specific units)
    pub amount: Option<u64>,
}

/// Full CLI configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Chain configurations
    pub chains: HashMap<Chain, ChainConfig>,
    /// Wallet configurations (per chain)
    pub wallets: HashMap<Chain, WalletConfig>,
    /// Faucet configurations
    pub faucets: HashMap<Chain, FaucetConfig>,
    /// Data directory for state persistence
    pub data_dir: String,
}

impl Default for Config {
    fn default() -> Self {
        let mut chains = HashMap::new();

        // Bitcoin Signet (default dev network)
        chains.insert(Chain::Bitcoin, ChainConfig {
            rpc_url: "https://mempool.space/signet/api/".to_string(),
            network: Network::Test,
            contract_address: None, // UTXO-native, no contract
            chain_id: None,
            finality_depth: 6,
            default_fee: Some(10), // 10 sat/vB
        });

        // Ethereum Sepolia
        chains.insert(Chain::Ethereum, ChainConfig {
            rpc_url: "https://rpc.sepolia.org".to_string(),
            network: Network::Test,
            contract_address: None, // Not deployed yet
            chain_id: Some(11155111),
            finality_depth: 15,
            default_fee: Some(20_000_000_000), // 20 gwei
        });

        // Sui Testnet
        chains.insert(Chain::Sui, ChainConfig {
            rpc_url: "https://fullnode.testnet.sui.io:443".to_string(),
            network: Network::Test,
            contract_address: None, // Not deployed yet
            chain_id: None,
            finality_depth: 1, // Checkpoint certified
            default_fee: Some(1000),
        });

        // Aptos Testnet
        chains.insert(Chain::Aptos, ChainConfig {
            rpc_url: "https://fullnode.testnet.aptoslabs.com/v1".to_string(),
            network: Network::Test,
            contract_address: None, // Not deployed yet
            chain_id: None,
            finality_depth: 1, // HotStuff consensus
            default_fee: Some(100),
        });

        let mut wallets = HashMap::new();
        let mut faucets = HashMap::new();

        // Bitcoin faucet
        faucets.insert(Chain::Bitcoin, FaucetConfig {
            url: "https://signet.bc-2.jp".to_string(),
            amount: Some(100_000), // 100k sats
        });

        // Sui faucet
        faucets.insert(Chain::Sui, FaucetConfig {
            url: "https://faucet.testnet.sui.io/v1/gas".to_string(),
            amount: Some(10_000_000_000), // 10 SUI
        });

        // Aptos faucet
        faucets.insert(Chain::Aptos, FaucetConfig {
            url: "https://faucet.testnet.aptoslabs.com".to_string(),
            amount: Some(100_000_000), // 1 APT
        });

        // Ethereum faucet
        faucets.insert(Chain::Ethereum, FaucetConfig {
            url: "https://sepoliafaucet.com".to_string(),
            amount: Some(100_000_000_000_000_000), // 0.1 ETH
        });

        Self {
            chains,
            wallets,
            faucets,
            data_dir: "~/.csv/data".to_string(),
        }
    }
}

impl Config {
    /// Load configuration from file or return defaults
    pub fn load(path: Option<&str>) -> anyhow::Result<Self> {
        let path = match path {
            Some(p) => expand_path(p),
            None => expand_path("~/.csv/config.toml"),
        };

        if Path::new(&path).exists() {
            let content = std::fs::read_to_string(&path)?;
            let config: Config = toml::from_str(&content)?;
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
        self.chains.get(chain).ok_or_else(|| {
            anyhow::anyhow!("Chain {} not configured", chain)
        })
    }

    /// Get wallet configuration
    pub fn wallet(&self, chain: &Chain) -> Option<&WalletConfig> {
        self.wallets.get(chain)
    }

    /// Get faucet configuration
    pub fn faucet(&self, chain: &Chain) -> Option<&FaucetConfig> {
        self.faucets.get(chain)
    }

    /// Set chain configuration
    pub fn set_chain(&mut self, chain: Chain, config: ChainConfig) {
        self.chains.insert(chain, config);
    }

    /// Set wallet configuration
    pub fn set_wallet(&mut self, chain: Chain, config: WalletConfig) {
        self.wallets.insert(chain, config);
    }
}

/// Expand ~ to home directory
fn expand_path(path: &str) -> String {
    if path.starts_with("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(&path[2..]).to_string_lossy().to_string();
        }
    }
    path.to_string()
}
