/// Repository for aggregate statistics queries.

use sqlx::SqlitePool;

use shared::{ChainCount, ChainPairCount, ExplorerStats, Result};

/// Repository for aggregate statistics.
#[derive(Clone)]
pub struct StatsRepository {
    pool: SqlitePool,
}

impl StatsRepository {
    /// Create a new repository wrapping the given pool.
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    /// Get full aggregate statistics.
    pub async fn get_stats(&self) -> Result<ExplorerStats> {
        let total_rights: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM rights").fetch_one(&self.pool).await?;

        let total_transfers: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM transfers").fetch_one(&self.pool).await?;

        let total_seals: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM seals").fetch_one(&self.pool).await?;

        let total_contracts: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM contracts").fetch_one(&self.pool).await?;

        // Rights by chain
        let rights_by_chain_rows = sqlx::query_as::<_, ChainCountRow>(
            "SELECT chain, COUNT(*) as count FROM rights GROUP BY chain ORDER BY count DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        let rights_by_chain = rights_by_chain_rows
            .into_iter()
            .map(|r| ChainCount {
                chain: r.chain,
                count: r.count,
            })
            .collect();

        // Transfers by chain pair
        let transfers_by_pair_rows = sqlx::query_as::<_, ChainPairCountRow>(
            "SELECT from_chain, to_chain, COUNT(*) as count \
             FROM transfers GROUP BY from_chain, to_chain ORDER BY count DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        let transfers_by_chain_pair = transfers_by_pair_rows
            .into_iter()
            .map(|r| ChainPairCount {
                from_chain: r.from_chain,
                to_chain: r.to_chain,
                count: r.count,
            })
            .collect();

        // Active seals by chain
        let active_seals_rows = sqlx::query_as::<_, ChainCountRow>(
            "SELECT chain, COUNT(*) as count FROM seals \
             WHERE status = 'available' GROUP BY chain ORDER BY count DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        let active_seals_by_chain = active_seals_rows
            .into_iter()
            .map(|r| ChainCount {
                chain: r.chain,
                count: r.count,
            })
            .collect();

        // Transfer success rate
        let completed: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM transfers WHERE status = 'completed'",
        )
        .fetch_one(&self.pool)
        .await?;

        let failed: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM transfers WHERE status = 'failed'",
        )
        .fetch_one(&self.pool)
        .await?;

        let total_finished = completed + failed;
        let transfer_success_rate = if total_finished > 0 {
            (completed as f64 / total_finished as f64) * 100.0
        } else {
            100.0
        };

        // Average transfer time (for completed transfers)
        let avg_transfer_time: Option<i64> = sqlx::query_scalar(
            "SELECT AVG(duration_ms) FROM transfers WHERE status = 'completed' AND duration_ms IS NOT NULL",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(ExplorerStats {
            total_rights: total_rights as u64,
            total_transfers: total_transfers as u64,
            total_seals: total_seals as u64,
            total_contracts: total_contracts as u64,
            rights_by_chain,
            transfers_by_chain_pair,
            active_seals_by_chain,
            transfer_success_rate,
            average_transfer_time_ms: avg_transfer_time.map(|v| v as u64),
        })
    }

    /// Get total rights count.
    pub async fn total_rights(&self) -> Result<u64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM rights").fetch_one(&self.pool).await?;
        Ok(count as u64)
    }

    /// Get total transfers count.
    pub async fn total_transfers(&self) -> Result<u64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM transfers").fetch_one(&self.pool).await?;
        Ok(count as u64)
    }

    /// Get total seals count.
    pub async fn total_seals(&self) -> Result<u64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM seals").fetch_one(&self.pool).await?;
        Ok(count as u64)
    }

    /// Get total contracts count.
    pub async fn total_contracts(&self) -> Result<u64> {
        let count: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM contracts").fetch_one(&self.pool).await?;
        Ok(count as u64)
    }
}

#[derive(Debug, sqlx::FromRow)]
struct ChainCountRow {
    chain: String,
    count: u64,
}

#[derive(Debug, sqlx::FromRow)]
struct ChainPairCountRow {
    from_chain: String,
    to_chain: String,
    count: u64,
}
