//! PostgreSQL-backed storage for runtime coordination.
//!
//! This module provides PostgreSQL implementations for runtime coordination
//! components that require distributed consistency, including:
//! - Transfer lease coordination with FOR UPDATE SKIP LOCKED
//! - Durable event sourcing
//! - Replay registry with atomic operations
//!
//! SQLite is no longer acceptable for runtime coordination in production.

#[cfg(feature = "postgres")]
use sqlx::postgres::PgPoolOptions;
#[cfg(feature = "postgres")]
use sqlx::{PgPool, Row};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::lease::TransferLease;
use crate::event_envelope::RuntimeEventEnvelope;
use csv_core::replay_record::GlobalReplayRecord;
use csv_core::sanad::SanadId;
use csv_core::hash::Hash;
use csv_core::protocol_version::ChainId;
use uuid::Uuid;

/// PostgreSQL-backed lease coordination store.
#[cfg(feature = "postgres")]
pub struct PostgresLeaseStore {
    pool: PgPool,
}

#[cfg(feature = "postgres")]
impl PostgresLeaseStore {
    /// Create a new PostgreSQL lease store.
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;

        // Run migrations to create tables
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS transfer_leases (
                transfer_id BYTEA PRIMARY KEY,
                epoch BIGINT NOT NULL,
                owner_runtime_id UUID NOT NULL,
                acquired_at TIMESTAMPTZ NOT NULL,
                expires_at TIMESTAMPTZ NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_transfer_leases_expires_at ON transfer_leases(expires_at);
            "#,
        )
        .execute(&pool)
        .await?;

