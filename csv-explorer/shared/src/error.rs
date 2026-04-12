/// Error types for the CSV Explorer.
///
/// All error types implement `std::error::Error` and `thiserror::Error`
/// for proper error propagation and reporting.

use thiserror::Error;

/// Top-level error type for the explorer.
#[derive(Error, Debug)]
pub enum ExplorerError {
    // ------------------------------------------------------------------
    // I/O and configuration
    // ------------------------------------------------------------------
    /// File system or I/O error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// TOML parsing or serialization error.
    #[error("TOML error: {0}")]
    Toml(#[from] toml::de::Error),

    /// JSON serialization/deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    // ------------------------------------------------------------------
    // Database
    // ------------------------------------------------------------------
    /// Database connection or query error.
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    /// Database migration error.
    #[error("Migration error: {0}")]
    Migration(String),

    /// Entity not found.
    #[error("Entity not found: {entity_type} with id {id}")]
    NotFound {
        entity_type: String,
        id: String,
    },

    // ------------------------------------------------------------------
    // RPC / network
    // ------------------------------------------------------------------
    /// HTTP request error.
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// RPC call to a chain failed.
    #[error("RPC error on chain {chain}: {message}")]
    RpcError {
        chain: String,
        message: String,
    },

    /// Failed to parse a response from a chain RPC.
    #[error("RPC parse error on chain {chain}: {message}")]
    RpcParseError {
        chain: String,
        message: String,
    },

    // ------------------------------------------------------------------
    // Indexer
    // ------------------------------------------------------------------
    /// Indexer was asked to stop.
    #[error("Indexer stopped")]
    IndexerStopped,

    /// Block processing error.
    #[error("Block processing error on chain {chain} at block {block}: {message}")]
    BlockError {
        chain: String,
        block: u64,
        message: String,
    },

    /// Chain reorganization detected (informational, not fatal).
    #[error("Chain reorg detected on {chain} at block {block}, depth: {depth}")]
    ChainReorg {
        chain: String,
        block: u64,
        depth: u64,
    },

    // ------------------------------------------------------------------
    // API
    // ------------------------------------------------------------------
    /// GraphQL query/mutation error.
    #[error("GraphQL error: {0}")]
    GraphQL(String),

    /// HTTP server error.
    #[error("HTTP server error: {0}")]
    HttpServer(#[from] std::io::Error),

    // ------------------------------------------------------------------
    // Encoding
    // ------------------------------------------------------------------
    /// Hex decoding error.
    #[error("Hex decode error: {0}")]
    Hex(#[from] hex::FromHexError),

    /// Generic parse error.
    #[error("Parse error: {0}")]
    Parse(String),

    // ------------------------------------------------------------------
    // Generic
    // ------------------------------------------------------------------
    /// Generic anyhow-compatible error for wrapping other errors.
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias using `ExplorerError`.
pub type Result<T> = std::result::Result<T, ExplorerError>;
