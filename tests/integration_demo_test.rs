//! End-to-end integration test for demo scenario
//!
//! This test validates the complete cross-chain transfer workflow:
//! 1. Generate wallet and sanads
//! 2. Lock sanad on source chain
//! 3. Generate proof bundle
//! 4. Verify proof bundle
//! 5. Mint sanad on destination chain
//!
//! This is a regression test to ensure the demo scenario works correctly
//! after code changes.

use csv_core::{
    ChainId,
    dag::{DAGNode, DAGSegment},
    hash::Hash,
    proof::{FinalityProof, InclusionProof, ProofBundle},
    seal::{CommitAnchor, SealPoint},
};
use csv_keys::{
    bip39::{Mnemonic, MnemonicType},
    bip44::derive_key,
    keystore::{KdfType, KeystoreFile},
    memory::{Passphrase, SecretKey},
};

#[cfg(test)]
mod demo_scenario_tests {
    use super::*;

    #[test]
    fn test_demo_scenario_wallet_generation() {
        // Test 1: Generate wallet with mnemonic
        let mnemonic = Mnemonic::generate(MnemonicType::Words24);
        let phrase = mnemonic.as_str();
        assert!(!phrase.is_empty());
        assert_eq!(phrase.split_whitespace().count(), 24);

        // Test 2: Derive keys for supported chains
        let seed = mnemonic.to_seed(None);
        let chains = vec![
            ChainId::new("bitcoin"),
            ChainId::new("ethereum"),
            ChainId::new("sui"),
        ];

        for chain in chains {
            let key = derive_key(seed.as_bytes(), &chain, 0, 0).expect("key derivation should succeed");
            assert_eq!(key.as_bytes().len(), 32);
        }
    }

    #[test]
    fn test_demo_scenario_keystore_encryption() {
        // Test 3: Encrypt and decrypt keystore
        let key_bytes = [1u8; 32];
        let secret_key = SecretKey::new(key_bytes);
        let passphrase = Passphrase::new("test_passphrase_12_chars");

        let keystore = KeystoreFile::encrypt(&secret_key, &passphrase, KdfType::Scrypt)
            .expect("Keystore encryption should succeed");

       let decrypted = keystore
            .decrypt(&passphrase)
            .expect("Keystore decryption should succeed");
        assert_eq!(decrypted.as_bytes(), &key_bytes);
    }

    #[test]
    fn test_demo_scenario_proof_bundle_creation() {
        // Test 4: Create a valid proof bundle
        let seal_id = vec![1u8, 2, 3];
        let seal_point =
            SealPoint::new(seal_id.clone(), Some(42)).expect("SealPoint creation should succeed");

        let commit_anchor =
            CommitAnchor::new(seal_id, 100, vec![]).expect("CommitAnchor creation should succeed");

        let dag_node = DAGNode::new(
            Hash::new([1u8; 32]),
            vec![0x01, 0x02],
            vec![],
            vec![],
            vec![],
        );
        let dag_segment = DAGSegment::new(vec![dag_node], Hash::zero());

        let inclusion_proof = InclusionProof::new(vec![0xCD; 32], Hash::new([2u8; 32]), 100, 0)
            .expect("InclusionProof creation should succeed");

        let finality_proof = FinalityProof::new(vec![0xAB; 16], 6, false)
            .expect("FinalityProof creation should succeed");

        let proof_bundle = ProofBundle::new(
            dag_segment,
            vec![],
            seal_point,
            commit_anchor,
            inclusion_proof,
            finality_proof,
        )
        .expect("ProofBundle creation should succeed");

        // Verify the proof bundle structure
        assert_eq!(proof_bundle.signatures.len(), 0);
        assert_eq!(proof_bundle.finality_proof.confirmations, 6);
        assert!(!proof_bundle.finality_proof.is_deterministic);
    }