        let pool_for_struct = pool.clone();
        Ok(Self { pool: pool_for_struct })
    }

    /// Acquire a lease for a transfer using FOR UPDATE SKIP LOCKED.
    ///
    /// This provides distributed lease coordination across multiple runtime instances.
    pub async fn acquire_lease(
        &self,
        transfer_id: SanadId,
        runtime_id: Uuid,
        ttl_secs: u64,
    ) -> Result<TransferLease, sqlx::Error> {
        let now = SystemTime::now();
        let expires_at = now
            .checked_add(std::time::Duration::from_secs(ttl_secs))
            .unwrap_or(now);

        // Convert SystemTime to i64 timestamp for PostgreSQL
        let acquired_ts = now.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;
        let expires_ts = expires_at.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;

        // Try to acquire lease with FOR UPDATE SKIP LOCKED
        let result = sqlx::query(
            r#"
            INSERT INTO transfer_leases (transfer_id, epoch, owner_runtime_id, acquired_at, expires_at)
            VALUES ($1, 1, $2, to_timestamp($3), to_timestamp($4))
            ON CONFLICT (transfer_id) DO NOTHING
            RETURNING transfer_id, epoch, owner_runtime_id, acquired_at, expires_at
            "#,
        )
        .bind(transfer_id.as_bytes())
        .bind(runtime_id)
        .bind(acquired_ts)
        .bind(expires_ts)
        .fetch_optional(&self.pool)
        .await?;

        match result {
            Some(row) => {
                // Lease acquired successfully
                Ok(TransferLease {
                    transfer_id: SanadId::new(row.get("transfer_id")),
                    epoch: row.get::<i64, _>("epoch") as u64,
                    owner_runtime_id: row.get("owner_runtime_id"),
                    acquired_at: UNIX_EPOCH + std::time::Duration::from_secs(row.get::<chrono::DateTime<chrono::Utc>, _>("acquired_at").timestamp() as u64),
                    expires_at: UNIX_EPOCH + std::time::Duration::from_secs(row.get::<chrono::DateTime<chrono::Utc>, _>("expires_at").timestamp() as u64),
                })
            }
            None => {
                // Lease already exists, try to acquire with SKIP LOCKED
                let row = sqlx::query(
                    r#"
                    SELECT transfer_id, epoch, owner_runtime_id, acquired_at, expires_at
                    FROM transfer_leases
                    WHERE transfer_id = $1 AND expires_at > NOW()
                    FOR UPDATE SKIP LOCKED
                    LIMIT 1
                    "#,
                )
                .bind(transfer_id.as_bytes())
                .fetch_one(&self.pool)
                .await?;

                // Check if we own this lease
                let owner: Uuid = row.get("owner_runtime_id");
                if owner == runtime_id {
                    Ok(TransferLease {
                        transfer_id: SanadId::new(row.get("transfer_id")),
                        epoch: row.get::<i64, _>("epoch") as u64,
                        owner_runtime_id: row.get("owner_runtime_id"),
                        acquired_at: UNIX_EPOCH + std::time::Duration::from_secs(row.get::<chrono::DateTime<chrono::Utc>, _>("acquired_at").timestamp() as u64),
                        expires_at: UNIX_EPOCH + std::time::Duration::from_secs(row.get::<chrono::DateTime<chrono::Utc>, _>("expires_at").timestamp() as u64),
                    })
                } else {
                    Err(sqlx::Error::RowNotFound)
                }
            }
        }
    }

    /// Renew an existing lease.
    pub async fn renew_lease(
        &self,
        transfer_id: SanadId,
        runtime_id: Uuid,
        ttl_secs: u64,
    ) -> Result<TransferLease, sqlx::Error> {
        let expires_at = SystemTime::now()
            .checked_add(std::time::Duration::from_secs(ttl_secs))
            .unwrap_or(SystemTime::now());
        let expires_ts = expires_at.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64;

        sqlx::query(
            r#"
            UPDATE transfer_leases
            SET expires_at = to_timestamp($1), epoch = epoch + 1
            WHERE transfer_id = $2 AND owner_runtime_id = $3
            RETURNING transfer_id, epoch, owner_runtime_id, acquired_at, expires_at
            "#,
        )
        .bind(expires_ts)
        .bind(transfer_id.as_bytes())
        .bind(runtime_id)
        .fetch_one(&self.pool)
        .await
        .map(|row| TransferLease {
            transfer_id: SanadId::new(row.get("transfer_id")),
            epoch: row.get::<i64, _>("epoch") as u64,
            owner_runtime_id: row.get("owner_runtime_id"),
            acquired_at: UNIX_EPOCH + std::time::Duration::from_secs(row.get::<chrono::DateTime<chrono::Utc>, _>("acquired_at").timestamp() as u64),
            expires_at: UNIX_EPOCH + std::time::Duration::from_secs(row.get::<chrono::DateTime<chrono::Utc>, _>("expires_at").timestamp() as u64),
        })
    }

    /// Release a lease.
    pub async fn release_lease(
        &self,
        transfer_id: SanadId,
        runtime_id: Uuid,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE FROM transfer_leases
            WHERE transfer_id = $1 AND owner_runtime_id = $2
            "#,
        )
        .bind(transfer_id.as_bytes())
        .bind(runtime_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

/// PostgreSQL-backed event store for durable event sourcing.
#[cfg(feature = "postgres")]
pub struct PostgresEventStore {
    pool: PgPool,
}

#[cfg(feature = "postgres")]
impl PostgresEventStore {
    /// Create a new PostgreSQL event store.
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;

        // Create events table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS runtime_events (
                event_id UUID PRIMARY KEY,
                transfer_id BYTEA NOT NULL,
                causation_id UUID,
                correlation_id UUID NOT NULL,
                event TEXT NOT NULL,
                timestamp TIMESTAMPTZ NOT NULL,
                runtime_id UUID NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_runtime_events_transfer_id ON runtime_events(transfer_id);
            CREATE INDEX IF NOT EXISTS idx_runtime_events_correlation_id ON runtime_events(correlation_id);
            CREATE INDEX IF NOT EXISTS idx_runtime_events_timestamp ON runtime_events(timestamp);
            "#,
        )
        .execute(&pool)
        .await?;

        let pool_for_struct = pool.clone();
        Ok(Self { pool: pool_for_struct })
    }

    /// Persist an event before publishing.
    ///
    /// This ensures events are durable before they are published to the bus.
    pub async fn persist_event(&self, envelope: &RuntimeEventEnvelope) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            INSERT INTO runtime_events (event_id, transfer_id, causation_id, correlation_id, event, timestamp, runtime_id)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(envelope.event_id)
        .bind(envelope.transfer_id.as_bytes())
        .bind(envelope.causation_id)
        .bind(envelope.correlation_id)
        .bind(&envelope.event)
        .bind(chrono::DateTime::<chrono::Utc>::from(envelope.timestamp))
        .bind(envelope.runtime_id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Load events for a transfer in chronological order.
    pub async fn load_events_for_transfer(
        &self,
        transfer_id: SanadId,
    ) -> Result<Vec<RuntimeEventEnvelope>, sqlx::Error> {
        let rows = sqlx::query(
            r#"
            SELECT event_id, transfer_id, causation_id, correlation_id, event, timestamp, runtime_id
            FROM runtime_events
            WHERE transfer_id = $1
            ORDER BY timestamp ASC
            "#,
        )
        .bind(transfer_id.as_bytes())
        .fetch_all(&self.pool)
        .await?;

        rows
            .iter()
            .map(|row| Ok(RuntimeEventEnvelope {
                event_id: row.get("event_id"),
                transfer_id: SanadId::new(row.get("transfer_id")),
                causation_id: row.get("causation_id"),
                correlation_id: row.get("correlation_id"),
                event: row.get("event"),
                timestamp: row.get::<chrono::DateTime<chrono::Utc>, _>("timestamp").into(),
                runtime_id: row.get("runtime_id"),
            }))
            .collect()
    }
}

