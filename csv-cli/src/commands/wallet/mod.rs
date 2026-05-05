//! Wallet management commands (refactored from 1283-line wallet.rs).
//!
//! This module provides wallet operations split into submodules:
//! - `types`: WalletAction enum and CLI types
//! - `generate`: Wallet generation for all chains
//! - `balance`: Balance checking
//! - `fund`: Faucet funding
//! - `import_export`: Import/export operations
//!
//! # Module Structure
//!
//! ```
//! wallet/
//! ├── mod.rs       # Command dispatcher
//! ├── types.rs     # CLI types and enums
//! ├── generate.rs  # Wallet generation (Bitcoin, Ethereum, Sui, Aptos, Solana)
//! ├── balance.rs   # Balance checking
//! ├── fund.rs      # Faucet funding
//! └── import_export.rs # Import/export operations
//! ```

pub mod balance;
pub mod fund;
pub mod generate;
pub mod import_export;
pub mod types;

pub use types::WalletAction;

use crate::config::Config;
use crate::state::UnifiedStateManager;
use anyhow::Result;

/// Execute wallet command.
pub async fn execute(
    action: WalletAction,
    config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    match action {
        WalletAction::Init {
            network,
            words,
            fund,
            account,
        } => generate::cmd_init(network, words, fund, account, config, state),
        WalletAction::Generate { chain, network } => {
            generate::cmd_generate(chain, network, config, state)
        }
        WalletAction::Balance { chain, address } => {
            balance::cmd_balance(chain, address, config, state).await
        }
        WalletAction::Fund { chain, address } => fund::cmd_fund(chain, address, config, state),
        WalletAction::Export { chain, format } => {
            import_export::cmd_export(chain, format, config, state)
        }
        WalletAction::Import { chain, secret } => {
            import_export::cmd_import(chain, secret, config, state)
        }
        WalletAction::List => balance::cmd_list(config, state),
        WalletAction::Address { chain, address } => {
            import_export::cmd_address(chain, address, state)
        }
        WalletAction::ImportCsvWallet { path } => {
            import_export::cmd_import_csv_wallet(path, config, state)
        }
        WalletAction::ExportCsvWallet { output } => {
            import_export::cmd_export_csv_wallet(output, config, state)
        }
        WalletAction::Sync { path } => import_export::cmd_sync_csv_wallet(path, config, state),
    }
}
