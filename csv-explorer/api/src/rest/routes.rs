/// REST API routes for the CSV Explorer.

use axum::{
    routing::get,
    Router,
};

use super::handlers;

type AppState = (async_graphql::Schema<
    crate::graphql::schema::Query,
    crate::graphql::schema::Mutation,
    async_graphql::EmptySubscription,
>, sqlx::SqlitePool);

/// Build the REST API router.
pub fn rest_routes() -> Router<AppState> {
    Router::new()
        // Rights
        .route("/rights", get(handlers::list_rights))
        .route("/rights/{id}", get(handlers::get_right))
        // Transfers
        .route("/transfers", get(handlers::list_transfers))
        .route("/transfers/{id}", get(handlers::get_transfer))
        // Seals
        .route("/seals", get(handlers::list_seals))
        .route("/seals/{id}", get(handlers::get_seal))
        // Stats
        .route("/stats", get(handlers::get_stats))
        // Chains
        .route("/chains", get(handlers::list_chains))
}

/// Build the full API v1 router with prefix.
pub fn api_v1_routes() -> Router<AppState> {
    Router::new()
        .nest("/api/v1", rest_routes())
}
