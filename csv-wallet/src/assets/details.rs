//! Asset details.
//!
//! Provides detailed information about individual assets.

use csv_adapter_core::{Chain, Right, RightId};
use serde::{Serialize, Deserialize};

/// Detailed asset information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDetails {
    /// Right ID
    pub right_id: String,
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
    /// Create asset details from a Right.
    pub fn from_right(right: &Right, chain: Chain) -> Self {
        Self {
            right_id: format!("{:x}", right.id.0),
            chain,
            commitment: format!("{:x}", right.commitment.0),
            owner_address: hex::encode(&right.owner.owner),
            seal_id: None,
            state_root: right.state_root.map(|h| format!("{:x}", h.0)),
            nullifier: right.nullifier.map(|h| format!("{:x}", h.0)),
            created_at: chrono::Utc::now(),
            last_transfer: None,
            transfer_count: 0,
            value_usd: None,
            metadata: serde_json::json!({}),
        }
    }

    /// Format the right ID for display.
    pub fn format_right_id(&self) -> String {
        if self.right_id.len() > 16 {
            format!("{}...{}", &self.right_id[..8], &self.right_id[self.right_id.len() - 8..])
        } else {
            self.right_id.clone()
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
        format!("{}/right/{}", network.explorer_url(), self.right_id)
    }
}
