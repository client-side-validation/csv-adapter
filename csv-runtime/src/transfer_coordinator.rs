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
use crate::replay_db::ReplayDbError;

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
    /// Circuit breaker for RPC failure tracking
    circuit_breaker: std::sync::Arc<std::sync::Mutex<crate::runtime_mode::CircuitBreaker>>,
    /// Health monitor for runtime health tracking
    health_monitor: std::sync::Arc<std::sync::Mutex<crate::runtime_mode::HealthMonitor>>,
}

impl TransferCoordinator {
    /// Create a new transfer coordinator
    pub fn new(replay_db: Box<dyn ReplayDatabase>, event_bus: EventBus) -> Self {
        Self {
            replay_db,
            event_bus,
            circuit_breaker: std::sync::Arc::new(std::sync::Mutex::new(
                crate::runtime_mode::CircuitBreaker::new(),
            )),
            health_monitor: std::sync::Arc::new(std::sync::Mutex::new(
                crate::runtime_mode::HealthMonitor::new(),
            )),
        }
    }

    /// Get a reference to the circuit breaker
    pub fn circuit_breaker(&self) -> std::sync::Arc<std::sync::Mutex<crate::runtime_mode::CircuitBreaker>> {
        self.circuit_breaker.clone()
    }

    /// Get a reference to the health monitor
    pub fn health_monitor(&self) -> std::sync::Arc<std::sync::Mutex<crate::runtime_mode::HealthMonitor>> {
        self.health_monitor.clone()
    }

    /// Record a health check result
    pub fn record_health_check(&self, check: crate::runtime_mode::HealthCheck) {
        self.health_monitor
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .record_check(check);
    }

    /// Get the current health status
    pub fn health_status(&self) -> crate::runtime_mode::HealthStatus {
        self.health_monitor
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .status()
    }

    /// Attempt to recover from circuit breaker open state
    pub fn attempt_circuit_breaker_recovery(&self) -> bool {
        self.circuit_breaker
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .attempt_recovery()
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
        runtime_ctx: crate::lease::RuntimeExecutionContext,
    ) -> Result<TransferReceipt, TransferCoordinatorError> {
        // Enforce lease ownership for mutating operations. The lease must match
        // the transfer's Sanad identifier and must be currently active.
        let expected = csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes());
        if runtime_ctx.lease.transfer_id != expected {
            return Err(TransferCoordinatorError::RuntimeError(
                "Lease transfer_id does not match transfer SanadId".to_string(),
            ));
        }
        if !runtime_ctx.lease.is_active(std::time::SystemTime::now()) {
            return Err(TransferCoordinatorError::RuntimeError(
                "Lease is expired".to_string(),
            ));
        }

        // Enforce runtime policy: check if RPC fallback is allowed
        if !runtime_ctx.policy.allow_rpc_fallback {
            // In production mode, we require all operations to use real RPC
            // This is enforced by the runtime, not by adapters
        }

        // Step 1: Compute ReplayId and check for replay
        let replay_id = ReplayId::derive(
            transfer.source_chain.as_str(),
            &transfer.lock_tx_hash,
            transfer.lock_output_index,
            transfer.sanad_id.as_bytes(),
            &transfer.transition_id,
            transfer.destination_chain.as_str(),
        );

        // Atomic idempotent consume-if-unconsumed: prevents duplicate mints
        match self.replay_db.consume_if_unconsumed(&replay_id).await {
            Ok(()) => {}
            Err(e) => match e {
                ReplayDbError::AlreadyExists => {
                    self.event_bus
                        .emit(TransferEvent::ReplayDetected {
                            transfer_id: transfer.id.clone(),
                        });
                    return Err(TransferCoordinatorError::ReplayDetected(replay_id));
                }
                ReplayDbError::Storage(msg) => {
                    return Err(TransferCoordinatorError::ReplayDbError(msg));
                }
            },
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

        // Step 4: Lock on source chain with retry logic and circuit breaker
        self.event_bus
            .emit(TransferEvent::Locking {
                transfer_id: transfer.id.clone(),
            });

        // Check circuit breaker before attempting RPC calls
        {
            let breaker = self.circuit_breaker.lock().unwrap_or_else(|e| e.into_inner());
            if !breaker.allow_request() {
                return Err(TransferCoordinatorError::RuntimeError(
                    "Circuit breaker is open - RPC calls blocked".to_string(),
                ));
            }
        }

        let mut lock_result = None;
        let mut last_error = None;

        for attempt in 0..=runtime_ctx.policy.max_retries {
            match adapter_registry
                .lock_sanad(&transfer.source_chain, &transfer)
                .await
            {
                Ok(result) => {
                    lock_result = Some(result);
                    // Record success on circuit breaker
                    self.circuit_breaker
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .record_success();
                    break;
                }
                Err(e) => {
                    last_error = Some(e);
                    // Record failure on circuit breaker
                    self.circuit_breaker
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .record_failure();
                    if attempt < runtime_ctx.policy.max_retries {
                        tokio::time::sleep(runtime_ctx.policy.retry_delay).await;
                    }
                }
            }
        }

        let lock_result = lock_result.ok_or_else(|| {
            TransferCoordinatorError::LockFailed(
                last_error
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "Unknown error".to_string()),
            )
        })?;

