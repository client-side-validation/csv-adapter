//! Explorer API contracts for wallet integration
//!
//! Provides REST API endpoints for wallet-to-explorer communication
//! and real-time transfer monitoring.

use crate::indexing::{IndexedSanad, IndexedTransfer, IndexingManager, IndexingMetrics};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio_tungstenite::WebSocketStream;
use warp::{Filter, Reply};

pub mod handlers;
pub mod websocket;

/// Explorer API server
pub struct ExplorerApi {
    indexing_manager: Arc<IndexingManager>,
    port: u16,
    is_running: bool,
}

/// API status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiStatus {
    pub is_running: bool,
    pub port: u16,
    pub uptime: std::time::Duration,
    pub requests_served: u64,
    pub average_response_time: std::time::Duration,
}

/// API response wrapper
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub error: Option<String>,
    pub timestamp: std::time::SystemTime,
}

impl<T> ApiResponse<T> {
    /// Create a successful response
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            timestamp: std::time::SystemTime::now(),
        }
    }

    /// Create an error response
    pub fn error(error: String) -> Self {
        Self {
            success: false,
            data: None,
            error: Some(error),
            timestamp: std::time::SystemTime::now(),
        }
    }
}

/// Sanads search request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanadsSearchRequest {
    pub owner: Option<String>,
    pub chain: Option<String>,
    pub status: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Transfers search request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransfersSearchRequest {
    pub from_chain: Option<String>,
    pub to_chain: Option<String>,
    pub status: Option<String>,
    pub start_time: Option<i64>, // Unix timestamp
    pub end_time: Option<i64>,   // Unix timestamp
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

/// Sanads response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanadsResponse {
    pub sanads: Vec<IndexedSanad>,
    pub total_count: u64,
    pub has_more: bool,
}

/// Transfers response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransfersResponse {
    pub transfers: Vec<IndexedTransfer>,
    pub total_count: u64,
    pub has_more: bool,
}

/// Real-time transfer monitoring request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferMonitorRequest {
    pub sanad_id: Option<String>,
    pub owner: Option<String>,
    pub chains: Option<Vec<String>>,
}

/// Transfer status update
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransferStatusUpdate {
    pub transfer_id: String,
    pub sanad_id: String,
    pub status: String,
    pub from_chain: String,
    pub to_chain: String,
    pub timestamp: std::time::SystemTime,
    pub metadata: serde_json::Value,
}

impl ExplorerApi {
    /// Create a new API server
    pub fn new(
        indexing_manager: Arc<IndexingManager>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            indexing_manager,
            port: 8080,
            is_running: false,
        })
    }

    /// Start the API server
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_running {
            return Err("API server already running".into());
        }

        println!("Starting Explorer API server on port {}", self.port);

        // Create API routes
        let routes = self.create_routes();

        // Start the server
        let addr: std::net::SocketAddr = ([0, 0, 0, 0], self.port).into();
        warp::serve(routes).run(addr).await;

        Ok(())
    }

    /// Stop the API server
    pub fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("Stopping Explorer API server");
        // In a real implementation, we'd use a graceful shutdown mechanism
        Ok(())
    }

    /// Create API routes
    fn create_routes(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        let indexing_manager = self.indexing_manager.clone();

        // Health check endpoint
        let health = warp::path("health")
            .and(warp::get())
            .map(|| warp::reply::json(&ApiResponse::success("OK")));

        // Get sanads by owner
        let sanads_by_owner = {
            let indexing_manager = indexing_manager.clone();
            warp::path("sanads")
                .and(warp::path("owner"))
                .and(warp::path::param::<String>())
                .and(warp::get())
                .and(warp::any().map(move || indexing_manager.clone()))
                .and_then(handlers::get_sanads_by_owner)
        };

        // Search sanads
        let search_sanads = {
            let indexing_manager = indexing_manager.clone();
            warp::path("sanads")
                .and(warp::path("search"))
                .and(warp::post())
                .and(warp::body::json())
                .and(warp::any().map(move || indexing_manager.clone()))
                .and_then(handlers::search_sanads)
        };

        // Get transfer by hash
        let transfer_by_hash = {
            let indexing_manager = indexing_manager.clone();
            warp::path("transfers")
                .and(warp::path::param::<String>())
                .and(warp::get())
                .and(warp::any().map(move || indexing_manager.clone()))
                .and_then(handlers::get_transfer_by_hash)
        };

        // Search transfers
        let search_transfers = {
            let indexing_manager = indexing_manager.clone();
            warp::path("transfers")
                .and(warp::path("search"))
                .and(warp::post())
                .and(warp::body::json())
                .and(warp::any().map(move || indexing_manager.clone()))
                .and_then(handlers::search_transfers)
        };

        // Get indexing metrics
        let metrics = {
            let indexing_manager = indexing_manager.clone();
            warp::path("metrics")
                .and(warp::get())
                .and(warp::any().map(move || indexing_manager.clone()))
                .and_then(handlers::get_metrics)
        };

        // WebSocket for real-time updates
        let websocket = {
            let indexing_manager = indexing_manager.clone();
            warp::path("ws")
                .and(warp::ws())
                .and(warp::any().map(move || indexing_manager.clone()))
                .map(|ws: warp::ws::Ws, indexing_manager| {
                    ws.on_upgrade(move |websocket| {
                        websocket::handle_websocket(websocket, indexing_manager)
                    })
                })
        };

        // CORS headers
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["content-type"])
            .allow_methods(vec!["GET", "POST", "PUT", "DELETE"]);

        // Combine all routes
        health
            .or(sanads_by_owner)
            .or(search_sanads)
            .or(transfer_by_hash)
            .or(search_transfers)
            .or(metrics)
            .or(websocket)
            .with(cors)
    }

    /// Get API status
    pub async fn get_status(&self) -> ApiStatus {
        ApiStatus {
            is_running: self.is_running,
            port: self.port,
            uptime: std::time::Duration::from_secs(0), // In real implementation, track actual uptime
            requests_served: 0, // In real implementation, track actual requests
            average_response_time: std::time::Duration::from_millis(0),
        }
    }
}

