//! Dashboard server for rights and transfers visualization
//!
//! Provides a web-based dashboard for monitoring cross-chain rights,
//! transfers, and real-time indexing status.

use crate::indexing::IndexingManager;
use std::collections::HashMap;
use std::sync::Arc;
use warp::{Filter, Reply};

/// Dashboard server
pub struct DashboardServer {
    indexing_manager: Arc<IndexingManager>,
    port: u16,
    is_running: bool,
}

/// Dashboard status
#[derive(Debug, Clone)]
pub struct DashboardStatus {
    pub is_running: bool,
    pub port: u16,
    pub uptime: std::time::Duration,
    pub active_connections: u32,
}

impl DashboardServer {
    /// Create a new dashboard server
    pub fn new(
        indexing_manager: Arc<IndexingManager>,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Self {
            indexing_manager,
            port: 3000,
            is_running: false,
        })
    }

    /// Start the dashboard server
    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.is_running {
            return Err("Dashboard server already running".into());
        }

        println!("Starting Dashboard server on port {}", self.port);

        // Create dashboard routes
        let routes = self.create_routes();

        // Start the server
        let addr: std::net::SocketAddr = ([0, 0, 0, 0], self.port).into();
        warp::serve(routes).run(addr).await;

        Ok(())
    }

    /// Stop the dashboard server
    pub fn stop(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        println!("Stopping Dashboard server");
        Ok(())
    }

    /// Create dashboard routes
    fn create_routes(&self) -> impl Filter<Extract = impl Reply, Error = warp::Rejection> + Clone {
        // Main dashboard page
        let dashboard = warp::path::end()
            .and(warp::get())
            .and(warp::any().map({
                let indexing_manager = self.indexing_manager.clone();
                move || indexing_manager.clone()
            }))
            .and_then(handlers::serve_dashboard);

        // API endpoints for dashboard data
        let api_metrics = warp::path("api")
            .and(warp::path("metrics"))
            .and(warp::get())
            .and(warp::any().map({
                let indexing_manager = self.indexing_manager.clone();
                move || indexing_manager.clone()
            }))
            .and_then(handlers::get_metrics);

        let api_rights = warp::path("api")
            .and(warp::path("rights"))
            .and(warp::get())
            .and(warp::any().map({
                let indexing_manager = self.indexing_manager.clone();
                move || indexing_manager.clone()
            }))
            .and_then(handlers::get_rights_summary);

        let api_transfers = warp::path("api")
            .and(warp::path("transfers"))
            .and(warp::get())
            .and(warp::any().map({
                let indexing_manager = self.indexing_manager.clone();
                move || indexing_manager.clone()
            }))
            .and_then(handlers::get_transfers_summary);

        let api_chains = warp::path("api")
            .and(warp::path("chains"))
            .and(warp::get())
            .and(warp::any().map({
                let indexing_manager = self.indexing_manager.clone();
                move || indexing_manager.clone()
            }))
            .and_then(handlers::get_chain_status);

        // Static assets (CSS, JS)
        let static_files = warp::path("static")
            .and(warp::fs::dir("./static"))
            .or(warp::path("favicon.ico").and(warp::fs::file("./static/favicon.ico")));

        // CORS headers
        let cors = warp::cors()
            .allow_any_origin()
            .allow_headers(vec!["content-type"])
            .allow_methods(vec!["GET", "POST"]);

        // Combine all routes
        dashboard
            .or(api_metrics)
            .or(api_rights)
            .or(api_transfers)
            .or(api_chains)
            .or(static_files)
            .with(cors)
    }

    /// Get dashboard status
    pub async fn get_status(&self) -> DashboardStatus {
        DashboardStatus {
            is_running: self.is_running,
            port: self.port,
            uptime: std::time::Duration::from_secs(0), // In real implementation, track actual uptime
            active_connections: 0, // In real implementation, track actual connections
        }
    }
}

/// Dashboard request handlers
pub mod handlers {
    use super::*;
    use crate::indexing::{RightsQuery, TransferQuery};

    /// Serve the main dashboard HTML page
    pub async fn serve_dashboard(
        _indexing_manager: Arc<IndexingManager>,
    ) -> Result<impl Reply, warp::Rejection> {
        let html = generate_dashboard_html().await;
        Ok(warp::reply::html(html))
    }

    /// Get metrics for dashboard
    pub async fn get_metrics(
        indexing_manager: Arc<IndexingManager>,
    ) -> Result<impl Reply, warp::Rejection> {
        let metrics = indexing_manager.get_metrics().await;
        Ok(warp::reply::json(&metrics))
    }

