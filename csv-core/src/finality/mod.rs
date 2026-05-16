//! Finality State Model
//!
//! This module provides a structured approach to defining and monitoring
//! different levels of transaction finality across chains.

pub mod monitor;
pub mod policy;
pub mod state;

// Re-exports
pub use monitor::FinalityMonitor;
pub use policy::{ChainFinalityPolicy, FinalityThreshold};
pub use state::{FinalityState, FinalityStatus};
