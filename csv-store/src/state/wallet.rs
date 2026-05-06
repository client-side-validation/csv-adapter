//! Wallet types.
//!
//! Defines wallet account structures that reference (but don't store)
//! private keys. Keys are stored in `csv-adapter-keystore`.

use super::core::{Chain, Network};
use serde::{Deserialize, Serialize};

/// Wallet account configuration.
///
/// Note: This struct intentionally does NOT store private keys.
/// Use `keystore_ref` to reference encrypted keys in `csv-adapter-keystore`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletAccount {
    /// Account ID (UUID or derived from public key).
    pub id: String,
    /// Chain this account belongs to.
    pub chain: Chain,
    /// Human-readable name.
    pub name: String,
    /// Public address.
    pub address: String,
    /// Keystore reference (UUID pointing to encrypted key in keystore).
    /// Never store the actual private key here!
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keystore_ref: Option<String>,
    /// Extended public key for HD wallets (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub xpub: Option<String>,
    /// Derivation path (BIP-44/86) for HD wallets.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub derivation_path: Option<String>,
}

impl WalletAccount {
    /// Create a new wallet account.
    pub fn new(
        id: impl Into<String>,
        chain: Chain,
        name: impl Into<String>,
        address: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            chain,
            name: name.into(),
            address: address.into(),
            keystore_ref: None,
            xpub: None,
            derivation_path: None,
        }
    }

    /// Check if this is a watch-only account (no keystore reference).
    pub fn is_watch_only(&self) -> bool {
        self.keystore_ref.is_none()
    }

    /// Set the keystore reference.
    pub fn with_keystore_ref(mut self, ref_id: impl Into<String>) -> Self {
        self.keystore_ref = Some(ref_id.into());
        self
    }

    /// Set the derivation path.
    pub fn with_derivation_path(mut self, path: impl Into<String>) -> Self {
        self.derivation_path = Some(path.into());
        self
    }
}

/// Wallet configuration - can use mnemonic or individual accounts.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WalletConfig {
    /// Master mnemonic phrase (encrypted at rest in keystore, optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mnemonic: Option<String>,
    /// Mnemonic passphrase (optional, encrypted at rest).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mnemonic_passphrase: Option<String>,
    /// Individual accounts (one per chain or multiple).
    #[serde(default)]
    pub accounts: Vec<WalletAccount>,
}

impl WalletConfig {
    /// Create empty wallet configuration.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an account to the wallet.
    pub fn add_account(&mut self, account: WalletAccount) {
        self.accounts.push(account);
    }

    /// Get account for a specific chain.
    pub fn get_account(&self, chain: &Chain) -> Option<&WalletAccount> {
        self.accounts.iter().find(|a| &a.chain == chain)
    }

    /// Get mutable account for a specific chain.
    pub fn get_account_mut(&mut self, chain: &Chain) -> Option<&mut WalletAccount> {
        self.accounts.iter_mut().find(|a| &a.chain == chain)
    }
}

/// Faucet configuration for testnet funding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FaucetConfig {
    /// Faucet endpoint URL.
    pub url: String,
    /// Amount to request (chain-specific units).
    pub amount: Option<u64>,
}

impl FaucetConfig {
    /// Default faucet configuration for a chain and network.
    pub fn default_for(chain: &Chain, network: &Network) -> Option<Self> {
        match network {
            Network::Main => None, // No faucets for mainnet
            _ => Some(match chain {
                Chain::Bitcoin => Self {
                    url: "https://signet.bc-2.jp".to_string(),
                    amount: Some(100_000),
                },
                Chain::Ethereum => Self {
                    url: "https://sepoliafaucet.com".to_string(),
                    amount: Some(100_000_000_000_000_000), // 0.1 ETH
                },
                Chain::Sui => Self {
                    url: "https://faucet.testnet.sui.io/v1/gas".to_string(),
                    amount: Some(10_000_000_000),
                },
                Chain::Aptos => Self {
                    url: "https://faucet.testnet.aptoslabs.com".to_string(),
                    amount: Some(100_000_000),
                },
                Chain::Solana => Self {
                    url: "https://faucet.devnet.solana.com".to_string(),
                    amount: Some(1_000_000_000),
                },
            }),
        }
    }
}

/// Gas payment account per chain.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GasAccount {
    /// Chain for this gas account.
    pub chain: Chain,
    /// Address to use for gas payment.
    pub address: String,
}
