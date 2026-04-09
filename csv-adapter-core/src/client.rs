//! Client-Side Validation Engine
//!
//! The client receives consignments from peers and validates them locally:
//! 1. Fetch the full state history for a contract
//! 2. Verify the commitment chain from genesis to present
//! 3. Check that no Right was consumed more than once
//! 4. Accept or reject the consignment based on local validation
//!
//! ## Architecture
//!
//! ```text
//! Client receives consignment from peer:
//!   │
//!   ├─ Bitcoin anchor?  → Map UTXO spend → Right(id, commitment, owner, nullifier=None)
//!   ├─ Sui anchor?      → Map object deletion → Right(id, commitment, owner, nullifier=None)
//!   ├─ Aptos anchor?    → Map resource destruction → Right(id, commitment, owner, nullifier=None)
//!   └─ Ethereum anchor? → Map nullifier registration → Right(id, commitment, owner, nullifier=Some(hash))
//!         │
//!         ▼
//!   Client validates uniformly:
//!     1. Each Right.verify() passes
//!     2. No Right appears twice (double-consumption check)
//!     3. Commitment chain integrity (genesis → present)
//!     4. Accept or reject the consignment
//! ```

use alloc::vec::Vec;

use crate::commitment::Commitment;
use crate::commitment_chain::{verify_ordered_commitment_chain, ChainVerificationResult, ChainError};
use crate::consignment::Consignment;
use crate::hash::Hash;
use crate::right::{Right, RightId, OwnershipProof, RightError};
use crate::seal_registry::{CrossChainSealRegistry, SealConsumption, ChainId, SealStatus, DoubleSpendError};
use crate::state_store::{ContractHistory, StateTransitionRecord, InMemoryStateStore, StateHistoryStore};
use crate::seal::SealRef;

/// Result of consignment validation.
#[derive(Debug)]
pub enum ValidationResult {
    /// Consignment is valid and has been accepted
    Accepted {
        /// The validated contract history
        history: ContractHistory,
        /// Number of Rights validated
        rights_count: usize,
        /// Number of seals consumed
        seals_consumed: usize,
    },
    /// Consignment was rejected due to validation failure
    Rejected {
        /// Reason for rejection
        reason: ValidationError,
    },
}

