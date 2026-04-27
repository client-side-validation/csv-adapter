//! Contract command types and enums.
//!
//! Defines the CLI interface for contract operations.

use crate::config::Chain;
use clap::Subcommand;

/// Contract management actions.
#[derive(Debug, Clone, Subcommand)]
pub enum ContractAction {
    /// Deploy contracts to a chain
    Deploy {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
        /// Network (dev/test/main)
        #[arg(short, long)]
        network: Option<String>,
        /// Deployer private key (Ethereum: hex private key, Sui/Aptos: uses CLI wallet)
        #[arg(long)]
        deployer_key: Option<String>,
        /// Account address to use for deployment
        #[arg(short, long)]
        account: Option<String>,
    },
    /// Show deployed contract info
    Status {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
    },
    /// Verify deployed contract
    Verify {
        /// Chain name
        #[arg(value_enum)]
        chain: Chain,
    },
    /// List all deployed contracts
    List,
    /// Fetch contracts from chain for stored addresses
    Fetch {
        /// Specific chain to fetch (optional, fetches all if omitted)
        #[arg(value_enum)]
        chain: Option<Chain>,
    },
}

/// A discovered contract from chain query.
#[derive(Debug, Clone)]
pub struct DiscoveredContract {
    /// Contract address.
    pub address: String,
    /// Contract description.
    pub description: String,
}
