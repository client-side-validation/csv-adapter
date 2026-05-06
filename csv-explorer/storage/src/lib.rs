/// Storage layer for the CSV Explorer.
///
/// Provides a typed repository pattern over SQLite for all indexed data,
/// including sanads, transfers, seals, contracts, sync progress, and statistics.
pub mod db;
pub mod repositories;

pub use db::{close_pool, init_pool};
