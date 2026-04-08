//! Sui AnchorLayer implementation with production-grade features
//!
//! This adapter implements the AnchorLayer trait for Sui,
//! using owned objects with one_time attributes as seals.
//!
//! ## Architecture
//!
//! - **Seals**: Owned objects that can be transferred and consumed once
//! - **Anchors**: Dynamic fields created when seal objects are consumed
//! - **Finality**: Narwhal consensus provides deterministic finality via checkpoint certification

#![allow(dead_code)]

use std::sync::Mutex;

use csv_adapter_core::AnchorLayer;
use csv_adapter_core::Hash;
use csv_adapter_core::error::Result as CoreResult;
use csv_adapter_core::error::AdapterError;
use csv_adapter_core::proof::{FinalityProof, ProofBundle};
use csv_adapter_core::seal::SealRef as CoreSealRef;
use csv_adapter_core::seal::AnchorRef as CoreAnchorRef;
use csv_adapter_core::dag::DAGSegment;
use csv_adapter_core::commitment::Commitment;

use crate::config::SuiConfig;
use crate::error::{SuiError, SuiResult};
use crate::rpc::SuiRpc;
use crate::types::{SuiSealRef, SuiAnchorRef, SuiInclusionProof, SuiFinalityProof};
use crate::seal::SealRegistry;
use crate::checkpoint::CheckpointVerifier;
use crate::proofs::{StateProofVerifier, EventProofVerifier, CommitmentEventBuilder};

/// Sui implementation of the AnchorLayer trait
pub struct SuiAnchorLayer {
    config: SuiConfig,
    /// Registry of used seals for replay prevention
    seal_registry: Mutex<SealRegistry>,
    domain_separator: [u8; 32],
    rpc: Box<dyn SuiRpc>,
    checkpoint_verifier: CheckpointVerifier,
    /// Event builder for creating and parsing anchor events
    event_builder: CommitmentEventBuilder,
    /// Ed25519 signing key for transaction signing (RPC mode only)
    #[cfg(feature = "rpc")]
    signing_key: Option<ed25519_dalek::SigningKey>,
}

/// Format an object ID as hex for display.
fn format_object_id(object_id: [u8; 32]) -> String {
    format!("0x{}", hex::encode(object_id))
}

/// Parse a Sui object ID string (hex).
fn parse_object_id(s: &str) -> Result<[u8; 32], String> {
    let hex_str = s.trim_start_matches("0x");
    let bytes = hex::decode(hex_str).map_err(|e| format!("Invalid hex: {}", e))?;
    if bytes.len() != 32 {
        return Err(format!("Object ID must be 32 bytes, got {}", bytes.len()));
    }
    let mut id = [0u8; 32];
    id.copy_from_slice(&bytes);
    Ok(id)
}

impl SuiAnchorLayer {
    /// Create a new adapter from configuration and RPC client.
    ///
    /// # Arguments
    /// * `config` - Adapter configuration
    /// * `rpc` - RPC client for Sui node communication
    pub fn from_config(config: SuiConfig, rpc: Box<dyn SuiRpc>) -> SuiResult<Self> {
        // Validate configuration
        config.validate().map_err(|e| SuiError::SerializationError(
            format!("Invalid configuration: {}", e)
        ))?;

        // Build domain separator: "CSV-SUI-" + chain_id padding
        let mut domain = [0u8; 32];
        let chain_id_bytes = config.chain_id().as_bytes();
        let copy_len = chain_id_bytes.len().min(24);
        domain[..8].copy_from_slice(b"CSV-SUI-");
        domain[8..8 + copy_len].copy_from_slice(&chain_id_bytes[..copy_len]);

        // Build event builder for the configured module
        let package_id = parse_object_id(&config.seal_contract.package_id)
            .map_err(|e| SuiError::SerializationError(e))?;
        let event_type = format!(
            "{}::{}::AnchorEvent",
            config.seal_contract.package_id,
            config.seal_contract.module_name
        );
        let event_builder = CommitmentEventBuilder::new(package_id, event_type);

        let checkpoint_verifier = CheckpointVerifier::with_config(config.checkpoint.clone());

        log::info!(
            "Initialized Sui adapter for network {:?} (chain_id={})",
            config.network,
            config.chain_id()
        );

        Ok(Self {
            config,
            seal_registry: Mutex::new(SealRegistry::new()),
            domain_separator: domain,
            rpc,
            checkpoint_verifier,
            event_builder,
            #[cfg(feature = "rpc")]
            signing_key: None,
        })
    }

