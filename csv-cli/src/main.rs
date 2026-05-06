//! CSV Adapter CLI — Cross-Chain Sanads, Proofs, Wallets, and End-to-End Testing
//!
//! ```bash
//! # Chain management
//! csv chain list
//! csv chain status --chain bitcoin
//! csv chain config --chain ethereum
//!
//! # Wallet operations
//! csv wallet generate --chain bitcoin --network signet
//! csv wallet balance --chain bitcoin --address bcrt1...
//! csv wallet fund --chain bitcoin --network signet
//! csv wallet export --chain bitcoin --format xpub
//!
//! # Sanad operations
//! csv sanad create --chain bitcoin --value 100000
//! csv sanad transfer --chain bitcoin --sanad-id 0x... --to bcrt1...
//! csv sanad consume --chain bitcoin --sanad-id 0x...
//! csv sanad list --chain bitcoin
//!
//! # Proof operations
//! csv proof generate --chain bitcoin --sanad-id 0x...
//! csv proof verify --chain sui --proof-file proof.json
//!
//! # Cross-chain transfers
//! csv cross-chain transfer --from bitcoin --to sui --sanad-id 0x...
//! csv cross-chain status --transfer-id 0x...
//!
//! # Contract deployment
//! csv contract deploy --chain sui --network testnet
//! csv contract status --chain ethereum
//!
//! # End-to-end tests
//! csv test run --chain-pair bitcoin:sui
//! csv test run-all
//! ```

use clap::{Parser, Subcommand};

mod chain_registry;
mod commands;
mod config;
mod output;
mod state;

use commands::*;
use config::Config;
use state::UnifiedStateManager;

/// CLI version from Cargo.toml - single source of truth
const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Parser)]
#[command(
    name = "csv",
    about = "CSV Adapter CLI — Cross-Chain Sanads, Proofs, and Transfers",
    version = VERSION,
    long_about = None
)]
struct Cli {
    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Configuration file path
    #[arg(short = 'C', long, global = true, default_value = "~/.csv/config.toml")]
    config: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    // ─── Chain Management ───
    /// List all supported chains
    Chain {
        #[command(subcommand)]
        action: ChainAction,
    },

    // ─── Wallet Operations ───
    /// Wallet management (generate, fund, balance, export)
    Wallet {
        #[command(subcommand)]
        action: WalletAction,
    },

    // ─── Sanad Operations ───
    /// Sanad lifecycle (create, transfer, consume, list, show)
    Sanad {
        #[command(subcommand)]
        action: SanadAction,
    },

    // ─── Proof Operations ───
    /// Proof generation and verification
    Proof {
        #[command(subcommand)]
        action: ProofAction,
    },

    // ─── Cross-Chain Transfers ───
    /// Cross-chain Sanad transfers
    #[command(name = "cross-chain")]
    CrossChain {
        #[command(subcommand)]
        action: CrossChainAction,
    },

    // ─── Contract Deployment ───
    /// Contract deployment and management
    Contract {
        #[command(subcommand)]
        action: ContractAction,
    },

    // ─── Seal Operations ───
    /// Seal management (create, consume, verify)
    Seal {
        #[command(subcommand)]
        action: SealAction,
    },

    // ─── End-to-End Testing ───
    /// Run end-to-end tests
    Test {
        #[command(subcommand)]
        action: TestAction,
    },

    // ─── Validation ───
    /// Validate consignments and proofs
    Validate {
        #[command(subcommand)]
        action: ValidateAction,
    },
}

use commands::chain::ChainAction;
use commands::contracts::ContractAction;
use commands::cross_chain::CrossChainAction;
use commands::proofs::ProofAction;
use commands::sanads::SanadAction;
use commands::seals::SealAction;
use commands::tests::TestAction;
use commands::validate::ValidateAction;
use commands::wallet::WalletAction;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    std::env::set_var("RUST_LOG", log_level);
    env_logger::init();

    // Load configuration
    let config = Config::load(cli.config.as_deref())?;
    let mut state = UnifiedStateManager::load()?;

    // Dispatch commands
    let result = match cli.command {
        Commands::Chain { action } => chain::execute(action, &config),
        Commands::Wallet { action } => wallet::execute(action, &config, &mut state).await,
        Commands::Sanad { action } => sanads::execute(action, &config, &mut state),
        Commands::Proof { action } => proofs::execute(action, &config, &state),
        Commands::CrossChain { action } => cross_chain::execute(action, &config, &mut state),
        Commands::Contract { action } => contracts::execute(action, &config, &mut state),
        Commands::Seal { action } => seals::execute(action, &config, &mut state),
        Commands::Test { action } => tests::execute(action, &config, &state),
        Commands::Validate { action } => validate::execute(action, &config, &state),
    };

    // Save state
    state.save()?;

    result
}
