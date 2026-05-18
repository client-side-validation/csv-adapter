
use serde::{Deserialize, Serialize};
use crate::protocol_version::ChainId;
use crate::hash::Hash;
use crate::sanad::SanadId as SealId;
use crate::sanad::SanadId as TransferId;

/// The canonical replay consumption state.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ReplayState {
    /// Consumption is in progress.
    Pending,
    /// Consumption finalized successfully.
    Finalized,
    /// Consumption was rolled back.
    RolledBack,
    /// Tombstoned (permanently marked as invalid/unusable).
    Tombstoned,
}

/// Global replay registry record linking a consumed seal to the transfer that
/// consumed it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct GlobalReplayRecord {
    /// The consumed seal identifier.
    pub seal_id: SealId,
    /// Chain that originated the seal.
    pub originating_chain: ChainId,
    /// Transfer that consumed this seal.
    pub consumed_by_transfer: TransferId,
    /// Hash of the proof used to consume the seal.
    pub consumption_proof_hash: Hash,
    /// Current canonical replay state.
    pub state: ReplayState,
}

impl GlobalReplayRecord {
    /// Idempotent consume-if-unconsumed operation.
    ///
    /// This is the ONLY safe way to consume a seal. It uses atomic
    /// compare-and-swap semantics to prevent double-consume races.
    ///
    /// Returns Ok(true) if the seal was successfully consumed.
    /// Returns Ok(false) if the seal was already consumed.
    /// Returns Err if the operation failed due to a database error.
    pub fn consume_if_unconsumed(&mut self, transfer_id: TransferId, proof_hash: Hash) -> Result<bool, String> {
        match self.state {
            ReplayState::Pending | ReplayState::RolledBack => {
                // Seal is available for consumption
                self.consumed_by_transfer = transfer_id;
                self.consumption_proof_hash = proof_hash;
                self.state = ReplayState::Finalized;
                Ok(true)
            }
            ReplayState::Finalized => {
                // Seal already consumed - idempotent success
                if self.consumed_by_transfer == transfer_id && self.consumption_proof_hash == proof_hash {
                    // Same transfer consuming again - idempotent
                    Ok(false)
                } else {
                    // Different transfer trying to consume - replay attack
                    Err(format!(
                        "Replay attack detected: seal already consumed by transfer {:?}",
                        self.consumed_by_transfer
                    ))
                }
            }
            ReplayState::Tombstoned => {
                // Seal is permanently invalid
                Err("Seal is tombstoned and cannot be consumed".to_string())
            }
        }
    }

    /// Rollback a consumption atomically.
    ///
    /// This is the ONLY safe way to rollback a consumption. It uses atomic
    /// compare-and-swap semantics to prevent rollback races.
    ///
    /// Returns Ok(true) if the seal was successfully rolled back.
    /// Returns Ok(false) if the seal was not in a consumable state.
    pub fn rollback_consumption(&mut self) -> Result<bool, String> {
        match self.state {
            ReplayState::Finalized => {
                self.state = ReplayState::RolledBack;
                Ok(true)
            }
            ReplayState::Pending => {
                // Was never finalized - no-op
                Ok(false)
            }
            ReplayState::RolledBack => {
                // Already rolled back - idempotent
                Ok(false)
            }
            ReplayState::Tombstoned => {
                // Cannot rollback tombstoned seals
                Err("Cannot rollback tombstoned seal".to_string())
            }
        }
    }

    /// Tombstone a seal permanently.
    ///
    /// This marks a seal as permanently invalid and unusable.
    pub fn tombstone(&mut self) -> Result<(), String> {
        match self.state {
            ReplayState::Tombstoned => {
                // Already tombstoned - idempotent
                Ok(())
            }
            _ => {
                self.state = ReplayState::Tombstoned;
                Ok(())
            }
        }
    }
}
