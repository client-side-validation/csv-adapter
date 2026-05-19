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
pub mod adversarial;
pub mod config;
pub mod deployment_profile;
pub mod error;
pub mod event_bus;
pub mod event_envelope;
pub mod event_store;
pub mod lease;
pub mod policy;
pub mod replay_db;
pub mod runtime_mode;
pub mod transfer_coordinator;

// Re-exports
pub use adapter_registry::{AdapterRegistryImpl, ChainAdapter};
pub use adversarial::{AdversarialConfig, AdversarialTestRunner, ConcurrentExecutor, HAFailoverScenario, RaceConditionScenario, RaceOutcome, SimulatedReorg};
pub use config::{CircuitBreakerConfig, ConfigValidationError, LeaseConfig, OperationalConfig, RetryConfig, RpcConfig, TimeoutConfig};
pub use deployment_profile::DeploymentProfile;
pub use error::{RuntimeError, TransferCoordinatorError};
pub use event_bus::{EventBus, TransferEvent};
pub use lease::{
    DEFAULT_LEASE_DURATION_SECS, LeaseValidationError, RuntimeExecutionContext, RuntimeId,
    TransferLease, MAX_LEASE_DURATION_SECS,
};
pub use policy::RuntimePolicy;
pub use replay_db::{ReplayDatabase, ReplayDbError, ReplayEntryState};
pub use runtime_mode::{CircuitBreaker, CircuitBreakerConfig as RuntimeCircuitBreakerConfig, CircuitBreakerState, HealthMonitor, HealthStatus, RuntimeMode};
pub use transfer_coordinator::TransferCoordinator;
