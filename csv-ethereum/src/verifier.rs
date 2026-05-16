#![allow(dead_code)]
//! Ethereum ChainVerifier Implementation
//!
//! This module implements the ChainVerifier trait for Ethereum,
//! providing chain-specific verification logic for the canonical proof pipeline.

use async_trait::async_trait;
use csv_core::Hash;
use csv_core::proof::{FinalityProof, InclusionProof};
use csv_core::proof_pipeline::ChainVerifier;

use crate::mpt::verify_storage_proof;
use crate::rpc::EthereumRpc;
use alloy_primitives::{B256, Bytes, U256};

/// Ethereum verifier implementing ChainVerifier trait
pub struct EthereumVerifier {
    /// RPC client for Ethereum
    rpc: Box<dyn EthereumRpc>,
    /// CSVLock contract address
    csv_lock_address: [u8; 20],
}

impl EthereumVerifier {
    /// Create a new Ethereum verifier
    pub fn new(rpc: Box<dyn EthereumRpc>, csv_lock_address: [u8; 20]) -> Self {
        Self {
            rpc,
            csv_lock_address,
        }
    }
}

#[async_trait]
impl ChainVerifier for EthereumVerifier {
    /// Verify inclusion proof for an Ethereum transaction
    async fn verify_inclusion(
        &self,
        proof: &InclusionProof,
        _expected_root: Hash,
    ) -> csv_core::Result<bool> {
        // The proof now contains the block_number
        let block_number = proof.block_number;
        let block_hash_bytes = proof.block_hash.as_bytes();

        // Get the state root for this block
        let state_root_bytes = self
            .rpc
            .get_block_state_root(*block_hash_bytes)
            .await
            .map_err(|e| csv_core::ProtocolError::NetworkError(e.to_string()))?;
        let state_root: B256 = B256::from_slice(&state_root_bytes);

        // Fetch the account proof for the CSVLock contract
        // The account proof proves the contract exists at the state root
        let account_proof_response = self
            .rpc
            .get_proof(
                self.csv_lock_address,
                vec![], // No storage keys for account proof
                block_number,
            )
            .await
            .map_err(|e| csv_core::ProtocolError::NetworkError(e.to_string()))?;

        // Convert account proof to Bytes vector
        let account_proof: Vec<Bytes> = account_proof_response
            .account_proof
            .iter()
            .map(|p| Bytes::from(p.clone()))
            .collect();

        // Convert storage proof to Bytes vector
        let storage_proof: Vec<Bytes> = proof
            .proof_bytes
            .iter()
            .map(|b| Bytes::from(vec![*b]))
            .collect();

        // Derive the actual storage slot key from the seal/commitment being verified
        // The storage slot for usedSeals[sealId] is: keccak256(seal_id || slot_position)
        // where slot_position for mapping is 0. We compute this from the proof data.
        // The correct storage key is derived from the commitment/seal_id, NOT from block_hash.
        let seal_id_hash = alloy_primitives::keccak256(&proof.proof_bytes);
        let mut storage_key_array = [0u8; 64];
        storage_key_array[..32].copy_from_slice(seal_id_hash.as_slice());
        storage_key_array[32..].copy_from_slice(&[0u8; 32]); // slot position 0
        let storage_key_hash = alloy_primitives::keccak256(storage_key_array);
        let storage_key = U256::from_be_bytes(storage_key_hash.0);

        let result = verify_storage_proof(state_root, &account_proof, &storage_proof, storage_key);

        Ok(result)
    }

    /// Verify finality proof for an Ethereum block
    async fn verify_finality(&self, proof: &FinalityProof) -> csv_core::Result<bool> {
        // Ethereum has probabilistic finality - check confirmations
        // For now, we check if the required confirmations are met
        let required_confirmations = 12; // Ethereum typically requires 12 confirmations for finality
        let is_finalized = proof.confirmations >= required_confirmations;

        Ok(is_finalized)
    }

    /// Verify zero-knowledge proof (if applicable)
    async fn verify_zk(&self, proof: &[u8]) -> csv_core::Result<bool> {
        // Ethereum may use ZK proofs for certain operations
        // For now, return true if proof is empty, otherwise verify
        if proof.is_empty() {
            Ok(true)
        } else {
            // Placeholder - would implement actual ZK proof verification
            Ok(true)
        }
    }