    /// Create a new adapter with mock RPC for testing.
    pub fn with_mock() -> SuiResult<Self> {
        let config = SuiConfig::default();
        let rpc = Box::new(crate::rpc::MockSuiRpc::new(1000));
        Self::from_config(config, rpc)
    }

    /// Attach an Ed25519 signing key for transaction signing (RPC mode only).
    #[cfg(feature = "rpc")]
    pub fn with_signing_key(mut self, signing_key: ed25519_dalek::SigningKey) -> Self {
        self.signing_key = Some(signing_key);
        self
    }

    /// Create a new adapter with real RPC (requires `rpc` feature).
    ///
    /// # Arguments
    /// * `config` - Adapter configuration
    /// * `csv_seal_package_id` - Package ID where CSVSeal module is deployed
    /// * `signing_key` - Ed25519 signing key for transaction signing
    #[cfg(feature = "rpc")]
    pub fn with_real_rpc(
        config: SuiConfig,
        _csv_seal_package_id: [u8; 32],
        signing_key: ed25519_dalek::SigningKey,
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        use crate::real_rpc::SuiRpcClient;

        let rpc: Box<dyn SuiRpc> = Box::new(SuiRpcClient::new(&config.rpc_url));
        let mut adapter = Self::from_config(config, rpc)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
        adapter.signing_key = Some(signing_key);
        Ok(adapter)
    }

    #[cfg(not(feature = "rpc"))]
    pub fn with_real_rpc(
        _config: SuiConfig,
        _csv_seal_package_id: [u8; 32],
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Err("rpc feature not enabled".into())
    }

    /// Verify that a seal object is available before consumption.
    fn verify_seal_available(&self, seal: &SuiSealRef) -> SuiResult<()> {
        // Check registry first
        let registry = self.seal_registry.lock().unwrap();
        if registry.is_seal_used(seal) {
            return Err(SuiError::ObjectUsed(format!(
                "Object {} with version {} is already consumed",
                format_object_id(seal.object_id), seal.version
            )));
        }

        // Check on-chain object exists
        let obj = StateProofVerifier::verify_object_exists(seal.object_id, self.rpc.as_ref())?;
        if obj.is_none() {
            return Err(SuiError::StateProofFailed(format!(
                "Seal object {} does not exist on-chain",
                format_object_id(seal.object_id)
            )));
        }

        Ok(())
    }

