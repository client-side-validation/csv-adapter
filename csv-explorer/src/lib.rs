//! CSV Explorer - Real-time indexing and visualization for cross-chain rights
//!
//! Provides real-time indexing, API contracts, and dashboard capabilities
//! for monitoring rights, transfers, and proofs across all supported chains.

pub mod indexing;
pub mod api;
pub mod dashboard;

use std::sync::Arc;
use chrono::{DateTime, Utc};
use indexing::IndexingManager;
use api::ExplorerApi;
use dashboard::DashboardServer;

/// Main explorer service
pub struct ExplorerService {
    indexing_manager: Arc<IndexingManager>,
    api_server: Arc<ExplorerApi>,
    dashboard_server: Arc<DashboardServer>,
}

impl ExplorerService {
    /// Create a new explorer service
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let indexing_manager = Arc::new(IndexingManager::new()?);
        let api_server = Arc::new(ExplorerApi::new(indexing_manager.clone())?);
        let dashboard_server = Arc::new(DashboardServer::new(indexing_manager.clone())?);
        
        Ok(Self {
            indexing_manager,
            api_server,
            dashboard_server,
        })
    }
    
    /// Start the explorer service
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("Starting CSV Explorer service...");

        // Start indexing pipeline
        let indexing_manager = self.indexing_manager.clone();
        tokio::spawn(async move {
            if let Err(e) = indexing_manager.start().await {
                eprintln!("Error starting indexing manager: {}", e);
            }
        });

        // Start API server
        self.api_server.start().await?;

        // Start dashboard server
        self.dashboard_server.start().await?;

        println!("Explorer service started successfully");
        println!("API available at: http://localhost:8080");
        println!("Dashboard available at: http://localhost:3000");

        Ok(())
    }
    
    /// Get service status
    pub async fn get_status(&self) -> ExplorerStatus {
        let indexing_metrics = self.indexing_manager.get_metrics().await;
        let api_status = self.api_server.get_status().await;
        let dashboard_status = self.dashboard_server.get_status().await;

        ExplorerStatus {
            indexing_metrics,
            api_status,
            dashboard_status,
            uptime: Utc::now(),
        }
    }
    
    /// Stop the explorer service
    pub async fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("Stopping CSV Explorer service...");
        
        self.api_server.stop()?;
        self.dashboard_server.stop()?;
        
        println!("Explorer service stopped");
        Ok(())
    }
}

/// Explorer service status
#[derive(Debug, Clone)]
pub struct ExplorerStatus {
    pub indexing_metrics: indexing::IndexingMetrics,
    pub api_status: api::ApiStatus,
    pub dashboard_status: dashboard::DashboardStatus,
    pub uptime: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    async fn test_explorer_service_creation() {
        let service = ExplorerService::new().await;
        assert!(service.is_ok());
    }
}
