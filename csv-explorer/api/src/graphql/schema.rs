/// GraphQL schema for the CSV Explorer API.
///
/// Provides queries for all indexed data types with filtering,
/// pagination, and aggregate statistics.
use async_graphql::*;
use sqlx::SqlitePool;

use csv_explorer_storage::repositories::{
    AdvancedProofRepository, ContractsRepository, RightsRepository, SealsRepository,
    StatsRepository, TransfersRepository,
};

use super::types::*;

// Re-export EmptySubscription for use in server
pub use async_graphql::EmptySubscription;

/// GraphQL context holding the database pool.
pub struct GraphqlContext {
    pub pool: SqlitePool,
}

/// Root query type.
pub struct Query;

#[Object]
impl Query {
    /// Get a single right by ID.
    async fn right(&self, ctx: &Context<'_>, id: String) -> Result<Option<Right>> {
        let gql_ctx = ctx
            .data::<GraphqlContext>()
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let repo = RightsRepository::new(gql_ctx.pool.clone());
        let record = repo
            .get(&id)
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        Ok(record.map(Right::from))
    }

    /// List rights with optional filtering and pagination.
    async fn rights(
        &self,
        ctx: &Context<'_>,
        filter: Option<RightFilterInput>,
    ) -> Result<RightConnection> {
        let gql_ctx = ctx
            .data::<GraphqlContext>()
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let repo = RightsRepository::new(gql_ctx.pool.clone());

        let filter = filter.unwrap_or_default();
        let limit = filter.limit.unwrap_or(20) as usize;
        let offset = filter.offset.unwrap_or(0) as usize;

        let shared_filter = csv_explorer_shared::RightFilter {
            chain: filter.chain,
            owner: filter.owner,
            status: filter.status.as_deref().map(|s| match s {
                "active" => csv_explorer_shared::RightStatus::Active,
                "spent" => csv_explorer_shared::RightStatus::Spent,
                "pending" => csv_explorer_shared::RightStatus::Pending,
                _ => csv_explorer_shared::RightStatus::Active,
            }),
            limit: Some(limit),
            offset: Some(offset),
        };

        let records = repo
            .list(shared_filter.clone())
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let total_count = repo
            .count(shared_filter)
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;

        let edges: Vec<RightEdge> = records
            .into_iter()
            .enumerate()
            .map(|(i, r)| RightEdge {
                node: Right::from(r),
                cursor: (offset + i).to_string(),
            })
            .collect();

        let has_next_page = (offset + limit) < total_count as usize;
        let has_previous_page = offset > 0;
        let start_cursor = edges.first().map(|e| e.cursor.clone());
        let end_cursor = edges.last().map(|e| e.cursor.clone());

        Ok(RightConnection {
            edges,
            page_info: PageInfo::new(has_next_page, has_previous_page, start_cursor, end_cursor),
            total_count: total_count as i64,
        })
    }

    /// Get a single transfer by ID.
    async fn transfer(&self, ctx: &Context<'_>, id: String) -> Result<Option<Transfer>> {
        let gql_ctx = ctx
            .data::<GraphqlContext>()
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let repo = TransfersRepository::new(gql_ctx.pool.clone());
        let record = repo
            .get(&id)
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        Ok(record.map(Transfer::from))
    }

