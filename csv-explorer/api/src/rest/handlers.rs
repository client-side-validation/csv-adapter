/// REST API handlers for the CSV Explorer.

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};

use csv_explorer_storage::repositories::{
    RightsRepository, SealsRepository, StatsRepository, TransfersRepository,
};
use sqlx::SqlitePool;

use csv_explorer_shared::{ExplorerError, RightFilter, SealFilter, TransferFilter};

// ---------------------------------------------------------------------------
// Application state
// ---------------------------------------------------------------------------

// The full application state tuple
type AppState = (async_graphql::Schema<
    crate::graphql::schema::Query,
    crate::graphql::schema::Mutation,
    crate::graphql::schema::EmptySubscription,
>, sqlx::SqlitePool);

// ---------------------------------------------------------------------------
// Response wrappers
// ---------------------------------------------------------------------------

#[derive(Serialize)]
pub struct ApiResponse<T> {
    pub data: T,
    pub success: bool,
}

#[derive(Serialize)]
pub struct PaginatedResponse<T> {
    pub data: Vec<T>,
    pub total: u64,
    pub limit: usize,
    pub offset: usize,
}

#[derive(Serialize)]
pub struct ErrorResponse {
    pub error: String,
    pub success: bool,
}

impl<T: Serialize> From<T> for ApiResponse<T> {
    fn from(data: T) -> Self {
        Self {
            data,
            success: true,
        }
    }
}

// ---------------------------------------------------------------------------
// Rights handlers
// ---------------------------------------------------------------------------

