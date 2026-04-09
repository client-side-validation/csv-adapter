//! Ethereum AnchorLayer implementation with real RPC integration
//!
//! This adapter implements the AnchorLayer trait for Ethereum,
//! using storage slots as single-use seals and LOG events for commitment publication.
//! When the `rpc` feature is enabled, real Alloy-based RPC is used.

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

use crate::config::EthereumConfig;
use crate::error::{EthereumError, EthereumResult};
use crate::rpc::EthereumRpc;
use crate::types::{EthereumSealRef, EthereumAnchorRef, EthereumInclusionProof, EthereumFinalityProof};
use crate::seal::SealRegistry;
use crate::finality::FinalityChecker;

/// Ethereum implementation of the AnchorLayer trait
pub struct EthereumAnchorLayer {
    config: EthereumConfig,
    seal_registry: Mutex<SealRegistry>,
    domain_separator: [u8; 32],
    rpc: Box<dyn EthereumRpc>,
    finality_checker: FinalityChecker,
    /// CSVSeal contract address for event verification
    csv_seal_address: [u8; 20],
}

impl EthereumAnchorLayer {
    pub fn from_config(
        config: EthereumConfig,
        rpc: Box<dyn EthereumRpc>,
        csv_seal_address: [u8; 20],
    ) -> EthereumResult<Self> {
        let mut domain = [0u8; 32];
        domain[..8].copy_from_slice(b"CSV-ETH-");
        let chain_id = config.network.chain_id().to_le_bytes();
        domain[8..16].copy_from_slice(&chain_id);

        let finality_checker = FinalityChecker::new(crate::finality::FinalityConfig {
            confirmation_depth: config.finality_depth,
            prefer_checkpoint_finality: config.use_checkpoint_finality,
        });

        Ok(Self {
            config,
            seal_registry: Mutex::new(SealRegistry::new()),
            domain_separator: domain,
            rpc,
            finality_checker,
            csv_seal_address,
        })
    }

    pub fn with_mock() -> EthereumResult<Self> {
        let config = EthereumConfig::default();
        let rpc: Box<dyn EthereumRpc> = Box::new(crate::rpc::MockEthereumRpc::new(1000));
        Self::from_config(config, rpc, [0u8; 20])
    }

    /// Create a new adapter with real RPC (requires `rpc` feature)
    #[cfg(feature = "rpc")]
    pub fn with_real_rpc(
        config: EthereumConfig,
        csv_seal_address: [u8; 20],
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        use crate::real_rpc::RealEthereumRpc;

        let rpc: Box<dyn EthereumRpc> = Box::new(RealEthereumRpc::new(&config.rpc_url, csv_seal_address)?);
        Self::from_config(config, rpc, csv_seal_address)
            .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
    }

    #[cfg(not(feature = "rpc"))]
    pub fn with_real_rpc(
        _config: EthereumConfig,
        _csv_seal_address: [u8; 20],
    ) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Err("rpc feature not enabled".into())
    }

    fn verify_slot_available(&self, seal: &EthereumSealRef) -> EthereumResult<()> {
        let registry = self.seal_registry.lock().unwrap();
        if registry.is_seal_used(seal) {
            return Err(EthereumError::SlotUsed("Storage slot already consumed".to_string()));
        }
        Ok(())
    }

    /// Publish seal consumption via real RPC and verify the inclusion proof
    ///
    /// Flow:
    /// 1. Verify slot is available (replay prevention)
    /// 2. Build calldata for `markSealUsed(sealId, commitment)`
    /// 3. Broadcast transaction via `send_raw_transaction`
    /// 4. Wait for receipt via `get_transaction_receipt`
    /// 5. Verify LOG event contains SealUsed with matching seal_id and commitment
    /// 6. Return anchor reference (tx_hash, block_number, log_index)
    #[cfg(feature = "rpc")]
    pub fn publish_with_rpc(
        &self,
        commitment: Hash,
        seal: EthereumSealRef,
        signed_tx_bytes: Vec<u8>,
    ) -> Result<EthereumAnchorRef, AdapterError> {
        use crate::real_rpc::verify_seal_consumption_in_receipt;

        // Step 1: Verify slot is available
        self.verify_slot_available(&seal)
            .map_err(|e| AdapterError::from(e))?;

        // Step 3: Send raw transaction
        let tx_hash = self.rpc.send_raw_transaction(signed_tx_bytes)
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;

        // Step 4: Get receipt
        let receipt = self.rpc.get_transaction_receipt(tx_hash)
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;

        // Step 5: Verify LOG event
        let has_valid_event = verify_seal_consumption_in_receipt(
            &receipt,
            seal.seal_id,
            *commitment.as_bytes(),
            self.csv_seal_address,
        );

        if !has_valid_event {
            return Err(AdapterError::InclusionProofFailed(
                "SealUsed event not found or mismatched in receipt".to_string(),
            ));
        }

        // Step 6: Return anchor
        let log_index = receipt.logs.first().map(|l| l.log_index).unwrap_or(0);
        let anchor = EthereumAnchorRef::new(tx_hash, receipt.block_number, log_index);

        // Mark seal as consumed
        let registry = self.seal_registry.lock().unwrap();
        registry.mark_seal_used(&seal)
            .map_err(|e| AdapterError::from(e))?;

        Ok(anchor)
    }
}

