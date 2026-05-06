//! Explorer integration service.

use csv_core::Chain;
use reqwest::Client;
use serde::{Deserialize, Serialize};

/// Explorer service configuration.
pub struct ExplorerConfig {
    /// Base URL for the CSV Explorer
    pub base_url: String,
    /// Transaction URL template (e.g., "https://mempool.space/tx/{}")
    pub tx_url_template: String,
    /// Address URL template (e.g., "https://mempool.space/address/{}")
    pub address_url_template: String,
}

impl ExplorerConfig {
    /// Get explorer config for a chain (testnet by default).
    pub fn for_chain(chain: Chain) -> Option<Self> {
        match chain {
            Chain::Bitcoin => Some(Self {
                base_url: "https://mempool.space/testnet".to_string(),
                tx_url_template: "https://mempool.space/testnet/tx/{}".to_string(),
                address_url_template: "https://mempool.space/testnet/address/{}".to_string(),
            }),
            Chain::Ethereum => Some(Self {
                base_url: "https://sepolia.etherscan.io".to_string(),
                tx_url_template: "https://sepolia.etherscan.io/tx/{}".to_string(),
                address_url_template: "https://sepolia.etherscan.io/address/{}".to_string(),
            }),
            Chain::Sui => Some(Self {
                base_url: "https://suiscan.xyz/testnet".to_string(),
                tx_url_template: "https://suiscan.xyz/testnet/tx/{}".to_string(),
                address_url_template: "https://suiscan.xyz/testnet/address/{}".to_string(),
            }),
            Chain::Aptos => Some(Self {
                base_url: "https://explorer.aptoslabs.com/testnet".to_string(),
                tx_url_template: "https://explorer.aptoslabs.com/txn/{}?network=testnet"
                    .to_string(),
                address_url_template: "https://explorer.aptoslabs.com/account/{}?network=testnet"
                    .to_string(),
            }),
            Chain::Solana => Some(Self {
                base_url: "https://explorer.solana.com".to_string(),
                tx_url_template: "https://explorer.solana.com/tx/{}?cluster=devnet".to_string(),
                address_url_template: "https://explorer.solana.com/address/{}?cluster=devnet"
                    .to_string(),
            }),
            _ => None,
        }
    }

    /// Get transaction URL for a given tx_hash.
    pub fn tx_url(&self, tx_hash: &str) -> String {
        self.tx_url_template.replace("{}", tx_hash)
    }

    /// Get address URL for a given address.
    pub fn address_url(&self, address: &str) -> String {
        self.address_url_template.replace("{}", address)
    }
}