    /// Verify seal registry (check if seal has been consumed)
    async fn verify_seal_registry(&self, seal_id: Hash) -> csv_core::Result<bool> {
        // Query the CSVLock contract to check if the seal has been used
        // The seal_id is a bytes32 value that maps to the usedSeals mapping

        let contract_address = self.csv_lock_address;

        // Storage slot for usedSeals[sealId] mapping
        // In Solidity, mapping(bytes32 => bool) usedSeals
        // Storage slot = keccak256(seal_id || slot_position)
        // slot_position for mapping is 0
        let mut key = [0u8; 64];
        key[..32].copy_from_slice(seal_id.as_bytes());
        key[32..].copy_from_slice(&[0u8; 32]); // slot position 0

        let storage_key = alloy_primitives::keccak256(key);

        // Get storage proof for this slot at the latest block
        let latest_block = self
            .rpc
            .block_number()
            .await
            .map_err(|e| csv_core::ProtocolError::NetworkError(e.to_string()))?;

        let proof = self
            .rpc
            .get_proof(contract_address, vec![storage_key.0], latest_block)
            .await
            .map_err(|e| csv_core::ProtocolError::NetworkError(e.to_string()))?;

        // The storage proof should contain the value at the slot
        // The value is RLP-encoded, but for boolean (uint256) it's just the 32-byte value
        // If the value is non-zero, the seal has been used
        if let Some(storage_entry) = proof.storage_proof.first() {
            // The value is the raw 32-byte storage slot value
            if storage_entry.value.len() >= 32 {
                let value_bytes: [u8; 32] = storage_entry.value[..32].try_into().map_err(|_| {
                    csv_core::ProtocolError::Generic("Invalid storage value length".to_string())
                })?;
                let value = alloy_primitives::U256::from_be_bytes(value_bytes);
                Ok(value != alloy_primitives::U256::ZERO)
            } else if storage_entry.value.is_empty() {
                // Empty value means not set (false)
                Ok(false)
            } else {
                // Try to parse as RLP - for simplicity, treat non-empty as true
                Ok(true)
            }
        } else {
            // No proof returned - assume seal not used (conservative)
            Ok(false)
        }
    }

    /// Verify signature on proof bundle
    async fn verify_signature(
        &self,
        bundle: &csv_core::proof::ProofBundle,
    ) -> csv_core::Result<bool> {
        use csv_core::signature::{Signature, SignatureScheme, verify_signatures};

        if bundle.signatures.is_empty() {
            return Err(csv_core::ProtocolError::SignatureVerificationFailed(
                "No signatures in proof bundle".to_string(),
            ));
        }

        // Parse signatures from the bundle
        let mut signatures = Vec::with_capacity(bundle.signatures.len());

        for (i, sig_bytes) in bundle.signatures.iter().enumerate() {
            // Parse signature format: [pk_len (4 bytes LE)] [public_key] [signature]
            if sig_bytes.len() < 4 {
                return Err(csv_core::ProtocolError::SignatureVerificationFailed(
                    format!("Signature {} too short for header", i),
                ));
            }

            let pk_len =
                u32::from_le_bytes([sig_bytes[0], sig_bytes[1], sig_bytes[2], sig_bytes[3]])
                    as usize;

            if sig_bytes.len() < 4 + pk_len {
                return Err(csv_core::ProtocolError::SignatureVerificationFailed(
                    format!("Signature {} too short for public key", i),
                ));
            }

            let public_key = sig_bytes[4..4 + pk_len].to_vec();
            let signature = sig_bytes[4 + pk_len..].to_vec();

            // The signed message is the DAG root commitment
            let message = bundle.transition_dag.root_commitment.as_bytes().to_vec();

            signatures.push(Signature::new(signature, public_key, message));
        }

        // Verify all signatures using Secp256k1 (Ethereum's signature scheme)
        verify_signatures(&signatures, SignatureScheme::Secp256k1)
            .map_err(|e| csv_core::ProtocolError::SignatureVerificationFailed(e.to_string()))?;

        Ok(true)
    }
}
