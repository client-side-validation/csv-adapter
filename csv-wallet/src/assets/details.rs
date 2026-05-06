//! Asset details.
//!
//! Provides detailed information about individual assets.

use csv_core::{Chain, Sanad, SanadId};
use serde::{Serialize, Deserialize};

/// Detailed asset information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDetails {
    /// Sanad ID
    pub sanad_id: String,
    /// Chain
    pub chain: Chain,
    /// Commitment
    pub commitment: String,
    /// Owner address
    pub owner_address: String,
    /// Seal ID
    pub seal_id: Option<String>,
    /// State root
    pub state_root: Option<String>,
    /// Nullifier (if consumed)
    pub nullifier: Option<String>,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last transfer timestamp
    pub last_transfer: Option<chrono::DateTime<chrono::Utc>>,
    /// Transfer count
    pub transfer_count: u64,
    /// Current value
    pub value_usd: Option<f64>,
    /// Metadata
    pub metadata: serde_json::Value,
}

impl AssetDetails {
    /// Create asset details from a Sanad.
    pub fn from_sanad(sanad: &Sanad, chain: Chain) -> Self {
        Self {
            sanad_id: format!("{:x}", sanad.id.0),
            chain,
            commitment: format!("{:x}", sanad.commitment.0),
            owner_address: hex::encode(&sanad.owner.owner),
            seal_id: None,
            state_root: sanad.state_root.map(|h| format!("{:x}", h.0)),
            nullifier: sanad.nullifier.map(|h| format!("{:x}", h.0)),
            created_at: chrono::Utc::now(),
            last_transfer: None,
            transfer_count: 0,
            value_usd: None,
            metadata: serde_json::json!({}),
        }
    }

    /// Format the sanad ID for display.
    pub fn format_sanad_id(&self) -> String {
        if self.sanad_id.len() > 16 {
            format!("{}...{}", &self.sanad_id[..8], &self.sanad_id[self.sanad_id.len() - 8..])
        } else {
            self.sanad_id.clone()
        }
    }

    /// Format commitment for display.
    pub fn format_commitment(&self) -> String {
        if self.commitment.len() > 16 {
            format!("{}...{}", &self.commitment[..8], &self.commitment[self.commitment.len() - 8..])
        } else {
            self.commitment.clone()
        }
    }

    /// Get explorer URL.
    pub fn explorer_url(&self, network: &crate::chains::ChainNetwork) -> String {
        format!("{}/sanad/{}", network.explorer_url(), self.sanad_id)
    }
}
