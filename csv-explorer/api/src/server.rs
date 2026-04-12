/// API server setup and configuration.
///
/// Combines GraphQL and REST APIs with CORS, tracing, and metrics.

use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use async_graphql_axum::{GraphQLRequest, GraphQLResponse};
use async_graphql::Schema;
use axum::{
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use sqlx::SqlitePool;
use tower::ServiceBuilder;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

use csv_explorer_storage::init_pool;

use crate::graphql::{create_schema, schema::GraphqlContext};
use crate::rest;
use shared::{ApiConfig, ExplorerConfig, Result};

/// The API server.
pub struct ApiServer {
    config: ApiConfig,
    pool: SqlitePool,
}

impl ApiServer {
    /// Create a new API server.
    pub async fn new(config: ExplorerConfig) -> Result<Self> {
        let pool = init_pool(&config.database.url, config.database.max_connections).await?;
        Ok(Self {
            config: config.api,
            pool,
        })
    }

    /// Start the API server.
    pub async fn start(self) -> Result<()> {
        let schema = create_schema();

        let graphql_handler = |state: SqlitePool| {
            move |req: GraphQLRequest| {
                let schema = schema.clone();
                let pool = state.clone();
                async move {
                    let gql_ctx = GraphqlContext { pool };
                    let response = schema.execute(req.into_inner()).data(gql_ctx).await;
                    GraphQLResponse::from(response)
                }
            }
        };

        let rest_state = rest::handlers::AppState {
            pool: self.pool.clone(),
        };

        let app = Router::new()
            // GraphQL endpoint
            .route(
                "/graphql",
                axum::routing::post(
                    move |req: GraphQLRequest| graphql_handler(self.pool.clone())(req),
                ),
            )
            // GraphQL Playground
            .route("/playground", get(graphql_playground))
            // REST API
            .nest("/api/v1", rest::routes::rest_routes(rest_state))
            // Prometheus metrics
            .route("/metrics", get(metrics_handler))
            // Health check
            .route("/health", get(health_handler))
            // Middleware
            .layer(CorsLayer::permissive())
            .layer(TraceLayer::new_for_http())
            .layer(ServiceBuilder::new());

        let listener = tokio::net::TcpListener::bind(&self.config.bind())
            .await
            .map_err(|e| shared::ExplorerError::Internal(format!("Failed to bind to {}: {}", self.config.bind(), e)))?;

        tracing::info!(addr = %self.config.bind(), "API server started");
        axum::serve(listener, app)
            .await
            .map_err(|e| shared::ExplorerError::Internal(format!("Server error: {}", e)))?;

        Ok(())
    }
}

/// Serve the GraphQL Playground HTML.
async fn graphql_playground() -> impl IntoResponse {
    Html(playground_source(GraphQLPlaygroundConfig::new("/graphql")))
}

/// Serve Prometheus metrics.
async fn metrics_handler() -> impl IntoResponse {
    let metrics = csv_explorer_indexer::metrics::encode_metrics();
    metrics
}

/// Health check handler.
async fn health_handler() -> impl IntoResponse {
    axum::Json(serde_json::json!({
        "status": "ok",
        "service": "csv-explorer-api"
    }))
}
