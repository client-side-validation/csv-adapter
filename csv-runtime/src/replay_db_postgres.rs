//! PostgreSQL-backed replay database with advisory-lock CAS semantics.
//!
//! Uses `INSERT ... ON CONFLICT DO NOTHING RETURNING` to get true
//! server-side compare-and-swap semantics across multiple coordinator processes.

use csv_core::proof::ReplayId;
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Row};

use crate::error::RuntimeError;
use crate::replay_db::{ReplayDbError, ReplayDatabase, ReplayEntryState};

/// PostgreSQL-backed replay database with advisory-lock CAS.
///
/// Uses `INSERT ... ON CONFLICT DO NOTHING RETURNING` to get true
/// server-side CAS semantics across multiple coordinator processes.
#[cfg(feature = "postgres")]
pub struct PostgresReplayDb {
    pool: PgPool,
}

#[cfg(feature = "postgres")]
impl PostgresReplayDb {
    /// Create a new PostgreSQL replay database.
    ///
    /// Creates the replay_entries table if it does not exist.
    pub async fn new(database_url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .connect(database_url)
            .await?;

        // Create table if not exists (idempotent)
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS replay_entries (
                replay_id BYTEA PRIMARY KEY,
                state TEXT NOT NULL CHECK (state IN ('Pending', 'Consumed', 'RolledBack')),
                created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
                updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
            );
            "#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { pool })
    }

    fn state_to_str(state: ReplayEntryState) -> &'static str {
        match state {
            ReplayEntryState::Pending => "Pending",
            ReplayEntryState::Consumed => "Consumed",
            ReplayEntryState::RolledBack => "RolledBack",
        }
    }

    fn state_from_str(s: &str) -> Result<ReplayEntryState, sqlx::Error> {
        match s {
            "Pending" => Ok(ReplayEntryState::Pending),
            "Consumed" => Ok(ReplayEntryState::Consumed),
            "RolledBack" => Ok(ReplayEntryState::RolledBack),
            _ => Err(sqlx::Error::RowNotFound),
        }
    }
}

#[cfg(feature = "postgres")]
#[async_trait::async_trait]
impl ReplayDatabase for PostgresReplayDb {
    async fn contains(&self, id: &ReplayId) -> Result<bool, RuntimeError> {
        let result = sqlx::query("SELECT 1 FROM replay_entries WHERE replay_id = $1 LIMIT 1")
            .bind(id.as_bytes())
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| RuntimeError::Storage(format!("PostgreSQL query failed: {}", e)))?;

