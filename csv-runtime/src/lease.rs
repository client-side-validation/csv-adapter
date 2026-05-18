//! Transfer lease primitives for runtime ownership
use core::fmt::Debug;
use std::time::SystemTime;

use uuid::Uuid;

use csv_core::SanadId as TransferId;

/// Runtime instance identifier
pub type RuntimeId = Uuid;

/// Transfer execution lease — single-owner execution guard
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct TransferLease {
    /// Transfer identifier this lease protects.
    pub transfer_id: TransferId,
    /// Epoch counter to prevent stale lease adoption.
    pub epoch: u64,
    /// Owning runtime instance identifier.
    pub owner_runtime_id: RuntimeId,
    /// Time when the lease was acquired.
    pub acquired_at: SystemTime,
    /// Expiration time for the lease.
    pub expires_at: SystemTime,
}

/// Execution context passed to mutating runtime operations.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RuntimeExecutionContext {
    /// Lease that authorizes execution for a transfer.
    pub lease: TransferLease,
    /// Runtime instance performing the operation.
    pub runtime_instance: RuntimeId,
}

impl TransferLease {
    /// Returns true if the lease is currently active relative to `now`.
    pub fn is_active(&self, now: SystemTime) -> bool {
        now < self.expires_at
    }
}