/// PostgreSQL-backed replay registry with atomic operations.
#[cfg(feature = "postgres")]
pub struct PostgresReplayRegistry {
    pool: PgPool,
}

#[cfg(feature = "postgres")]
impl PostgresReplayRegistry {
    /// Create a new PostgreSQL replay registry.
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;

        // Create replay records table
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS global_replay_records (
                seal_id BYTEA PRIMARY KEY,
                originating_chain TEXT NOT NULL,
                consumed_by_transfer BYTEA NOT NULL,
                consumption_proof_hash BYTEA NOT NULL,
                state TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_global_replay_records_state ON global_replay_records(state);
            "#,
        )
        .execute(&pool)
        .await?;

        let pool_for_struct = pool.clone();
        Ok(Self { pool: pool_for_struct })
    }

    /// Idempotent consume-if-unconsumed operation.
    pub async fn consume_if_unconsumed(
        &self,
        seal_id: Hash,
        transfer_id: SanadId,
        proof_hash: Hash,
        originating_chain: ChainId,
    ) -> Result<bool, sqlx::Error> {
        // Use INSERT ... ON CONFLICT to implement atomic consume-if-unconsumed
        let result = sqlx::query(
            r#"
            INSERT INTO global_replay_records (seal_id, originating_chain, consumed_by_transfer, consumption_proof_hash, state)
            VALUES ($1, $2, $3, $4, 'Finalized')
            ON CONFLICT (seal_id) DO NOTHING
            RETURNING seal_id
            "#,
        )
        .bind(seal_id.as_bytes())
        .bind(originating_chain.as_str())
        .bind(transfer_id.as_bytes())
        .bind(proof_hash.as_bytes())
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.is_some())
    }

    /// Rollback a consumption atomically.
    pub async fn rollback_consumption(&self, seal_id: Hash) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE global_replay_records
            SET state = 'RolledBack'
            WHERE seal_id = $1 AND state = 'Finalized'
            RETURNING seal_id
            "#,
        )
        .bind(seal_id.as_bytes())
        .fetch_optional(&self.pool)
        .await?;

        Ok(result.is_some())
    }

    /// Get the current state of a seal.
    pub async fn get_record(&self, seal_id: Hash) -> Result<Option<GlobalReplayRecord>, sqlx::Error> {
        let row = sqlx::query(
            r#"
            SELECT seal_id, originating_chain, consumed_by_transfer, consumption_proof_hash, state
            FROM global_replay_records
            WHERE seal_id = $1
            "#,
        )
        .bind(seal_id.as_bytes())
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => {
                let state_str: String = row.get("state");
                let state = match state_str.as_str() {
                    "Pending" => csv_core::replay_record::ReplayState::Pending,
                    "Finalized" => csv_core::replay_record::ReplayState::Finalized,
                    "RolledBack" => csv_core::replay_record::ReplayState::RolledBack,
                    "Tombstoned" => csv_core::replay_record::ReplayState::Tombstoned,
                    _ => return Err(sqlx::Error::RowNotFound),
                };

                Ok(Some(GlobalReplayRecord {
                    seal_id: SanadId::new(row.get("seal_id")),
                    originating_chain: ChainId::new(row.get("originating_chain")),
                    consumed_by_transfer: SanadId::new(row.get("consumed_by_transfer")),
                    consumption_proof_hash: Hash::new(row.get("consumption_proof_hash")),
                    state,
                }))
            }
            None => Ok(None),
        }
    }
}
