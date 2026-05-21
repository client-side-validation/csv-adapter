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

#[cfg(all(target_arch = "wasm32", feature = "persistent"))]
compile_error!(
    "The 'persistent' feature requires native I/O (rocksdb) and is incompatible with wasm32. \
     Disable the 'persistent' feature for wasm32 builds."
);

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
#[cfg(feature = "persistent")]
pub mod replay_db_rocksdb;
#[cfg(feature = "postgres")]
pub mod replay_db_postgres;
#[cfg(feature = "postgres")]
pub mod coordinator_lease_postgres;
pub mod runtime_mode;
pub mod transfer_coordinator;
pub mod coordinator_lease;

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

// Coordinator lease re-exports
pub use coordinator_lease::{
    CoordinatorId, CoordinatorLease, InMemoryLease, LeaseError, LeaseGuard, MintCoordinator,
    MintProvider, MintReceipt,
};

#[cfg(feature = "postgres")]
pub use coordinator_lease_postgres::PostgresCoordinatorLease;