    /// List transfers with optional filtering and pagination.
    async fn transfers(
        &self,
        ctx: &Context<'_>,
        filter: Option<TransferFilterInput>,
    ) -> Result<TransferConnection> {
        let gql_ctx = ctx
            .data::<GraphqlContext>()
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let repo = TransfersRepository::new(gql_ctx.pool.clone());

        let filter = filter.unwrap_or_default();
        let limit = filter.limit.unwrap_or(20) as usize;
        let offset = filter.offset.unwrap_or(0) as usize;

        let shared_filter = csv_explorer_shared::TransferFilter {
            right_id: filter.right_id,
            from_chain: filter.from_chain,
            to_chain: filter.to_chain,
            status: filter.status.as_deref().map(|s| match s {
                "pending" => csv_explorer_shared::TransferStatus::Initiated,
                "in_progress" => csv_explorer_shared::TransferStatus::SubmittingProof,
                "completed" => csv_explorer_shared::TransferStatus::Completed,
                "failed" => csv_explorer_shared::TransferStatus::Failed {
                    error_code: "UNKNOWN".to_string(),
                    retryable: true,
                },
                _ => csv_explorer_shared::TransferStatus::Initiated,
            }),
            limit: Some(limit),
            offset: Some(offset),
        };

        let records = repo
            .list(shared_filter.clone())
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let total_count = repo
            .count(shared_filter)
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;

        let edges: Vec<TransferEdge> = records
            .into_iter()
            .enumerate()
            .map(|(i, t)| TransferEdge {
                node: Transfer::from(t),
                cursor: (offset + i).to_string(),
            })
            .collect();

        let has_next_page = (offset + limit) < total_count as usize;
        let has_previous_page = offset > 0;
        let start_cursor = edges.first().map(|e| e.cursor.clone());
        let end_cursor = edges.last().map(|e| e.cursor.clone());

        Ok(TransferConnection {
            edges,
            page_info: PageInfo::new(has_next_page, has_previous_page, start_cursor, end_cursor),
            total_count: total_count as i64,
        })
    }

    /// Get a single seal by ID.
    async fn seal(&self, ctx: &Context<'_>, id: String) -> Result<Option<Seal>> {
        let gql_ctx = ctx
            .data::<GraphqlContext>()
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let repo = SealsRepository::new(gql_ctx.pool.clone());
        let record = repo
            .get(&id)
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        Ok(record.map(Seal::from))
    }

    /// List seals with optional filtering and pagination.
    async fn seals(
        &self,
        ctx: &Context<'_>,
        filter: Option<SealFilterInput>,
    ) -> Result<SealConnection> {
        let gql_ctx = ctx
            .data::<GraphqlContext>()
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let repo = SealsRepository::new(gql_ctx.pool.clone());

        let filter = filter.unwrap_or_default();
        let limit = filter.limit.unwrap_or(20) as usize;
        let offset = filter.offset.unwrap_or(0) as usize;

        let shared_filter = csv_explorer_shared::SealFilter {
            chain: filter.chain,
            seal_type: filter.seal_type.as_deref().map(|s| match s {
                "utxo" => csv_explorer_shared::SealType::Utxo,
                "object" => csv_explorer_shared::SealType::Object,
                "resource" => csv_explorer_shared::SealType::Resource,
                "nullifier" => csv_explorer_shared::SealType::Nullifier,
                "account" => csv_explorer_shared::SealType::Account,
                _ => csv_explorer_shared::SealType::Utxo,
            }),
            status: filter.status.as_deref().map(|s| match s {
                "available" => csv_explorer_shared::SealStatus::Available,
                "consumed" => csv_explorer_shared::SealStatus::Consumed,
                _ => csv_explorer_shared::SealStatus::Available,
            }),
            right_id: filter.right_id,
            limit: Some(limit),
            offset: Some(offset),
        };

        let records = repo
            .list(shared_filter.clone())
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let total_count = repo
            .count(shared_filter)
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;

        let edges: Vec<SealEdge> = records
            .into_iter()
            .enumerate()
            .map(|(i, s)| SealEdge {
                node: Seal::from(s),
                cursor: (offset + i).to_string(),
            })
            .collect();

        let has_next_page = (offset + limit) < total_count as usize;
        let has_previous_page = offset > 0;
        let start_cursor = edges.first().map(|e| e.cursor.clone());
        let end_cursor = edges.last().map(|e| e.cursor.clone());

        Ok(SealConnection {
            edges,
            page_info: PageInfo::new(has_next_page, has_previous_page, start_cursor, end_cursor),
            total_count: total_count as i64,
        })
    }

    /// Get a seal by its chain-native identifier (e.g., Bitcoin txid:vout, Sui ObjectId, Ethereum storage slot).
    ///
    /// This enables cross-chain seal lookup — given a seal reference from any chain,
    /// you can find the corresponding seal record in the explorer.
    async fn seal_by_chain_native_id(
        &self,
        ctx: &Context<'_>,
        chain_native_id: String,
    ) -> Result<Option<Seal>> {
        let gql_ctx = ctx
            .data::<GraphqlContext>()
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let repo = SealsRepository::new(gql_ctx.pool.clone());
        let record = repo
            .get_by_chain_native_id(&chain_native_id)
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        Ok(record.map(Seal::from))
    }