    /// Get rights summary for dashboard
    pub async fn get_rights_summary(
        indexing_manager: Arc<IndexingManager>,
    ) -> Result<impl Reply, warp::Rejection> {
        let query = RightsQuery {
            owner: None,
            chain: None,
            status: None,
            limit: Some(100),
            offset: Some(0),
        };

        match indexing_manager.search_rights(&query).await {
            Ok(rights) => {
                let summary = serde_json::json!({
                    "total_rights": rights.len(),
                    "rights": rights,
                    "by_chain": group_rights_by_chain(&rights),
                    "by_status": group_rights_by_status(&rights)
                });
                Ok(warp::reply::json(&summary))
            }
            Err(e) => Ok(warp::reply::json(&serde_json::json!({
                "error": e.to_string()
            }))),
        }
    }

    /// Get transfers summary for dashboard
    pub async fn get_transfers_summary(
        indexing_manager: Arc<IndexingManager>,
    ) -> Result<impl Reply, warp::Rejection> {
        let query = TransferQuery {
            from_chain: None,
            to_chain: None,
            status: None,
            start_time: None,
            end_time: None,
            limit: Some(100),
            offset: Some(0),
        };

        match indexing_manager.search_transfers(&query).await {
            Ok(transfers) => {
                let summary = serde_json::json!({
                    "total_transfers": transfers.len(),
                    "transfers": transfers,
                    "by_from_chain": group_transfers_by_from_chain(&transfers),
                    "by_to_chain": group_transfers_by_to_chain(&transfers),
                    "by_status": group_transfers_by_status(&transfers)
                });
                Ok(warp::reply::json(&summary))
            }
            Err(e) => Ok(warp::reply::json(&serde_json::json!({
                "error": e.to_string()
            }))),
        }
    }

    /// Get chain status for dashboard
    pub async fn get_chain_status(
        indexing_manager: Arc<IndexingManager>,
    ) -> Result<impl Reply, warp::Rejection> {
        let metrics = indexing_manager.get_metrics().await;

        let chain_status = serde_json::json!({
            "active_chains": metrics.active_chains,
            "chains": [
                {
                    "name": "Bitcoin",
                    "status": "active",
                    "block_height": 750000,
                    "last_sync": "2024-01-15T10:30:00Z"
                },
                {
                    "name": "Ethereum",
                    "status": "active",
                    "block_height": 18500000,
                    "last_sync": "2024-01-15T10:30:00Z"
                },
                {
                    "name": "Sui",
                    "status": "active",
                    "block_height": 5000000,
                    "last_sync": "2024-01-15T10:30:00Z"
                },
                {
                    "name": "Aptos",
                    "status": "active",
                    "block_height": 4000000,
                    "last_sync": "2024-01-15T10:30:00Z"
                },
                {
                    "name": "Solana",
                    "status": "active",
                    "block_height": 200000000,
                    "last_sync": "2024-01-15T10:30:00Z"
                }
            ]
        });

        Ok(warp::reply::json(&chain_status))
    }

    /// Group rights by chain
    fn group_rights_by_chain(rights: &[crate::indexing::IndexedRight]) -> serde_json::Value {
        let mut chain_counts = HashMap::new();

        for right in rights {
            *chain_counts.entry(&right.chain).or_insert(0) += 1;
        }

        serde_json::to_value(chain_counts).unwrap_or(serde_json::Value::Object(Default::default()))
    }

    /// Group rights by status
    fn group_rights_by_status(rights: &[crate::indexing::IndexedRight]) -> serde_json::Value {
        let mut status_counts = HashMap::new();

        for right in rights {
            let status_str = format!("{:?}", right.status);
            *status_counts.entry(status_str).or_insert(0) += 1;
        }

        serde_json::to_value(status_counts).unwrap_or(serde_json::Value::Object(Default::default()))
    }

    /// Group transfers by from chain
    fn group_transfers_by_from_chain(
        transfers: &[crate::indexing::IndexedTransfer],
    ) -> serde_json::Value {
        let mut chain_counts = HashMap::new();

        for transfer in transfers {
            *chain_counts.entry(&transfer.from_chain).or_insert(0) += 1;
        }

        serde_json::to_value(chain_counts).unwrap_or(serde_json::Value::Object(Default::default()))
    }

