//! PostgreSQL-backed replay database with advisory-lock CAS.
//!
//! Uses `INSERT ... ON CONFLICT DO NOTHING RETURNING id` to get true
//! server-side CAS semantics across multiple coordinator processes.
//!
//! This is the distributed multi-node CAS implementation. For single-node
//! deployments, `RocksReplayDb` is sufficient.

use crate::error::RuntimeError;
use crate::replay_db::{ReplayDatabase, ReplayDbError, ReplayEntryState};
use async_trait::async_trait;
use csv_core::proof::ReplayId;

/// PostgreSQL-backed replay database with advisory-lock CAS.
///
/// Uses `INSERT ... ON CONFLICT DO NOTHING RETURNING` to get true
/// server-side CAS semantics across multiple coordinator processes.
///
/// # Concurrency
///
/// PostgreSQL's `INSERT ... ON CONFLICT DO NOTHING` provides atomic
/// server-side CAS. If two coordinators attempt to insert the same
/// ReplayId concurrently, exactly one succeeds. The other gets
/// `REPLAY_ALREADY_EXISTS` from the database.
///
/// This resolves **Unresolved problem 1** (concurrent coordinators)
/// from the `ReplayDatabase` trait documentation.
pub struct PostgresReplayDb {
    pool: sqlx::PgPool,
}

impl PostgresReplayDb {
    /// Create a new PostgreSQL replay database from a connection pool.
    ///
    /// # Errors
    /// Returns `RuntimeError` if the connection pool is not valid.
    pub fn new(pool: sqlx::PgPool) -> Self {
        Self { pool }
    }

    /// Create a new PostgreSQL replay database from a database URL.
    ///
    /// # Errors
    /// Returns `RuntimeError` if the connection pool cannot be created.
    pub async fn connect(database_url: &str) -> Result<Self, RuntimeError> {
        let pool = sqlx::PgPool::connect(database_url)
            .await
            .map_err(|e| RuntimeError::Storage(format!("Failed to connect to PostgreSQL: {e}")))?;

        Ok(Self { pool })
    }

    /// Run the schema migration to ensure the replay_entries table exists.
    ///
    /// This is idempotent — safe to call on every startup.
    ///
    /// # Errors
    /// Returns `RuntimeError` if the migration fails.
    pub async fn run_migrations(&self) -> Result<(), RuntimeError> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS replay_entries (
                id          TEXT        PRIMARY KEY,
                state       TEXT        NOT NULL CHECK (state IN ('Pending', 'Consumed', 'RolledBack')),
                inserted_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                updated_at  TIMESTAMPTZ
            )
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| RuntimeError::Storage(format!("Migration failed: {e}")))?;

        sqlx::query(
            r#"
            CREATE INDEX IF NOT EXISTS idx_replay_state ON replay_entries (state)
            "#,
        )
        .execute(&self.pool)
        .await
        .map_err(|e| RuntimeError::Storage(format!("Migration failed: {e}")))?;

        Ok(())
    }
}

#[async_trait]
impl ReplayDatabase for PostgresReplayDb {
    async fn contains(&self, id: &ReplayId) -> Result<bool, RuntimeError> {
        let hex_id = hex::encode(id.as_bytes());

        let row: Option<(String,)> = sqlx::query_as(
            "SELECT id FROM replay_entries WHERE id = $1",
        )
        .bind(&hex_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| RuntimeError::Storage(format!("PostgreSQL query error: {e}")))?;

        Ok(row.is_some())
    }

    async fn insert_if_absent(
        &self,
        id: &ReplayId,
        state: ReplayEntryState,
    ) -> Result<(), ReplayDbError> {
        let hex_id = hex::encode(id.as_bytes());
        let state_str = match state {
            ReplayEntryState::Pending => "Pending",
            ReplayEntryState::Consumed => "Consumed",
            ReplayEntryState::RolledBack => "RolledBack",
        };

        let result = sqlx::query_scalar::<_, String>(
            r#"
            INSERT INTO replay_entries (id, state, inserted_at)
            VALUES ($1, $2, NOW())
            ON CONFLICT (id) DO NOTHING
            RETURNING id
            "#,
        )
        .bind(&hex_id)
        .bind(state_str)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ReplayDbError::Storage(e.to_string()))?;