        Ok(result.is_some())
    }

    async fn insert_if_absent(
        &self,
        id: &ReplayId,
        state: ReplayEntryState,
    ) -> Result<(), ReplayDbError> {
        let state_str = Self::state_to_str(state);
        let created_ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        // True CAS: ON CONFLICT DO NOTHING ensures only one writer succeeds
        let result = sqlx::query(
            r#"
            INSERT INTO replay_entries (replay_id, state, created_at, updated_at)
            VALUES ($1, $2, to_timestamp($3), to_timestamp($3))
            ON CONFLICT (replay_id) DO NOTHING
            RETURNING replay_id
            "#,
        )
        .bind(id.as_bytes())
        .bind(state_str)
        .bind(created_ts)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ReplayDbError::Storage(format!("PostgreSQL insert failed: {}", e)))?;

        match result {
            Some(_) => Ok(()), // Insert succeeded
            None => Err(ReplayDbError::AlreadyExists), // Key already exists
        }
    }

    async fn consume_if_unconsumed(&self, id: &ReplayId) -> Result<(), ReplayDbError> {
        // Try to insert as Pending (CAS via ON CONFLICT DO NOTHING)
        let created_ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64;

        let result = sqlx::query(
            r#"
            INSERT INTO replay_entries (replay_id, state, created_at, updated_at)
            VALUES ($1, 'Pending', to_timestamp($2), to_timestamp($2))
            ON CONFLICT (replay_id) DO NOTHING
            RETURNING state
            "#,
        )
        .bind(id.as_bytes())
        .bind(created_ts)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ReplayDbError::Storage(format!("PostgreSQL query failed: {}", e)))?;

        match result {
            Some(_row) => {
                // Insert succeeded (entry was absent)
                Ok(())
            }
            None => {
                // Entry exists — check its state
                let row = sqlx::query("SELECT state FROM replay_entries WHERE replay_id = $1")
                    .bind(id.as_bytes())
                    .fetch_one(&self.pool)
                    .await
                    .map_err(|e| ReplayDbError::Storage(format!("PostgreSQL query failed: {}", e)))?;

                let state = Self::state_from_str(&row.get::<String, _>("state"))
                    .map_err(|e| ReplayDbError::Storage(format!("Invalid state in database: {}", e)))?;

                match state {
                    ReplayEntryState::Consumed => Ok(()), // Idempotent
                    ReplayEntryState::Pending | ReplayEntryState::RolledBack => {
                        Err(ReplayDbError::AlreadyExists)
                    }
                }
            }
        }
    }

    async fn confirm_consumed(&self, id: &ReplayId) -> Result<(), ReplayDbError> {
        // Atomic update: only succeed if currently Pending
        let result = sqlx::query(
            r#"
            UPDATE replay_entries
            SET state = 'Consumed', updated_at = now()
            WHERE replay_id = $1 AND state = 'Pending'
            RETURNING state
            "#,
        )
        .bind(id.as_bytes())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ReplayDbError::Storage(format!("PostgreSQL update failed: {}", e)))?;

        match result {
            Some(_) => Ok(()), // Promoted from Pending to Consumed
            None => {
                // Check if already Consumed (idempotent) or invalid state
                let row = sqlx::query("SELECT state FROM replay_entries WHERE replay_id = $1")
                    .bind(id.as_bytes())
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(|e| ReplayDbError::Storage(format!("PostgreSQL query failed: {}", e)))?;

                match row {
                    Some(r) => {
                        let state = Self::state_from_str(&r.get::<String, _>("state"))
                            .map_err(|e| ReplayDbError::Storage(format!("Invalid state: {}", e)))?;
                        match state {
                            ReplayEntryState::Consumed => Ok(()), // Already consumed
                            ReplayEntryState::RolledBack => Err(ReplayDbError::Storage(
                                "Entry is in RolledBack state, cannot confirm consumed".to_string(),
                            )),
                            ReplayEntryState::Pending => Err(ReplayDbError::Storage(
                                "Entry was not in Pending state".to_string(),
                            )),
                        }
                    }
                    None => Err(ReplayDbError::Storage("Entry not found".to_string())),
                }
            }
        }
    }

    async fn mark_rolled_back(&self, id: &ReplayId) -> Result<(), RuntimeError> {
        // Atomic update: only succeed if currently Pending
        let result = sqlx::query(
            r#"
            UPDATE replay_entries
            SET state = 'RolledBack', updated_at = now()
            WHERE replay_id = $1 AND state = 'Pending'
            RETURNING state
            "#,
        )
        .bind(id.as_bytes())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RuntimeError::Storage(format!("PostgreSQL update failed: {}", e)))?;

        match result {
            Some(_) => Ok(()),
            None => {
                // Check current state for better error message
                let row = sqlx::query("SELECT state FROM replay_entries WHERE replay_id = $1")
                    .bind(id.as_bytes())
                    .fetch_optional(&self.pool)
                    .await
                    .map_err(|e| RuntimeError::Storage(format!("PostgreSQL query failed: {}", e)))?;

                match row {
                    Some(r) => {
                        let state = Self::state_from_str(&r.get::<String, _>("state"))
                            .map_err(|e| RuntimeError::Storage(format!("Invalid state: {}", e)))?;
                        match state {
                            ReplayEntryState::Consumed => Err(RuntimeError::InvalidState(
                                "Entry is already Consumed, cannot roll back".to_string(),
                            )),
                            ReplayEntryState::RolledBack => Ok(()), // Idempotent
                            ReplayEntryState::Pending => Err(RuntimeError::InvalidState(
                                "Entry was not in Pending state".to_string(),
                            )),
                        }
                    }
                    None => Err(RuntimeError::TransferNotFound(format!(
                        "ReplayId {:?} not found",
                        id.as_bytes()
                    ))),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_replay_id() -> ReplayId {
        ReplayId::derive(
            "bitcoin",
            &[1u8; 32],
            0,
            &[2u8; 32],
            &[3u8; 32],
            "ethereum",
        )
    }

    /// Integration test — requires a running PostgreSQL instance.
    /// Skip by default; run with `cargo test --features postgres -- --ignored`
    #[tokio::test]
    #[ignore]
    async fn test_postgres_replay_db() {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgresql://localhost/replay_test".to_string());

        let db = PostgresReplayDb::new(&database_url).await.unwrap();
        let id = test_replay_id();

        // contains should return false initially
        assert!(!db.contains(&id).await.unwrap());

        // insert_if_absent should succeed
        db.insert_if_absent(&id, ReplayEntryState::Pending)
            .await
            .unwrap();

        // contains should return true
        assert!(db.contains(&id).await.unwrap());

        // insert_if_absent should fail for duplicate
        let result = db.insert_if_absent(&id, ReplayEntryState::Pending).await;
        assert!(matches!(result, Err(ReplayDbError::AlreadyExists)));

        // confirm_consumed should succeed
        db.confirm_consumed(&id).await.unwrap();

        // consume_if_unconsumed should be idempotent (already consumed)
        db.consume_if_unconsumed(&id).await.unwrap();
    }
}