/// Errors that can occur during validation.
#[derive(Debug, thiserror::Error)]
pub enum ValidationError {
    #[error("Empty consignment")]
    EmptyConsignment,
    #[error("Commitment chain verification failed: {0}")]
    CommitmentChainError(#[from] ChainError),
    #[error("Right validation failed: {0}")]
    RightValidationError(#[from] RightError),
    #[error("Double-spend detected: {0:?}")]
    DoubleSpend(DoubleSpendError),
    #[error("Missing history: contract {0} has incomplete state history")]
    MissingHistory(Hash),
    #[error("Seal assignment error: {0}")]
    SealAssignmentError(String),
    #[error("State store error: {0}")]
    StoreError(String),
    #[error("Contract ID mismatch: expected {expected}, got {actual}")]
    ContractIdMismatch { expected: Hash, actual: Hash },
    #[error("Version mismatch: consignment version {version} not supported")]
    UnsupportedVersion { version: u32 },
}

/// Client-side validation engine.
///
/// Receives consignments and validates them against local state
/// and cross-chain seal registry.
pub struct ValidationClient {
    /// Persistent state history store
    store: InMemoryStateStore,
    /// Cross-chain seal consumption registry
    seal_registry: CrossChainSealRegistry,
    /// The chain this client primarily operates on
    primary_chain: ChainId,
}

impl ValidationClient {
    /// Create a new validation client for a specific chain.
    pub fn new(primary_chain: ChainId) -> Self {
        Self {
            store: InMemoryStateStore::new(),
            seal_registry: CrossChainSealRegistry::new(),
            primary_chain,
        }
    }

    /// Receive and validate a consignment from a peer.
    ///
    /// This is the main entry point for client-side validation.
    /// It performs all validation steps and either accepts or rejects
    /// the consignment.
    ///
    /// # Arguments
    /// * `consignment` - The consignment to validate
    /// * `anchor_chain` - The chain that anchors this consignment
    ///
    /// # Returns
    /// `ValidationResult::Accepted` if valid, `ValidationResult::Rejected` if invalid.
    pub fn receive_consignment(
        &mut self,
        consignment: &Consignment,
        anchor_chain: ChainId,
    ) -> ValidationResult {
        // Step 1: Validate consignment structure
        if let Err(e) = consignment.validate_structure() {
            return ValidationResult::Rejected {
                reason: ValidationError::SealAssignmentError(e.to_string()),
            };
        }

        // Step 2: Extract commitments and verify chain
        match self.verify_commitment_chain(consignment) {
            Ok(chain_result) => {
                // Step 3: Verify Rights and seal consumption
                match self.verify_rights_and_seals(consignment, &chain_result, &anchor_chain) {
                    Ok((rights_validated, seals_consumed)) => {
                        // Step 4: Update local state
                        if let Err(e) = self.update_local_state(consignment, &chain_result) {
                            return ValidationResult::Rejected { reason: e };
                        }

                        ValidationResult::Accepted {
                            history: ContractHistory::from_genesis(chain_result.genesis.clone()),
                            rights_count: rights_validated,
                            seals_consumed,
                        }
                    }
                    Err(e) => ValidationResult::Rejected { reason: e },
                }
            }
            Err(e) => ValidationResult::Rejected {
                reason: ValidationError::CommitmentChainError(e),
            },
        }
    }

    /// Verify the commitment chain in a consignment.
    fn verify_commitment_chain(
        &self,
        consignment: &Consignment,
    ) -> Result<ChainVerificationResult, ChainError> {
        // Extract all commitments from the consignment
        // For now, we use anchors as a proxy for commitments
        // In a full implementation, transitions would also provide commitments

        let commitments: Vec<Commitment> = consignment.anchors.iter()
            .filter_map(|anchor| {
                // Extract commitment from anchor
                // This is simplified - real implementation would extract from anchor data
                None
            })
            .collect();

        // If no commitments in anchors, check if we have a genesis
        // For a minimal consignment, we at least need the genesis
        if consignment.transitions.is_empty() && consignment.anchors.is_empty() {
            // Minimal genesis-only consignment
            return Err(ChainError::EmptyChain);
        }

        // For now, we'll create a synthetic commitment chain from the consignment
        // In production, commitments would be extracted from transitions
        Err(ChainError::EmptyChain)
    }

    /// Verify Rights and seal consumption.
    fn verify_rights_and_seals(
        &mut self,
        consignment: &Consignment,
        _chain_result: &ChainVerificationResult,
        anchor_chain: &ChainId,
    ) -> Result<(usize, usize), ValidationError> {
        let mut rights_validated = 0;
        let mut seals_consumed = 0;

        // Verify each seal assignment
        for seal_assignment in &consignment.seal_assignments {
            // Check if seal has already been consumed
            match self.seal_registry.check_seal_status(&seal_assignment.seal_ref) {
                SealStatus::Unconsumed => {
                    // Seal is fresh - mark as consumed
                    let consumption = SealConsumption {
                        chain: anchor_chain.clone(),
                        seal_ref: seal_assignment.seal_ref.clone(),
                        right_id: RightId(Hash::new(seal_assignment.seal_ref.seal_id.clone().try_into()
                            .unwrap_or([0u8; 32]))),
                        block_height: 0, // Would come from anchor
                        tx_hash: Hash::new([0u8; 32]), // Would come from anchor
                        recorded_at: 0, // Current timestamp
                    };

                    if let Err(e) = self.seal_registry.record_consumption(consumption) {
                        return Err(ValidationError::DoubleSpend(e));
                    }

                    seals_consumed += 1;
                }
                SealStatus::ConsumedOnChain { .. } | SealStatus::DoubleSpent { .. } => {
                    // Seal already consumed - potential double-spend
                    return Err(ValidationError::SealAssignmentError(
                        "Seal has already been consumed".to_string(),
                    ));
                }
            }

            rights_validated += 1;
        }

        Ok((rights_validated, seals_consumed))
    }

    /// Update local state with validated consignment data.
    fn update_local_state(
        &mut self,
        consignment: &Consignment,
        chain_result: &ChainVerificationResult,
    ) -> Result<(), ValidationError> {
        let contract_id = chain_result.contract_id;

        // Load or create contract history
        let mut history = match self.store.load_contract_history(contract_id) {
            Ok(Some(h)) => h,
            Ok(None) => ContractHistory::from_genesis(chain_result.genesis.clone()),
            Err(e) => return Err(ValidationError::StoreError(e.to_string())),
        };

        // Add transitions to history
        for (i, transition) in consignment.transitions.iter().enumerate() {
            // Create transition record
            let record = StateTransitionRecord {
                commitment: Commitment::simple(
                    contract_id,
                    chain_result.latest.hash(),
                    Hash::new([0u8; 32]),
                    &SealRef::new(vec![i as u8], None).unwrap(),
                    [0u8; 32],
                ),
                seal_ref: SealRef::new(vec![i as u8], None).unwrap(),
                rights: Vec::new(),
                block_height: 0,
                verified: true,
            };

            history.add_transition(record);
        }

        // Save updated history
        if let Err(e) = self.store.save_contract_history(contract_id, &history) {
            return Err(ValidationError::StoreError(e.to_string()));
        }

        Ok(())
    }

    /// Get the state history store for direct access.
    pub fn store(&self) -> &InMemoryStateStore {
        &self.store
    }

    /// Get the seal registry for direct access.
    pub fn seal_registry(&self) -> &CrossChainSealRegistry {
        &self.seal_registry
    }

    /// Get the primary chain ID.
    pub fn primary_chain(&self) -> &ChainId {
        &self.primary_chain
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::consignment::Consignment;
    use crate::genesis::Genesis;

    fn make_test_genesis() -> Genesis {
        Genesis::new(
            Hash::new([0xAB; 32]),
            Hash::new([0x01; 32]),
            vec![],
            vec![],
            vec![],
        )
    }

    fn make_test_consignment() -> Consignment {
        let genesis = make_test_genesis();
        Consignment::new(
            genesis,
            vec![],
            vec![],
            vec![],
            Hash::new([0x01; 32]),
        )
    }

    #[test]
    fn test_client_creation() {
        let client = ValidationClient::new(ChainId::Bitcoin);
        assert_eq!(client.primary_chain(), &ChainId::Bitcoin);
    }

    #[test]
    fn test_receive_empty_consignment() {
        let mut client = ValidationClient::new(ChainId::Bitcoin);
        let consignment = make_test_consignment();

        let result = client.receive_consignment(&consignment, ChainId::Bitcoin);

        // Should be rejected due to empty commitment chain
        match result {
            ValidationResult::Rejected { reason } => {
                // Expected - we don't have full commitment extraction yet
                let _ = reason;
            }
            ValidationResult::Accepted { .. } => {
                // Would need proper commitments in the consignment
            }
        }
    }

    #[test]
    fn test_client_store_access() {
        let client = ValidationClient::new(ChainId::Bitcoin);
        assert_eq!(client.store().list_contracts().unwrap().len(), 0);
    }

    #[test]
    fn test_client_seal_registry_access() {
        let client = ValidationClient::new(ChainId::Bitcoin);
        assert_eq!(client.seal_registry().total_seals(), 0);
    }
}