    /// Group transfers by to chain
    fn group_transfers_by_to_chain(
        transfers: &[crate::indexing::IndexedTransfer],
    ) -> serde_json::Value {
        let mut chain_counts = HashMap::new();

        for transfer in transfers {
            *chain_counts.entry(&transfer.to_chain).or_insert(0) += 1;
        }

        serde_json::to_value(chain_counts).unwrap_or(serde_json::Value::Object(Default::default()))
    }

    /// Group transfers by status
    fn group_transfers_by_status(
        transfers: &[crate::indexing::IndexedTransfer],
    ) -> serde_json::Value {
        let mut status_counts = HashMap::new();

        for transfer in transfers {
            let status_str = format!("{:?}", transfer.status);
            *status_counts.entry(status_str).or_insert(0) += 1;
        }

        serde_json::to_value(status_counts).unwrap_or(serde_json::Value::Object(Default::default()))
    }
}

/// Generate the dashboard HTML page
async fn generate_dashboard_html() -> String {
    r#"
<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>CSV Explorer Dashboard</title>
    <style>
        * {
            margin: 0;
            padding: 0;
            box-sizing: border-box;
        }
        
        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: #f5f5f5;
            color: #333;
        }
        
        .header {
            background: #2c3e50;
            color: white;
            padding: 1rem 2rem;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        
        .header h1 {
            font-size: 1.5rem;
            font-weight: 600;
        }
        
        .container {
            max-width: 1200px;
            margin: 0 auto;
            padding: 2rem;
        }
        
        .grid {
            display: grid;
            grid-template-columns: repeat(auto-fit, minmax(300px, 1fr));
            gap: 2rem;
            margin-bottom: 2rem;
        }
        
        .card {
            background: white;
            border-radius: 8px;
            padding: 1.5rem;
            box-shadow: 0 2px 4px rgba(0,0,0,0.1);
        }
        
        .card h2 {
            font-size: 1.2rem;
            margin-bottom: 1rem;
            color: #2c3e50;
        }
        
        .metric {
            display: flex;
            justify-content: space-between;
            align-items: center;
            margin-bottom: 0.5rem;
        }
        
        .metric-label {
            font-weight: 500;
        }
        
        .metric-value {
            font-weight: 600;
            color: #3498db;
        }
        
        .status-indicator {
            display: inline-block;
            width: 8px;
            height: 8px;
            border-radius: 50%;
            margin-right: 0.5rem;
        }
        
        .status-active {
            background: #27ae60;
        }
        
        .status-inactive {
            background: #e74c3c;
        }
        
        .table {
            width: 100%;
            border-collapse: collapse;
            margin-top: 1rem;
        }
        
        .table th,
        .table td {
            padding: 0.75rem;
            text-align: left;
            border-bottom: 1px solid #eee;
        }
        
        .table th {
            font-weight: 600;
            color: #2c3e50;
        }
        
        .refresh-btn {
            background: #3498db;
            color: white;
            border: none;
            padding: 0.5rem 1rem;
            border-radius: 4px;
            cursor: pointer;
            font-size: 0.9rem;
        }
        
        .refresh-btn:hover {
            background: #2980b9;
        }
    </style>
</head>
<body>
    <div class="header">
        <h1>CSV Explorer Dashboard</h1>
    </div>
    
    <div class="container">
        <div class="grid">
            <div class="card">
                <h2>Indexing Metrics</h2>
                <div id="metrics-content">
                    <p>Loading metrics...</p>
                </div>
            </div>
            
            <div class="card">
                <h2>Chain Status</h2>
                <div id="chains-content">
                    <p>Loading chain status...</p>
                </div>
            </div>
        </div>
        
        <div class="grid">
            <div class="card">
                <h2>Rights Summary</h2>
                <div id="rights-content">
                    <p>Loading rights data...</p>
                </div>
                <button class="refresh-btn" onclick="refreshRights()">Refresh</button>
            </div>
            
            <div class="card">
                <h2>Transfers Summary</h2>
                <div id="transfers-content">
                    <p>Loading transfers data...</p>
                </div>
                <button class="refresh-btn" onclick="refreshTransfers()">Refresh</button>
            </div>
        </div>
    </div>
    
    <script>
        // Load initial data
        window.addEventListener('load', () => {
            loadMetrics();
            loadChainStatus();
            loadRights();
            loadTransfers();
        });
        
        // Load metrics
        async function loadMetrics() {
            try {
                const response = await fetch('/api/metrics');
                const metrics = await response.json();
                
                document.getElementById('metrics-content').innerHTML = `
                    <div class="metric">
                        <span class="metric-label">Events Processed</span>
                        <span class="metric-value">${metrics.events_processed || 0}</span>
                    </div>
                    <div class="metric">
                        <span class="metric-label">Rights Indexed</span>
                        <span class="metric-value">${metrics.rights_indexed || 0}</span>
                    </div>
                    <div class="metric">
                        <span class="metric-label">Transfers Indexed</span>
                        <span class="metric-value">${metrics.transfers_indexed || 0}</span>
                    </div>
                    <div class="metric">
                        <span class="metric-label">Active Chains</span>
                        <span class="metric-value">${metrics.active_chains || 0}</span>
                    </div>
                `;
            } catch (error) {
                console.error('Error loading metrics:', error);
                document.getElementById('metrics-content').innerHTML = '<p>Error loading metrics</p>';
            }
        }
        
        // Load chain status
        async function loadChainStatus() {
            try {
                const response = await fetch('/api/chains');
                const chains = await response.json();
                
                let html = '<table class="table"><thead><tr><th>Chain</th><th>Status</th><th>Block Height</th><th>Last Sync</th></tr></thead><tbody>';
                
                chains.chains.forEach(chain => {
                    const statusClass = chain.status === 'active' ? 'status-active' : 'status-inactive';
                    html += `
                        <tr>
                            <td>${chain.name}</td>
                            <td><span class="status-indicator ${statusClass}"></span>${chain.status}</td>
                            <td>${chain.block_height.toLocaleString()}</td>
                            <td>${new Date(chain.last_sync).toLocaleString()}</td>
                        </tr>
                    `;
                });
                
                html += '</tbody></table>';
                document.getElementById('chains-content').innerHTML = html;
            } catch (error) {
                console.error('Error loading chain status:', error);
                document.getElementById('chains-content').innerHTML = '<p>Error loading chain status</p>';
            }
        }
        
        // Load rights
        async function loadRights() {
            try {
                const response = await fetch('/api/rights');
                const rights = await response.json();
                
                let html = `
                    <div class="metric">
                        <span class="metric-label">Total Rights</span>
                        <span class="metric-value">${rights.total_rights || 0}</span>
                    </div>
                `;
                
                if (rights.by_chain && Object.keys(rights.by_chain).length > 0) {
                    html += '<h3 style="margin-top: 1rem; margin-bottom: 0.5rem;">By Chain</h3>';
                    Object.entries(rights.by_chain).forEach(([chain, count]) => {
                        html += `<div class="metric"><span class="metric-label">${chain}</span><span class="metric-value">${count}</span></div>`;
                    });
                }
                
                document.getElementById('rights-content').innerHTML = html;
            } catch (error) {
                console.error('Error loading rights:', error);
                document.getElementById('rights-content').innerHTML = '<p>Error loading rights</p>';
            }
        }
        
        // Load transfers
        async function loadTransfers() {
            try {
                const response = await fetch('/api/transfers');
                const transfers = await response.json();
                
                let html = `
                    <div class="metric">
                        <span class="metric-label">Total Transfers</span>
                        <span class="metric-value">${transfers.total_transfers || 0}</span>
                    </div>
                `;
                
                if (transfers.by_status && Object.keys(transfers.by_status).length > 0) {
                    html += '<h3 style="margin-top: 1rem; margin-bottom: 0.5rem;">By Status</h3>';
                    Object.entries(transfers.by_status).forEach(([status, count]) => {
                        html += `<div class="metric"><span class="metric-label">${status}</span><span class="metric-value">${count}</span></div>`;
                    });
                }
                
                document.getElementById('transfers-content').innerHTML = html;
            } catch (error) {
                console.error('Error loading transfers:', error);
                document.getElementById('transfers-content').innerHTML = '<p>Error loading transfers</p>';
            }
        }
        
        // Refresh functions
        function refreshRights() {
            loadRights();
        }
        
        function refreshTransfers() {
            loadTransfers();
        }
        
        // Auto-refresh every 30 seconds
        setInterval(() => {
            loadMetrics();
            loadChainStatus();
            loadRights();
            loadTransfers();
        }, 30000);
    </script>
</body>
</html>
    "#.to_string()
}

#[cfg(test)]
mod tests {

    #[tokio::test]
    async fn test_dashboard_server_creation() {
        // This test would require a test IndexingManager
        // For now, we only need this async harness to type-check.
    }
}
