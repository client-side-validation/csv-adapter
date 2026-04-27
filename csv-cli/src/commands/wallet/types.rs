//! Wallet command types and enums.
//!
//! Defines the CLI interface for wallet operations.

use crate::config::{Chain, Network};
use clap::Subcommand;

/// Wallet management actions.
#[derive(Subcommand)]
pub enum WalletAction {
    /// Initialize wallet with one-command setup (generate, fund, configure)
    Init {
        /// Network (dev/test/main)
        #[arg(value_enum, default_value = "dev")]
        network: Network,
        /// Generate mnemonic (12 or 24 words)
        #[arg(short, long, default_value = "12")]
        words: u8,
        /// Auto-fund from faucets
        #[arg(long, default_value = "true")]
        fund: bool,
    },
    /// Generate a new wallet
    Generate {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
        /// Network (dev/test/main)
        #[arg(value_enum, default_value = "test")]
        network: Network,
    },
    /// Show wallet balance
    Balance {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
        /// Address (uses stored address if not provided)
        #[arg(short, long)]
        address: Option<String>,
    },
    /// Fund wallet from faucet
    Fund {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
        /// Address (uses stored address if not provided)
        #[arg(short, long)]
        address: Option<String>,
    },
    /// Export wallet (xpub, mnemonic, or private key)
    Export {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
        /// Export format
        #[arg(short, long, default_value = "address")]
        format: String,
    },
    /// Import wallet from private key or mnemonic
    Import {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
        /// Private key (hex) or mnemonic phrase
        secret: String,
    },
    /// List wallets
    List,
    /// Set or get address for a chain
    Address {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
        /// Address to set (if not provided, shows current address)
        #[arg(value_name = "ADDRESS")]
        address: Option<String>,
    },
    /// Import full wallet from csv-wallet JSON export
    ImportCsvWallet {
        /// Path to csv-wallet JSON file (default: ~/.csv/wallet/csv-wallet.json)
        #[arg(short, long)]
        path: Option<String>,
    },
    /// Export wallet to csv-wallet JSON format
    ExportCsvWallet {
        /// Output file path (default: ~/.csv/wallet/csv-wallet-export.json)
        #[arg(short, long)]
        output: Option<String>,
    },
    /// Sync with csv-wallet (import all accounts, update addresses)
    Sync {
        /// Path to csv-wallet JSON (default: ~/.csv/wallet/csv-wallet.json)
        #[arg(short, long)]
        path: Option<String>,
    },
}

/// Wallet export formats.
pub enum ExportFormat {
    /// Export address only.
    Address,
    /// Export extended public key.
    Xpub,
    /// Export mnemonic (requires passphrase).
    Mnemonic,
    /// Export private key (dangerous!).
    PrivateKey,
}

impl std::str::FromStr for ExportFormat {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "address" => Ok(ExportFormat::Address),
            "xpub" => Ok(ExportFormat::Xpub),
            "mnemonic" => Ok(ExportFormat::Mnemonic),
            "private-key" | "privatekey" => Ok(ExportFormat::PrivateKey),
            _ => Err(format!("Unknown export format: {}", s)),
        }
    }
}
