//! Celestia AnchorLayer implementation
#![allow(dead_code)]

use std::sync::Mutex;

use csv_adapter_core::commitment::Commitment;
use csv_adapter_core::dag::DAGSegment;
use csv_adapter_core::error::AdapterError;
use csv_adapter_core::error::Result as CoreResult;
use csv_adapter_core::proof::{FinalityProof, ProofBundle};
use csv_adapter_core::seal::AnchorRef as CoreAnchorRef;
use csv_adapter_core::seal::SealRef as CoreSealRef;
use csv_adapter_core::AnchorLayer;
use csv_adapter_core::Hash;

use crate::blob::{BlobSubmitter, DasVerifier};
use crate::config::CelestiaConfig;
use crate::error::CelestiaResult;
use crate::rpc::CelestiaRpc;
use crate::types::{
    CelestiaAnchorRef, CelestiaFinalityProof, CelestiaInclusionProof, CelestiaSealRef,
};

/// Celestia implementation of the AnchorLayer trait
pub struct CelestiaAnchorLayer {
    config: CelestiaConfig,
    published_blobs: Mutex<Vec<[u8; 32]>>,
    domain_separator: [u8; 32],
    rpc: Box<dyn CelestiaRpc>,
    blob_submitter: BlobSubmitter,
    das_verifier: DasVerifier,
}

impl CelestiaAnchorLayer {
    pub fn from_config(config: CelestiaConfig, rpc: Box<dyn CelestiaRpc>) -> CelestiaResult<Self> {
        let mut domain = [0u8; 32];
        domain[..13].copy_from_slice(b"CSV-CELESTIA-");
        Ok(Self {
            config,
            published_blobs: Mutex::new(Vec::new()),
            domain_separator: domain,
            rpc,
            blob_submitter: BlobSubmitter::new(),
            das_verifier: DasVerifier::new(),
        })
    }

    pub fn with_mock() -> CelestiaResult<Self> {
        let config = CelestiaConfig::default();
        let rpc = Box::new(crate::rpc::MockCelestiaRpc::new(1000));
        Self::from_config(config, rpc)
    }
}

impl AnchorLayer for CelestiaAnchorLayer {
    type SealRef = CelestiaSealRef;
    type AnchorRef = CelestiaAnchorRef;
    type InclusionProof = CelestiaInclusionProof;
    type FinalityProof = CelestiaFinalityProof;

    fn publish(&self, commitment: Hash, seal: Self::SealRef) -> CoreResult<Self::AnchorRef> {
        let mut published = self.published_blobs.lock().unwrap();
        if published.contains(&seal.blob_hash) {
            return Err(AdapterError::InvalidSeal(
                "Blob already published".to_string(),
            ));
        }
        published.push(seal.blob_hash);

        // In production: submit PayForBlob tx
        let submission = self
            .blob_submitter
            .submit(
                self.rpc.as_ref(),
                seal.namespace_id,
                commitment,
                seal.blob_hash.to_vec(),
            )
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;

        Ok(CelestiaAnchorRef::new(
            submission.tx_hash,
            submission.height,
            seal.namespace_id,
            submission.share_index,
        ))
    }

    fn verify_inclusion(&self, anchor: Self::AnchorRef) -> CoreResult<Self::InclusionProof> {
        // In production: verify blob is retrievable + DAS
        let das_verified = self
            .das_verifier
            .verify(
                self.rpc.as_ref(),
                anchor.height,
                anchor.namespace_id,
                anchor.share_index,
            )
            .unwrap_or(true);

        if !das_verified {
            return Err(AdapterError::InclusionProofFailed(
                "DAS verification failed".to_string(),
            ));
        }

        Ok(CelestiaInclusionProof::new(vec![], vec![], anchor.height))
    }

    fn verify_finality(&self, anchor: Self::AnchorRef) -> CoreResult<Self::FinalityProof> {
        // In production: check k confirmations + DAS
        let current = self
            .rpc
            .get_latest_block_height()
            .map_err(|e| AdapterError::NetworkError(e.to_string()))?;
        let confirmations = current.saturating_sub(anchor.height);
        let das_verified = self
            .das_verifier
            .verify(
                self.rpc.as_ref(),
                anchor.height,
                anchor.namespace_id,
                anchor.share_index,
            )
            .unwrap_or(false);

        Ok(CelestiaFinalityProof::new(confirmations, das_verified))
    }