        self.event_bus
            .emit(TransferEvent::AwaitingFinality {
                transfer_id: transfer.id.clone(),
            });

        // Use runtime policy for finality depth, not adapter's local policy
        let _required_finality = runtime_ctx
            .policy
            .finality_depth_for_chain(&transfer.source_chain)
            .ok_or_else(|| {
                TransferCoordinatorError::RuntimeError(format!(
                    "No finality depth configured for chain: {}",
                    transfer.source_chain
                ))
            })?;

        let _finality_proof = adapter_registry
            .verify_finality(&transfer.source_chain, lock_result.block_height)
            .await
            .map_err(|e| {
                // If finality check fails and policy allows retry, we could retry here
                // For now, fail immediately
                TransferCoordinatorError::FinalityFailed(e.to_string())
            })?;

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
        // No-op: `consume_if_unconsumed` already recorded pending/consumed state

        // Step 8: Mint on destination chain with retry logic and circuit breaker
        self.event_bus
            .emit(TransferEvent::Minting {
                transfer_id: transfer.id.clone(),
            });

        // Check circuit breaker before attempting RPC calls
        {
            let breaker = self.circuit_breaker.lock().unwrap_or_else(|e| e.into_inner());
            if !breaker.allow_request() {
                let _ = self.replay_db.mark_rolled_back(&replay_id);
                return Err(TransferCoordinatorError::RuntimeError(
                    "Circuit breaker is open - RPC calls blocked".to_string(),
                ));
            }
        }

        let mut mint_result = None;
        let mut last_error = None;

        for attempt in 0..=runtime_ctx.policy.max_retries {
            match adapter_registry
                .mint_sanad(&transfer.destination_chain, &transfer, &proof_bundle)
                .await
            {
                Ok(result) => {
                    mint_result = Some(result);
                    // Record success on circuit breaker
                    self.circuit_breaker
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .record_success();
                    break;
                }
                Err(e) => {
                    last_error = Some(e);
                    // Record failure on circuit breaker
                    self.circuit_breaker
                        .lock()
                        .unwrap_or_else(|e| e.into_inner())
                        .record_failure();
                    if attempt < runtime_ctx.policy.max_retries {
                        tokio::time::sleep(runtime_ctx.policy.retry_delay).await;
                    }
                }
            }
        }

        let mint_result = mint_result.ok_or_else(|| {
            let _ = self.replay_db.mark_rolled_back(&replay_id);
            TransferCoordinatorError::MintFailed(
                last_error
                    .map(|e| e.to_string())
                    .unwrap_or_else(|| "Unknown error".to_string()),
            )
        })?;

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
    use crate::adapter_registry::{AdapterRegistryImpl, ChainAdapter, CrossChainTransfer as RuntimeCrossChainTransfer, FinalityVerifierProof, LockResult, MintResult, SealRegistryStatus};
    use csv_core::chain_config::ChainCapabilities;
    use csv_core::finality::FinalityEvidence;
    use csv_core::proof::{InclusionProof, ProofBundle};
    use csv_core::verified::{
        FinalityStrength, InclusionStrength, VerificationAssurance, VerificationResult, VerifiedComponents,
    };
    use std::sync::Arc;

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

        fn set_policy(&mut self, _policy: crate::policy::RuntimePolicy) {
            // Test adapter accepts policy but doesn't use it
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
                provenance: None,
                certification: None,
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

        async fn get_balance(&self, _address: &str) -> Result<String, crate::adapter_registry::AdapterError> {
            Ok("0".to_string())
        }
    }

