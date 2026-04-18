/// Repository for `RightRecord` operations.
use sqlx::sqlite::SqliteRow;
use sqlx::{Row, SqlitePool};

use csv_explorer_shared::{Result, RightFilter, RightRecord, RightStatus};

/// Typed repository for the `rights` table.
#[derive(Clone)]
pub struct RightsRepository {
    pool: SqlitePool,
}

impl RightsRepository {
    /// Create a new repository wrapping the given pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Insert a new right record.
    pub async fn insert(&self, right: &RightRecord) -> Result<()> {
        let metadata_json = right
            .metadata
            .as_ref()
            .map(|v| serde_json::to_string(v))
            .transpose()?;
        let last_transfer_at = right.last_transfer_at.map(|dt| dt);

        sqlx::query(
            r#"
            INSERT INTO rights (id, chain, seal_ref, commitment, owner, created_at, created_tx,
                                status, metadata, transfer_count, last_transfer_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            ON CONFLICT(id) DO UPDATE SET
                status = excluded.status,
                owner = excluded.owner,
                transfer_count = excluded.transfer_count,
                last_transfer_at = excluded.last_transfer_at,
                metadata = excluded.metadata
            "#,
        )
        .bind(&right.id)
        .bind(&right.chain)
        .bind(&right.seal_ref)
        .bind(&right.commitment)
        .bind(&right.owner)
        .bind(right.created_at)
        .bind(&right.created_tx)
        .bind(right.status.to_string())
        .bind(metadata_json)
        .bind(right.transfer_count as i64)
        .bind(last_transfer_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Get a single right by ID.
    pub async fn get(&self, id: &str) -> Result<Option<RightRecord>> {
        let row = sqlx::query(
            r#"SELECT id, chain, seal_ref, commitment, owner, created_at, created_tx,
                      status, metadata, transfer_count, last_transfer_at
               FROM rights WHERE id = $1"#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;

        match row {
            Some(row) => Ok(Some(row_to_right(&row)?)),
            None => Ok(None),
        }
    }

    /// List rights matching the given filter.
    pub async fn list(&self, filter: RightFilter) -> Result<Vec<RightRecord>> {
        let mut sql = String::from(
            "SELECT id, chain, seal_ref, commitment, owner, created_at, created_tx, \
             status, metadata, transfer_count, last_transfer_at FROM rights WHERE 1=1",
        );
        let mut records = Vec::new();

        if filter.chain.is_some() {
            sql.push_str(" AND chain = ?");
        }
        if filter.owner.is_some() {
            sql.push_str(" AND owner = ?");
        }
        if filter.status.is_some() {
            sql.push_str(" AND status = ?");
        }

        sql.push_str(" ORDER BY created_at DESC");

        if let Some(limit) = filter.limit {
            sql.push_str(&format!(" LIMIT {}", limit));
        }
        if let Some(offset) = filter.offset {
            sql.push_str(&format!(" OFFSET {}", offset));
        }

        let mut query = sqlx::query(&sql);
        if let Some(ref chain) = filter.chain {
            query = query.bind(chain);
        }
        if let Some(ref owner) = filter.owner {
            query = query.bind(owner);
        }
        if let Some(status) = filter.status {
            query = query.bind(status.to_string());
        }

        let rows = query.fetch_all(&self.pool).await?;
        for row in rows {
            records.push(row_to_right(&row)?);
        }

        Ok(records)
    }

    /// Update the status of a right.
    pub async fn update_status(&self, id: &str, status: RightStatus) -> Result<()> {
        sqlx::query("UPDATE rights SET status = $1 WHERE id = $2")
            .bind(status.to_string())
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Count rights matching the filter.
    pub async fn count(&self, filter: RightFilter) -> Result<u64> {
        let mut sql = String::from("SELECT COUNT(*) FROM rights WHERE 1=1");

        if filter.chain.is_some() {
            sql.push_str(" AND chain = ?");
        }
        if filter.owner.is_some() {
            sql.push_str(" AND owner = ?");
        }
        if filter.status.is_some() {
            sql.push_str(" AND status = ?");
        }

        let mut query = sqlx::query_scalar::<_, i64>(&sql);
        if let Some(ref chain) = filter.chain {
            query = query.bind(chain);
        }
        if let Some(ref owner) = filter.owner {
            query = query.bind(owner);
        }
        if let Some(status) = filter.status {
            query = query.bind(status.to_string());
        }

        let count = query.fetch_one(&self.pool).await?;
        Ok(count as u64)
    }

    /// Get all rights owned by a specific address.
    pub async fn by_owner(&self, owner: &str) -> Result<Vec<RightRecord>> {
        let rows = sqlx::query(
            "SELECT id, chain, seal_ref, commitment, owner, created_at, created_tx, \
             status, metadata, transfer_count, last_transfer_at \
             FROM rights WHERE owner = ? ORDER BY created_at DESC",
        )
        .bind(owner)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_right).collect()
    }

    /// Get all rights on a specific chain.
    pub async fn by_chain(&self, chain: &str) -> Result<Vec<RightRecord>> {
        let rows = sqlx::query(
            "SELECT id, chain, seal_ref, commitment, owner, created_at, created_tx, \
             status, metadata, transfer_count, last_transfer_at \
             FROM rights WHERE chain = ? ORDER BY created_at DESC",
        )
        .bind(chain)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_right).collect()
    }

    /// Get recently created rights.
    pub async fn recent(&self, limit: usize) -> Result<Vec<RightRecord>> {
        let rows = sqlx::query(
            "SELECT id, chain, seal_ref, commitment, owner, created_at, created_tx, \
             status, metadata, transfer_count, last_transfer_at \
             FROM rights ORDER BY created_at DESC LIMIT ?",
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(row_to_right).collect()
    }
}

fn row_to_right(row: &SqliteRow) -> Result<RightRecord> {
    let status_str: String = row.try_get("status")?;
    let status = match status_str.as_str() {
        "active" => RightStatus::Active,
        "spent" => RightStatus::Spent,
        "pending" => RightStatus::Pending,
        _ => RightStatus::Active,
    };

    let metadata: Option<serde_json::Value> = row
        .try_get::<Option<String>, _>("metadata")?
        .map(|s: String| serde_json::from_str(&s))
        .transpose()?;

    Ok(RightRecord {
        id: row.try_get("id")?,
        chain: row.try_get("chain")?,
        seal_ref: row.try_get("seal_ref")?,
        commitment: row.try_get("commitment")?,
        owner: row.try_get("owner")?,
        created_at: row.try_get::<chrono::DateTime<chrono::Utc>, _>("created_at")?,
        created_tx: row.try_get("created_tx")?,
        status,
        metadata,
        transfer_count: row.try_get::<i64, _>("transfer_count")? as u64,
        last_transfer_at: row
            .try_get::<Option<chrono::DateTime<chrono::Utc>>, _>("last_transfer_at")?
            .map(|dt| dt),
    })
}