/// API client for wallet integration
pub struct ExplorerApiClient {
    base_url: String,
    client: reqwest::Client,
}

impl ExplorerApiClient {
    /// Create a new API client
    pub fn new(base_url: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Get sanads by owner
    pub async fn get_sanads_by_owner(
        &self,
        owner: &str,
    ) -> Result<SanadsResponse, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/sanads/owner/{}", self.base_url, owner);
        let response = self.client.get(&url).send().await?;
        let api_response: ApiResponse<SanadsResponse> = response.json().await?;

        if api_response.success {
            api_response
                .data
                .ok_or_else(|| "No data in response".into())
        } else {
            Err(api_response
                .error
                .unwrap_or_else(|| "Unknown error".to_string())
                .into())
        }
    }

    /// Search sanads
    pub async fn search_sanads(
        &self,
        request: &SanadsSearchRequest,
    ) -> Result<SanadsResponse, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/sanads/search", self.base_url);
        let response = self.client.post(&url).json(request).send().await?;
        let api_response: ApiResponse<SanadsResponse> = response.json().await?;

        if api_response.success {
            api_response
                .data
                .ok_or_else(|| "No data in response".into())
        } else {
            Err(api_response
                .error
                .unwrap_or_else(|| "Unknown error".to_string())
                .into())
        }
    }

    /// Get transfer by hash
    pub async fn get_transfer_by_hash(
        &self,
        hash: &str,
    ) -> Result<Option<IndexedTransfer>, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/transfers/{}", self.base_url, hash);
        let response = self.client.get(&url).send().await?;
        let api_response: ApiResponse<IndexedTransfer> = response.json().await?;

        if api_response.success {
            Ok(api_response.data)
        } else {
            Err(api_response
                .error
                .unwrap_or_else(|| "Unknown error".to_string())
                .into())
        }
    }

    /// Search transfers
    pub async fn search_transfers(
        &self,
        request: &TransfersSearchRequest,
    ) -> Result<TransfersResponse, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/transfers/search", self.base_url);
        let response = self.client.post(&url).json(request).send().await?;
        let api_response: ApiResponse<TransfersResponse> = response.json().await?;

        if api_response.success {
            api_response
                .data
                .ok_or_else(|| "No data in response".into())
        } else {
            Err(api_response
                .error
                .unwrap_or_else(|| "Unknown error".to_string())
                .into())
        }
    }

    /// Get indexing metrics
    pub async fn get_metrics(
        &self,
    ) -> Result<IndexingMetrics, Box<dyn std::error::Error + Send + Sync>> {
        let url = format!("{}/metrics", self.base_url);
        let response = self.client.get(&url).send().await?;
        let metrics: IndexingMetrics = response.json().await?;
        Ok(metrics)
    }

    /// Create WebSocket connection for real-time updates
    pub async fn connect_websocket(
        &self,
    ) -> Result<
        WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
        Box<dyn std::error::Error + Send + Sync>,
    > {
        let ws_url = format!("{}/ws", self.base_url.replace("http", "ws"));
        let (stream, _) = tokio_tungstenite::connect_async(&ws_url).await?;
        Ok(stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_api_response_creation() {
        let success_response = ApiResponse::success("test data");
        assert!(success_response.success);
        assert!(success_response.data.is_some());
        assert!(success_response.error.is_none());

        let error_response: ApiResponse<String> = ApiResponse::error("test error".to_string());
        assert!(!error_response.success);
        assert!(error_response.data.is_none());
        assert!(error_response.error.is_some());
    }

    #[test]
    fn test_api_client_creation() {
        let client = ExplorerApiClient::new("http://localhost:8080");
        assert_eq!(client.base_url, "http://localhost:8080");
    }
}
