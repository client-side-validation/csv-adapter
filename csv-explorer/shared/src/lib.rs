/// Shared types and configuration for the CSV Explorer.
///
/// This crate contains all the data types shared across the indexer,
/// storage, API, and UI crates.

pub mod config;
pub mod error;
pub mod types;

// Re-export commonly used types at the crate root for convenience.
pub use config::ExplorerConfig;
pub use error::{ExplorerError, Result};
pub use types::*;