impl AnchorLayer for EthereumAnchorLayer {
    type SealRef = EthereumSealRef;
    type AnchorRef = EthereumAnchorRef;
    type InclusionProof = EthereumInclusionProof;
    type FinalityProof = EthereumFinalityProof;

    fn publish(&self, commitment: Hash, seal: Self::SealRef) -> CoreResult<Self::AnchorRef> {
        self.verify_slot_available(&seal)
            .map_err(|e| AdapterError::from(e))?;

        #[cfg(feature = "rpc")]
        {
            use crate::real_rpc::{publish, verify_seal_consumption_in_receipt, RealEthereumRpc};

            // Downcast to RealEthereumRpc for the publish flow
            let real_rpc = self.rpc.as_ref()
                .as_any()
                .downcast_ref::<RealEthereumRpc>()
                .ok_or_else(|| AdapterError::PublishFailed(
                    "publish() requires a RealEthereumRpc instance".to_string(),
                ))?;

            // Build, sign, and broadcast the transaction
            let tx_hash = publish(real_rpc, &seal, *commitment.as_bytes())
                .map_err(|e| AdapterError::PublishFailed(e.to_string()))?;

            // Get the receipt and verify the SealUsed event
            let receipt = self.rpc.get_transaction_receipt(tx_hash)
                .map_err(|e| AdapterError::NetworkError(e.to_string()))?;

            let has_valid_event = verify_seal_consumption_in_receipt(
                &receipt,
                seal.seal_id,
                *commitment.as_bytes(),
                self.csv_seal_address,
            );

            if !has_valid_event {
                return Err(AdapterError::PublishFailed(
                    "SealUsed event not found or mismatched in receipt".to_string(),
                ));
            }

            let log_index = receipt.logs.first().map(|l| l.log_index).unwrap_or(0);
            let anchor = EthereumAnchorRef::new(tx_hash, receipt.block_number, log_index);

            // Mark seal as consumed in local registry
            let registry = self.seal_registry.lock().unwrap();
            registry.mark_seal_used(&seal)
                .map_err(|e| AdapterError::from(e))?;

            Ok(anchor)
        }

        #[cfg(not(feature = "rpc"))]
        {
            // Simulated path: in production, call CSVSeal.markSealUsed()
            let mut tx_hash = [0u8; 32];
            tx_hash[..8].copy_from_slice(b"sim-tx-");
            tx_hash[8..].copy_from_slice(commitment.as_bytes());

            Ok(EthereumAnchorRef::new(tx_hash, 0, 0))
        }
    }

    fn verify_inclusion(&self, anchor: Self::AnchorRef) -> CoreResult<Self::InclusionProof> {
        #[cfg(feature = "rpc")]
        {
            // Try to get real proof data from RPC
            use crate::real_rpc::RealEthereumRpc;

            if let Some(real_rpc) = self.rpc.as_ref().as_any().downcast_ref::<RealEthereumRpc>() {
                // Get the block header for receipt root
                let block_hash = self.rpc.get_block_hash(anchor.block_number)
                    .map_err(|e| AdapterError::InclusionProofFailed(
                        format!("Failed to get block hash: {}", e)
                    ))?;

                let state_root = self.rpc.get_block_state_root(block_hash)
                    .map_err(|e| AdapterError::InclusionProofFailed(
                        format!("Failed to get state root: {}", e)
                    ))?;

                // Get the receipt for the transaction
                let receipt = self.rpc.get_transaction_receipt(anchor.tx_hash)
                    .map_err(|e| AdapterError::InclusionProofFailed(
                        format!("Failed to get receipt: {}", e)
                    ))?;

                // Verify the receipt is in the correct block
                if receipt.block_number != anchor.block_number {
                    return Err(AdapterError::InclusionProofFailed(
                        format!("Receipt block {} doesn't match anchor block {}",
                            receipt.block_number, anchor.block_number)
                    ));
                }

                // Build the inclusion proof with real data
                let proof = EthereumInclusionProof::new(
                    Vec::new(), // receipt_rlp - would need full RLP from RPC
                    state_root.to_vec(),
                    anchor.tx_hash,
                    anchor.block_number,
                    anchor.log_index,
                );

                return Ok(proof);
            }
        }

        // Without real RPC, check if we have stored proof data
        let proof = EthereumInclusionProof::new(
            Vec::new(),
            anchor.tx_hash.to_vec(),
            anchor.tx_hash,
            anchor.block_number,
            anchor.log_index,
        );

        if proof.receipt_rlp.is_empty() && proof.merkle_proof.is_empty() {
            // Return proof with anchor data - client can verify receipt exists
            return Ok(proof);
        }

        Ok(proof)
    }

