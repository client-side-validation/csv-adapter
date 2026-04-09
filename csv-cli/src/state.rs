//! CLI state management — persistent state across CLI invocations

use std::collections::HashMap;
use std::path::Path;

use csv_adapter_core::hash::Hash;
use serde::{Deserialize, Serialize};

use crate::config;

/// A Right tracked by the CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedRight {
    /// Right ID
    pub id: Hash,
    /// Chain where this Right is anchored
    pub chain: config::Chain,
    /// Seal reference (chain-specific)
    pub seal_ref: Vec<u8>,
    /// Current owner
    pub owner: Vec<u8>,
    /// Commitment hash
    pub commitment: Hash,
    /// Nullifier (if consumed)
    pub nullifier: Option<Hash>,
    /// Whether this Right has been consumed
    pub consumed: bool,
}

/// A cross-chain transfer tracked by the CLI
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrackedTransfer {
    /// Transfer ID (hash of source seal + dest chain)
    pub id: Hash,
    /// Source chain
    pub source_chain: config::Chain,
    /// Destination chain
    pub dest_chain: config::Chain,
    /// Right ID being transferred
    pub right_id: Hash,
    /// Source transaction hash
    pub source_tx_hash: Option<Hash>,
    /// Destination transaction hash
    pub dest_tx_hash: Option<Hash>,
    /// Inclusion proof (JSON bytes)
    pub proof: Option<Vec<u8>>,
    /// Transfer status
    pub status: TransferStatus,
    /// Created timestamp
    pub created_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum TransferStatus {
    Initiated,
    Locked,
    ProofGenerated,
    Verified,
    Minted,
    Completed,
    Failed { reason: String },
}

/// Deployed contract info
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployedContract {
    pub chain: config::Chain,
    pub address: String,
    pub tx_hash: String,
    pub deployed_at: u64,
}

/// Persistent CLI state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    /// Tracked Rights
    pub rights: Vec<TrackedRight>,
    /// Tracked transfers
    pub transfers: Vec<TrackedTransfer>,
    /// Deployed contracts
    pub contracts: HashMap<config::Chain, DeployedContract>,
    /// Known addresses per chain
    pub addresses: HashMap<config::Chain, String>,
    /// Seal consumption registry (simplified)
    pub consumed_seals: Vec<Vec<u8>>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            rights: Vec::new(),
            transfers: Vec::new(),
            contracts: HashMap::new(),
            addresses: HashMap::new(),
            consumed_seals: Vec::new(),
        }
    }
}

impl State {
    /// Load state from file
    pub fn load() -> anyhow::Result<Self> {
        let path = state_path();
        if Path::new(&path).exists() {
            let content = std::fs::read_to_string(&path)?;
            let state: State = serde_json::from_str(&content)?;
            Ok(state)
        } else {
            let state = State::default();
            state.save()?;
            Ok(state)
        }
    }

    #[allow(dead_code)]
    /// Save state to file
    pub fn save(&self) -> anyhow::Result<()> {
        let path = state_path();
        if let Some(parent) = Path::new(&path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Add a Right to tracking
    #[allow(dead_code)]
    pub fn add_right(&mut self, right: TrackedRight) {
        self.rights.push(right);
    }

    /// Get a Right by ID
    pub fn get_right(&self, id: &Hash) -> Option<&TrackedRight> {
        self.rights.iter().find(|r| r.id == *id)
    }

    /// Mark a Right as consumed
    #[allow(dead_code)]
    pub fn consume_right(&mut self, id: &Hash) -> anyhow::Result<()> {
        let right = self.rights.iter_mut()
            .find(|r| r.id == *id)
            .ok_or_else(|| anyhow::anyhow!("Right {:?} not found", id))?;
        right.consumed = true;
        Ok(())
    }

    /// Add a transfer to tracking
    pub fn add_transfer(&mut self, transfer: TrackedTransfer) {
        self.transfers.push(transfer);
    }

    /// Get a transfer by ID
    pub fn get_transfer(&self, id: &Hash) -> Option<&TrackedTransfer> {
        self.transfers.iter().find(|t| t.id == *id)
    }

    /// Update transfer status
    #[allow(dead_code)]
    pub fn update_transfer_status(&mut self, id: &Hash, status: TransferStatus) -> anyhow::Result<()> {
        let transfer = self.transfers.iter_mut()
            .find(|t| t.id == *id)
            .ok_or_else(|| anyhow::anyhow!("Transfer {:?} not found", id))?;
        transfer.status = status;
        Ok(())
    }

    /// Check if a seal has been consumed
    pub fn is_seal_consumed(&self, seal_bytes: &[u8]) -> bool {
        self.consumed_seals.contains(&seal_bytes.to_vec())
    }

    /// Record a seal consumption
    pub fn record_seal_consumption(&mut self, seal_bytes: Vec<u8>) {
        if !self.is_seal_consumed(&seal_bytes) {
            self.consumed_seals.push(seal_bytes);
        }
    }

    /// Store deployed contract info
    pub fn store_contract(&mut self, contract: DeployedContract) {
        self.contracts.insert(contract.chain.clone(), contract);
    }

    /// Get deployed contract for a chain
    pub fn get_contract(&self, chain: &config::Chain) -> Option<&DeployedContract> {
        self.contracts.get(chain)
    }

    /// Store an address for a chain
    pub fn store_address(&mut self, chain: config::Chain, address: String) {
        self.addresses.insert(chain, address);
    }

    /// Get address for a chain
    pub fn get_address(&self, chain: &config::Chain) -> Option<&String> {
        self.addresses.get(chain)
    }
}

fn state_path() -> String {
    let data_dir = if let Some(home) = dirs::home_dir() {
        home.join(".csv/data")
    } else {
        std::env::temp_dir().join("csv-data")
    };
    data_dir.join("state.json").to_string_lossy().to_string()
}
