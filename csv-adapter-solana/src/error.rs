//! Error types for Solana adapter

use thiserror::Error;

/// Solana-specific errors
#[derive(Debug, Error)]
pub enum SolanaError {
    /// RPC client error
    #[error("RPC error: {0}")]
    Rpc(String),

    /// Transaction error
    #[error("Transaction error: {0}")]
    Transaction(String),

    /// Account not found
    #[error("Account not found: {0}")]
    AccountNotFound(String),

    /// Invalid program ID
    #[error("Invalid program ID: {0}")]
    InvalidProgramId(String),

    /// Invalid instruction
    #[error("Invalid instruction: {0}")]
    InvalidInstruction(String),

    /// Insufficient funds
    #[error("Insufficient funds: required {required}, available {available}")]
    InsufficientFunds { required: u64, available: u64 },

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// Deserialization error
    #[error("Deserialization error: {0}")]
    Deserialization(String),

    /// Keypair error
    #[error("Keypair error: {0}")]
    Keypair(String),

    /// Wallet error
    #[error("Wallet error: {0}")]
    Wallet(String),

    /// Commitment error
    #[error("Commitment error: {0}")]
    Commitment(String),

    /// Seal creation error
    #[error("Seal creation error: {0}")]
    SealCreation(String),

    /// Anchor creation error
    #[error("Anchor creation error: {0}")]
    AnchorCreation(String),

    /// Proof generation error
    #[error("Proof generation error: {0}")]
    ProofGeneration(String),

    /// Invalid input
    #[error("Invalid input: {0}")]
    InvalidInput(String),
}

/// Result type for Solana operations
pub type SolanaResult<T> = std::result::Result<T, SolanaError>;

impl From<solana_sdk::transport::TransportError> for SolanaError {
    fn from(err: solana_sdk::transport::TransportError) -> Self {
        Self::Rpc(format!("Transport error: {}", err))
    }
}

impl From<solana_sdk::program_error::ProgramError> for SolanaError {
    fn from(err: solana_sdk::program_error::ProgramError) -> Self {
        Self::Transaction(format!("Program error: {}", err))
    }
}

impl From<bs58::decode::Error> for SolanaError {
    fn from(err: bs58::decode::Error) -> Self {
        Self::Keypair(format!("Base58 decode error: {}", err))
    }
}

impl From<serde_json::Error> for SolanaError {
    fn from(err: serde_json::Error) -> Self {
        Self::Serialization(format!("JSON error: {}", err))
    }
}

impl From<ed25519_dalek::ed25519::Error> for SolanaError {
    fn from(err: ed25519_dalek::ed25519::Error) -> Self {
        Self::Keypair(format!("Ed25519 error: {}", err))
    }
}

impl From<SolanaError> for csv_adapter_core::error::AdapterError {
    fn from(err: SolanaError) -> Self {
        csv_adapter_core::error::AdapterError::NetworkError(format!("Solana: {}", err))
    }
}