/// Query parameters for listing rights.
#[derive(Deserialize)]
pub struct ListRightsQuery {
    pub chain: Option<String>,
    pub owner: Option<String>,
    pub status: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// GET /api/v1/rights
pub async fn list_rights(
    Query(query): Query<ListRightsQuery>,
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<PaginatedResponse<csv_explorer_shared::RightRecord>>>, (StatusCode, Json<ErrorResponse>)> {
    let repo = RightsRepository::new(pool);

    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    let filter = RightFilter {
        chain: query.chain,
        owner: query.owner,
        status: query.status.as_deref().map(|s| match s {
            "active" => csv_explorer_shared::RightStatus::Active,
            "spent" => csv_explorer_shared::RightStatus::Spent,
            "pending" => csv_explorer_shared::RightStatus::Pending,
            _ => csv_explorer_shared::RightStatus::Active,
        }),
        limit: Some(limit),
        offset: Some(offset),
    };

    let total = repo.count(filter.clone())
        .await
        .map_err(|e| server_error(&e))?;

    let data = repo.list(filter)
        .await
        .map_err(|e| server_error(&e))?;

    Ok(Json(ApiResponse::from(PaginatedResponse {
        data,
        total,
        limit,
        offset,
    })))
}

/// GET /api/v1/rights/:id
pub async fn get_right(
    Path(id): Path<String>,
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<csv_explorer_shared::RightRecord>>, (StatusCode, Json<ErrorResponse>)> {
    let repo = RightsRepository::new(pool);

    let right = repo.get(&id)
        .await
        .map_err(|e| server_error(&e))?;

    match right {
        Some(r) => Ok(Json(ApiResponse::from(r))),
        None => Err(not_found(&format!("Right {} not found", id))),
    }
}

// ---------------------------------------------------------------------------
// Transfers handlers
// ---------------------------------------------------------------------------

/// Query parameters for listing transfers.
#[derive(Deserialize)]
pub struct ListTransfersQuery {
    pub right_id: Option<String>,
    pub from_chain: Option<String>,
    pub to_chain: Option<String>,
    pub status: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// GET /api/v1/transfers
pub async fn list_transfers(
    Query(query): Query<ListTransfersQuery>,
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<PaginatedResponse<csv_explorer_shared::TransferRecord>>>, (StatusCode, Json<ErrorResponse>)> {
    let repo = TransfersRepository::new(pool);

    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    let filter = TransferFilter {
        right_id: query.right_id,
        from_chain: query.from_chain,
        to_chain: query.to_chain,
        status: query.status.as_deref().map(|s| match s {
            "pending" => csv_explorer_shared::TransferStatus::Pending,
            "in_progress" => csv_explorer_shared::TransferStatus::InProgress,
            "completed" => csv_explorer_shared::TransferStatus::Completed,
            "failed" => csv_explorer_shared::TransferStatus::Failed,
            _ => csv_explorer_shared::TransferStatus::Pending,
        }),
        limit: Some(limit),
        offset: Some(offset),
    };

    let total = repo.count(filter.clone())
        .await
        .map_err(|e| server_error(&e))?;

    let data = repo.list(filter)
        .await
        .map_err(|e| server_error(&e))?;

    Ok(Json(ApiResponse::from(PaginatedResponse {
        data,
        total,
        limit,
        offset,
    })))
}

/// GET /api/v1/transfers/:id
pub async fn get_transfer(
    Path(id): Path<String>,
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<csv_explorer_shared::TransferRecord>>, (StatusCode, Json<ErrorResponse>)> {
    let repo = TransfersRepository::new(pool);

    let transfer = repo.get(&id)
        .await
        .map_err(|e| server_error(&e))?;

    match transfer {
        Some(t) => Ok(Json(ApiResponse::from(t))),
        None => Err(not_found(&format!("Transfer {} not found", id))),
    }
}

// ---------------------------------------------------------------------------
// Seals handlers
// ---------------------------------------------------------------------------

/// Query parameters for listing seals.
#[derive(Deserialize)]
pub struct ListSealsQuery {
    pub chain: Option<String>,
    pub seal_type: Option<String>,
    pub status: Option<String>,
    pub right_id: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// GET /api/v1/seals
pub async fn list_seals(
    Query(query): Query<ListSealsQuery>,
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<PaginatedResponse<csv_explorer_shared::SealRecord>>>, (StatusCode, Json<ErrorResponse>)> {
    let repo = SealsRepository::new(pool);

    let limit = query.limit.unwrap_or(20);
    let offset = query.offset.unwrap_or(0);

    let filter = SealFilter {
        chain: query.chain,
        seal_type: query.seal_type.as_deref().map(|s| match s {
            "utxo" => csv_explorer_shared::SealType::Utxo,
            "object" => csv_explorer_shared::SealType::Object,
            "resource" => csv_explorer_shared::SealType::Resource,
            "nullifier" => csv_explorer_shared::SealType::Nullifier,
            "account" => csv_explorer_shared::SealType::Account,
            _ => csv_explorer_shared::SealType::Utxo,
        }),
        status: query.status.as_deref().map(|s| match s {
            "available" => csv_explorer_shared::SealStatus::Available,
            "consumed" => csv_explorer_shared::SealStatus::Consumed,
            _ => csv_explorer_shared::SealStatus::Available,
        }),
        right_id: query.right_id,
        limit: Some(limit),
        offset: Some(offset),
    };

    let total = repo.count(filter.clone())
        .await
        .map_err(|e| server_error(&e))?;

    let data = repo.list(filter)
        .await
        .map_err(|e| server_error(&e))?;

    Ok(Json(ApiResponse::from(PaginatedResponse {
        data,
        total,
        limit,
        offset,
    })))
}

/// GET /api/v1/seals/:id
pub async fn get_seal(
    Path(id): Path<String>,
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<csv_explorer_shared::SealRecord>>, (StatusCode, Json<ErrorResponse>)> {
    let repo = SealsRepository::new(pool);

    let seal = repo.get(&id)
        .await
        .map_err(|e| server_error(&e))?;

    match seal {
        Some(s) => Ok(Json(ApiResponse::from(s))),
        None => Err(not_found(&format!("Seal {} not found", id))),
    }
}

// ---------------------------------------------------------------------------
// Stats handlers
// ---------------------------------------------------------------------------

/// GET /api/v1/stats
pub async fn get_stats(
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<csv_explorer_shared::ExplorerStats>>, (StatusCode, Json<ErrorResponse>)> {
    let repo = StatsRepository::new(pool);

    let stats = repo.get_stats()
        .await
        .map_err(|e| server_error(&e))?;

    Ok(Json(ApiResponse::from(stats)))
}

// ---------------------------------------------------------------------------
// Chains handlers
// ---------------------------------------------------------------------------

/// GET /api/v1/chains
pub async fn list_chains(
    _state: State<AppState>,
) -> Result<Json<ApiResponse<Vec<csv_explorer_shared::ChainInfo>>>, (StatusCode, Json<ErrorResponse>)> {
    // In production, this would query the indexer for current chain status
    Ok(Json(ApiResponse::from(Vec::new())))
}

// ---------------------------------------------------------------------------
// Health check
// ---------------------------------------------------------------------------

/// GET /health
pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "service": "csv-explorer-api"
    }))
}

// ---------------------------------------------------------------------------
// Error helpers
// ---------------------------------------------------------------------------

fn server_error(e: &ExplorerError) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(ErrorResponse {
            error: e.to_string(),
            success: false,
        }),
    )
}

fn not_found(message: &str) -> (StatusCode, Json<ErrorResponse>) {
    (
        StatusCode::NOT_FOUND,
        Json(ErrorResponse {
            error: message.to_string(),
            success: false,
        }),
    )
}
