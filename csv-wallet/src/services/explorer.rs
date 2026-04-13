//! Explorer integration service.

use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Explorer service configuration.
pub struct ExplorerConfig {
    /// Base URL for the CSV Explorer
    pub base_url: String,
}

impl Default for ExplorerConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8080".to_string(),
        }
    }
}

/// Explorer service for querying on-chain data.
pub struct ExplorerService {
    client: Client,
    config: ExplorerConfig,
}

impl ExplorerService {
    /// Create new explorer service.
    pub fn new(config: ExplorerConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    /// Get right details by ID.
    pub async fn get_right(&self, right_id: &str) -> Result<RightInfo, String> {
        let url = format!("{}/api/rights/{}", self.config.base_url, right_id);
        
        self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch right: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Get seals by owner address.
    pub async fn get_seals_by_owner(&self, address: &str) -> Result<Vec<SealInfo>, String> {
        let url = format!("{}/api/seals?owner={}", self.config.base_url, address);
        
        self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch seals: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Get transfer history.
    pub async fn get_transfers(&self, address: &str) -> Result<Vec<TransferInfo>, String> {
        let url = format!("{}/api/transfers?address={}", self.config.base_url, address);
        
        self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch transfers: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }
}

/// Right information from explorer.
#[derive(Debug, Serialize, Deserialize)]
pub struct RightInfo {
    pub id: String,
    pub chain: String,
    pub commitment: String,
    pub owner: String,
    pub seal_id: Option<String>,
    pub created_at: String,
}

/// Seal information from explorer.
#[derive(Debug, Serialize, Deserialize)]
pub struct SealInfo {
    pub id: String,
    pub chain: String,
    pub status: String,
    pub right_id: Option<String>,
    pub created_at: String,
}

/// Transfer information from explorer.
#[derive(Debug, Serialize, Deserialize)]
pub struct TransferInfo {
    pub id: String,
    pub right_id: String,
    pub from_chain: String,
    pub to_chain: String,
    pub from_address: String,
    pub to_address: String,
    pub status: String,
    pub created_at: String,
}