    /// Get a single contract by ID.
    async fn contract(&self, ctx: &Context<'_>, id: String) -> Result<Option<CsvContractGql>> {
        let gql_ctx = ctx
            .data::<GraphqlContext>()
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let repo = ContractsRepository::new(gql_ctx.pool.clone());
        let record = repo
            .get(&id)
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        Ok(record.map(CsvContractGql::from))
    }

    /// List contracts with optional filtering and pagination.
    async fn contracts(
        &self,
        ctx: &Context<'_>,
        filter: Option<ContractFilterInput>,
    ) -> Result<ContractConnection> {
        let gql_ctx = ctx
            .data::<GraphqlContext>()
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let repo = ContractsRepository::new(gql_ctx.pool.clone());

        let filter = filter.unwrap_or_default();
        let limit = filter.limit.unwrap_or(20) as usize;
        let offset = filter.offset.unwrap_or(0) as usize;

        let shared_filter = csv_explorer_shared::ContractFilter {
            chain: filter.chain,
            contract_type: filter.contract_type.as_deref().map(|s| match s {
                "nullifier_registry" => csv_explorer_shared::ContractType::NullifierRegistry,
                "state_commitment" => csv_explorer_shared::ContractType::StateCommitment,
                "right_registry" => csv_explorer_shared::ContractType::RightRegistry,
                "bridge" => csv_explorer_shared::ContractType::Bridge,
                _ => csv_explorer_shared::ContractType::Other,
            }),
            status: filter.status.as_deref().map(|s| match s {
                "active" => csv_explorer_shared::ContractStatus::Active,
                "deprecated" => csv_explorer_shared::ContractStatus::Deprecated,
                "error" => csv_explorer_shared::ContractStatus::Error,
                _ => csv_explorer_shared::ContractStatus::Active,
            }),
            limit: Some(limit),
            offset: Some(offset),
        };

        let records = repo
            .list(shared_filter.clone())
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let total_count = repo
            .count(shared_filter)
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;

        let edges: Vec<ContractEdge> = records
            .into_iter()
            .enumerate()
            .map(|(i, c)| ContractEdge {
                node: CsvContractGql::from(c),
                cursor: (offset + i).to_string(),
            })
            .collect();

        let has_next_page = (offset + limit) < total_count as usize;
        let has_previous_page = offset > 0;
        let start_cursor = edges.first().map(|e| e.cursor.clone());
        let end_cursor = edges.last().map(|e| e.cursor.clone());

        Ok(ContractConnection {
            edges,
            page_info: PageInfo::new(has_next_page, has_previous_page, start_cursor, end_cursor),
            total_count: total_count as i64,
        })
    }

    /// Get aggregate statistics.
    async fn stats(&self, ctx: &Context<'_>) -> Result<Stats> {
        let gql_ctx = ctx
            .data::<GraphqlContext>()
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let repo = StatsRepository::new(gql_ctx.pool.clone());
        let stats = repo
            .get_stats()
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        Ok(Stats::from(stats))
    }

    /// Get the status of all indexed chains.
    async fn chain_status(&self, _ctx: &Context<'_>) -> Result<Vec<ChainInfoGql>> {
        // In a real implementation, this would query the indexer for current status
        // For now, return empty as the indexer would need to be wired in
        Ok(Vec::new())
    }

    /// Get rights by owner address.
    async fn rights_by_owner(&self, ctx: &Context<'_>, owner: String) -> Result<Vec<Right>> {
        let gql_ctx = ctx
            .data::<GraphqlContext>()
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let repo = RightsRepository::new(gql_ctx.pool.clone());
        let records = repo
            .by_owner(&owner)
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        Ok(records.into_iter().map(Right::from).collect())
    }