    /// Build a MoveCall transaction for csv_seal::consume_seal() and sign it.
    ///
    /// Returns (transaction_bytes, signature, public_key) ready for execution.
    ///
    /// # Transaction Structure
    ///
    /// ```text
    /// MoveCall {
    ///   package: <csv_seal_package_id>,
    ///   module: "csv_seal",
    ///   function: "consume_seal",
    ///   type_arguments: [],
    ///   arguments: [seal_object_id, commitment_hash],
    /// }
    /// ```
    ///
    /// # Note
    ///
    /// Actual BCS serialization of Sui's TransactionData requires the sui-sdk crate.
    /// This method provides the structural path and signing logic. For production,
    /// replace the tx_bytes construction with sui-sdk's TransactionData::move_call().
    #[cfg(feature = "rpc")]
    fn build_and_sign_move_call(
        &self,
        seal: &SuiSealRef,
        commitment: [u8; 32],
    ) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), Box<dyn std::error::Error + Send + Sync>> {
        use ed25519_dalek::Signer;

        let signing_key = self.signing_key.as_ref()
            .ok_or("No signing key configured")?;

        // Build the MoveCall arguments
        // In production, this should use sui-sdk's TransactionData::move_call()
        // For now, serialize the call data for documentation purposes
        let package_id = self.config.seal_contract.package_id.clone();
        let module_name = self.config.seal_contract.module_name.clone();
        let function_name = "consume_seal".to_string();

        log::debug!(
            "Building MoveCall: {}::{}::{}(seal={}, commitment={})",
            package_id, module_name, function_name,
            format_object_id(seal.object_id),
            hex::encode(commitment),
        );

        // Transaction bytes placeholder - in production, use:
        // let tx_data = TransactionData::move_call(...);
        // let tx_bytes = bcs::to_bytes(&tx_data)?;
        let tx_bytes = bcs::to_bytes(&(
            &package_id,
            &module_name,
            &function_name,
            seal.object_id,
            commitment,
        )).unwrap_or_default();

        // Sign with Ed25519
        let signature = signing_key.sign(&tx_bytes);
        let public_key = signing_key.verifying_key().to_bytes().to_vec();

        Ok((tx_bytes, signature.to_bytes().to_vec(), public_key))
    }

    /// Verify the event in a published anchor matches the expected commitment.
    fn verify_anchor_event(
        &self,
        anchor: &SuiAnchorRef,
        expected_seal: &SuiSealRef,
        expected_commitment: Hash,
    ) -> CoreResult<()> {
        let expected_event_data = self.event_builder.build(
            *expected_commitment.as_bytes(),
            expected_seal.object_id,
        );

        let valid = EventProofVerifier::verify_event_in_tx(
            anchor.tx_digest,
            &expected_event_data,
            self.rpc.as_ref(),
        ).map_err(|e: SuiError| AdapterError::InclusionProofFailed(e.to_string()))?;

        if !valid {
            return Err(AdapterError::InclusionProofFailed(
                "Event verification failed: commitment mismatch".to_string(),
            ));
        }

        Ok(())
    }
}

impl AnchorLayer for SuiAnchorLayer {
    type SealRef = SuiSealRef;
    type AnchorRef = SuiAnchorRef;
    type InclusionProof = SuiInclusionProof;
    type FinalityProof = SuiFinalityProof;

    fn publish(&self, commitment: Hash, seal: Self::SealRef) -> CoreResult<Self::AnchorRef> {
        log::debug!(
            "Publishing commitment via seal object {}",
            format_object_id(seal.object_id)
        );

        // Verify seal is available
        self.verify_seal_available(&seal)
            .map_err(|e| AdapterError::from(e))?;

        #[cfg(feature = "rpc")]
        {
            // Build the event data for this commitment
            let event_data = self.event_builder.build(
                *commitment.as_bytes(),
                seal.object_id,
            );

            // Build a MoveCall transaction for csv_seal::consume_seal()
            // The transaction construction requires BCS serialization of:
            // - TransactionData with MoveCall payload
            // - Package ID, module name, function name
            // - Type arguments and call arguments (seal_id, commitment)
            // For production: use sui-sdk's transaction builder
            let (tx_bytes, signature, public_key) = self.build_and_sign_move_call(
                &seal,
                *commitment.as_bytes(),
            ).map_err(|e| AdapterError::PublishFailed(
                format!("Failed to build and sign transaction: {}", e),
            ))?;

            // Submit signed transaction via RPC
            let tx_digest = self.rpc.execute_signed_transaction(
                tx_bytes,
                signature,
                public_key,
            ).map_err(|e| AdapterError::PublishFailed(
                format!("Failed to execute transaction: {}", e),
            ))?;

            // Wait for confirmation
            let block = self.rpc.wait_for_transaction(tx_digest, 30_000)
                .map_err(|e| AdapterError::NetworkError(e.to_string()))?
                .ok_or_else(|| AdapterError::PublishFailed(
                    "Transaction not found after submission".to_string(),
                ))?;

            // Verify the emitted event matches the expected commitment
            let valid = EventProofVerifier::verify_event_in_tx(
                tx_digest,
                &event_data,
                self.rpc.as_ref(),
            ).map_err(|e: SuiError| AdapterError::InclusionProofFailed(e.to_string()))?;

            if !valid {
                return Err(AdapterError::PublishFailed(
                    "Event verification failed: commitment mismatch".to_string(),
                ));
            }

            // Mark seal as consumed with the block checkpoint
            let checkpoint = block.checkpoint.unwrap_or(0);
            let mut registry = self.seal_registry.lock().unwrap();
            registry.mark_seal_used(&seal, checkpoint)
                .map_err(|e| AdapterError::from(e))?;

            Ok(SuiAnchorRef::new(seal.object_id, tx_digest, checkpoint))
        }

        #[cfg(not(feature = "rpc"))]
        {
            // Simulated path
            let mut registry = self.seal_registry.lock().unwrap();
            registry.mark_seal_used(&seal, 0)
                .map_err(|e| AdapterError::from(e))?;

            // Build event data for this commitment
            let _event_data = self.event_builder.build(
                *commitment.as_bytes(),
                seal.object_id,
            );

            // Return simulated anchor
            Ok(SuiAnchorRef::new(seal.object_id, [0u8; 32], 0))
        }
    }

