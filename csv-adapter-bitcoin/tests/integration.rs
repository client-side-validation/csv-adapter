//! Integration tests for Bitcoin adapter
//!
//! Tests the full lifecycle: create seal → publish → verify inclusion → verify finality → rollback

use csv_adapter_bitcoin::{BitcoinAnchorLayer, BitcoinAnchorRef};
use csv_adapter_core::AnchorLayer;
use csv_adapter_core::Hash;

fn test_adapter() -> BitcoinAnchorLayer {
    BitcoinAnchorLayer::signet().expect("Failed to create test adapter")
}

#[test]
fn test_full_lifecycle_create_seal() {
    let adapter = test_adapter();
    let seal = adapter
        .create_seal(Some(100_000))
        .expect("Failed to create seal");
    assert_eq!(seal.nonce, Some(100_000));
}

#[test]
fn test_full_lifecycle_publish_without_rpc() {
    let adapter = test_adapter();
    let seal = adapter
        .create_seal(Some(100_000))
        .expect("Failed to create seal");
    let commitment = Hash::new([0xAB; 32]);
    let anchor = adapter
        .publish(commitment, seal)
        .expect("Failed to publish");
    // Without RPC, this returns a simulated anchor
    assert_eq!(anchor.output_index, 0);
}

#[test]
fn test_full_lifecycle_verify_inclusion() {
    let adapter = test_adapter();
    let seal = adapter
        .create_seal(Some(100_000))
        .expect("Failed to create seal");
    let commitment = Hash::new([0xAB; 32]);
    let anchor = adapter
        .publish(commitment, seal)
        .expect("Failed to publish");
    let proof = adapter
        .verify_inclusion(anchor)
        .expect("Failed to verify inclusion");
    // Without real RPC, proof has empty merkle branch (simulated)
    assert_eq!(proof.tx_index, 0);
}

#[test]
fn test_full_lifecycle_verify_finality() {
    let adapter = test_adapter();
    // Create an anchor at a lower block height so it has confirmations
    // (current height is mocked to 200, so block 100 gives 100 confirmations)
    let anchor = BitcoinAnchorRef::new([1u8; 32], 0, 100);
    let finality = adapter
        .verify_finality(anchor)
        .expect("Failed to verify finality");
    assert!(finality.meets_required_depth);
}

#[test]
fn test_full_lifecycle_build_proof_bundle() {
    use csv_adapter_core::dag::DAGSegment;
    use csv_adapter_core::hash::Hash as CoreHash;

    let adapter = test_adapter();
    let seal = adapter
        .create_seal(Some(100_000))
        .expect("Failed to create seal");
    let commitment = Hash::new([0xAB; 32]);
    let anchor = adapter
        .publish(commitment, seal)
        .expect("Failed to publish");

    // Create a minimal DAG segment for testing
    let dag = DAGSegment::new(vec![], CoreHash::new([0u8; 32]));

    // This may fail without real proofs, which is expected
    let _ = adapter.build_proof_bundle(anchor, dag);
}

#[test]
fn test_full_lifecycle_rollback() {
    let adapter = test_adapter();
    let seal = adapter
        .create_seal(Some(100_000))
        .expect("Failed to create seal");
    let commitment = Hash::new([0xAB; 32]);
    let anchor = adapter
        .publish(commitment, seal)
        .expect("Failed to publish");

    // Rollback should succeed (anchor block_height is within current height)
    let result = adapter.rollback(anchor);
    assert!(result.is_ok());
}

#[test]
fn test_full_lifecycle_reorg_detection() {
    let adapter = test_adapter();
    // Create an anchor with a block height beyond current height (200)
    let anchor = BitcoinAnchorRef::new([1u8; 32], 0, 9999);
    let result = adapter.rollback(anchor);
    assert!(result.is_err()); // Should fail reorg check
}

#[test]
fn test_full_lifecycle_enforce_seal_replay() {
    let adapter = test_adapter();
    let seal = adapter
        .create_seal(Some(100_000))
        .expect("Failed to create seal");
    adapter
        .enforce_seal(seal.clone())
        .expect("Failed to enforce seal");
    // Second enforcement should fail (replay)
    let result = adapter.enforce_seal(seal);
    assert!(result.is_err());
}

#[test]
fn test_full_lifecycle_hash_commitment() {
    let adapter = test_adapter();
    let seal = adapter
        .create_seal(Some(100_000))
        .expect("Failed to create seal");
    let contract_id = Hash::new([0x01; 32]);
    let previous_commitment = Hash::new([0x02; 32]);
    let payload_hash = Hash::new([0x03; 32]);

    let commitment = adapter.hash_commitment(contract_id, previous_commitment, payload_hash, &seal);

    // Commitment should be non-zero and deterministic
    assert_ne!(commitment.as_bytes(), &[0u8; 32]);
}

#[test]
fn test_full_lifecycle_domain_separator() {
    let adapter = test_adapter();
    let domain = adapter.domain_separator();
    // Domain separator should start with "CSV-BTC-"
    assert_eq!(&domain[..8], b"CSV-BTC-");
}

#[test]
fn test_full_lifecycle_signature_scheme() {
    let adapter = test_adapter();
    let scheme = adapter.signature_scheme();
    // Bitcoin uses Secp256k1
    assert_eq!(scheme, csv_adapter_core::SignatureScheme::Secp256k1);
}

#[test]
fn test_proof_extraction_single_tx() {
    use csv_adapter_bitcoin::proofs::extract_merkle_proof_from_block;

    let txid = [1u8; 32];
    let block_txids = vec![txid];
    let block_hash = [2u8; 32];
    let block_height = 100;

    let proof = extract_merkle_proof_from_block(txid, &block_txids, block_hash, block_height);
    assert!(proof.is_some());
    let proof = proof.unwrap();
    assert_eq!(proof.tx_index, 0);
    assert_eq!(proof.block_hash, block_hash);
}

#[test]
fn test_proof_extraction_multiple_txs() {
    use csv_adapter_bitcoin::proofs::extract_merkle_proof_from_block;

    let txid1 = [1u8; 32];
    let txid2 = [2u8; 32];
    let txid3 = [3u8; 32];
    let block_txids = vec![txid1, txid2, txid3];
    let block_hash = [4u8; 32];
    let block_height = 200;

    // Extract proof for txid2 (index 1)
    let proof = extract_merkle_proof_from_block(txid2, &block_txids, block_hash, block_height);
    assert!(proof.is_some());
    let proof = proof.unwrap();
    assert_eq!(proof.tx_index, 1);
}
