//! Dioxus hooks for state management.

mod use_balance;
mod use_network;
mod use_wallet;
mod use_wallet_connection;

// Re-export from use_balance - REAL IMPLEMENTATION
#[allow(unused_imports)]
pub use use_balance::{
    AccountBalance, BalanceContext, BalanceProvider, chain_symbol, format_balance_display,
    use_balance,
};

// Re-export from use_network
#[allow(unused_imports)]
pub use use_network::{NetworkContext, NetworkProvider, use_network};

// Re-export from use_wallet
#[allow(unused_imports)]
pub use use_wallet::{WalletContext, WalletProvider, use_wallet};

// Re-export from use_wallet_connection
#[allow(unused_imports)]
pub use use_wallet_connection::{
    WalletConnectButton, WalletConnectionContext, WalletConnectionProvider, use_wallet_connection,
};
