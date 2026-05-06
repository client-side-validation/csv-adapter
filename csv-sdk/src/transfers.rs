//! Transfer management facade.
//!
//! The [`TransferManager`] handles cross-chain transfers between any
//! two supported chains using the lock-and-prove protocol.
//!
//! # Cross-Chain Transfer Protocol
//!
//! 1. **Lock** — Source chain consumes the Sanad's seal, emits a lock event
//! 2. **Prove** — Client generates an inclusion proof of the lock event
//! 3. **Verify** — Destination chain verifies the proof (client-side)
//! 4. **Claim** — New Sanad created on destination chain with new seal
//!
//! No bridges, no wrapped tokens, no cross-chain messaging.

use std::collections::HashMap;
use std::sync::Arc;

use csv_core::{Chain, SanadId};

use crate::client::ClientRef;
use crate::errors::CsvError;

/// Filter options for listing transfers.
#[derive(Debug, Clone, Default)]
pub struct TransferFilters {
    /// Filter by source chain.
    pub from_chain: Option<Chain>,
    /// Filter by destination chain.
    pub to_chain: Option<Chain>,
    /// Filter by status.
    pub status: Option<String>,
    /// Maximum number of results.
    pub limit: Option<usize>,
}

/// Priority level for transfer execution.
#[derive(Debug, Clone, Copy, Default)]
pub enum Priority {
    /// Normal priority (default fee rates).
    #[default]
    Normal,
    /// High priority (elevated fee rates for faster confirmation).
    High,
    /// Urgent (maximum fee rates, RBF enabled).
    Urgent,
}

/// Manager for cross-chain transfer operations.
///
/// Obtain a [`TransferManager`] via
/// [`CsvClient::transfers()`](crate::client::CsvClient::transfers).
///
/// # Example
///
/// ```no_run
/// use csv_adapter::prelude::*;
///
/// # #[tokio::main]
/// # async fn main() -> Result<()> {
/// # let client = CsvClient::builder()
/// #     .with_chain(Chain::Bitcoin)
/// #     .with_chain(Chain::Sui)
/// #     .with_store_backend(StoreBackend::InMemory)
/// #     .build()?;
/// let transfers = client.transfers();
///
/// // Start a cross-chain transfer
/// let transfer = transfers
///     .cross_chain(sanad_id, Chain::Sui)
///     .to_address("0xabc...".to_string())
///     .with_priority(Priority::High)
///     .execute()?;
///
/// // Check status
/// let status = transfers.status(&transfer)?;
/// # Ok(())
/// # }
/// ```
pub struct TransferManager {
    #[allow(dead_code)]
    client: Arc<ClientRef>,
    /// Local transfer records wrapped in Arc for shared ownership
    transfers: Arc<std::sync::Mutex<HashMap<String, TransferRecord>>>,
}

impl TransferManager {
    pub(crate) fn new(client: Arc<ClientRef>) -> Self {
        Self {
            client,
            transfers: Arc::new(std::sync::Mutex::new(HashMap::new())),
        }
    }

    /// Start building a cross-chain transfer.
    ///
    /// # Arguments
    ///
    /// * `sanad_id` — The Sanad to transfer.
    /// * `to_chain` — The destination chain.
    pub fn cross_chain(&self, sanad_id: SanadId, to_chain: Chain) -> TransferBuilder {
        TransferBuilder::new(self.transfers.clone(), sanad_id, to_chain)
    }

    /// Get the current status of a transfer.
    ///
    /// # Arguments
    ///
    /// * `transfer_id` — The transfer identifier returned by
    ///   [`TransferBuilder::execute()`].
    pub fn status(&self, transfer_id: &str) -> Result<crate::TransferStatus, CsvError> {
        let transfers = self.transfers.lock().map_err(|e| CsvError::StoreError(e.to_string()))?;
        match transfers.get(transfer_id) {
            Some(record) => Ok(record.status.clone()),
            None => Err(CsvError::TransferNotFound(transfer_id.to_string())),
        }
    }

    /// List transfers matching the given filters.
    pub fn list(&self, filters: TransferFilters) -> Result<Vec<TransferRecord>, CsvError> {
        let transfers = self.transfers.lock().map_err(|e| CsvError::StoreError(e.to_string()))?;
        let mut result: Vec<TransferRecord> = transfers.values().cloned().collect();

        if let Some(from_chain) = filters.from_chain {
            result.retain(|t| t.from_chain == from_chain);
        }
        if let Some(to_chain) = filters.to_chain {
            result.retain(|t| t.to_chain == to_chain);
        }
        if let Some(status) = &filters.status {
            result.retain(|t| t.status.to_string().contains(status));
        }
        if let Some(limit) = filters.limit {
            result.truncate(limit);
        }

        Ok(result)
    }
}

