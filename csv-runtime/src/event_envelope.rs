//! Durable runtime event envelope
use uuid::Uuid;
use std::time::SystemTime;

use csv_core::SanadId as TransferId;

/// Durable runtime event envelope persisted before publishing to the bus.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct RuntimeEventEnvelope {
    /// Unique event identifier.
    pub event_id: Uuid,
    /// Associated transfer identifier.
    pub transfer_id: TransferId,
    /// Optional causation event id.
    pub causation_id: Option<Uuid>,
    /// Correlation id for grouping related events.
    pub correlation_id: Uuid,
    /// Serialized event payload (opaque JSON or binary encoded string).
    pub event: String,
    /// Timestamp when the event was generated.
    pub timestamp: SystemTime,
    /// Runtime instance that generated the event.
    pub runtime_id: Uuid,
}