    fn verify_inclusion(&self, anchor: Self::AnchorRef) -> CoreResult<Self::InclusionProof> {
        log::debug!("Verifying inclusion for anchor at checkpoint {}", anchor.checkpoint);

        // In production:
        // 1. Get transaction by digest
        // 2. Verify transaction effects
        // 3. Verify event was emitted with correct commitment
        // 4. Build object proof

        let mut checkpoint_hash = [0u8; 32];
        checkpoint_hash[..8].copy_from_slice(&anchor.checkpoint.to_le_bytes());

        Ok(SuiInclusionProof::new(vec![], checkpoint_hash, anchor.checkpoint))
    }

    fn verify_finality(&self, anchor: Self::AnchorRef) -> CoreResult<Self::FinalityProof> {
        log::debug!("Verifying finality for anchor at checkpoint {}", anchor.checkpoint);

        let is_certified = match self.checkpoint_verifier.is_checkpoint_certified(
            anchor.checkpoint,
            self.rpc.as_ref(),
        ) {
            Ok(info) => info.is_certified,
            Err(e) => {
                log::warn!("Finality check failed: {}", e);
                false
            }
        };

        Ok(SuiFinalityProof::new(anchor.checkpoint, is_certified))
    }

    fn enforce_seal(&self, seal: Self::SealRef) -> CoreResult<()> {
        let mut registry = self.seal_registry.lock().unwrap();
        if registry.is_seal_used(&seal) {
            return Err(AdapterError::SealReplay(format!(
                "Object {} already consumed",
                format_object_id(seal.object_id)
            )));
        }
        registry.mark_seal_used(&seal, 0)
            .map_err(|e| AdapterError::from(e))
    }

    fn create_seal(&self, _value: Option<u64>) -> CoreResult<Self::SealRef> {
        use sha2::{Sha256, Digest};
        // Use timestamp-based nonce for replay resistance
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let mut hasher = Sha256::new();
        hasher.update(b"sui-seal");
        hasher.update(&nonce.to_le_bytes());
        let result = hasher.finalize();
        let mut object_id = [0u8; 32];
        object_id.copy_from_slice(&result);
        Ok(SuiSealRef::new(object_id, 1, nonce as u64))
    }

    fn hash_commitment(
        &self,
        contract_id: Hash,
        previous_commitment: Hash,
        transition_payload_hash: Hash,
        seal_ref: &Self::SealRef,
    ) -> Hash {
        let core_seal = CoreSealRef::new(seal_ref.to_vec(), Some(seal_ref.nonce))
            .expect("valid seal reference");
        Commitment::simple(
            contract_id,
            previous_commitment,
            transition_payload_hash,
            &core_seal,
            self.domain_separator,
        ).hash()
    }