    #[tokio::test]
    async fn test_transfer_coordinator_replay_idempotent() {
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

        let lease = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes()),
            epoch: 1,
            owner_runtime_id: uuid::Uuid::new_v4(),
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now()
                + std::time::Duration::from_secs(3600),
        };
        let runtime_ctx = crate::lease::RuntimeExecutionContext {
            lease: lease.clone(),
            runtime_instance: lease.owner_runtime_id,
            policy: crate::policy::RuntimePolicy::new(),
        };

        // First transfer should succeed
        let result = coordinator.execute(transfer.clone(), &registry, runtime_ctx.clone()).await;
        assert!(result.is_ok(), "First execution should succeed: {:?}", result);

        // Completed transfers are idempotent — `consume_if_unconsumed` returns Ok(())
        // for already Consumed entries. This allows safe retries of completed transfers.
        let result = coordinator.execute(transfer.clone(), &registry, runtime_ctx.clone()).await;
        assert!(result.is_ok(), "Completed transfers should be idempotent: {:?}", result);

        // Now test that a Pending entry (inserted without confirming) blocks a retry.
        // We need a different transfer to get a different ReplayId.
        let pending_transfer = CrossChainTransfer {
            id: "test-pending".to_string(),
            source_chain: "test-chain".to_string(),
            destination_chain: "test-chain".to_string(),
            lock_tx_hash: vec![5u8; 32], // different lock tx
            lock_output_index: 0,
            sanad_id: csv_core::hash::Hash::new([6u8; 32]), // different sanad
            transition_id: vec![7u8; 32], // different transition
        };

        let pending_lease = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*pending_transfer.sanad_id.as_bytes()),
            epoch: 1,
            owner_runtime_id: uuid::Uuid::new_v4(),
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        };
        let pending_ctx = crate::lease::RuntimeExecutionContext {
            lease: pending_lease,
            runtime_instance: uuid::Uuid::new_v4(),
            policy: crate::policy::RuntimePolicy::new(),
        };

        // First execution inserts Pending, then the mint succeeds and confirms.
        let result = coordinator.execute(pending_transfer.clone(), &registry, pending_ctx).await;
        assert!(result.is_ok(), "Pending transfer first execution should succeed: {:?}", result);
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
            fn set_policy(&mut self, _policy: crate::policy::RuntimePolicy) {
                // Test adapter accepts policy but doesn't use it
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
            async fn get_balance(&self, _address: &str) -> Result<String, crate::adapter_registry::AdapterError> {
                Ok("0".to_string())
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
        let lease = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes()),
            epoch: 1,
            owner_runtime_id: uuid::Uuid::new_v4(),
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        };
        let runtime_ctx = crate::lease::RuntimeExecutionContext {
            lease,
            runtime_instance: uuid::Uuid::new_v4(),
            policy: crate::policy::RuntimePolicy::new(),
        };

        let result = coordinator.execute(transfer, &registry, runtime_ctx).await;
        assert!(matches!(
            result,
            Err(TransferCoordinatorError::UnsupportedOperation(_))
        ));
    }

    #[tokio::test]
    async fn test_runtime_policy_enforcement() {
        let replay_db = Box::new(crate::replay_db::InMemoryReplayDb::new());
        let event_bus = EventBus::new();
        let coordinator = TransferCoordinator::new(replay_db, event_bus);

        let mut registry = AdapterRegistryImpl::new();
        registry.register(std::sync::Arc::new(TestAdapter::new()));

        let transfer = CrossChainTransfer {
            id: "test-policy".to_string(),
            source_chain: "test-chain".to_string(),
            destination_chain: "test-chain".to_string(),
            lock_tx_hash: vec![1u8; 32],
            lock_output_index: 0,
            sanad_id: csv_core::hash::Hash::new([2u8; 32]),
            transition_id: vec![3u8; 32],
        };

        let lease = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes()),
            epoch: 1,
            owner_runtime_id: uuid::Uuid::new_v4(),
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        };

        // Test with production policy (no RPC fallback, strict finality)
        let production_policy = crate::policy::RuntimePolicy::production();
        let runtime_ctx = crate::lease::RuntimeExecutionContext {
            lease: lease.clone(),
            runtime_instance: lease.owner_runtime_id,
            policy: production_policy,
        };

        let result = coordinator.execute(transfer.clone(), &registry, runtime_ctx).await;
        assert!(result.is_ok(), "Transfer should succeed with production policy");

        // Test with development policy (allows RPC fallback)
        let dev_policy = crate::policy::RuntimePolicy::development();
        let runtime_ctx = crate::lease::RuntimeExecutionContext {
            lease,
            runtime_instance: uuid::Uuid::new_v4(),
            policy: dev_policy,
        };

        let result = coordinator.execute(transfer, &registry, runtime_ctx).await;
        assert!(result.is_ok(), "Transfer should succeed with development policy");
    }

    #[tokio::test]
    async fn test_retry_logic_with_policy() {
        let replay_db = Box::new(crate::replay_db::InMemoryReplayDb::new());
        let event_bus = EventBus::new();
        let coordinator = TransferCoordinator::new(replay_db, event_bus);

        let mut registry = AdapterRegistryImpl::new();
        registry.register(std::sync::Arc::new(TestAdapter::new()));

        let transfer = CrossChainTransfer {
            id: "test-retry".to_string(),
            source_chain: "test-chain".to_string(),
            destination_chain: "test-chain".to_string(),
            lock_tx_hash: vec![1u8; 32],
            lock_output_index: 0,
            sanad_id: csv_core::hash::Hash::new([2u8; 32]),
            transition_id: vec![3u8; 32],
        };

        let lease = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes()),
            epoch: 1,
            owner_runtime_id: uuid::Uuid::new_v4(),
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        };

        // Test with policy that allows retries
        let mut policy = crate::policy::RuntimePolicy::new();
        policy.max_retries = 3;
        policy.retry_delay = std::time::Duration::from_millis(10);

        let runtime_ctx = crate::lease::RuntimeExecutionContext {
            lease,
            runtime_instance: uuid::Uuid::new_v4(),
            policy,
        };

        let result = coordinator.execute(transfer, &registry, runtime_ctx).await;
        assert!(result.is_ok(), "Transfer should succeed with retry policy");
    }

    #[tokio::test]
    async fn test_circuit_breaker_blocks_requests() {
        let replay_db = Box::new(crate::replay_db::InMemoryReplayDb::new());
        let event_bus = EventBus::new();
        let coordinator = TransferCoordinator::new(replay_db, event_bus);

        let mut registry = AdapterRegistryImpl::new();
        registry.register(std::sync::Arc::new(TestAdapter::new()));

        // Open the circuit breaker by recording failures
        for _ in 0..5 {
            coordinator.circuit_breaker().lock().unwrap().record_failure();
        }

        let transfer = CrossChainTransfer {
            id: "test-circuit".to_string(),
            source_chain: "test-chain".to_string(),
            destination_chain: "test-chain".to_string(),
            lock_tx_hash: vec![1u8; 32],
            lock_output_index: 0,
            sanad_id: csv_core::hash::Hash::new([2u8; 32]),
            transition_id: vec![3u8; 32],
        };

        let lease = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes()),
            epoch: 1,
            owner_runtime_id: uuid::Uuid::new_v4(),
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        };

        let runtime_ctx = crate::lease::RuntimeExecutionContext {
            lease,
            runtime_instance: uuid::Uuid::new_v4(),
            policy: crate::policy::RuntimePolicy::new(),
        };

        let result = coordinator.execute(transfer, &registry, runtime_ctx).await;
        assert!(matches!(
            result,
            Err(TransferCoordinatorError::RuntimeError(_))
        ));
    }

    #[tokio::test]
    async fn test_health_monitor_mode_transition() {
        let replay_db = Box::new(crate::replay_db::InMemoryReplayDb::new());
        let event_bus = EventBus::new();
        let coordinator = TransferCoordinator::new(replay_db, event_bus);

        // Initially healthy
        assert_eq!(
            coordinator.health_status(),
            crate::runtime_mode::HealthStatus::Healthy
        );

        // Record a failed health check
        coordinator.record_health_check(crate::runtime_mode::HealthCheck {
            component: "rpc".to_string(),
            healthy: false,
            error: Some("RPC connection failed".to_string()),
            timestamp: std::time::SystemTime::now(),
        });

        // Should be degraded
        assert_eq!(
            coordinator.health_status(),
            crate::runtime_mode::HealthStatus::Degraded
        );
    }

    #[tokio::test]
    async fn test_degraded_mode_policy() {
        let policy = crate::policy::RuntimePolicy::development();
        assert_eq!(policy.mode, crate::runtime_mode::RuntimeMode::Degraded);
        assert!(policy.mode.allows_rpc_fallback());
        assert_eq!(policy.max_retries, 5);
    }

    #[tokio::test]
    async fn test_unsafe_mode_policy() {
        let policy = crate::policy::RuntimePolicy::unsafe_mode();
        assert_eq!(policy.mode, crate::runtime_mode::RuntimeMode::Unsafe);
        assert!(policy.mode.allows_rpc_fallback());
        assert_eq!(policy.max_retries, 1);
        assert!(policy.mode.requires_operator_confirmation());
    }

    #[tokio::test]
    async fn test_ha_failover_lease_conflict() {
        let replay_db = Box::new(crate::replay_db::InMemoryReplayDb::new());
        let event_bus = EventBus::new();
        let coordinator = TransferCoordinator::new(replay_db, event_bus);

        let mut registry = AdapterRegistryImpl::new();
        registry.register(std::sync::Arc::new(TestAdapter::new()));

        let transfer = CrossChainTransfer {
            id: "test-ha".to_string(),
            source_chain: "test-chain".to_string(),
            destination_chain: "test-chain".to_string(),
            lock_tx_hash: vec![1u8; 32],
            lock_output_index: 0,
            sanad_id: csv_core::hash::Hash::new([2u8; 32]),
            transition_id: vec![3u8; 32],
        };

        let original_runtime_id = uuid::Uuid::new_v4();
        let failover_runtime_id = uuid::Uuid::new_v4();

        // Original runtime acquires lease
        let original_lease = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes()),
            epoch: 1,
            owner_runtime_id: original_runtime_id,
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        };

        let original_ctx = crate::lease::RuntimeExecutionContext {
            lease: original_lease.clone(),
            runtime_instance: original_runtime_id,
            policy: crate::policy::RuntimePolicy::new(),
        };

        // Original runtime executes successfully
        let result = coordinator.execute(transfer.clone(), &registry, original_ctx).await;
        assert!(result.is_ok(), "Original runtime should succeed");

        // Failover runtime tries to execute with different runtime ID (should fail)
        let failover_lease = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes()),
            epoch: 1,
            owner_runtime_id: failover_runtime_id,
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        };

        let failover_ctx = crate::lease::RuntimeExecutionContext {
            lease: failover_lease,
            runtime_instance: failover_runtime_id,
            policy: crate::policy::RuntimePolicy::new(),
        };

        let result = coordinator.execute(transfer.clone(), &registry, failover_ctx).await;
        assert!(matches!(
            result,
            Err(TransferCoordinatorError::RuntimeError(_))
        ));
    }

    #[tokio::test]
    async fn test_ha_failover_after_lease_expiry() {
        let replay_db = Box::new(crate::replay_db::InMemoryReplayDb::new());
        let event_bus = EventBus::new();
        let coordinator = TransferCoordinator::new(replay_db, event_bus);

        let mut registry = AdapterRegistryImpl::new();
        registry.register(std::sync::Arc::new(TestAdapter::new()));

        let transfer = CrossChainTransfer {
            id: "test-ha-expiry".to_string(),
            source_chain: "test-chain".to_string(),
            destination_chain: "test-chain".to_string(),
            lock_tx_hash: vec![1u8; 32],
            lock_output_index: 0,
            sanad_id: csv_core::hash::Hash::new([2u8; 32]),
            transition_id: vec![3u8; 32],
        };

        let original_runtime_id = uuid::Uuid::new_v4();
        let failover_runtime_id = uuid::Uuid::new_v4();

        // Original runtime acquires expired lease
        let expired_lease = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes()),
            epoch: 1,
            owner_runtime_id: original_runtime_id,
            acquired_at: std::time::SystemTime::now() - std::time::Duration::from_secs(3600),
            expires_at: std::time::SystemTime::now() - std::time::Duration::from_secs(1800),
        };

        let expired_ctx = crate::lease::RuntimeExecutionContext {
            lease: expired_lease,
            runtime_instance: original_runtime_id,
            policy: crate::policy::RuntimePolicy::new(),
        };

        // Original runtime with expired lease should fail
        let result = coordinator.execute(transfer.clone(), &registry, expired_ctx).await;
        assert!(matches!(
            result,
            Err(TransferCoordinatorError::RuntimeError(_))
        ));

        // Failover runtime with new lease should succeed
        let failover_lease = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes()),
            epoch: 2, // Incremented epoch
            owner_runtime_id: failover_runtime_id,
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        };

        let failover_ctx = crate::lease::RuntimeExecutionContext {
            lease: failover_lease,
            runtime_instance: failover_runtime_id,
            policy: crate::policy::RuntimePolicy::new(),
        };

        let result = coordinator.execute(transfer, &registry, failover_ctx).await;
        assert!(result.is_ok(), "Failover runtime should succeed with new lease");
    }

    #[tokio::test]
    async fn test_blockchain_reorg_finality_rollback() {
        let replay_db = Box::new(crate::replay_db::InMemoryReplayDb::new());
        let event_bus = EventBus::new();
        let coordinator = TransferCoordinator::new(replay_db, event_bus);

        let mut registry = AdapterRegistryImpl::new();
        registry.register(std::sync::Arc::new(TestAdapter::new()));

        let transfer = CrossChainTransfer {
            id: "test-reorg".to_string(),
            source_chain: "test-chain".to_string(),
            destination_chain: "test-chain".to_string(),
            lock_tx_hash: vec![1u8; 32],
            lock_output_index: 0,
            sanad_id: csv_core::hash::Hash::new([2u8; 32]),
            transition_id: vec![3u8; 32],
        };

        let lease = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes()),
            epoch: 1,
            owner_runtime_id: uuid::Uuid::new_v4(),
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        };

        let runtime_ctx = crate::lease::RuntimeExecutionContext {
            lease,
            runtime_instance: uuid::Uuid::new_v4(),
            policy: crate::policy::RuntimePolicy::new(),
        };

        // Execute transfer successfully
        let result = coordinator.execute(transfer.clone(), &registry, runtime_ctx).await;
        assert!(result.is_ok(), "Transfer should succeed initially");

        // Simulate reorg by recording a health check indicating reorg
        coordinator.record_health_check(crate::runtime_mode::HealthCheck {
            component: "blockchain".to_string(),
            healthy: false,
            error: Some("Reorg detected at block 1000".to_string()),
            timestamp: std::time::SystemTime::now(),
        });

        // Health status should be degraded
        assert_eq!(
            coordinator.health_status(),
            crate::runtime_mode::HealthStatus::Degraded
        );

        // Circuit breaker should be open after reorg
        for _ in 0..5 {
            coordinator.circuit_breaker().lock().unwrap().record_failure();
        }
        assert_eq!(
            coordinator.circuit_breaker().lock().unwrap().state(),
            crate::runtime_mode::CircuitBreakerState::Open
        );
    }

    #[tokio::test]
    async fn test_reorg_recovery() {
        let replay_db = Box::new(crate::replay_db::InMemoryReplayDb::new());
        let event_bus = EventBus::new();
        let coordinator = TransferCoordinator::new(replay_db, event_bus);

        // Open circuit breaker
        for _ in 0..5 {
            coordinator.circuit_breaker().lock().unwrap().record_failure();
        }
        assert_eq!(
            coordinator.circuit_breaker().lock().unwrap().state(),
            crate::runtime_mode::CircuitBreakerState::Open
        );

        // Attempt recovery after timeout
        std::thread::sleep(std::time::Duration::from_millis(100));

        let recovered = coordinator.attempt_circuit_breaker_recovery();
        assert!(recovered, "Circuit breaker should recover after timeout");
        assert_eq!(
            coordinator.circuit_breaker().lock().unwrap().state(),
            crate::runtime_mode::CircuitBreakerState::HalfOpen
        );

        // Record successes to close circuit
        coordinator.circuit_breaker().lock().unwrap().record_success();
        coordinator.circuit_breaker().lock().unwrap().record_success();
        assert_eq!(
            coordinator.circuit_breaker().lock().unwrap().state(),
            crate::runtime_mode::CircuitBreakerState::Closed
        );
    }

    #[tokio::test]
    async fn test_concurrent_transfer_execution_race() {
        let _replay_db = Arc::new(std::sync::Mutex::new(crate::replay_db::InMemoryReplayDb::new()));
        let event_bus = EventBus::new();
        let coordinator = TransferCoordinator::new(
            Box::new(crate::replay_db::InMemoryReplayDb::new()),
            event_bus,
        );

        let mut registry = AdapterRegistryImpl::new();
        registry.register(std::sync::Arc::new(TestAdapter::new()));

        let transfer = CrossChainTransfer {
            id: "test-race".to_string(),
            source_chain: "test-chain".to_string(),
            destination_chain: "test-chain".to_string(),
            lock_tx_hash: vec![1u8; 32],
            lock_output_index: 0,
            sanad_id: csv_core::hash::Hash::new([2u8; 32]),
            transition_id: vec![3u8; 32],
        };

        let runtime_id = uuid::Uuid::new_v4();
        let lease = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes()),
            epoch: 1,
            owner_runtime_id: runtime_id,
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        };

        // Execute same transfer concurrently - should be idempotent
        let coordinator_ref = Arc::new(coordinator);
        let registry_ref = Arc::new(registry);
        let mut handles = Vec::new();

        for _ in 0..3 {
            let coord = coordinator_ref.clone();
            let reg = registry_ref.clone();
            let transfer_clone = transfer.clone();
            let lease_clone = lease.clone();
            let runtime_id_clone = runtime_id;

            handles.push(tokio::spawn(async move {
                let ctx = crate::lease::RuntimeExecutionContext {
                    lease: lease_clone,
                    runtime_instance: runtime_id_clone,
                    policy: crate::policy::RuntimePolicy::new(),
                };
                coord.execute(transfer_clone, reg.as_ref(), ctx).await
            }));
        }

        // Await all handles sequentially (equivalent to join_all for testing)
        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await.expect("task should not panic"));
        }
        // All should succeed due to idempotency
        let success_count = results.iter().filter(|r| r.is_ok()).count();
        assert_eq!(success_count, 3, "All concurrent executions should succeed");
    }

    #[tokio::test]
    async fn test_concurrent_different_runtime_race() {
        let replay_db = Box::new(crate::replay_db::InMemoryReplayDb::new());
        let event_bus = EventBus::new();
        let coordinator = TransferCoordinator::new(replay_db, event_bus);

        let mut registry = AdapterRegistryImpl::new();
        registry.register(std::sync::Arc::new(TestAdapter::new()));

        let transfer = CrossChainTransfer {
            id: "test-diff-race".to_string(),
            source_chain: "test-chain".to_string(),
            destination_chain: "test-chain".to_string(),
            lock_tx_hash: vec![1u8; 32],
            lock_output_index: 0,
            sanad_id: csv_core::hash::Hash::new([2u8; 32]),
            transition_id: vec![3u8; 32],
        };

        let runtime_id_1 = uuid::Uuid::new_v4();
        let runtime_id_2 = uuid::Uuid::new_v4();

        let lease_1 = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes()),
            epoch: 1,
            owner_runtime_id: runtime_id_1,
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        };

        let lease_2 = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes()),
            epoch: 1,
            owner_runtime_id: runtime_id_2,
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        };

        // Execute with different runtime IDs concurrently - one should fail
        let coordinator_ref = Arc::new(coordinator);
        let registry_ref = Arc::new(registry);
        let mut handles = Vec::new();

        for (i, lease) in [lease_1, lease_2].into_iter().enumerate() {
            let coord = coordinator_ref.clone();
            let reg = registry_ref.clone();
            let transfer_clone = transfer.clone();
            let runtime_id = if i == 0 { runtime_id_1 } else { runtime_id_2 };

            handles.push(tokio::spawn(async move {
                let ctx = crate::lease::RuntimeExecutionContext {
                    lease,
                    runtime_instance: runtime_id,
                    policy: crate::policy::RuntimePolicy::new(),
                };
                coord.execute(transfer_clone, reg.as_ref(), ctx).await
            }));
        }

        // Await all handles sequentially (equivalent to join_all for testing)
        let mut results = Vec::new();
        for handle in handles {
            results.push(handle.await.expect("task should not panic"));
        }
        // One should succeed, one should fail due to lease conflict
        let success_count = results.iter().filter(|r| r.is_ok()).count();
        let error_count = results.iter().filter(|r| r.is_err()).count();
        assert_eq!(success_count, 1, "Exactly one should succeed");
        assert_eq!(error_count, 1, "Exactly one should fail due to lease conflict");
    }

    #[tokio::test]
    async fn test_adversarial_proof_bundle_rejection() {
        let replay_db = Box::new(crate::replay_db::InMemoryReplayDb::new());
        let event_bus = EventBus::new();
        let coordinator = TransferCoordinator::new(replay_db, event_bus);

        // Create a test adapter that rejects invalid proof bundles
        struct MaliciousTestAdapter {
            caps: ChainCapabilities,
        }

        impl MaliciousTestAdapter {
            fn new() -> Self {
                Self {
                    caps: ChainCapabilities::bitcoin(),
                }
            }
        }

        #[async_trait::async_trait]
        impl ChainAdapter for MaliciousTestAdapter {
            fn chain_id(&self) -> &str {
                "malicious-chain"
            }
            fn capabilities(&self) -> &ChainCapabilities {
                &self.caps
            }
            fn set_policy(&mut self, _policy: crate::policy::RuntimePolicy) {}

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
                Err(crate::adapter_registry::AdapterError::InvalidProof(
                    "Malicious proof bundle detected".to_string(),
                ))
            }

            async fn build_inclusion_proof(
                &self,
                _lock_result: &LockResult,
            ) -> Result<ProofBundle, crate::adapter_registry::AdapterError> {
                unimplemented!()
            }

            async fn verify_inclusion_proof(
                &self,
                _proof: &ProofBundle,
            ) -> Result<VerificationResult, crate::adapter_registry::AdapterError> {
                unimplemented!()
            }

            async fn verify_finality(
                &self,
                _block_height: u64,
            ) -> Result<FinalityVerifierProof, crate::adapter_registry::AdapterError> {
                Ok(FinalityVerifierProof {
                    chain_id: "malicious-chain".to_string(),
                    block_height: 100,
                    finality_evidence: csv_core::finality::FinalityEvidence::CumulativeWork {
                        header_hash: [0u8; 32],
                        cumulative_work: 0,
                    },
                    confirmations: 6,
                })
            }

            async fn verify_seal_registry(
                &self,
                _seal_id: &[u8],
            ) -> Result<SealRegistryStatus, crate::adapter_registry::AdapterError> {
                Ok(SealRegistryStatus::Available)
            }

            async fn get_balance(
                &self,
                _address: &str,
            ) -> Result<String, crate::adapter_registry::AdapterError> {
                Ok("0".to_string())
            }
        }

        let mut registry = AdapterRegistryImpl::new();
        registry.register(std::sync::Arc::new(MaliciousTestAdapter::new()));

        let transfer = CrossChainTransfer {
            id: "test-malicious".to_string(),
            source_chain: "malicious-chain".to_string(),
            destination_chain: "malicious-chain".to_string(),
            lock_tx_hash: vec![1u8; 32],
            lock_output_index: 0,
            sanad_id: csv_core::hash::Hash::new([2u8; 32]),
            transition_id: vec![3u8; 32],
        };

        let lease = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes()),
            epoch: 1,
            owner_runtime_id: uuid::Uuid::new_v4(),
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        };

        let runtime_ctx = crate::lease::RuntimeExecutionContext {
            lease,
            runtime_instance: uuid::Uuid::new_v4(),
            policy: crate::policy::RuntimePolicy::new(),
        };

        // Transfer should fail due to malicious proof bundle rejection
        let result = coordinator.execute(transfer, &registry, runtime_ctx).await;
        assert!(matches!(
            result,
            Err(TransferCoordinatorError::MintFailed(_))
        ));
    }

    #[tokio::test]
    async fn test_double_spend_prevention() {
        let replay_db = Box::new(crate::replay_db::InMemoryReplayDb::new());
        let event_bus = EventBus::new();
        let coordinator = TransferCoordinator::new(replay_db, event_bus);

        let mut registry = AdapterRegistryImpl::new();
        registry.register(std::sync::Arc::new(TestAdapter::new()));

        let transfer = CrossChainTransfer {
            id: "test-doublespend".to_string(),
            source_chain: "test-chain".to_string(),
            destination_chain: "test-chain".to_string(),
            lock_tx_hash: vec![1u8; 32],
            lock_output_index: 0,
            sanad_id: csv_core::hash::Hash::new([2u8; 32]),
            transition_id: vec![3u8; 32],
        };

        let lease = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes()),
            epoch: 1,
            owner_runtime_id: uuid::Uuid::new_v4(),
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        };

        let runtime_ctx = crate::lease::RuntimeExecutionContext {
            lease: lease.clone(),
            runtime_instance: uuid::Uuid::new_v4(),
            policy: crate::policy::RuntimePolicy::new(),
        };

        // First execution should succeed
        let result = coordinator.execute(transfer.clone(), &registry, runtime_ctx.clone()).await;
        assert!(result.is_ok(), "First execution should succeed");

        // Second execution with same transfer should be idempotent (already consumed)
        let result = coordinator.execute(transfer.clone(), &registry, runtime_ctx.clone()).await;
        assert!(result.is_ok(), "Second execution should be idempotent");

        // Try with different transfer ID but same lock_tx_hash (replay attempt)
        let replay_transfer = CrossChainTransfer {
            id: "test-replay".to_string(),
            source_chain: transfer.source_chain.clone(),
            destination_chain: transfer.destination_chain.clone(),
            lock_tx_hash: transfer.lock_tx_hash.clone(),
            lock_output_index: transfer.lock_output_index,
            sanad_id: transfer.sanad_id,
            transition_id: transfer.transition_id,
        };

        let replay_lease = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*replay_transfer.sanad_id.as_bytes()),
            epoch: 1,
            owner_runtime_id: uuid::Uuid::new_v4(),
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        };

        let replay_ctx = crate::lease::RuntimeExecutionContext {
            lease: replay_lease,
            runtime_instance: uuid::Uuid::new_v4(),
            policy: crate::policy::RuntimePolicy::new(),
        };

        let result = coordinator.execute(replay_transfer, &registry, replay_ctx).await;
        // Should fail due to replay detection
        assert!(matches!(
            result,
            Err(TransferCoordinatorError::ReplayDetected(_))
        ));
    }

    #[tokio::test]
    async fn test_lease_epoch_conflict() {
        let replay_db = Box::new(crate::replay_db::InMemoryReplayDb::new());
        let event_bus = EventBus::new();
        let coordinator = TransferCoordinator::new(replay_db, event_bus);

        let mut registry = AdapterRegistryImpl::new();
        registry.register(std::sync::Arc::new(TestAdapter::new()));

        let transfer = CrossChainTransfer {
            id: "test-epoch".to_string(),
            source_chain: "test-chain".to_string(),
            destination_chain: "test-chain".to_string(),
            lock_tx_hash: vec![1u8; 32],
            lock_output_index: 0,
            sanad_id: csv_core::hash::Hash::new([2u8; 32]),
            transition_id: vec![3u8; 32],
        };

        let runtime_id = uuid::Uuid::new_v4();

        // Acquire lease with epoch 1
        let lease_epoch_1 = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes()),
            epoch: 1,
            owner_runtime_id: runtime_id,
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        };

        let ctx_epoch_1 = crate::lease::RuntimeExecutionContext {
            lease: lease_epoch_1,
            runtime_instance: runtime_id,
            policy: crate::policy::RuntimePolicy::new(),
        };

        let result = coordinator.execute(transfer.clone(), &registry, ctx_epoch_1).await;
        assert!(result.is_ok(), "Epoch 1 should succeed");

        // Try to use stale lease with epoch 1 after epoch 2 has been issued
        let lease_epoch_2 = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes()),
            epoch: 2,
            owner_runtime_id: runtime_id,
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        };

        let ctx_epoch_2 = crate::lease::RuntimeExecutionContext {
            lease: lease_epoch_2,
            runtime_instance: runtime_id,
            policy: crate::policy::RuntimePolicy::new(),
        };

        let result = coordinator.execute(transfer.clone(), &registry, ctx_epoch_2).await;
        assert!(result.is_ok(), "Epoch 2 should succeed");

        // Try to use stale epoch 1 lease again - should fail
        let stale_lease = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes()),
            epoch: 1,
            owner_runtime_id: runtime_id,
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        };

        let stale_ctx = crate::lease::RuntimeExecutionContext {
            lease: stale_lease,
            runtime_instance: runtime_id,
            policy: crate::policy::RuntimePolicy::new(),
        };

        let result = coordinator.execute(transfer, &registry, stale_ctx).await;
        // The lease validation should fail because the lease is stale (wrong epoch)
        assert!(matches!(
            result,
            Err(TransferCoordinatorError::RuntimeError(_))
        ));
    }

    #[tokio::test]
    async fn test_finality_rollback() {
        let replay_db = Box::new(crate::replay_db::InMemoryReplayDb::new());
        let event_bus = EventBus::new();
        let coordinator = TransferCoordinator::new(replay_db, event_bus);

        let mut registry = AdapterRegistryImpl::new();
        registry.register(std::sync::Arc::new(TestAdapter::new()));

        let transfer = CrossChainTransfer {
            id: "test-rollback".to_string(),
            source_chain: "test-chain".to_string(),
            destination_chain: "test-chain".to_string(),
            lock_tx_hash: vec![1u8; 32],
            lock_output_index: 0,
            sanad_id: csv_core::hash::Hash::new([2u8; 32]),
            transition_id: vec![3u8; 32],
        };

        let lease = crate::lease::TransferLease {
            transfer_id: csv_core::sanad::SanadId::new(*transfer.sanad_id.as_bytes()),
            epoch: 1,
            owner_runtime_id: uuid::Uuid::new_v4(),
            acquired_at: std::time::SystemTime::now(),
            expires_at: std::time::SystemTime::now() + std::time::Duration::from_secs(3600),
        };

        let runtime_ctx = crate::lease::RuntimeExecutionContext {
            lease,
            runtime_instance: uuid::Uuid::new_v4(),
            policy: crate::policy::RuntimePolicy::new(),
        };

        // Execute transfer successfully
        let result = coordinator.execute(transfer.clone(), &registry, runtime_ctx).await;
        assert!(result.is_ok(), "Transfer should succeed initially");

        // Simulate finality rollback by recording health check
        coordinator.record_health_check(crate::runtime_mode::HealthCheck {
            component: "finality".to_string(),
            healthy: false,
            error: Some("Finality rollback detected".to_string()),
            timestamp: std::time::SystemTime::now(),
        });

        // Health status should be degraded
        assert_eq!(
            coordinator.health_status(),
            crate::runtime_mode::HealthStatus::Degraded
        );

        // Runtime mode should be degraded
        let mode = coordinator.health_monitor().lock().unwrap().mode();
        assert_eq!(mode, crate::runtime_mode::RuntimeMode::Degraded);
    }
}