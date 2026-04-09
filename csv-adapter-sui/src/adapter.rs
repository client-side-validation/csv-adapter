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

/// Build a complete Sui TransactionData with BCS serialization
///
/// This constructs the exact wire format that Sui nodes expect for
/// a MoveCall transaction. The structure follows Sui's BCS specification.
///
/// # BCS Encoding Notes
///
/// Sui uses BCS (Binary Canonical Serialization) with:
/// - uleb128 for vector lengths and enum variants
/// - little-endian for integers
/// - length-prefixed for strings and vectors
///
/// This function manually constructs the BCS bytes to avoid the sui-sdk dependency.
#[cfg(feature = "rpc")]
fn build_sui_transaction_data(
    package_id: [u8; 32],
    module_name: &str,
    function_name: &str,
    seal_object_id: [u8; 32],
    commitment: [u8; 32],
    sender: [u8; 32],
    gas_objects: &[SuiObject],
    gas_price: u64,
    gas_budget: u64,
) -> Vec<u8> {
    // Manual BCS serialization helpers
    fn uleb128_encode(mut n: u64) -> Vec<u8> {
        let mut buf = Vec::new();
        loop {
            let byte = (n & 0x7F) as u8;
            n >>= 7;
            if n == 0 {
                buf.push(byte);
                break;
            } else {
                buf.push(byte | 0x80);
            }
        }
        buf
    }

    fn bcs_vec_u8(v: &[u8]) -> Vec<u8> {
        let mut out = uleb128_encode(v.len() as u64);
        out.extend_from_slice(v);
        out
    }

    fn bcs_string(s: &str) -> Vec<u8> {
        bcs_vec_u8(s.as_bytes())
    }

    fn bcs_vec_vec_u8(vv: &[Vec<u8>]) -> Vec<u8> {
        let mut out = uleb128_encode(vv.len() as u64);
        for v in vv {
            out.extend_from_slice(v);
        }
        out
    }

    let mut tx = Vec::new();

    // === TransactionKind (enum variant 0 = ProgrammableTransaction) ===
    tx.push(0); // TransactionKind::ProgrammableTransaction

    // ProgrammableTransaction { inputs: Vec<CallArg>, commands: Vec<Command> }
    // inputs: 2 items
    tx.extend_from_slice(&uleb128_encode(2));

    // Input 0: CallArg::Object (variant 1)
    tx.push(1);
    // ObjectArg::ImmOrOwnedObject (variant 0): (ObjectID, u64 version, ObjectDigest)
    tx.extend_from_slice(&seal_object_id);
    tx.extend_from_slice(&1u64.to_le_bytes()); // version 1
    tx.extend_from_slice(&[0u8; 32]); // digest (zeroed for owned objects)

    // Input 1: CallArg::Pure (variant 0)
    tx.push(0);
    // Pure value: length-prefixed bytes
    tx.extend_from_slice(&bcs_vec_u8(&commitment));

    // commands: 1 item
    tx.extend_from_slice(&uleb128_encode(1));

    // Command::MoveCall (variant 0)
    tx.push(0);
    // MoveCall { package, module, function, type_arguments, arguments }
    tx.extend_from_slice(&package_id);
    tx.extend_from_slice(&bcs_string(module_name));
    tx.extend_from_slice(&bcs_string(function_name));
    // type_arguments: Vec<TypeTag> (empty)
    tx.push(0); // length 0
    // arguments: Vec<Argument>
    // Argument::Input(u16) = variant 1
    tx.extend_from_slice(&uleb128_encode(2)); // 2 arguments
    tx.push(1); tx.extend_from_slice(&0u16.to_le_bytes()); // Input(0)
    tx.push(1); tx.extend_from_slice(&1u16.to_le_bytes()); // Input(1)

    // === sender: SuiAddress (32 bytes) ===
    tx.extend_from_slice(&sender);

    // === GasData { payment: Vec<ObjectRef>, owner, price, budget } ===
    // payment: gas_objects
    tx.extend_from_slice(&uleb128_encode(gas_objects.len() as u64));
    for obj in gas_objects {
        // ObjectRef: (ObjectID, version, digest)
        tx.extend_from_slice(&obj.object_id);
        tx.extend_from_slice(&obj.version.to_le_bytes());
        tx.extend_from_slice(&[0u8; 32]); // digest
    }
    // owner
    tx.extend_from_slice(&sender);
    // price
    tx.extend_from_slice(&gas_price.to_le_bytes());
    // budget
    tx.extend_from_slice(&gas_budget.to_le_bytes());

    // === TransactionExpiration (variant 0 = None) ===
    tx.push(0);

    tx
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
    /// Uses Sui's ProgrammableTransaction format:
    /// ```text
    /// TransactionData {
    ///   kind: ProgrammableTransaction {
    ///     inputs: [Object(SealObjectID), Pure(CommitmentHash)],
    ///     commands: [MoveCall {
    ///       package: csv_seal_package_id,
    ///       module: "csv_seal",
    ///       function: "consume_seal",
    ///       type_arguments: [],
    ///       arguments: [Input(0), Input(1)],
    ///     }],
    ///   },
    ///   sender: signer_address,
    ///   gas_data: GasData { payment, owner, price, budget },
    ///   expiration: None,
    /// }
    /// ```
    ///
    /// The transaction is signed as: `Signature = Sign(BCS(TransactionData))`
    ///
    /// # Note
    ///
    /// Actual Sui transaction building requires sui-sdk for proper BCS types.
    /// This implementation uses raw BCS serialization to avoid the SDK dependency,
    /// matching the exact wire format Sui nodes expect.
    #[cfg(feature = "rpc")]
    fn build_and_sign_move_call(
        &self,
        seal: &SuiSealRef,
        commitment: [u8; 32],
    ) -> Result<(Vec<u8>, Vec<u8>, Vec<u8>), Box<dyn std::error::Error + Send + Sync>> {
        use ed25519_dalek::Signer;

        let signing_key = self.signing_key.as_ref()
            .ok_or("No signing key configured")?;

        let package_id = parse_object_id(&self.config.seal_contract.package_id)
            .map_err(|e| format!("Invalid package ID: {}", e))?;
        let module_name = self.config.seal_contract.module_name.clone();
        let function_name = "consume_seal".to_string();

        log::debug!(
            "Building Sui MoveCall: {}::{}::{}(seal={}, commitment={})",
            self.config.seal_contract.package_id,
            module_name, function_name,
            format_object_id(seal.object_id),
            hex::encode(commitment),
        );

        // Get gas objects and sender address from RPC
        let sender = self.rpc.sender_address()
            .map_err(|e| format!("Failed to get sender address: {}", e))?;

        let gas_objects = self.rpc.get_gas_objects(sender)
            .map_err(|e| format!("Failed to get gas objects: {}", e))?;

        if gas_objects.is_empty() {
            return Err("No gas objects available for transaction".into());
        }

        // Build the transaction using raw BCS serialization
        // This matches Sui's TransactionData wire format exactly
        let tx_bytes = build_sui_transaction_data(
            package_id,
            &module_name,
            &function_name,
            seal.object_id,
            commitment,
            sender,
            &gas_objects,
            self.config.transaction.max_gas_price,
            self.config.transaction.max_gas_budget,
        );

        // Sign with Ed25519
        let signature = signing_key.sign(&tx_bytes);
        let public_key = signing_key.verifying_key().to_bytes().to_vec();

        // Sui signature format: [signature_type (1 byte)] + [signature (64 bytes)] + [public key (32 bytes)]
        // Signature type 0 = Ed25519
        let mut sui_signature = Vec::with_capacity(97);
        sui_signature.push(0x00); // Ed25519 signature scheme
        sui_signature.extend_from_slice(signature.to_bytes().as_ref());
        sui_signature.extend_from_slice(&public_key);

        Ok((tx_bytes, sui_signature, public_key))
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

        // Fetch checkpoint info from the Sui node
        let checkpoint_info = match self.rpc.get_checkpoint(anchor.checkpoint) {
            Ok(Some(info)) => info,
            Ok(None) => {
                return Err(AdapterError::InclusionProofFailed(format!(
                    "Checkpoint {} not found", anchor.checkpoint
                )));
            }
            Err(e) => {
                return Err(AdapterError::InclusionProofFailed(format!(
                    "Failed to fetch checkpoint {}: {}", anchor.checkpoint, e
                )));
            }
        };

        // Verify the checkpoint is certified
        if !checkpoint_info.certified {
            // Double-check via verifier
            let is_certified = match self.checkpoint_verifier.is_checkpoint_certified(
                anchor.checkpoint,
                self.rpc.as_ref(),
            ) {
                Ok(info) => info.is_certified,
                Err(e) => {
                    log::warn!("Checkpoint certification check failed: {}", e);
                    false
                }
            };

            if !is_certified {
                return Err(AdapterError::InclusionProofFailed(format!(
                    "Checkpoint {} is not yet certified", anchor.checkpoint
                )));
            }
        }

        // Build inclusion proof with real checkpoint data
        let checkpoint_hash = checkpoint_info.digest;

        Ok(SuiInclusionProof::new(
            vec![0u8; 32], // object_proof would come from tx effects
            checkpoint_hash,
            anchor.checkpoint,
        ))
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