impl Default for ExplorerConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:8181".to_string(),
            tx_url_template: "http://localhost:8181/tx/{}".to_string(),
            address_url_template: "http://localhost:8181/address/{}".to_string(),
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

    // -----------------------------------------------------------------------
    // Priority indexing APIs
    // -----------------------------------------------------------------------

    /// Register an address for priority indexing.
    pub async fn register_priority_address(
        &self,
        address: &str,
        chain: &str,
        network: &str,
        priority: &str,
        wallet_id: &str,
    ) -> Result<serde_json::Value, String> {
        let url = format!("{}/api/v1/wallet/addresses", self.config.base_url);

        let request = RegisterAddressRequest {
            address: address.to_string(),
            chain: chain.to_string(),
            network: network.to_string(),
            priority: priority.to_string(),
            wallet_id: wallet_id.to_string(),
        };

        self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to register address: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Unregister an address from priority indexing.
    pub async fn unregister_priority_address(
        &self,
        address: &str,
        chain: &str,
        network: &str,
        wallet_id: &str,
    ) -> Result<serde_json::Value, String> {
        let url = format!("{}/api/v1/wallet/addresses", self.config.base_url);

        let request = UnregisterAddressRequest {
            address: address.to_string(),
            chain: chain.to_string(),
            network: network.to_string(),
            wallet_id: wallet_id.to_string(),
        };

        self.client
            .delete(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| format!("Failed to unregister address: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Get all registered addresses for a wallet.
    pub async fn get_wallet_addresses(
        &self,
        wallet_id: &str,
    ) -> Result<Vec<PriorityAddressInfo>, String> {
        let url = format!(
            "{}/api/v1/wallet/{}/addresses",
            self.config.base_url, wallet_id
        );

        let response: WalletAddressesResponse = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch wallet addresses: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(response.data)
    }

    /// Get complete data for an address (rights, seals, transfers).
    pub async fn get_address_data(&self, address: &str) -> Result<AddressDataResponse, String> {
        let url = format!(
            "{}/api/v1/wallet/address/{}/data",
            self.config.base_url, address
        );

        self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch address data: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))
    }

    /// Get rights for a specific address.
    pub async fn get_address_rights(&self, address: &str) -> Result<Vec<RightInfo>, String> {
        let url = format!(
            "{}/api/v1/wallet/address/{}/rights",
            self.config.base_url, address
        );

        let response: AddressRightsResponse = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch address rights: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(response.data)
    }

    /// Get seals for a specific address.
    pub async fn get_address_seals(&self, address: &str) -> Result<Vec<SealInfo>, String> {
        let url = format!(
            "{}/api/v1/wallet/address/{}/seals",
            self.config.base_url, address
        );

        let response: AddressSealsResponse = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch address seals: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(response.data)
    }

    /// Get transfers for a specific address.
    pub async fn get_address_transfers(&self, address: &str) -> Result<Vec<TransferInfo>, String> {
        let url = format!(
            "{}/api/v1/wallet/address/{}/transfers",
            self.config.base_url, address
        );

        let response: AddressTransfersResponse = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch address transfers: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(response.data)
    }

    /// Get priority indexing status.
    pub async fn get_priority_indexing_status(&self) -> Result<PriorityIndexingStatusInfo, String> {
        let url = format!("{}/api/v1/wallet/priority/status", self.config.base_url);

        let response: PriorityStatusResponse = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to fetch priority status: {}", e))?
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;

        Ok(response.data)
    }
}

// ---------------------------------------------------------------------------
// Request/Response types for priority indexing
// ---------------------------------------------------------------------------

/// Request to register an address for priority indexing.
#[derive(Debug, Serialize)]
pub struct RegisterAddressRequest {
    pub address: String,
    pub chain: String,
    pub network: String,
    pub priority: String,
    pub wallet_id: String,
}

/// Request to unregister an address from priority indexing.
#[derive(Debug, Serialize)]
pub struct UnregisterAddressRequest {
    pub address: String,
    pub chain: String,
    pub network: String,
    pub wallet_id: String,
}

/// Priority address information.
#[derive(Debug, Deserialize)]
pub struct PriorityAddressInfo {
    pub address: String,
    pub chain: String,
    pub network: String,
    pub priority: String,
    pub wallet_id: String,
    pub registered_at: String,
    pub last_indexed_at: Option<String>,
    pub is_active: bool,
}

/// Response wrapper for wallet addresses.
#[derive(Debug, Deserialize)]
pub struct WalletAddressesResponse {
    pub data: Vec<PriorityAddressInfo>,
    pub success: bool,
}

/// Response wrapper for address data.
#[derive(Debug, Deserialize)]
pub struct AddressDataResponse {
    pub data: serde_json::Value,
    pub success: bool,
}

/// Response wrapper for address rights.
#[derive(Debug, Deserialize)]
pub struct AddressRightsResponse {
    pub data: Vec<RightInfo>,
    pub success: bool,
}

/// Response wrapper for address seals.
#[derive(Debug, Deserialize)]
pub struct AddressSealsResponse {
    pub data: Vec<SealInfo>,
    pub success: bool,
}

/// Response wrapper for address transfers.
#[derive(Debug, Deserialize)]
pub struct AddressTransfersResponse {
    pub data: Vec<TransferInfo>,
    pub success: bool,
}

/// Priority indexing status.
#[derive(Debug, Deserialize)]
pub struct PriorityIndexingStatusInfo {
    pub total_addresses: u64,
    pub active_indexing: u64,
    pub completed_indexing: u64,
    pub recent_activities: Vec<IndexingActivityInfo>,
}

/// Indexing activity information.
#[derive(Debug, Deserialize)]
pub struct IndexingActivityInfo {
    pub address: String,
    pub chain: String,
    pub network: String,
    pub indexed_type: String,
    pub items_count: u64,
    pub timestamp: String,
    pub success: bool,
    pub error: Option<String>,
}

/// Response wrapper for priority status.
#[derive(Debug, Deserialize)]
pub struct PriorityStatusResponse {
    pub data: PriorityIndexingStatusInfo,
    pub success: bool,
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