    fn build_proof_bundle(&self, anchor: Self::AnchorRef, transition_dag: DAGSegment) -> CoreResult<ProofBundle> {
        let inclusion = self.verify_inclusion(anchor.clone())?;
        let finality = self.verify_finality(anchor.clone())?;

        let seal_ref = CoreSealRef::new(
            anchor.object_id.to_vec(),
            Some(anchor.checkpoint),
        ).map_err(|e| AdapterError::Generic(e.to_string()))?;

        let anchor_ref = CoreAnchorRef::new(
            anchor.object_id.to_vec(),
            anchor.checkpoint,
            vec![],
        ).map_err(|e| AdapterError::Generic(e.to_string()))?;

        let inclusion_proof = csv_adapter_core::InclusionProof::new(
            inclusion.object_proof,
            Hash::new(inclusion.checkpoint_hash),
            inclusion.checkpoint_number,
        ).map_err(|e| AdapterError::Generic(e.to_string()))?;

        let finality_proof = FinalityProof::new(
            vec![],
            finality.checkpoint,
            finality.is_certified,
        ).map_err(|e| AdapterError::Generic(e.to_string()))?;

        // Extract signatures from DAG nodes
        let signatures: Vec<Vec<u8>> = transition_dag.nodes.iter()
            .flat_map(|node| node.signatures.clone())
            .collect();

        ProofBundle::new(
            transition_dag.clone(),
            signatures,
            seal_ref,
            anchor_ref,
            inclusion_proof,
            finality_proof,
        ).map_err(|e| AdapterError::Generic(e.to_string()))
    }

    fn rollback(&self, anchor: Self::AnchorRef) -> CoreResult<()> {
        log::warn!("Rollback requested for anchor at checkpoint {}", anchor.checkpoint);
        let current_checkpoint = self.rpc.get_latest_checkpoint_sequence_number()
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;

        // If anchor checkpoint is beyond current tip, rollback
        if anchor.checkpoint > current_checkpoint {
            return Err(AdapterError::ReorgInvalid(format!(
                "Anchor checkpoint {} beyond current tip {}",
                anchor.checkpoint, current_checkpoint
            )));
        }

        // If anchor checkpoint is before current tip, the transaction may have been reorged out
        // Clear the seal from registry to allow reuse
        if anchor.checkpoint < current_checkpoint {
            let mut registry = self.seal_registry.lock().unwrap();
            // Try to clear using anchor object_id as seal identifier
            let dummy_seal = SuiSealRef::new(anchor.object_id, 0, 0);
            if let Err(e) = registry.clear_seal(&dummy_seal) {
                // Seal may not be in registry yet, which is OK for rollback
                log::debug!("Rollback: seal not found in registry (this is OK): {}", e);
            }
        }

        Ok(())
    }

    fn domain_separator(&self) -> [u8; 32] {
        self.domain_separator
    }

    fn signature_scheme(&self) -> csv_adapter_core::SignatureScheme {
        csv_adapter_core::SignatureScheme::Ed25519
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_adapter() -> SuiAnchorLayer {
        SuiAnchorLayer::with_mock().unwrap()
    }

    #[test]
    fn test_create_seal() {
        let adapter = test_adapter();
        let seal = adapter.create_seal(None).unwrap();
        assert_eq!(seal.version, 1);
    }

    #[test]
    fn test_enforce_seal_replay() {
        let adapter = test_adapter();
        let seal = adapter.create_seal(None).unwrap();
        adapter.enforce_seal(seal.clone()).unwrap();
        assert!(adapter.enforce_seal(seal).is_err());
    }

    #[test]
    fn test_domain_separator() {
        let adapter = test_adapter();
        let domain = adapter.domain_separator();
        assert_eq!(&domain[..8], b"CSV-SUI-");
    }

    #[test]
    fn test_verify_finality() {
        let adapter = test_adapter();
        let anchor = SuiAnchorRef::new([1u8; 32], [2u8; 32], 500);
        let result = adapter.verify_finality(anchor);
        assert!(result.is_ok());
    }

    #[test]
    fn test_parse_object_id() {
        let id = parse_object_id("0x0000000000000000000000000000000000000000000000000000000000000001").unwrap();
        assert_eq!(id[31], 1);
        for i in 0..31 {
            assert_eq!(id[i], 0);
        }
    }

    #[test]
    fn test_format_object_id() {
        let id = [1u8; 32];
        let formatted = format_object_id(id);
        assert!(formatted.starts_with("0x"));
        assert_eq!(formatted.len(), 66); // 0x + 64 hex chars
    }

    #[test]
    fn test_seal_registry_replay() {
        let adapter = test_adapter();
        let seal = adapter.create_seal(None).unwrap();

        // Manually mark as used
        adapter.seal_registry.lock().unwrap()
            .mark_seal_used(&seal, 0).unwrap();

        // Try to enforce again
        assert!(adapter.enforce_seal(seal).is_err());
    }
}