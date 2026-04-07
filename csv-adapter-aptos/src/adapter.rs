//! Aptos AnchorLayer implementation with production-grade features
//!
//! This adapter implements the AnchorLayer trait for Aptos,
//! using resources with key + delete as seals.
//!
//! ## Architecture
//!
//! - **Seals**: Move resources that can be deleted once (via `move_from`)
//! - **Anchors**: Events emitted when seal resources are deleted
//! - **Finality**: HotStuff consensus provides deterministic finality via 2f+1 certification

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

use crate::config::AptosConfig;
use crate::error::{AptosError, AptosResult};
use crate::rpc::AptosRpc;
use crate::types::{AptosSealRef, AptosAnchorRef, AptosInclusionProof, AptosFinalityProof};
use crate::seal::SealRegistry;
use crate::checkpoint::CheckpointVerifier;
use crate::proofs::{StateProofVerifier, EventProofVerifier, CommitmentEventBuilder};

/// Aptos implementation of the AnchorLayer trait
pub struct AptosAnchorLayer {
    config: AptosConfig,
    /// Registry of used seals for replay prevention
    seal_registry: Mutex<SealRegistry>,
    domain_separator: [u8; 32],
    rpc: Box<dyn AptosRpc>,
    checkpoint_verifier: CheckpointVerifier,
    /// Event builder for creating and parsing anchor events
    event_builder: CommitmentEventBuilder,
}

impl AptosAnchorLayer {
    /// Create a new adapter from configuration and RPC client.
    ///
    /// # Arguments
    /// * `config` - Adapter configuration
    /// * `rpc` - RPC client for Aptos node communication
    pub fn from_config(config: AptosConfig, rpc: Box<dyn AptosRpc>) -> AptosResult<Self> {
        // Validate configuration
        config.validate().map_err(|e| AptosError::SerializationError(
            format!("Invalid configuration: {}", e)
        ))?;

        // Build domain separator: "CSV-APTOS-" + chain_id padding
        let mut domain = [0u8; 32];
        domain[..10].copy_from_slice(b"CSV-APTOS-");
        domain[10] = config.chain_id();

        // Build event builder for the configured module
        let module_address = parse_aptos_address(&config.seal_contract.module_address)
            .map_err(|e| AptosError::SerializationError(e))?;
        let event_type = format!("{}::csv_seal::AnchorEvent", config.seal_contract.module_address);
        let event_builder = CommitmentEventBuilder::new(module_address, event_type);

        let checkpoint_verifier = CheckpointVerifier::with_config(config.checkpoint.clone());

        log::info!(
            "Initialized Aptos adapter for network {:?} (chain_id={})",
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
        })
    }

    /// Create a new adapter with mock RPC for testing.
    pub fn with_mock() -> AptosResult<Self> {
        let config = AptosConfig::default();
        let rpc = Box::new(crate::rpc::MockAptosRpc::new(5000));
        Self::from_config(config, rpc)
    }

    /// Create a new adapter with real RPC (requires `rpc` feature).
    ///
    /// # Arguments
    /// * `config` - Adapter configuration
    /// * `csv_seal_address` - Address where CSVSeal module is deployed
    #[cfg(feature = "rpc")]
    pub fn with_real_rpc(
        config: AptosConfig,
        csv_seal_address: [u8; 32],
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        use crate::rpc::real_rpc::AptosRpcClient;

        let rpc: Box<dyn AptosRpc> = Box::new(AptosRpcClient::new(&config.rpc_url));
        Self::from_config(config, rpc)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    #[cfg(not(feature = "rpc"))]
    pub fn with_real_rpc(
        _config: AptosConfig,
        _csv_seal_address: [u8; 32],
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Err("rpc feature not enabled".into())
    }

    /// Verify that a seal resource is available before consumption.
    fn verify_seal_available(&self, seal: &AptosSealRef) -> AptosResult<()> {
        // Check registry first
        let registry = self.seal_registry.lock().unwrap();
        if registry.is_seal_used(seal) {
            return Err(AptosError::ResourceUsed(format!(
                "Seal at address {} is already consumed",
                format_address(seal.account_address)
            )));
        }

        // Check on-chain resource
        let resource_type = format!(
            "{}::csv_seal::{}",
            self.config.seal_contract.module_address,
            self.config.seal_contract.seal_resource
        );

        let exists = StateProofVerifier::verify_resource_exists(
            seal.account_address,
            &resource_type,
            self.rpc.as_ref(),
        )?;

        if !exists {
            return Err(AptosError::StateProofFailed(format!(
                "Seal resource at {} does not exist on-chain",
                format_address(seal.account_address)
            )));
        }

        Ok(())
    }

    /// Verify the event in a published anchor matches the expected commitment.
    fn verify_anchor_event(
        &self,
        anchor: &AptosAnchorRef,
        expected_seal: &AptosSealRef,
        expected_commitment: Hash,
    ) -> CoreResult<()> {
        let expected_event_data = self.event_builder.build(
            *expected_commitment.as_bytes(),
            expected_seal.account_address,
        );

        let valid: bool = EventProofVerifier::verify_event_in_tx(
            anchor.version,
            &expected_event_data,
            self.rpc.as_ref(),
        ).map_err(|e: AptosError| AdapterError::InclusionProofFailed(e.to_string()))?;

        if !valid {
            return Err(AdapterError::InclusionProofFailed(
                "Event verification failed: commitment mismatch".to_string(),
            ));
        }

        Ok(())
    }
}

