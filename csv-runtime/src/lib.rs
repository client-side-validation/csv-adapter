//! csv-runtime — CSV Protocol orchestration engine
//!
//! This crate provides the runtime layer for cross-chain transfer execution.
//! It depends only on csv-core — no chain adapter imports.
//!
//! ## Architecture
//!
//! - **TransferCoordinator**: Single source of truth for transfer execution
//! - **ReplayDatabase**: Durable replay prevention with compare-and-swap semantics
//! - **AdapterRegistry**: Dependency injection for chain adapters
//! - **EventBus**: Structured events for observability

#![warn(missing_docs)]

pub mod adapter_registry;
pub mod deployment_profile;
pub mod error;
pub mod event_bus;
pub mod replay_db;
pub mod transfer_coordinator;

// Re-exports
pub use adapter_registry::{AdapterRegistryImpl, ChainAdapter};
pub use deployment_profile::DeploymentProfile;
pub use error::{RuntimeError, TransferCoordinatorError};
pub use event_bus::{EventBus, TransferEvent};
pub use replay_db::{ReplayDatabase, ReplayDbError, ReplayEntryState};
pub use transfer_coordinator::TransferCoordinator;
