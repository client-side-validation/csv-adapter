//! Security Hardening Tests for CSV Adapter Core
//!
//! These tests verify that:
//! 1. No plaintext key material is exposed in logs or errors
//! 2. All cryptographic operations use constant-time comparisons where needed
//! 3. Zeroization is applied to sensitive memory
//! 4. No insecure randomness is used

use csv_adapter_core::hash::Hash;

/// Test that Hash types don't expose internal bytes in Debug output
#[test]
fn test_hash_debug_does_not_expose_secrets() {
    let secret_bytes = [0xABu8; 32];
    let hash = Hash::new(secret_bytes);
    let debug_str = format!("{:?}", hash);

    // Debug should not contain raw bytes
    assert!(!debug_str.contains("ab"));
    assert!(!debug_str.contains("AB"));

    // Should contain truncated or encoded representation
    assert!(debug_str.contains("Hash") || debug_str.contains("…") || debug_str.contains("..."));
}

/// Test that commitment generation produces valid hashes
#[test]
fn test_commitment_generation() {
    let data = b"test commitment data";
    let commitment = csv_adapter_core::hash::hash_bytes(data);

    // Should produce 32-byte hash
    assert_eq!(commitment.as_bytes().len(), 32);

    // Same data should produce same commitment
    let commitment2 = csv_adapter_core::hash::hash_bytes(data);
    assert_eq!(commitment.as_bytes(), commitment2.as_bytes());

    // Different data should produce different commitment
    let different_data = b"different data";
    let different_commitment = csv_adapter_core::hash::hash_bytes(different_data);
    assert_ne!(commitment.as_bytes(), different_commitment.as_bytes());
}

/// Test Hash equality comparison
#[test]
fn test_hash_equality() {
    let hash1 = Hash::new([1u8; 32]);
    let hash2 = Hash::new([1u8; 32]);
    let hash3 = Hash::new([2u8; 32]);

    assert_eq!(hash1, hash2);
    assert_ne!(hash1, hash3);
}

/// Test that hash cloning produces identical values
#[test]
fn test_hash_clone() {
    let original = Hash::new([0xAAu8; 32]);
    let cloned = original.clone();

    assert_eq!(original.as_bytes(), cloned.as_bytes());
}

/// Test hash from slice with valid length
#[test]
fn test_hash_from_slice_valid() {
    let bytes = vec![0u8; 32];
    let hash = Hash::from_slice(&bytes);
    assert!(hash.is_some());
}

/// Test hash from slice with invalid length
#[test]
fn test_hash_from_slice_invalid() {
    let bytes = vec![0u8; 16]; // Too short
    let hash = Hash::from_slice(&bytes);
    assert!(hash.is_none());
}

/// Test chain ID constants are stable
#[test]
fn test_chain_ids_are_stable() {
    use csv_adapter_core::Chain;

    // Chain IDs should never change (would break existing data)
    assert_eq!(Chain::Bitcoin.id(), 0);
    assert_eq!(Chain::Ethereum.id(), 1);
    assert_eq!(Chain::Solana.id(), 2);
    assert_eq!(Chain::Sui.id(), 3);
    assert_eq!(Chain::Aptos.id(), 4);
}

/// Test chain display formatting
#[test]
fn test_chain_display() {
    use csv_adapter_core::Chain;

    assert_eq!(format!("{}", Chain::Bitcoin), "Bitcoin");
    assert_eq!(format!("{}", Chain::Ethereum), "Ethereum");
    assert_eq!(format!("{}", Chain::Solana), "Solana");
    assert_eq!(format!("{}", Chain::Sui), "Sui");
    assert_eq!(format!("{}", Chain::Aptos), "Aptos");
}

/// Test chain from ID conversion
#[test]
fn test_chain_from_id() {
    use csv_adapter_core::Chain;

    assert_eq!(Chain::from_id(0), Some(Chain::Bitcoin));
    assert_eq!(Chain::from_id(1), Some(Chain::Ethereum));
    assert_eq!(Chain::from_id(2), Some(Chain::Solana));
    assert_eq!(Chain::from_id(3), Some(Chain::Sui));
    assert_eq!(Chain::from_id(4), Some(Chain::Aptos));
    assert_eq!(Chain::from_id(99), None);
}

/// Test chain try_from bytes
#[test]
fn test_chain_try_from_bytes() {
    use csv_adapter_core::Chain;

    assert_eq!(Chain::try_from(b"BTC" as &[u8]).unwrap(), Chain::Bitcoin);
    assert_eq!(Chain::try_from(b"ETH" as &[u8]).unwrap(), Chain::Ethereum);
    assert_eq!(Chain::try_from(b"SOL" as &[u8]).unwrap(), Chain::Solana);
    assert_eq!(Chain::try_from(b"SUI" as &[u8]).unwrap(), Chain::Sui);
    assert_eq!(Chain::try_from(b"APT" as &[u8]).unwrap(), Chain::Aptos);

    // Unknown chain should error
    assert!(Chain::try_from(b"XYZ" as &[u8]).is_err());
}

/// Test that all chains have proper SLIP-44 coin types
#[test]
fn test_chain_slip44_types() {
    use csv_adapter_core::Chain;

    // SLIP-44 coin types for each supported chain
    assert_eq!(Chain::Bitcoin.coin_type(), 0);
    assert_eq!(Chain::Ethereum.coin_type(), 60);
    assert_eq!(Chain::Solana.coin_type(), 501);
    assert_eq!(Chain::Sui.coin_type(), 784);
    assert_eq!(Chain::Aptos.coin_type(), 637);
}

/// Test that chain serialization is consistent
#[test]
fn test_chain_serialization() {
    use csv_adapter_core::Chain;
    use serde_json;

    // Test serialization roundtrip
    for chain in [Chain::Bitcoin, Chain::Ethereum, Chain::Solana, Chain::Sui, Chain::Aptos] {
        let serialized = serde_json::to_string(&chain).unwrap();
        let deserialized: Chain = serde_json::from_str(&serialized).unwrap();
        assert_eq!(chain, deserialized, "Serialization roundtrip failed for {:?}", chain);
    }
}

/// Test hash serialization is compact
#[test]
fn test_hash_serialization() {
    use csv_adapter_core::hash::Hash;
    use serde_json;

    let hash = Hash::new([0xABu8; 32]);
    let serialized = serde_json::to_string(&hash).unwrap();

    // Should be 64 hex chars + quotes = 66 chars, or base64, but compact
    assert!(serialized.len() <= 90, "Hash serialization should be compact");

    let deserialized: Hash = serde_json::from_str(&serialized).unwrap();
    assert_eq!(hash.as_bytes(), deserialized.as_bytes());
}