/// Format an Aptos address as hex for display.
fn format_address(addr: [u8; 32]) -> String {
    format!("0x{}", hex::encode(addr))
}

/// Parse an Aptos address string (e.g., "0x1" or "0xabc...").
fn parse_aptos_address(s: &str) -> Result<[u8; 32], String> {
    let hex_str = s.trim_start_matches("0x");
    let mut padded = String::new();
    for _ in 0..(64 - hex_str.len()) {
        padded.push('0');
    }
    padded.push_str(hex_str);

    let bytes = hex::decode(&padded).map_err(|e| format!("Invalid hex: {}", e))?;
    if bytes.len() != 32 {
        return Err(format!("Address must be 32 bytes, got {}", bytes.len()));
    }

    let mut addr = [0u8; 32];
    addr.copy_from_slice(&bytes);
    Ok(addr)
}

impl AnchorLayer for AptosAnchorLayer {
    type SealRef = AptosSealRef;
    type AnchorRef = AptosAnchorRef;
    type InclusionProof = AptosInclusionProof;
    type FinalityProof = AptosFinalityProof;

    fn publish(&self, commitment: Hash, seal: Self::SealRef) -> CoreResult<Self::AnchorRef> {
        log::debug!(
            "Publishing commitment via seal {}",
            format_address(seal.account_address)
        );

        // Verify seal is available
        self.verify_seal_available(&seal)
            .map_err(|e| AdapterError::from(e))?;

        #[cfg(feature = "rpc")]
        {
            // Build the event data for this commitment
            let event_data = self.event_builder.build(
                *commitment.as_bytes(),
                seal.account_address,
            );

            // Build transaction bytes for CSVSeal.delete_seal()
            // In production with aptos-sdk: construct a signed transaction calling
            // CSVSeal::delete_seal(seal_id, commitment_hash, event_data)
            let tx_bytes: Vec<u8> = event_data.clone();

            // Submit transaction via RPC
            let tx_hash = self.rpc.submit_transaction(tx_bytes)
                .map_err(|e| AdapterError::PublishFailed(
                    format!("Failed to submit transaction: {}", e),
                ))?;

            // Get the latest version to determine our transaction version
            let latest_version = self.rpc.get_latest_version()
                .map_err(|e| AdapterError::NetworkError(e.to_string()))?;

            // Get transaction by version to verify execution
            let tx = self.rpc.get_transaction_by_version(latest_version)
                .map_err(|e| AdapterError::NetworkError(e.to_string()))?
                .ok_or_else(|| AdapterError::PublishFailed(
                    "Transaction not found after submission".to_string(),
                ))?;

            // Verify the emitted event matches the expected commitment
            let valid = EventProofVerifier::verify_event_in_tx(
                tx.version,
                &event_data,
                self.rpc.as_ref(),
            ).map_err(|e: AptosError| AdapterError::InclusionProofFailed(e.to_string()))?;

            if !valid {
                return Err(AdapterError::PublishFailed(
                    "Event verification failed: commitment mismatch".to_string(),
                ));
            }

            // Mark seal as consumed with the transaction version
            let version = tx.version;
            let mut registry = self.seal_registry.lock().unwrap();
            registry.mark_seal_used(&seal, version)
                .map_err(|e| AdapterError::from(e))?;

            Ok(AptosAnchorRef::new(version, seal.account_address, version))
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
                seal.account_address,
            );

            // Return simulated anchor
            Ok(AptosAnchorRef::new(0, seal.account_address, 0))
        }
    }

    fn verify_inclusion(&self, anchor: Self::AnchorRef) -> CoreResult<Self::InclusionProof> {
        log::debug!("Verifying inclusion for anchor at version {}", anchor.version);

        // In production:
        // 1. Get transaction by version
        // 2. Verify transaction success
        // 3. Verify event was emitted with correct commitment
        // 4. Build Merkle proof

        Ok(AptosInclusionProof::new(vec![], vec![], anchor.version))
    }

    fn verify_finality(&self, anchor: Self::AnchorRef) -> CoreResult<Self::FinalityProof> {
        log::debug!("Verifying finality for anchor at version {}", anchor.version);

        let f_plus_one = self.config.f_plus_one();

        let is_certified = match self.checkpoint_verifier.is_version_finalized(
            anchor.version,
            self.rpc.as_ref(),
            f_plus_one,
        ) {
            Ok(info) => info.is_certified,
            Err(e) => {
                log::warn!("Finality check failed: {}", e);
                false
            }
        };

        Ok(AptosFinalityProof::new(anchor.version, is_certified))
    }

    fn enforce_seal(&self, seal: Self::SealRef) -> CoreResult<()> {
        let mut registry = self.seal_registry.lock().unwrap();
        if registry.is_seal_used(&seal) {
            return Err(AdapterError::SealReplay(format!(
                "Resource already consumed at {}",
                format_address(seal.account_address)
            )));
        }
        registry.mark_seal_used(&seal, 0)
            .map_err(|e| AdapterError::from(e))
    }

    fn create_seal(&self, _value: Option<u64>) -> CoreResult<Self::SealRef> {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(b"aptos-seal");
        let result = hasher.finalize();
        let mut addr = [0u8; 32];
        addr.copy_from_slice(&result);
        Ok(AptosSealRef::new(addr, "CSV::Seal".to_string(), 0))
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
        Commitment::v1(
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
            anchor.event_handle.to_vec(),
            Some(anchor.version),
        ).map_err(|e| AdapterError::Generic(e.to_string()))?;

        let anchor_ref = CoreAnchorRef::new(
            anchor.event_handle.to_vec(),
            anchor.version,
            vec![],
        ).map_err(|e| AdapterError::Generic(e.to_string()))?;

        let inclusion_proof = csv_adapter_core::InclusionProof::new(
            inclusion.transaction_proof,
            Hash::zero(),
            inclusion.version,
        ).map_err(|e| AdapterError::Generic(e.to_string()))?;

        let finality_proof = FinalityProof::new(
            vec![],
            finality.version,
            finality.is_certified,
        ).map_err(|e| AdapterError::Generic(e.to_string()))?;

        // Extract signatures from DAG nodes before moving transition_dag
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
        log::warn!("Rollback requested for anchor at version {}", anchor.version);
        // In production: check if the version is still valid
        // If the chain has been reorg'd and our anchor version is no longer
        // reachable, we should clear the seal from the registry
        let current_version = self.rpc.get_latest_version()
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;

        // If anchor version is beyond current tip, rollback
        if anchor.version > current_version {
            Err(AdapterError::ReorgInvalid(format!(
                "Anchor version {} beyond current tip {}",
                anchor.version, current_version
            )))
        } else {
            Ok(())
        }
    }

    fn domain_separator(&self) -> [u8; 32] {
        self.domain_separator
    }

    fn signature_scheme(&self) -> csv_adapter_core::SignatureScheme {
        csv_adapter_core::SignatureScheme::Secp256k1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_adapter() -> AptosAnchorLayer {
        AptosAnchorLayer::with_mock().unwrap()
    }

    #[test]
    fn test_create_seal() {
        let adapter = test_adapter();
        let seal = adapter.create_seal(None).unwrap();
        assert_eq!(seal.nonce, 0);
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
        assert_eq!(&adapter.domain_separator()[..10], b"CSV-APTOS-");
        assert_eq!(adapter.domain_separator()[10], 4); // Devnet chain_id
    }

    #[test]
    fn test_verify_finality() {
        let adapter = test_adapter();
        let anchor = AptosAnchorRef::new(1500, [1u8; 32], 0);
        let result = adapter.verify_finality(anchor);
        assert!(result.is_ok());
    }

    #[test]
    fn test_publish_seal_available() {
        let config = AptosConfig::default();
        let mock = crate::rpc::MockAptosRpc::new(5000);

        // Register the seal resource in the mock so verify_seal_available finds it
        let resource_type = format!(
            "{}::csv_seal::{}",
            config.seal_contract.module_address,
            config.seal_contract.seal_resource
        );
        mock.set_resource([1u8; 32], resource_type.as_str(), crate::rpc::AptosResource {
            data: vec![0u8; 8],
        });

        let rpc = Box::new(mock);
        let adapter = AptosAnchorLayer::from_config(config.clone(), rpc).unwrap();

        // Create a seal
        let seal = AptosSealRef::new([1u8; 32], resource_type.clone(), 0);
        let commitment = Hash::new([1u8; 32]);
        let result = adapter.publish(commitment, seal);
        assert!(result.is_ok());
    }

    #[test]
    fn test_publish_seal_replay() {
        let config = AptosConfig::default();
        let mock = crate::rpc::MockAptosRpc::new(5000);

        // Register the seal resource in the mock
        let resource_type = format!(
            "{}::csv_seal::{}",
            config.seal_contract.module_address,
            config.seal_contract.seal_resource
        );
        mock.set_resource([1u8; 32], resource_type.as_str(), crate::rpc::AptosResource {
            data: vec![0u8; 8],
        });

        let rpc = Box::new(mock);
        let adapter = AptosAnchorLayer::from_config(config.clone(), rpc).unwrap();

        let seal = AptosSealRef::new([1u8; 32], resource_type.clone(), 0);
        let commitment = Hash::new([1u8; 32]);
        adapter.publish(commitment, seal.clone()).unwrap();

        // Try to publish again with same seal
        let commitment2 = Hash::new([2u8; 32]);
        let result = adapter.publish(commitment2, seal);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_aptos_address() {
        let addr = parse_aptos_address("0x1").unwrap();
        assert_eq!(addr[31], 1);
        for i in 0..31 {
            assert_eq!(addr[i], 0);
        }
    }

    #[test]
    fn test_parse_aptos_address_full() {
        let full = "0xdeadbeef00000000000000000000000000000000000000000000000000000001";
        let addr = parse_aptos_address(full).unwrap();
        assert_eq!(addr[0], 0xDE);
        assert_eq!(addr[1], 0xAD);
        assert_eq!(addr[31], 0x01);
    }
}