    /// Get transfers for a specific right.
    async fn transfers_by_right(
        &self,
        ctx: &Context<'_>,
        right_id: String,
    ) -> Result<Vec<Transfer>> {
        let gql_ctx = ctx
            .data::<GraphqlContext>()
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let repo = TransfersRepository::new(gql_ctx.pool.clone());
        let records = repo
            .by_right(&right_id)
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        Ok(records.into_iter().map(Transfer::from).collect())
    }

    /// Get seals for a specific right.
    async fn seals_by_right(&self, ctx: &Context<'_>, right_id: String) -> Result<Vec<Seal>> {
        let gql_ctx = ctx
            .data::<GraphqlContext>()
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let repo = SealsRepository::new(gql_ctx.pool.clone());
        let records = repo
            .by_right(&right_id)
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        Ok(records.into_iter().map(Seal::from).collect())
    }

    // -----------------------------------------------------------------------
    // Advanced commitment and proof queries
    // -----------------------------------------------------------------------

    /// Get enhanced rights with commitment scheme and proof metadata.
    async fn enhanced_rights(
        &self,
        ctx: &Context<'_>,
        scheme: Option<String>,
        proof_type: Option<String>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<EnhancedRight>> {
        let gql_ctx = ctx
            .data::<GraphqlContext>()
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let repo = AdvancedProofRepository::new(gql_ctx.pool.clone());

        let filter = csv_explorer_shared::RightProofFilter {
            chain: None,
            owner: None,
            commitment_scheme: scheme
                .as_deref()
                .and_then(|s| csv_explorer_shared::CommitmentScheme::from_str(s)),
            inclusion_proof_type: proof_type
                .as_deref()
                .and_then(|s| csv_explorer_shared::InclusionProofType::from_str(s)),
            finality_proof_type: None,
            limit: limit.map(|v| v as usize),
            offset: offset.map(|v| v as usize),
        };

        let records = repo
            .query_enhanced_rights(filter)
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        Ok(records.into_iter().map(EnhancedRight::from).collect())
    }

    /// Get enhanced seals with proof metadata.
    async fn enhanced_seals(
        &self,
        ctx: &Context<'_>,
        limit: Option<i32>,
        offset: Option<i32>,
    ) -> Result<Vec<EnhancedSeal>> {
        let gql_ctx = ctx
            .data::<GraphqlContext>()
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let repo = AdvancedProofRepository::new(gql_ctx.pool.clone());

        let filter = csv_explorer_shared::SealProofFilter {
            chain: None,
            seal_type: None,
            seal_proof_type: None,
            seal_proof_verified: None,
            limit: limit.map(|v| v as usize),
            offset: offset.map(|v| v as usize),
        };

        let records = repo
            .query_enhanced_seals(filter)
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        Ok(records.into_iter().map(EnhancedSeal::from).collect())
    }

    /// Get proof statistics across all indexed data.
    async fn proof_statistics(&self, ctx: &Context<'_>) -> Result<ProofStatisticsGql> {
        let gql_ctx = ctx
            .data::<GraphqlContext>()
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        let repo = AdvancedProofRepository::new(gql_ctx.pool.clone());
        let stats = repo
            .get_proof_statistics()
            .await
            .map_err(|e| ServerError::new(format!("{:?}", e), None))?;
        Ok(ProofStatisticsGql::from(stats))
    }
}

/// Root mutation type.
pub struct Mutation;

#[Object]
impl Mutation {
    /// Trigger a refresh/sync for a specific chain.
    async fn refresh_chain(&self, ctx: &Context<'_>, chain: String) -> Result<bool> {
        // In production, this would signal the indexer to sync this chain
        let _ = ctx;
        let _ = chain;
        Ok(true)
    }

    /// Trigger reindexing from a specific block.
    async fn reindex_from(&self, ctx: &Context<'_>, chain: String, block: i64) -> Result<bool> {
        // In production, this would signal the indexer to reindex
        let _ = ctx;
        let _ = chain;
        let _ = block;
        Ok(true)
    }
}

/// Build the GraphQL schema.
pub fn create_schema() -> Schema<Query, Mutation, EmptySubscription> {
    Schema::build(Query, Mutation, EmptySubscription).finish()
}