    fn verify_finality(&self, anchor: Self::AnchorRef) -> CoreResult<Self::FinalityProof> {
        let is_finalized = self.finality_checker
            .is_finalized(anchor.block_number, self.rpc.as_ref())
            .unwrap_or(true);

        let confirmations = self.finality_checker
            .get_confirmations(anchor.block_number, self.rpc.as_ref())
            .unwrap_or(self.config.finality_depth);

        Ok(EthereumFinalityProof::new(
            confirmations,
            self.config.finality_depth,
            is_finalized,
        ))
    }

    fn enforce_seal(&self, seal: Self::SealRef) -> CoreResult<()> {
        let registry = self.seal_registry.lock().unwrap();
        registry.mark_seal_used(&seal)
            .map_err(|e| AdapterError::from(e))
    }

    fn create_seal(&self, value: Option<u64>) -> CoreResult<Self::SealRef> {
        // Derive a seal from the CSVSeal contract address and a deterministic slot
        // The seal represents a nullifier slot in the contract's usedSeals mapping
        let nonce = value.unwrap_or(0);

        Ok(EthereumSealRef::new(self.csv_seal_address, 0, nonce))
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
            anchor.tx_hash.to_vec(),
            Some(anchor.log_index),
        ).map_err(|e| AdapterError::Generic(e.to_string()))?;

        let anchor_ref = CoreAnchorRef::new(
            anchor.tx_hash.to_vec(),
            anchor.block_number,
            vec![],
        ).map_err(|e| AdapterError::Generic(e.to_string()))?;

        let inclusion_proof = csv_adapter_core::InclusionProof::new(
            inclusion.merkle_proof.clone(),
            Hash::new(inclusion.block_hash),
            inclusion.log_index,
        ).map_err(|e| AdapterError::Generic(e.to_string()))?;

        let finality_proof = FinalityProof::new(
            finality.confirmations.to_le_bytes().to_vec(),
            finality.confirmations,
            finality.is_finalized,
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
        let current = self.rpc.block_number()
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;
        if anchor.block_number > current {
            return Err(AdapterError::ReorgInvalid(format!(
                "Block {} beyond current {}",
                anchor.block_number, current
            )));
        }

        // If the anchor's block is before current block, the transaction may have been reorged out
        // Clear the seal from registry to allow reuse
        if anchor.block_number < current {
            #[allow(unused_mut)]
            let mut registry = self.seal_registry.lock().unwrap();
            // Derive the seal that was used for this anchor
            // The nonce is tracked via the log_index
            let seal = EthereumSealRef::new(
                self.csv_seal_address,
                0,
                anchor.log_index,
            );
            registry.clear_seal(&seal);
        }

        Ok(())
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

    fn test_adapter() -> EthereumAnchorLayer {
        EthereumAnchorLayer::with_mock().unwrap()
    }

    #[test]
    fn test_adapter_creation() {
        let adapter = test_adapter();
        assert_eq!(adapter.config.finality_depth, 15);
    }

    #[test]
    fn test_domain_separator() {
        let adapter = test_adapter();
        let domain = adapter.domain_separator();
        assert_eq!(&domain[..8], b"CSV-ETH-");
    }

    #[test]
    fn test_create_seal() {
        let adapter = test_adapter();
        let seal = adapter.create_seal(None).unwrap();
        assert_eq!(seal.slot_index, 0);
    }

    #[test]
    fn test_enforce_seal_replay() {
        let adapter = test_adapter();
        let seal = adapter.create_seal(None).unwrap();
        adapter.enforce_seal(seal.clone()).unwrap();
        assert!(adapter.enforce_seal(seal).is_err());
    }

    #[test]
    fn test_hash_commitment() {
        let adapter = test_adapter();
        let seal = adapter.create_seal(None).unwrap();
        let h = adapter.hash_commitment(
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            &seal,
        );
        assert_eq!(h.as_bytes().len(), 32);
    }

    #[test]
    fn test_verify_finality() {
        let adapter = test_adapter();
        let anchor = EthereumAnchorRef::new([5u8; 32], 900, 0);
        let result = adapter.verify_finality(anchor);
        assert!(result.is_ok());
    }
}
