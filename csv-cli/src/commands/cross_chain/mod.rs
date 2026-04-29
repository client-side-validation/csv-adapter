//! Cross-chain transfer commands

use anyhow::Result;
use clap::{ArgAction, Subcommand};

use csv_adapter_core::cross_chain::{
    ChainId, CrossChainFinalityProof, CrossChainSealRegistry, CrossChainTransferProof,
    LockProvider, MintProvider, TransferVerifier,
};
use csv_adapter_core::hash::Hash;
use csv_adapter_core::right::OwnershipProof;

use crate::config::{Chain, Config};
use crate::output;
use crate::state::{UnifiedStateManager, RightRecord, TransferRecord, TransferStatus};

pub mod aptos;
pub mod bitcoin;
pub mod ethereum;
pub mod status;
pub mod transfer;
pub mod utils;

use super::cross_chain_impl::*;

/// RPC response for block height queries
#[derive(Debug, serde::Deserialize)]
struct JsonRpcResponse<T> {
    result: Option<T>,
    error: Option<JsonRpcError>,
}

#[derive(Debug, serde::Deserialize)]
struct JsonRpcError {
    message: String,
}

/// Bitcoin REST API block height response
#[derive(Debug, serde::Deserialize)]
#[allow(dead_code)]
struct BitcoinBlockHeight {
    height: u64,
}

#[derive(Subcommand)]
pub enum CrossChainAction {
    /// Execute a cross-chain Right transfer
    Transfer {
        /// Source chain
        #[arg(long)]
        from: Chain,
        /// Destination chain
        #[arg(long)]
        to: Chain,
        /// Right ID to transfer (hex)
        #[arg(long)]
        right_id: String,
        /// Destination owner address (hex)
        #[arg(long)]
        dest_owner: Option<String>,
        /// Run using simulated providers (demo mode, not explorer-verifiable)
        #[arg(long, action = ArgAction::SetTrue)]
        simulation: bool,
    },
    /// Check transfer status
    Status {
        /// Transfer ID (hex)
        transfer_id: String,
    },
    /// List all transfers
    List {
        /// Filter by source chain
        #[arg(long, value_enum)]
        from: Option<Chain>,
        /// Filter by destination chain
        #[arg(long, value_enum)]
        to: Option<Chain>,
    },
    /// Retry a failed transfer
    Retry {
        /// Transfer ID (hex)
        transfer_id: String,
    },
}

pub fn execute(action: CrossChainAction, config: &Config, state: &mut UnifiedStateManager) -> Result<()> {
    match action {
        CrossChainAction::Transfer {
            from,
            to,
            right_id,
            dest_owner,
            simulation,
        } => transfer::cmd_transfer(from, to, right_id, dest_owner, simulation, config, state),
        CrossChainAction::Status { transfer_id } => status::cmd_status(transfer_id, state),
        CrossChainAction::List { from, to } => status::cmd_list(from, to, state),
        CrossChainAction::Retry { transfer_id } => status::cmd_retry(transfer_id, config, state),
    }
}