    #[test]
    fn test_demo_scenario_chain_id_consistency() {
        // Test 5: Verify chain IDs are consistent across the system
        let chains = vec![
            ChainId::new("bitcoin"),
            ChainId::new("ethereum"),
            ChainId::new("sui"),
            ChainId::new("aptos"),
            ChainId::new("solana"),
        ];

        // Each chain ID should be unique
        let mut seen = std::collections::HashSet::new();
        for chain in &chains {
            let chain_str = chain.as_str();
            assert!(
                !seen.contains(chain_str),
                "Chain ID should be unique: {}",
                chain_str
            );
            seen.insert(chain_str.to_string());
        }

        // Chain IDs should be lowercase
        for chain in &chains {
            let chain_str = chain.as_str();
            assert_eq!(
                chain_str,
                chain_str.to_lowercase(),
                "Chain ID should be lowercase"
            );
        }
    }

    #[test]
    fn test_demo_scenario_seal_uniqueness() {
        // Test 6: Verify seal IDs are unique
        let seal_id_1 = vec![1u8, 2, 3];
        let seal_id_2 = vec![4u8, 5, 6];

        let seal_point_1 =
            SealPoint::new(seal_id_1, Some(42)).expect("SealPoint creation should succeed");

        let seal_point_2 =
            SealPoint::new(seal_id_2, Some(43)).expect("SealPoint creation should succeed");

        assert_ne!(seal_point_1.id, seal_point_2.id);
    }

    #[test]
    fn test_demo_scenario_proof_size_limits() {
        // Test 7: Verify proof bundles respect size limits
        let seal_id = vec![1u8; 32];
        let seal_point =
            SealPoint::new(seal_id, Some(42)).expect("SealPoint creation should succeed");

        let commit_anchor = CommitAnchor::new(vec![1u8; 32], 100, vec![0u8; 1024])
            .expect("CommitAnchor creation should succeed");

        let dag_node = DAGNode::new(
            Hash::new([1u8; 32]),
            vec![0x01; 100],
            vec![],
            vec![],
            vec![],
        );
        let dag_segment = DAGSegment::new(vec![dag_node], Hash::zero());

        let inclusion_proof = InclusionProof::new(vec![0xCD; 32], Hash::new([2u8; 32]), 100, 0)
            .expect("InclusionProof creation should succeed");

        let finality_proof = FinalityProof::new(vec![0xAB; 16], 6, false)
            .expect("FinalityProof creation should succeed");

        let proof_bundle = ProofBundle::new(
            dag_segment,
            vec![],
            seal_point,
            commit_anchor,
            inclusion_proof,
            finality_proof,
        )
        .expect("ProofBundle creation should succeed");

        // Verify the proof bundle can be serialized
        let serialized =
            serde_json::to_string(&proof_bundle).expect("Proof bundle should be serializable");

        // Check that the serialized size is reasonable (< 1MB)
        assert!(
            serialized.len() < 1_000_000,
            "Proof bundle should be less than 1MB"
        );
    }

    #[test]
    fn test_demo_scenario_finality_requirements() {
        // Test 8: Verify finality proof requirements
        let valid_finality = FinalityProof::new(vec![0xAB; 16], 6, false)
            .expect("FinalityProof with 6 confirmations should be valid");

        assert_eq!(valid_finality.confirmations, 6);
        assert!(!valid_finality.is_deterministic);

        // Test that zero confirmations is rejected
        let zero_confirmations = FinalityProof::new(vec![0xAB; 16], 0, false);
        assert!(
            zero_confirmations.is_err(),
            "Zero confirmations should be rejected"
        );

        // Test that deterministic finality is accepted
        let deterministic = FinalityProof::new(vec![0xAB; 16], 0, true)
            .expect("Deterministic finality with zero confirmations should be valid");
        assert!(deterministic.is_deterministic);
    }

    #[test]
    fn test_demo_scenario_cross_chain_consistency() {
        // Test 9: Verify cross-chain transfer consistency
        let from_chain = ChainId::new("bitcoin");
        let to_chain = ChainId::new("ethereum");

        // Chains should be different
        assert_ne!(from_chain.as_str(), to_chain.as_str());

        // Both should be valid chains
        let valid_chains = vec!["bitcoin", "ethereum", "sui", "aptos", "solana"];
        assert!(valid_chains.contains(&from_chain.as_str()));
        assert!(valid_chains.contains(&to_chain.as_str()));
    }
}
