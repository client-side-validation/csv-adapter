//! ZK Proof Integration Tests - Phase 5
//!
//! Tests the complete ZK proof flow:
//! 1. Create a seal
//! 2. Generate a ZK proof of seal consumption
//! 3. Verify the ZK proof structure
//! 4. Test serialization/deserialization

use csv_adapter_bitcoin::zk_prover::BitcoinSpvProver;
use csv_core::hash::Hash;
use csv_core::protocol_version::Chain;
use csv_core::seal::SealPoint;
use csv_core::zk_proof::{ChainWitness, ProofSystem, ZkProver, ZkSealProof};

/// Test the complete ZK proof lifecycle
#[test]
fn test_zk_proof_lifecycle() {
    // 1. Create a seal reference (simulating a Bitcoin UTXO)
    let seal_id = [0xABu8; 32];
    let seal = SealPoint::new(seal_id.to_vec(), Some(0)).expect("valid seal");

    // 2. Create witness data (simulating Bitcoin SPV data)
    let witness = ChainWitness {
        chain: Chain::Bitcoin,
        block_hash: Hash::new([0x01; 32]),
        block_height: 800_000,
        tx_data: vec![0x02; 64], // Simulated transaction data
        inclusion_proof: vec![0x03; 32], // Simulated Merkle branch
        finality_proof: vec![0x04; 16],
        timestamp: 1_700_000_000,
    };

    // 3. Generate ZK proof using Bitcoin SPV prover
    let prover = BitcoinSpvProver::new();
    let proof = prover
        .prove_seal_consumption(&seal, &witness)
        .expect("proof generation should succeed");

    // 4. Verify proof properties
    assert!(!proof.proof_bytes.is_empty(), "proof should have bytes");
    assert_eq!(proof.verifier_key.chain, Chain::Bitcoin);
    assert_eq!(proof.verifier_key.proof_system, ProofSystem::SP1);
    assert!(proof.is_structurally_valid(), "proof should be structurally valid");

    // 5. Verify the proof public inputs
    assert_eq!(proof.public_inputs.seal_ref.seal_id, seal_id.to_vec());
    assert_eq!(proof.public_inputs.source_chain, Chain::Bitcoin);
    assert_eq!(proof.public_inputs.block_height, 800_000);
}

/// Test ZK proof serialization and deserialization
#[test]
fn test_zk_proof_serialization() {
    let seal = SealPoint::new(vec![0xCD; 32], Some(42)).expect("valid seal");
    let witness = ChainWitness {
        chain: Chain::Bitcoin,
        block_hash: Hash::new([0xEF; 32]),
        block_height: 900_000,
        tx_data: vec![0xAA; 64],
        inclusion_proof: vec![0xBB; 32],
        finality_proof: vec![0xCC; 16],
        timestamp: 1_800_000_000,
    };

    let prover = BitcoinSpvProver::new();
    let proof = prover
        .prove_seal_consumption(&seal, &witness)
        .expect("proof generation should succeed");

    // Serialize
    let serialized = proof.to_bytes().expect("serialization should succeed");
    assert!(!serialized.is_empty(), "serialized proof should not be empty");

    // Deserialize
    let deserialized = ZkSealProof::from_bytes(&serialized).expect("deserialization should succeed");

    // Verify equivalence
    assert_eq!(proof.proof_bytes, deserialized.proof_bytes);
    assert_eq!(proof.verifier_key.chain, deserialized.verifier_key.chain);
    assert_eq!(proof.public_inputs.block_height, deserialized.public_inputs.block_height);
}

/// Test that wrong chain witness fails
#[test]
fn test_wrong_chain_fails() {
    let seal = SealPoint::new(vec![0x12; 32], Some(1)).expect("valid seal");

    // Create witness for wrong chain
    let witness = ChainWitness {
        chain: Chain::Ethereum, // Wrong chain!
        block_hash: Hash::new([0x34; 32]),
        block_height: 19_000_000,
        tx_data: vec![0x56; 64],
        inclusion_proof: vec![0x78; 32],
        finality_proof: vec![0x9A; 16],
        timestamp: 1_700_000_000,
    };

    let prover = BitcoinSpvProver::new();
    let result = prover.prove_seal_consumption(&seal, &witness);

    assert!(result.is_err(), "should fail with wrong chain");
}

/// Test that missing inclusion proof fails
#[test]
fn test_missing_inclusion_proof_fails() {
    let seal = SealPoint::new(vec![0xAB; 32], Some(1)).expect("valid seal");

    let witness = ChainWitness {
        chain: Chain::Bitcoin,
        block_hash: Hash::new([0x01; 32]),
        block_height: 800_000,
        tx_data: vec![0x02; 64],
        inclusion_proof: vec![], // Empty!
        finality_proof: vec![0x04; 16],
        timestamp: 1_700_000_000,
    };

    let prover = BitcoinSpvProver::new();
    let result = prover.prove_seal_consumption(&seal, &witness);

    assert!(result.is_err(), "should fail with missing inclusion proof");
}

/// Test proof witness hash consistency
#[test]
fn test_proof_witness_consistency() {
    use csv_core::zk_proof::ChainWitness;

    // Create two identical witnesses
    let witness1 = ChainWitness {
        chain: Chain::Bitcoin,
        block_hash: Hash::new([0x01; 32]),
        block_height: 800_000,
        tx_data: vec![0x02; 64],
        inclusion_proof: vec![0x03; 32],
        finality_proof: vec![0x04; 16],
        timestamp: 1_700_000_000,
    };

    let witness2 = ChainWitness {
        chain: Chain::Bitcoin,
        block_hash: Hash::new([0x01; 32]),
        block_height: 800_000,
        tx_data: vec![0x02; 64],
        inclusion_proof: vec![0x03; 32],
        finality_proof: vec![0x04; 16],
        timestamp: 1_700_000_000,
    };

    // Same witness should produce same hash
    assert_eq!(witness1.hash(), witness2.hash());

    // Different witness should produce different hash
    let witness3 = ChainWitness {
        chain: Chain::Ethereum, // Different chain
        block_hash: Hash::new([0x01; 32]),
        block_height: 800_000,
        tx_data: vec![0x02; 64],
        inclusion_proof: vec![0x03; 32],
        finality_proof: vec![0x04; 16],
        timestamp: 1_700_000_000,
    };
    assert_ne!(witness1.hash(), witness3.hash());
}

/// Test proof generation with different seals produces different proofs
#[test]
fn test_different_seals_different_proofs() {
    let seal1 = SealPoint::new(vec![0xAA; 32], Some(1)).expect("valid seal");
    let seal2 = SealPoint::new(vec![0xBB; 32], Some(2)).expect("valid seal");

    let witness = ChainWitness {
        chain: Chain::Bitcoin,
        block_hash: Hash::new([0x01; 32]),
        block_height: 800_000,
        tx_data: vec![0x02; 64],
        inclusion_proof: vec![0x03; 32],
        finality_proof: vec![0x04; 16],
        timestamp: 1_700_000_000,
    };

    let prover = BitcoinSpvProver::new();
    let proof1 = prover
        .prove_seal_consumption(&seal1, &witness)
        .expect("proof generation should succeed");
    let proof2 = prover
        .prove_seal_consumption(&seal2, &witness)
        .expect("proof generation should succeed");

    // Different seals should produce different public inputs
    assert_ne!(proof1.public_inputs.seal_ref.seal_id, proof2.public_inputs.seal_ref.seal_id);
}
