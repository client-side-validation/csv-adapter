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
// Wallet priority indexing handlers
// ---------------------------------------------------------------------------

/// Request body for registering a wallet address.
#[derive(Deserialize, Serialize)]
pub struct RegisterWalletAddressRequest {
    pub address: String,
    pub chain: String,
    pub network: String,
    pub priority: String,
    pub wallet_id: String,
}

/// POST /api/v1/wallet/addresses
pub async fn register_wallet_address(
    Json(request): Json<RegisterWalletAddressRequest>,
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    use csv_explorer_shared::{Network, PriorityLevel};

    let network = match request.network.to_lowercase().as_str() {
        "mainnet" => Network::Mainnet,
        "testnet" => Network::Testnet,
        "devnet" => Network::Devnet,
        _ => return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid network. Must be: mainnet, testnet, or devnet".to_string(),
                success: false,
            }),
        )),
    };

    let priority = match request.priority.to_lowercase().as_str() {
        "high" => PriorityLevel::High,
        "normal" | "medium" => PriorityLevel::Normal,
        "low" => PriorityLevel::Low,
        _ => return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid priority. Must be: high, normal, or low".to_string(),
                success: false,
            }),
        )),
    };

    // Register the address in the priority repository
    let priority_repo = csv_explorer_storage::repositories::PriorityAddressRepository::new(pool);
    
    priority_repo
        .register_address(&request.address, &request.chain, network, priority, &request.wallet_id)
        .await
        .map_err(|e| server_error(&ExplorerError::Internal(e.to_string())))?;

    Ok(Json(ApiResponse::from(serde_json::json!({
        "message": "Address registered for priority indexing",
        "address": request.address,
        "chain": request.chain,
        "network": request.network,
        "priority": request.priority,
    }))))
}

/// Request body for unregistering a wallet address.
#[derive(Deserialize, Serialize)]
pub struct UnregisterWalletAddressRequest {
    pub address: String,
    pub chain: String,
    pub network: String,
    pub wallet_id: String,
}

/// DELETE /api/v1/wallet/addresses
pub async fn unregister_wallet_address(
    Json(request): Json<UnregisterWalletAddressRequest>,
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    use csv_explorer_shared::Network;

    let network = match request.network.to_lowercase().as_str() {
        "mainnet" => Network::Mainnet,
        "testnet" => Network::Testnet,
        "devnet" => Network::Devnet,
        _ => return Err((
            StatusCode::BAD_REQUEST,
            Json(ErrorResponse {
                error: "Invalid network. Must be: mainnet, testnet, or devnet".to_string(),
                success: false,
            }),
        )),
    };

    let priority_repo = csv_explorer_storage::repositories::PriorityAddressRepository::new(pool);
    
    let removed = priority_repo
        .unregister_address(&request.address, &request.chain, network, &request.wallet_id)
        .await
        .map_err(|e| server_error(&ExplorerError::Internal(e.to_string())))?;

    if !removed {
        return Err(not_found("Address not found or already unregistered"));
    }

    Ok(Json(ApiResponse::from(serde_json::json!({
        "message": "Address unregistered from priority indexing",
        "address": request.address,
        "chain": request.chain,
    }))))
}