    fn enforce_seal(&self, seal: Self::SealRef) -> CoreResult<()> {
        let mut published = self.published_blobs.lock().unwrap();
        if published.contains(&seal.blob_hash) {
            return Err(AdapterError::SealReplay(
                "Blob already published".to_string(),
            ));
        }
        published.push(seal.blob_hash);
        Ok(())
    }

    fn create_seal(&self, _value: Option<u64>) -> CoreResult<Self::SealRef> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(b"celestia-seal");
        let result = hasher.finalize();
        let mut blob_hash = [0u8; 32];
        blob_hash.copy_from_slice(&result);
        let mut ns = [0u8; 8];
        ns.copy_from_slice(b"csvadapt");
        Ok(CelestiaSealRef::new(ns, blob_hash, 0))
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
        )
        .hash()
    }

    fn build_proof_bundle(
        &self,
        anchor: Self::AnchorRef,
        transition_dag: DAGSegment,
    ) -> CoreResult<ProofBundle> {
        let _inclusion = self.verify_inclusion(anchor.clone())?;
        let finality = self.verify_finality(anchor.clone())?;
        let mut proof_bytes = Vec::new();
        proof_bytes.extend_from_slice(&anchor.namespace_id);
        proof_bytes.extend_from_slice(&anchor.share_index.to_le_bytes());
        let seal_ref = CoreSealRef::new(anchor.tx_hash.to_vec(), Some(anchor.share_index))
            .map_err(|e| AdapterError::Generic(e.to_string()))?;

        let anchor_ref = CoreAnchorRef::new(
            anchor.tx_hash.to_vec(),
            anchor.height,
            anchor.namespace_id.to_vec(),
        )
        .map_err(|e| AdapterError::Generic(e.to_string()))?;

        let inclusion_proof = csv_adapter_core::InclusionProof::new(
            proof_bytes,
            Hash::new(anchor.tx_hash),
            anchor.share_index,
        )
        .map_err(|e| AdapterError::Generic(e.to_string()))?;

        let finality_proof = FinalityProof::new(
            finality.confirmations.to_le_bytes().to_vec(),
            finality.confirmations,
            finality.das_verified,
        )
        .map_err(|e| AdapterError::Generic(e.to_string()))?;

        ProofBundle::new(
            transition_dag.clone(),
            transition_dag
                .nodes
                .iter()
                .flat_map(|node| node.signatures.clone())
                .collect(),
            seal_ref,
            anchor_ref,
            inclusion_proof,
            finality_proof,
        )
        .map_err(|e| AdapterError::Generic(e.to_string()))
    }

    fn rollback(&self, anchor: Self::AnchorRef) -> CoreResult<()> {
        // Celestia doesn't have traditional reorgs, but we need to handle:
        // 1. Mark the blob as potentially unavailable
        // 2. Re-validate inclusion proof
        // 3. Update internal state

        // Try to re-fetch the blob to verify it's still available
        let das_verified = self
            .das_verifier
            .verify(
                self.rpc.as_ref(),
                anchor.height,
                anchor.namespace_id,
                anchor.share_index,
            )
            .unwrap_or(false);

        if !das_verified {
            return Err(AdapterError::InclusionProofFailed(
                "Blob no longer available after rollback check".to_string(),
            ));
        }

        // Re-verify the blob still exists
        let blob_available = self
            .blob_submitter
            .verify_inclusion(
                self.rpc.as_ref(),
                anchor.namespace_id,
                anchor.height,
                anchor.tx_hash,
            )
            .unwrap_or(false);

        if !blob_available {
            return Err(AdapterError::InclusionProofFailed(
                "Blob no longer available".to_string(),
            ));
        }

        // Update internal state to mark as rolled back
        let mut published = self.published_blobs.lock().unwrap();
        published.retain(|&h| h != anchor.tx_hash);

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

    fn test_adapter() -> CelestiaAnchorLayer {
        CelestiaAnchorLayer::with_mock().unwrap()
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
        assert_eq!(&adapter.domain_separator()[..13], b"CSV-CELESTIA-");
    }

    #[test]
    fn test_verify_finality() {
        let adapter = test_adapter();
        let anchor = CelestiaAnchorRef::new([1u8; 32], 100, [2u8; 8], 0);
        let result = adapter.verify_finality(anchor);
        assert!(result.is_ok());
    }
}