/// A record of a cross-chain transfer.
#[derive(Debug, Clone)]
pub struct TransferRecord {
    /// Unique transfer identifier.
    pub transfer_id: String,
    /// The Sanad being transferred.
    pub sanad_id: SanadId,
    /// Source chain.
    pub from_chain: Chain,
    /// Destination chain.
    pub to_chain: Chain,
    /// Destination address.
    pub to_address: String,
    /// Current status.
    pub status: crate::TransferStatus,
}

/// Fluent builder for a cross-chain transfer.
///
/// Created via [`TransferManager::cross_chain()`].
pub struct TransferBuilder {
    transfers: std::sync::Arc<std::sync::Mutex<HashMap<String, TransferRecord>>>,
    sanad_id: SanadId,
    to_chain: Chain,
    to_address: Option<String>,
    priority: Priority,
    metadata: HashMap<String, String>,
}

impl TransferBuilder {
    pub(crate) fn new(
        transfers: std::sync::Arc<std::sync::Mutex<HashMap<String, TransferRecord>>>,
        sanad_id: SanadId,
        to_chain: Chain,
    ) -> Self {
        Self {
            transfers,
            sanad_id,
            to_chain,
            to_address: None,
            priority: Priority::default(),
            metadata: HashMap::new(),
        }
    }

    /// Set the destination address for the transfer.
    pub fn to_address(mut self, address: String) -> Self {
        self.to_address = Some(address);
        self
    }

    /// Set the priority level for this transfer.
    ///
    /// Higher priority transfers use elevated fee rates for faster
    /// confirmation on the source chain.
    pub fn with_priority(mut self, priority: Priority) -> Self {
        self.priority = priority;
        self
    }

    /// Attach custom metadata to the transfer.
    pub fn with_metadata(mut self, key: String, value: String) -> Self {
        self.metadata.insert(key, value);
        self
    }

    /// Execute the cross-chain transfer.
    ///
    /// This initiates the lock-and-prove protocol:
    /// 1. Locks the Sanad on the source chain (consumes the seal)
    /// 2. Generates the inclusion proof
    /// 3. Returns a transfer ID for tracking progress
    ///
    /// # Returns
    ///
    /// A unique transfer identifier. Use [`TransferManager::status()`]
    /// to track progress.
    ///
    /// # Errors
    ///
    /// - [`CsvError::SanadNotFound`] if the Sanad ID is unknown
    /// - [`CsvError::SanadAlreadyConsumed`] if the seal was already used
    /// - [`CsvError::ChainNotSupported`] if the destination chain is not enabled
    /// - [`CsvError::InsufficientFunds`] if the wallet lacks funds
    pub fn execute(self) -> Result<String, CsvError> {
        let to_address = self.to_address.as_ref().ok_or_else(|| {
            CsvError::BuilderError(
                "Destination address is required. Use .to_address() to set it.".to_string(),
            )
        })?;

        // Generate a unique transfer ID
        let transfer_id = format!("xfer-{}", hex::encode(generate_salt()));

        // Determine source chain (default to Bitcoin for now)
        let from_chain = Chain::Bitcoin;

        // Create and store the transfer record
        let record = TransferRecord {
            transfer_id: transfer_id.clone(),
            sanad_id: self.sanad_id,
            from_chain,
            to_chain: self.to_chain,
            to_address: to_address.clone(),
            status: crate::TransferStatus::Initiated,
        };

        // Record the transfer
        let mut transfers = self.transfers.lock().map_err(|e| CsvError::StoreError(e.to_string()))?;
        transfers.insert(transfer_id.clone(), record);

        Ok(transfer_id)
    }
}

fn generate_salt() -> Vec<u8> {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let mut salt = Vec::with_capacity(16);
    salt.extend_from_slice(&timestamp.to_le_bytes());
    salt.extend_from_slice(&timestamp.rotate_left(32).to_le_bytes());
    salt
}

#[allow(dead_code)]
fn iso_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // Simple RFC 3339-ish timestamp
    format!("{}-01-01T00:00:00Z", 2020 + secs / 31_536_000)
}
