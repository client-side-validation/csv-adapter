//! Transfer coordinator — single source of truth for cross-chain transfer execution
//!
//! All applications (CLI, wallet, SDK) MUST use this coordinator.
//! No application may implement its own transfer execution.

#![allow(missing_docs)]

use csv_core::proof::ReplayId;

use crate::adapter_registry::{AdapterRegistry, CrossChainTransfer};
use crate::error::TransferCoordinatorError;
use crate::event_bus::{EventBus, TransferEvent};
use crate::replay_db::ReplayDatabase;

/// Receipt returned after a successful transfer
#[derive(Debug, Clone)]
pub struct TransferReceipt {
    /// Transfer ID
    pub transfer_id: String,
    /// Replay ID used for this transfer
    pub replay_id: ReplayId,
    /// Transaction hash of the lock on source chain
    pub lock_tx_hash: String,
    /// Transaction hash of the mint on destination chain
    pub mint_tx_hash: String,
}

/// The single source of truth for cross-chain transfer execution.
pub struct TransferCoordinator {
    replay_db: Box<dyn ReplayDatabase>,
    event_bus: EventBus,
}

impl TransferCoordinator {
    /// Create a new transfer coordinator
    pub fn new(replay_db: Box<dyn ReplayDatabase>, event_bus: EventBus) -> Self {
        Self {
            replay_db,
            event_bus,
        }
    }

    /// Execute a cross-chain transfer through the complete state machine.
    ///
    /// Preconditions checked by this function:
    /// 1. ReplayId is unique (not in replay_db)
    /// 2. Source chain capabilities permit cross-chain source
    /// 3. Destination chain capabilities permit mint
    ///
    /// This function is the ONLY place that may call `mint_sanad_on_chain`.
    pub async fn execute(
        &self,
        transfer: CrossChainTransfer,
        adapter_registry: &dyn AdapterRegistry,
    ) -> Result<TransferReceipt, TransferCoordinatorError> {
        // Step 1: Compute ReplayId and check for replay
        let replay_id = ReplayId::derive(
            transfer.source_chain.as_str(),
            &transfer.lock_tx_hash,
            transfer.lock_output_index,
            transfer.sanad_id.as_bytes(),
            &transfer.transition_id,
            transfer.destination_chain.as_str(),
        );

        if self.replay_db.contains(&replay_id).await.map_err(|e| {
            TransferCoordinatorError::RuntimeError(e.to_string())
        })? {
            self.event_bus
                .emit(TransferEvent::ReplayDetected {
                    transfer_id: transfer.id.clone(),
                });
            return Err(TransferCoordinatorError::ReplayDetected(replay_id));
        }

        // Step 2: Verify source chain capabilities
        let src_caps = adapter_registry
            .capabilities(&transfer.source_chain)
            .ok_or(TransferCoordinatorError::UnknownChain(
                transfer.source_chain.clone(),
            ))?;

        if !src_caps.can_authorize_mint() {
            return Err(TransferCoordinatorError::UnsupportedOperation(
                format!("{} cannot be a cross-chain source", transfer.source_chain),
            ));
        }

        // Step 3: Verify destination chain capabilities
        let dst_caps = adapter_registry
            .capabilities(&transfer.destination_chain)
            .ok_or(TransferCoordinatorError::UnknownChain(
                transfer.destination_chain.clone(),
            ))?;

        if !dst_caps.can_authorize_mint() {
            return Err(TransferCoordinatorError::UnsupportedOperation(
                format!(
                    "{} cannot be a cross-chain destination",
                    transfer.destination_chain
                ),
            ));
        }

        // Step 4: Lock on source chain and await finality
        self.event_bus
            .emit(TransferEvent::Locking {
                transfer_id: transfer.id.clone(),
            });

        let lock_result = adapter_registry
            .lock_sanad(&transfer.source_chain, &transfer)
            .await
            .map_err(|e| TransferCoordinatorError::LockFailed(e.to_string()))?;

        self.event_bus
            .emit(TransferEvent::AwaitingFinality {
                transfer_id: transfer.id.clone(),
            });
        let _finality_proof = adapter_registry
            .verify_finality(&transfer.source_chain, lock_result.block_height)
            .await
            .map_err(|e| TransferCoordinatorError::FinalityFailed(e.to_string()))?;

        // Step 5: Build inclusion proof
        self.event_bus
            .emit(TransferEvent::BuildingProof {
                transfer_id: transfer.id.clone(),
            });
        let proof_bundle = adapter_registry
            .build_inclusion_proof(&transfer.source_chain, &lock_result)
            .await
            .map_err(|e| TransferCoordinatorError::ProofBuildFailed(e.to_string()))?;

        // Step 6: Verify proof before mint
        let verification = adapter_registry
            .verify_proof_bundle(&proof_bundle)
            .await
            .map_err(TransferCoordinatorError::VerificationFailed)?;

        verification
            .meets_chain_thresholds(src_caps)
            .map_err(TransferCoordinatorError::VerificationFailed)?;

        // Step 7: Record ReplayId BEFORE minting (prevents duplicate mints on retry)
        self.replay_db
            .insert_if_absent(&replay_id, crate::replay_db::ReplayEntryState::Pending)
            .await
            .map_err(|e| match e {
                crate::replay_db::ReplayDbError::AlreadyExists => {
                    TransferCoordinatorError::ReplayDetected(replay_id)
                }
                crate::replay_db::ReplayDbError::Storage(msg) => {
                    TransferCoordinatorError::RuntimeError(msg)
                }
            })?;

        // Step 8: Mint on destination chain
        self.event_bus
            .emit(TransferEvent::Minting {
                transfer_id: transfer.id.clone(),
            });
        let mint_result = adapter_registry
            .mint_sanad(&transfer.destination_chain, &transfer, &proof_bundle)
            .await
            .map_err(|e| TransferCoordinatorError::MintFailed(e.to_string()))?;

        // Confirm the replay entry as consumed
        self.replay_db
            .confirm_consumed(&replay_id)
            .await
            .map_err(|e| TransferCoordinatorError::RuntimeError(e.to_string()))?;

        self.event_bus
            .emit(TransferEvent::Complete {
                transfer_id: transfer.id.clone(),
                mint_tx_hash: mint_result.tx_hash.clone(),
            });

        Ok(TransferReceipt {
            transfer_id: transfer.id,
            replay_id,
            lock_tx_hash: lock_result.tx_hash,
            mint_tx_hash: mint_result.tx_hash,
        })
    }

