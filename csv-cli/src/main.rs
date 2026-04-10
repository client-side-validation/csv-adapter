//! CSV Adapter CLI — Cross-Chain Rights, Proofs, Wallets, and End-to-End Testing
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
//! # Right operations
//! csv right create --chain bitcoin --value 100000
//! csv right transfer --chain bitcoin --right-id 0x... --to bcrt1...
//! csv right consume --chain bitcoin --right-id 0x...
//! csv right list --chain bitcoin
//!
//! # Proof operations
//! csv proof generate --chain bitcoin --right-id 0x...
//! csv proof verify --chain sui --proof-file proof.json
//!
//! # Cross-chain transfers
//! csv cross-chain transfer --from bitcoin --to sui --right-id 0x...
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

mod commands;
mod config;
mod output;
mod state;

use commands::*;
use config::Config;
use state::State;

#[derive(Parser)]
#[command(
    name = "csv",
    about = "CSV Adapter CLI — Cross-Chain Rights, Proofs, and Transfers",
    version = "0.1.0",
    long_about = None
)]
struct Cli {
    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,

    /// Configuration file path
    #[arg(short, long, global = true, default_value = "~/.csv/config.toml")]
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

    // ─── Right Operations ───
    /// Right lifecycle (create, transfer, consume, list, show)
    Right {
        #[command(subcommand)]
        action: RightAction,
    },

    // ─── Proof Operations ───
    /// Proof generation and verification
    Proof {
        #[command(subcommand)]
        action: ProofAction,
    },

    // ─── Cross-Chain Transfers ───
    /// Cross-chain Right transfers
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
use commands::rights::RightAction;
use commands::seals::SealAction;
use commands::tests::TestAction;
use commands::validate::ValidateAction;
use commands::wallet::WalletAction;

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    let log_level = if cli.verbose { "debug" } else { "info" };
    std::env::set_var("RUST_LOG", log_level);
    env_logger::init();

    // Load configuration
    let config = Config::load(cli.config.as_deref())?;
    let mut state = State::load()?;

    // Dispatch commands
    let result = match cli.command {
        Commands::Chain { action } => chain::execute(action, &config),
        Commands::Wallet { action } => wallet::execute(action, &config, &mut state),
        Commands::Right { action } => rights::execute(action, &config, &state),
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