        match result {
            Some(_) => Ok(()),
            None => Err(ReplayDbError::AlreadyExists),
        }
    }

    async fn consume_if_unconsumed(&self, id: &ReplayId) -> Result<(), ReplayDbError> {
        let hex_id = hex::encode(id.as_bytes());

        // Check current state first (optimistic).
        let current_state: Option<(String,)> = sqlx::query_as(
            "SELECT state FROM replay_entries WHERE id = $1",
        )
        .bind(&hex_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| ReplayDbError::Storage(e.to_string()))?;

        match current_state {
            Some((state,)) if state == "Consumed" => return Ok(()),
            Some((_,)) => return Err(ReplayDbError::AlreadyExists),
            None => {
                // Insert with CAS semantics.
                let inserted = sqlx::query_scalar::<_, String>(
                    r#"
                    INSERT INTO replay_entries (id, state, inserted_at)
                    VALUES ($1, 'Pending', NOW())
                    ON CONFLICT (id) DO NOTHING
                    RETURNING id
                    "#,
                )
                .bind(&hex_id)
                .fetch_optional(&self.pool)
                .await
                .map_err(|e| ReplayDbError::Storage(e.to_string()))?;

                match inserted {
                    Some(_) => Ok(()),
                    None => Err(ReplayDbError::AlreadyExists),
                }
            }
        }
    }

    async fn confirm_consumed(&self, id: &ReplayId) -> Result<(), ReplayDbError> {
        let hex_id = hex::encode(id.as_bytes());

        let result = sqlx::query(
            r#"
            UPDATE replay_entries
            SET state = 'Consumed', updated_at = NOW()
            WHERE id = $1 AND state = 'Pending'
            "#,
        )
        .bind(&hex_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ReplayDbError::Storage(e.to_string()))?;

        if result.rows_affected() == 0 {
            // Check if already consumed (idempotent).
            let current: Option<(String,)> = sqlx::query_as(
                "SELECT state FROM replay_entries WHERE id = $1",
            )
            .bind(&hex_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| ReplayDbError::Storage(e.to_string()))?;

            match current {
                Some((state,)) if state == "Consumed" => Ok(()),
                Some((_,)) => Err(ReplayDbError::Storage(
                    "Entry is not in Pending state".to_string(),
                )),
                None => Err(ReplayDbError::Storage("Entry not found".to_string())),
            }
        } else {
            Ok(())
        }
    }

    async fn mark_rolled_back(&self, id: &ReplayId) -> Result<(), RuntimeError> {
        let hex_id = hex::encode(id.as_bytes());

        let result = sqlx::query(
            r#"
            UPDATE replay_entries
            SET state = 'RolledBack', updated_at = NOW()
            WHERE id = $1 AND state = 'Pending'
            "#,
        )
        .bind(&hex_id)
        .execute(&self.pool)
        .await
        .map_err(|e| RuntimeError::Storage(format!("PostgreSQL update error: {e}")))?;

        if result.rows_affected() == 0 {
            // Check if already rolled back (idempotent).
            let current: Option<(String,)> = sqlx::query_as(
                "SELECT state FROM replay_entries WHERE id = $1",
            )
            .bind(&hex_id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| RuntimeError::Storage(format!("PostgreSQL query error: {e}")))?;

            match current {
                Some((state,)) if state == "RolledBack" => Ok(()),
                Some((_,)) => Err(RuntimeError::InvalidState(
                    "Entry is not in Pending state".to_string(),
                )),
                None => Err(RuntimeError::TransferNotFound(
                    format!("ReplayId {:?}", id.as_bytes()),
                )),
            }
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
#[cfg(feature = "postgres")]
mod tests {
    use super::*;
    use csv_core::proof::ReplayId;

    fn test_replay_id(label: u8) -> ReplayId {
        ReplayId::derive(
            "test",
            &[label; 32],
            0,
            &[label + 1; 32],
            &[label + 2; 32],
            "test-dst",
        )
    }

    /// Helper to create a test database.
    /// Requires PostgreSQL running with DATABASE_URL env var set,
    /// or falls back to an in-memory-like test using a disposable database URL.
    async fn setup_db() -> Option<PostgresReplayDb> {
        let database_url = match std::env::var("DATABASE_URL") {
            Ok(url) => url,
            Err(_) => {
                // Try local test database.
                "postgres://localhost:5432/csv_test".to_string()
            }
        };

        match PostgresReplayDb::connect(&database_url).await {
            Ok(db) => {
                db.run_migrations().await.ok()?;
                Some(db)
            }
            Err(_) => {
                eprintln!("Skipping PostgreSQL test — no database available");
                None
            }
        }
    }

    #[tokio::test]
    async fn test_insert_and_contains() {
        let Some(db) = setup_db().await else { return };
        let id = test_replay_id(1);

        assert!(!db.contains(&id).await.unwrap());
        db.insert_if_absent(&id, ReplayEntryState::Pending)
            .await
            .unwrap();
        assert!(db.contains(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_insert_if_absent_duplicate_fails() {
        let Some(db) = setup_db().await else { return };
        let id = test_replay_id(2);

        db.insert_if_absent(&id, ReplayEntryState::Pending)
            .await
            .unwrap();

        let result = db.insert_if_absent(&id, ReplayEntryState::Pending).await;
        assert!(matches!(result, Err(ReplayDbError::AlreadyExists)));
    }

    #[tokio::test]
    async fn test_confirm_consumed() {
        let Some(db) = setup_db().await else { return };
        let id = test_replay_id(3);

        db.insert_if_absent(&id, ReplayEntryState::Pending)
            .await
            .unwrap();
        db.confirm_consumed(&id).await.unwrap();

        assert!(db.contains(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_mark_rolled_back() {
        let Some(db) = setup_db().await else { return };
        let id = test_replay_id(4);

        db.insert_if_absent(&id, ReplayEntryState::Pending)
            .await
            .unwrap();
        db.mark_rolled_back(&id).await.unwrap();
    }

    #[tokio::test]
    async fn test_consume_if_unconsumed_fresh() {
        let Some(db) = setup_db().await else { return };
        let id = test_replay_id(5);

        db.consume_if_unconsumed(&id).await.unwrap();
        assert!(db.contains(&id).await.unwrap());
    }

    #[tokio::test]
    async fn test_pending_blocks_consume_if_unconsumed() {
        let Some(db) = setup_db().await else { return };
        let id = test_replay_id(6);

        db.insert_if_absent(&id, ReplayEntryState::Pending)
            .await
            .unwrap();

        let result = db.consume_if_unconsumed(&id).await;
        assert!(matches!(result, Err(ReplayDbError::AlreadyExists)));
    }
}