    /// Subscribe to transfer events
    pub fn subscribe(&mut self, subscriber: crate::event_bus::EventSubscriber) {
        self.event_bus.subscribe(subscriber);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapter_registry::{AdapterRegistryImpl, ChainAdapter, CrossChainTransfer as RuntimeCrossChainTransfer, LockResult, MintResult};
    use csv_core::chain_config::ChainCapabilities;
    use csv_core::finality::FinalityEvidence;
    use csv_core::proof::{InclusionProof, ProofBundle};
    use csv_core::verified::{
        FinalityStrength, InclusionStrength, VerificationAssurance, VerificationResult, VerifiedComponents,
    };

    struct TestAdapter {
        caps: ChainCapabilities,
    }

    impl TestAdapter {
        fn new() -> Self {
            Self {
                caps: ChainCapabilities::bitcoin(),
            }
        }
    }

    #[async_trait::async_trait]
    impl ChainAdapter for TestAdapter {
        fn chain_id(&self) -> &str {
            "test-chain"
        }

        fn capabilities(&self) -> &ChainCapabilities {
            &self.caps
        }

        async fn lock_sanad(
            &self,
            _transfer: &CrossChainTransfer,
        ) -> Result<LockResult, crate::adapter_registry::AdapterError> {
            Ok(LockResult {
                tx_hash: "0xlock".to_string(),
                block_height: 100,
            })
        }

        async fn mint_sanad(
            &self,
            _transfer: &CrossChainTransfer,
            _proof_bundle: &ProofBundle,
        ) -> Result<MintResult, crate::adapter_registry::AdapterError> {
            Ok(MintResult {
                tx_hash: "0xmint".to_string(),
            })
        }

        async fn build_inclusion_proof(
            &self,
            _lock_result: &LockResult,
        ) -> Result<ProofBundle, crate::adapter_registry::AdapterError> {
            // Return a minimal valid proof bundle
            Ok(ProofBundle {
                transition_dag: csv_core::dag::DAGSegment::new(
                    vec![],
                    csv_core::hash::Hash::new([0u8; 32]),
                ),
                signatures: vec![],
                seal_ref: csv_core::seal::SealPoint::new(vec![0u8; 32], Some(0)).unwrap(),
                anchor_ref: csv_core::seal::CommitAnchor::new(vec![0u8; 32], 0, vec![]).unwrap(),
                inclusion_proof: InclusionProof::new(vec![], csv_core::hash::Hash::new([0u8; 32]), 100, 0).unwrap(),
                finality_proof: csv_core::proof::FinalityProof::new(vec![], 6, false).unwrap(),
            })
        }

        async fn verify_inclusion_proof(
            &self,
            _proof: &ProofBundle,
        ) -> Result<VerificationResult, crate::adapter_registry::AdapterError> {
            Ok(VerificationResult {
                valid: true,
                assurance: VerificationAssurance::ConsensusBound,
                verified_components: VerifiedComponents {
                    inclusion: InclusionStrength::MerklePath,
                    finality: FinalityStrength::Probabilistic { confirmations: 6 },
                    replay_checked: true,
                    ownership_signature: true,
                },
                error: None,
            })
        }

        async fn verify_finality(
            &self,
            _block_height: u64,
        ) -> Result<csv_core::finality::FinalityProof, crate::adapter_registry::AdapterError> {
            Ok(csv_core::finality::FinalityProof {
                chain_id: "test-chain".to_string(),
                block_height: 100,
                finality_evidence: FinalityEvidence::CumulativeWork {
                    header_hash: [0u8; 32],
                    cumulative_work: 0,
                },
                confirmations: 6,
            })
        }

        async fn verify_seal_registry(
            &self,
            _seal_id: &[u8],
        ) -> Result<crate::adapter_registry::SealRegistryStatus, crate::adapter_registry::AdapterError> {
            Ok(crate::adapter_registry::SealRegistryStatus::Available)
        }
    }

    #[tokio::test]
    async fn test_transfer_coordinator_replay_detection() {
        let replay_db = Box::new(crate::replay_db::InMemoryReplayDb::new());
        let event_bus = EventBus::new();
        let coordinator = TransferCoordinator::new(replay_db, event_bus);

        let mut registry = AdapterRegistryImpl::new();
        registry.register(std::sync::Arc::new(TestAdapter::new()));

        let transfer = CrossChainTransfer {
            id: "test-1".to_string(),
            source_chain: "test-chain".to_string(),
            destination_chain: "test-chain".to_string(),
            lock_tx_hash: vec![1u8; 32],
            lock_output_index: 0,
            sanad_id: csv_core::hash::Hash::new([2u8; 32]),
            transition_id: vec![3u8; 32],
        };

        // First transfer should succeed
        let result = coordinator.execute(transfer.clone(), &registry).await;
        assert!(result.is_ok());

        // Second transfer with same parameters should be detected as replay
        let result = coordinator.execute(transfer, &registry).await;
        assert!(matches!(result, Err(TransferCoordinatorError::ReplayDetected(_))));
    }

    #[tokio::test]
    async fn test_transfer_coordinator_capability_gate() {
        let replay_db = Box::new(crate::replay_db::InMemoryReplayDb::new());
        let event_bus = EventBus::new();
        let coordinator = TransferCoordinator::new(replay_db, event_bus);

        let mut registry = AdapterRegistryImpl::new();
        // Register celestia which cannot authorize mints (DA only)
        let celestia_caps = ChainCapabilities::celestia();
        struct CelestiaAdapter {
            caps: ChainCapabilities,
        }
        #[async_trait::async_trait]
        impl ChainAdapter for CelestiaAdapter {
            fn chain_id(&self) -> &str {
                "celestia"
            }
            fn capabilities(&self) -> &ChainCapabilities {
                &self.caps
            }
            async fn lock_sanad(&self, _t: &RuntimeCrossChainTransfer) -> Result<LockResult, crate::adapter_registry::AdapterError> {
                unimplemented!()
            }
            async fn mint_sanad(&self, _t: &RuntimeCrossChainTransfer, _p: &ProofBundle) -> Result<MintResult, crate::adapter_registry::AdapterError> {
                unimplemented!()
            }
            async fn build_inclusion_proof(&self, _l: &LockResult) -> Result<ProofBundle, crate::adapter_registry::AdapterError> {
                unimplemented!()
            }
            async fn verify_inclusion_proof(&self, _p: &ProofBundle) -> Result<VerificationResult, crate::adapter_registry::AdapterError> {
                unimplemented!()
            }
            async fn verify_finality(&self, _h: u64) -> Result<csv_core::finality::FinalityProof, crate::adapter_registry::AdapterError> {
                unimplemented!()
            }
            async fn verify_seal_registry(&self, _s: &[u8]) -> Result<crate::adapter_registry::SealRegistryStatus, crate::adapter_registry::AdapterError> {
                unimplemented!()
            }
        }
        registry.register(std::sync::Arc::new(CelestiaAdapter { caps: celestia_caps }));

        let transfer = RuntimeCrossChainTransfer {
            id: "test-1".to_string(),
            source_chain: "celestia".to_string(),
            destination_chain: "celestia".to_string(),
            lock_tx_hash: vec![1u8; 32],
            lock_output_index: 0,
            sanad_id: csv_core::hash::Hash::new([2u8; 32]),
            transition_id: vec![3u8; 32],
        };

        // Celestia cannot be a source (DA only)
        let result = coordinator.execute(transfer, &registry).await;
        assert!(matches!(
            result,
            Err(TransferCoordinatorError::UnsupportedOperation(_))
        ));
    }
}
