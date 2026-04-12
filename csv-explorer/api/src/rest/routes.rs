/// REST API routes for the CSV Explorer.

use axum::{
    routing::get,
    Router,
};

use super::handlers;

/// Build the REST API router.
pub fn rest_routes(state: handlers::AppState) -> Router {
    Router::new()
        // Health
        .route("/health", get(handlers::health_check))
        // Rights
        .route("/rights", get(handlers::list_rights))
        .route("/rights/:id", get(handlers::get_right))
        // Transfers
        .route("/transfers", get(handlers::list_transfers))
        .route("/transfers/:id", get(handlers::get_transfer))
        // Seals
        .route("/seals", get(handlers::list_seals))
        .route("/seals/:id", get(handlers::get_seal))
        // Stats
        .route("/stats", get(handlers::get_stats))
        // Chains
        .route("/chains", get(handlers::list_chains))
        .with_state(state)
}
