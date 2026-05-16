//! Wallet metadata types.

use serde::{Deserialize, Serialize};

/// Wallet metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletMetadata {
    /// Wallet ID (unique identifier)
    pub id: String,
    /// Wallet name (user-defined)
    pub name: Option<String>,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last access timestamp
    pub last_accessed: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether this wallet is the active wallet
    pub is_active: bool,
}

/// Bitcoin network type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BitcoinNetwork {
    Mainnet,
    #[default]
    Testnet,
    Signet,
    Regtest,
}
