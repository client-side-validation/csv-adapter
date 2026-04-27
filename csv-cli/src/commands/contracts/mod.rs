//! Contract deployment commands (refactored from 591-line contracts.rs).
//!
//! This module provides contract deployment using the deploy modules
//! from csv-adapter-{chain} crates via RPC, replacing CLI subprocess calls.
//!
//! # Module Structure
//!
//! ```
//! contracts/
//! ├── mod.rs       # Command dispatcher
//! ├── types.rs     # ContractAction enum and types
//! ├── deploy.rs    # Chain-specific deployment (Ethereum, Sui, Aptos, Solana)
//! └── status.rs    # Status, verify, list, fetch operations
//! ```

pub mod deploy;
pub mod status;
pub mod types;

pub use types::ContractAction;

use crate::config::Config;
use crate::state::UnifiedStateManager;
use anyhow::Result;

/// Execute contract command.
pub fn execute(
    action: ContractAction,
    config: &Config,
    state: &mut UnifiedStateManager,
) -> Result<()> {
    match action {
        ContractAction::Deploy {
            chain,
            network,
            deployer_key,
            account,
        } => deploy::cmd_deploy(chain, network, deployer_key, account, config, state),
        ContractAction::Status { chain } => status::cmd_status(chain, config, state),
        ContractAction::Verify { chain } => status::cmd_verify(chain, config, state),
        ContractAction::List => status::cmd_list(state),
        ContractAction::Fetch { chain } => status::cmd_fetch(chain, config, state),
    }
}