/// GET /api/v1/wallet/{wallet_id}/addresses
pub async fn get_wallet_addresses(
    Path(wallet_id): Path<String>,
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<Vec<csv_explorer_shared::PriorityAddress>>>, (StatusCode, Json<ErrorResponse>)> {
    let priority_repo = csv_explorer_storage::repositories::PriorityAddressRepository::new(pool);
    
    let addresses = priority_repo
        .get_addresses_by_wallet(&wallet_id)
        .await
        .map_err(|e| server_error(&ExplorerError::Internal(e.to_string())))?;

    Ok(Json(ApiResponse::from(addresses)))
}

/// GET /api/v1/wallet/address/{address}/data
pub async fn get_address_data(
    Path(address): Path<String>,
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<serde_json::Value>>, (StatusCode, Json<ErrorResponse>)> {
    use csv_explorer_shared::{RightFilter, SealFilter, TransferFilter};

    // Get rights for this address
    let rights_repo = RightsRepository::new(pool.clone());
    let rights_filter = RightFilter {
        owner: Some(address.clone()),
        limit: Some(100),
        offset: Some(0),
        chain: None,
        status: None,
    };
    let rights = rights_repo
        .list(rights_filter)
        .await
        .map_err(|e| server_error(&e))?;

    // Get seals for this address
    let seals_repo = SealsRepository::new(pool.clone());
    // Note: SealFilter doesn't have owner field, so we'll get all seals
    let seals_filter = SealFilter {
        limit: Some(100),
        offset: Some(0),
        chain: None,
        seal_type: None,
        status: None,
        right_id: None,
    };
    let seals = seals_repo
        .list(seals_filter)
        .await
        .map_err(|e| server_error(&e))?;

    // Get transfers for this address
    let transfers_repo = TransfersRepository::new(pool.clone());
    let transfers_filter = TransferFilter {
        limit: Some(100),
        offset: Some(0),
        right_id: None,
        from_chain: None,
        to_chain: None,
        status: None,
    };
    let transfers = transfers_repo
        .list(transfers_filter)
        .await
        .map_err(|e| server_error(&e))?;

    // Filter transfers where this address is involved
    let filtered_transfers: Vec<_> = transfers.into_iter()
        .filter(|t| t.from_owner == address || t.to_owner == address)
        .collect();

    Ok(Json(ApiResponse::from(serde_json::json!({
        "address": address,
        "rights": rights,
        "seals": seals,
        "transfers": filtered_transfers,
        "summary": {
            "total_rights": rights.len(),
            "total_seals": seals.len(),
            "total_transfers": filtered_transfers.len(),
        }
    }))))
}

/// GET /api/v1/wallet/address/{address}/rights
pub async fn get_address_rights(
    Path(address): Path<String>,
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<Vec<csv_explorer_shared::RightRecord>>>, (StatusCode, Json<ErrorResponse>)> {
    let repo = RightsRepository::new(pool);

    let filter = RightFilter {
        owner: Some(address),
        limit: Some(100),
        offset: Some(0),
        chain: None,
        status: None,
    };

    let rights = repo
        .list(filter)
        .await
        .map_err(|e| server_error(&e))?;

    Ok(Json(ApiResponse::from(rights)))
}

/// GET /api/v1/wallet/address/{address}/seals
pub async fn get_address_seals(
    Path(address): Path<String>,
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<Vec<csv_explorer_shared::SealRecord>>>, (StatusCode, Json<ErrorResponse>)> {
    let repo = SealsRepository::new(pool);

    let filter = SealFilter {
        limit: Some(100),
        offset: Some(0),
        chain: None,
        seal_type: None,
        status: None,
        right_id: None,
    };

    let seals = repo
        .list(filter)
        .await
        .map_err(|e| server_error(&e))?;

    Ok(Json(ApiResponse::from(seals)))
}

/// GET /api/v1/wallet/address/{address}/transfers
pub async fn get_address_transfers(
    Path(address): Path<String>,
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<Vec<csv_explorer_shared::TransferRecord>>>, (StatusCode, Json<ErrorResponse>)> {
    let repo = TransfersRepository::new(pool);

    let filter = TransferFilter {
        limit: Some(100),
        offset: Some(0),
        right_id: None,
        from_chain: None,
        to_chain: None,
        status: None,
    };

    let transfers = repo
        .list(filter)
        .await
        .map_err(|e| server_error(&e))?;

    // Filter transfers where this address is involved
    let filtered_transfers: Vec<_> = transfers.into_iter()
        .filter(|t| t.from_owner == address || t.to_owner == address)
        .collect();

    Ok(Json(ApiResponse::from(filtered_transfers)))
}

/// GET /api/v1/wallet/priority/status
pub async fn get_priority_indexing_status(
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<csv_explorer_shared::PriorityIndexingStatus>>, (StatusCode, Json<ErrorResponse>)> {
    let priority_repo = csv_explorer_storage::repositories::PriorityAddressRepository::new(pool);
    
    let status = priority_repo
        .get_priority_indexing_status()
        .await
        .map_err(|e| server_error(&ExplorerError::Internal(e.to_string())))?;

    Ok(Json(ApiResponse::from(status)))
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

// ---------------------------------------------------------------------------
// Enhanced rights and proof metadata handlers
// ---------------------------------------------------------------------------

/// Query parameters for listing enhanced rights.
#[derive(Deserialize)]
pub struct EnhancedRightsQuery {
    pub chain: Option<String>,
    pub owner: Option<String>,
    pub commitment_scheme: Option<String>,
    pub inclusion_proof_type: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// GET /api/v1/rights/enhanced
pub async fn list_enhanced_rights(
    Query(query): Query<EnhancedRightsQuery>,
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<Vec<csv_explorer_shared::EnhancedRightRecord>>>, (StatusCode, Json<ErrorResponse>)> {
    use csv_explorer_shared::RightProofFilter;
    use csv_explorer_storage::repositories::AdvancedProofRepository;

    let repo = AdvancedProofRepository::new(pool);

    let filter = RightProofFilter {
        chain: query.chain,
        owner: query.owner,
        commitment_scheme: query.commitment_scheme.as_deref().and_then(|s| csv_explorer_shared::CommitmentScheme::from_str(s)),
        inclusion_proof_type: query.inclusion_proof_type.as_deref().and_then(|s| csv_explorer_shared::InclusionProofType::from_str(s)),
        limit: query.limit,
        offset: query.offset,
    };

    let records = repo
        .query_enhanced_rights(filter)
        .await
        .map_err(|e| server_error(&ExplorerError::Internal(e.to_string())))?;

    Ok(Json(ApiResponse::from(records)))
}

/// GET /api/v1/rights/enhanced/:id
pub async fn get_enhanced_right(
    Path(id): Path<String>,
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<csv_explorer_shared::EnhancedRightRecord>>, (StatusCode, Json<ErrorResponse>)> {
    use csv_explorer_shared::RightProofFilter;
    use csv_explorer_storage::repositories::AdvancedProofRepository;

    let repo = AdvancedProofRepository::new(pool);

    let filter = RightProofFilter {
        chain: None,
        owner: None,
        commitment_scheme: None,
        inclusion_proof_type: None,
        limit: Some(1),
        offset: Some(0),
    };

    let records = repo
        .query_enhanced_rights(filter)
        .await
        .map_err(|e| server_error(&ExplorerError::Internal(e.to_string())))?;

    let record = records.into_iter().find(|r| r.id == id);

    match record {
        Some(r) => Ok(Json(ApiResponse::from(r))),
        None => Err(not_found(&format!("Enhanced right {} not found", id))),
    }
}

/// GET /api/v1/seals/enhanced
pub async fn list_enhanced_seals(
    Query(query): Query<crate::rest::handlers::ListSealsQuery>,
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<Vec<csv_explorer_shared::EnhancedSealRecord>>>, (StatusCode, Json<ErrorResponse>)> {
    use csv_explorer_shared::SealProofFilter;
    use csv_explorer_storage::repositories::AdvancedProofRepository;

    let repo = AdvancedProofRepository::new(pool);

    let filter = SealProofFilter {
        chain: query.chain,
        seal_type: query.seal_type,
        seal_proof_type: None,
        seal_proof_verified: None,
        limit: query.limit,
        offset: query.offset,
    };

    let records = repo
        .query_enhanced_seals(filter)
        .await
        .map_err(|e| server_error(&ExplorerError::Internal(e.to_string())))?;

    Ok(Json(ApiResponse::from(records)))
}

/// GET /api/v1/seals/enhanced/:id
pub async fn get_enhanced_seal(
    Path(id): Path<String>,
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<csv_explorer_shared::EnhancedSealRecord>>, (StatusCode, Json<ErrorResponse>)> {
    use csv_explorer_shared::SealProofFilter;
    use csv_explorer_storage::repositories::AdvancedProofRepository;

    let repo = AdvancedProofRepository::new(pool);

    let filter = SealProofFilter {
        chain: None,
        seal_type: None,
        seal_proof_type: None,
        seal_proof_verified: None,
        limit: Some(1),
        offset: Some(0),
    };

    let records = repo
        .query_enhanced_seals(filter)
        .await
        .map_err(|e| server_error(&ExplorerError::Internal(e.to_string())))?;

    let record = records.into_iter().find(|r| r.id == id);

    match record {
        Some(r) => Ok(Json(ApiResponse::from(r))),
        None => Err(not_found(&format!("Enhanced seal {} not found", id))),
    }
}

/// GET /api/v1/proofs/statistics
pub async fn get_proof_statistics(
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<csv_explorer_shared::ProofStatistics>>, (StatusCode, Json<ErrorResponse>)> {
    use csv_explorer_storage::repositories::AdvancedProofRepository;

    let repo = AdvancedProofRepository::new(pool);

    let stats = repo
        .get_proof_statistics()
        .await
        .map_err(|e| server_error(&ExplorerError::Internal(e.to_string())))?;

    Ok(Json(ApiResponse::from(stats)))
}

/// GET /api/v1/rights/by-scheme/:scheme
pub async fn get_rights_by_scheme(
    Path(scheme): Path<String>,
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<Vec<csv_explorer_shared::EnhancedRightRecord>>>, (StatusCode, Json<ErrorResponse>)> {
    use csv_explorer_shared::{CommitmentScheme, RightProofFilter};
    use csv_explorer_storage::repositories::AdvancedProofRepository;

    let commitment_scheme = CommitmentScheme::from_str(&scheme)
        .ok_or_else(|| not_found(&format!("Unknown commitment scheme: {}", scheme)))?;

    let repo = AdvancedProofRepository::new(pool);

    let filter = RightProofFilter {
        chain: None,
        owner: None,
        commitment_scheme: Some(commitment_scheme),
        inclusion_proof_type: None,
        limit: Some(100),
        offset: Some(0),
    };

    let records = repo
        .query_enhanced_rights(filter)
        .await
        .map_err(|e| server_error(&ExplorerError::Internal(e.to_string())))?;

    Ok(Json(ApiResponse::from(records)))
}

/// GET /api/v1/rights/by-proof/:proof_type
pub async fn get_rights_by_proof_type(
    Path(proof_type): Path<String>,
    State((_, pool)): State<AppState>,
) -> Result<Json<ApiResponse<Vec<csv_explorer_shared::EnhancedRightRecord>>>, (StatusCode, Json<ErrorResponse>)> {
    use csv_explorer_shared::{InclusionProofType, RightProofFilter};
    use csv_explorer_storage::repositories::AdvancedProofRepository;

    let inclusion_proof_type = InclusionProofType::from_str(&proof_type)
        .ok_or_else(|| not_found(&format!("Unknown inclusion proof type: {}", proof_type)))?;

    let repo = AdvancedProofRepository::new(pool);

    let filter = RightProofFilter {
        chain: None,
        owner: None,
        commitment_scheme: None,
        inclusion_proof_type: Some(inclusion_proof_type),
        limit: Some(100),
        offset: Some(0),
    };

    let records = repo
        .query_enhanced_rights(filter)
        .await
        .map_err(|e| server_error(&ExplorerError::Internal(e.to_string())))?;

    Ok(Json(ApiResponse::from(records)))
}
