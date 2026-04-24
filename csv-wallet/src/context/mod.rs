//! Application context and state management - modular structure.

pub mod types;
pub mod state;
pub mod wallet;
pub mod utils;

// Re-export all types from types module
pub use types::*;

// Re-export AppState from state module
pub use state::AppState;

// Re-export WalletContext and related items from wallet module
pub use wallet::{WalletContext, WalletProvider, use_wallet_context};

// Re-export utility functions
pub use utils::generate_id;
