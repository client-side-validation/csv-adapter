//! Wallet context - Re-export from context/wallet.rs during transition.
//!
//! This module re-exports the wallet context from its existing location.
//! During the full consolidation, the implementation will be moved here.

#[allow(unused_imports)]
pub use crate::context::wallet::{WalletContext, WalletProvider, use_wallet_context};

// Re-export types that the context depends on
#[allow(unused_imports)]
pub use crate::context::state::AppState;
#[allow(unused_imports)]
pub use crate::context::types::*;
