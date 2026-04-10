//! Property-based tests for csv-adapter-core critical invariants.
//!
//! These tests use `proptest` to verify that:
//! 1. Serialization roundtrips are lossless
//! 2. Deserialization rejects malformed inputs
//! 3. Critical invariants hold across randomized inputs
//! 4. No panics on adversarial byte sequences

use csv_adapter_core::hash::Hash;
use csv_adapter_core::right::{Right, OwnershipProof, RightId};
use csv_adapter_core::seal::{SealRef, AnchorRef, MAX_SEAL_ID_SIZE, MAX_ANCHOR_ID_SIZE, MAX_ANCHOR_METADATA_SIZE};
use csv_adapter_core::seal_registry::{CrossChainSealRegistry, SealConsumption, ChainId};
use csv_adapter_core::right::RightError;
use proptest::prelude::*;
use proptest::collection::vec;

// ── Strategy helpers ────────────────────────────────────────────────────────

/// Generate arbitrary 32-byte hashes
fn arb_hash() -> impl Strategy<Value = Hash> {
    prop::array::uniform32(any::<u8>()).prop_map(Hash::new)
}

/// Generate arbitrary seal identifiers (bounded by protocol limits)
fn arb_seal_id() -> impl Strategy<Value = Vec<u8>> {
    vec(any::<u8>(), 1..=128) // Well under 1KB limit
}

/// Generate arbitrary anchor identifiers
fn arb_anchor_id() -> impl Strategy<Value = Vec<u8>> {
    vec(any::<u8>(), 1..=128)
}

/// Generate arbitrary anchor metadata
fn arb_anchor_metadata() -> impl Strategy<Value = Vec<u8>> {
    vec(any::<u8>(), 0..=256) // Well under 4KB limit
}

/// Generate arbitrary ownership proof data
fn arb_proof_data() -> impl Strategy<Value = Vec<u8>> {
    vec(any::<u8>(), 0..=256)
}

/// Generate arbitrary owner identifier
fn arb_owner_data() -> impl Strategy<Value = Vec<u8>> {
    vec(any::<u8>(), 1..=64)
}

/// Generate arbitrary salt
fn arb_salt() -> impl Strategy<Value = Vec<u8>> {
    vec(any::<u8>(), 0..=64)
}

// ── SealRef roundtrip tests ─────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_seal_ref_roundtrip(
        seal_id in arb_seal_id(),
        nonce in any::<Option<u64>>(),
    ) {
        let seal = SealRef::new(seal_id.clone(), nonce).unwrap();
        let bytes = seal.to_vec();
        let restored = SealRef::from_bytes(&bytes).unwrap();

        prop_assert_eq!(restored.seal_id, seal_id);
        prop_assert_eq!(restored.nonce, nonce);
    }

    #[test]
    fn prop_seal_ref_none_vs_zero_distinct(
        seal_id in arb_seal_id(),
    ) {
        let seal_none = SealRef::new(seal_id.clone(), None).unwrap();
        let seal_zero = SealRef::new(seal_id, Some(0)).unwrap();

        // These must produce different byte sequences
        prop_assert_ne!(seal_none.to_vec(), seal_zero.to_vec());

        // Both must roundtrip correctly
        let restored_none = SealRef::from_bytes(&seal_none.to_vec()).unwrap();
        let restored_zero = SealRef::from_bytes(&seal_zero.to_vec()).unwrap();

        prop_assert_eq!(restored_none.nonce, None);
        prop_assert_eq!(restored_zero.nonce, Some(0));
    }

    #[test]
    fn prop_seal_ref_rejects_oversized(
        seal_id in vec(any::<u8>(), (MAX_SEAL_ID_SIZE + 1)..=(MAX_SEAL_ID_SIZE + 100)),
    ) {
        prop_assert!(SealRef::new(seal_id, None).is_err());
    }

    #[test]
    fn prop_seal_ref_rejects_empty(
        nonce in any::<Option<u64>>(),
    ) {
        prop_assert!(SealRef::new(vec![], nonce).is_err());
    }

    #[test]
    fn prop_seal_ref_from_bytes_rejects_malformed(
        bytes in vec(any::<u8>(), 0..=50),
    ) {
        // Any bytes sequence should either parse successfully or return an error (never panic)
        let _ = SealRef::from_bytes(&bytes);
    }

    #[test]
    fn prop_seal_ref_from_bytes_rejects_invalid_nonce_flag(
        seal_id in arb_seal_id(),
    ) {
        // Create bytes with invalid nonce flag (2-255)
        let mut bytes = vec![2u8]; // Invalid flag
        bytes.push(0); // length byte 1 of 4
        bytes.push(0);
        bytes.push(0);
        bytes.push(0);
        bytes.extend(&seal_id);

        prop_assert!(SealRef::from_bytes(&bytes).is_err());
    }
}

// ── AnchorRef tests ─────────────────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_anchor_ref_to_vec_is_deterministic(
        anchor_id in arb_anchor_id(),
        block_height in any::<u64>(),
        metadata in arb_anchor_metadata(),
    ) {
        let anchor = AnchorRef::new(anchor_id.clone(), block_height, metadata.clone()).unwrap();
        let bytes1 = anchor.to_vec();
        let bytes2 = anchor.to_vec();

        prop_assert_eq!(bytes1, bytes2);
    }

    #[test]
    fn prop_anchor_ref_rejects_oversized_id(
        anchor_id in vec(any::<u8>(), (MAX_ANCHOR_ID_SIZE + 1)..=(MAX_ANCHOR_ID_SIZE + 100)),
    ) {
        prop_assert!(AnchorRef::new(anchor_id, 0, vec![]).is_err());
    }

    #[test]
    fn prop_anchor_ref_rejects_oversized_metadata(
        metadata in vec(any::<u8>(), (MAX_ANCHOR_METADATA_SIZE + 1)..=(MAX_ANCHOR_METADATA_SIZE + 100)),
    ) {
        prop_assert!(AnchorRef::new(vec![1, 2, 3], 0, metadata).is_err());
    }
}

// ── Right serialization roundtrip tests ─────────────────────────────────────

proptest! {
    #[test]
    fn prop_right_canonical_roundtrip(
        commitment in arb_hash(),
        proof in arb_proof_data(),
        owner in arb_owner_data(),
        salt in arb_salt(),
    ) {
        let right = Right::new(
            commitment,
            OwnershipProof {
                proof: proof.clone(),
                owner: owner.clone(),
                scheme: None,
            },
            &salt,
        );

        let bytes = right.to_canonical_bytes();
        let restored = Right::from_canonical_bytes(&bytes).unwrap();

        prop_assert_eq!(restored.id, right.id);
        prop_assert_eq!(restored.commitment, right.commitment);
        prop_assert_eq!(restored.owner.proof, proof);
        prop_assert_eq!(restored.owner.owner, owner);
        prop_assert_eq!(restored.salt, salt);
        prop_assert_eq!(restored.nullifier, right.nullifier);
        prop_assert_eq!(restored.state_root, right.state_root);
    }

    #[test]
    fn prop_right_rejects_spoofed_id(
        commitment in arb_hash(),
        proof in arb_proof_data(),
        owner in arb_owner_data(),
        salt in arb_salt(),
    ) {
        let right = Right::new(
            commitment,
            OwnershipProof {
                proof,
                owner,
                scheme: None,
            },
            &salt,
        );

        let mut bytes = right.to_canonical_bytes();

        // Tamper with the RightId in the serialized bytes (first 32 bytes)
        for byte in &mut bytes[0..32] {
            *byte ^= 0xFF;
        }

        // Should reject the tampered RightId
        prop_assert_eq!(Right::from_canonical_bytes(&bytes), Err(RightError::InvalidRightId));
    }

    #[test]
    fn prop_right_from_canonical_bytes_never_panics(
        bytes in vec(any::<u8>(), 0..=500),
    ) {
        // Any byte sequence should either parse successfully or return an error (never panic)
        let _ = Right::from_canonical_bytes(&bytes);
    }

    #[test]
    fn prop_right_verify_rejects_tampered_commitment(
        proof in vec(any::<u8>(), 1..=64), // Non-empty proof
        owner in arb_owner_data(),
        salt in arb_salt(),
    ) {
        let right = Right::new(
            Hash::new([0xAB; 32]),
            OwnershipProof {
                proof,
                owner,
                scheme: None,
            },
            &salt,
        );

        // Tamper with the commitment
        let mut tampered = right.clone();
        tampered.commitment = Hash::new([0u8; 32]);

        // Recompute ID to match tampered commitment so we test the commitment check
        let mut data = Vec::with_capacity(32 + tampered.salt.len());
        data.extend_from_slice(tampered.commitment.as_bytes());
        data.extend_from_slice(&tampered.salt);
        tampered.id = RightId(Hash::new(csv_adapter_core::tagged_hash::csv_tagged_hash("right-id", &data)));

        // Should reject zero commitment
        prop_assert_eq!(tampered.verify(), Err(RightError::InvalidCommitment));
    }
}

// ── Seal registry invariant tests ───────────────────────────────────────────

proptest! {
    #[test]
    fn prop_seal_registry_detects_double_spend(
        seal_id in arb_seal_id(),
        right_id_bytes in prop::array::uniform32(any::<u8>()),
    ) {
        let chains = [ChainId::Bitcoin, ChainId::Sui, ChainId::Aptos, ChainId::Ethereum];

        for (i, chain1) in chains.iter().enumerate() {
            for chain2 in chains.iter().skip(i + 1) {
                let mut registry = CrossChainSealRegistry::new();
                let seal = SealRef::new(seal_id.clone(), None).unwrap();
                let right_id = RightId(Hash::new(right_id_bytes));

                // First consumption should succeed
                let consumption1 = SealConsumption {
                    chain: chain1.clone(),
                    seal_ref: seal.clone(),
                    right_id: right_id.clone(),
                    block_height: 100,
                    tx_hash: Hash::new([0xAB; 32]),
                    recorded_at: 12345,
                };
                prop_assert!(registry.record_consumption(consumption1).is_ok());

                // Second consumption on different chain should be detected as double-spend
                let consumption2 = SealConsumption {
                    chain: chain2.clone(),
                    seal_ref: seal.clone(),
                    right_id: right_id.clone(),
                    block_height: 200,
                    tx_hash: Hash::new([0xCD; 32]),
                    recorded_at: 12346,
                };
                prop_assert!(registry.record_consumption(consumption2).is_err());
            }
        }
    }

    #[test]
    fn prop_seal_registry_allows_different_seals(
        seal_id1 in arb_seal_id(),
        seal_id2 in arb_seal_id(),
        right_id_bytes in prop::array::uniform32(any::<u8>()),
    ) {
        prop_assume!(seal_id1 != seal_id2);

        let mut registry = CrossChainSealRegistry::new();
        let right_id = RightId(Hash::new(right_id_bytes));

        let seal1 = SealRef::new(seal_id1, None).unwrap();
        let seal2 = SealRef::new(seal_id2, None).unwrap();

        let consumption1 = SealConsumption {
            chain: ChainId::Bitcoin,
            seal_ref: seal1,
            right_id: right_id.clone(),
            block_height: 100,
            tx_hash: Hash::new([0xAB; 32]),
            recorded_at: 12345,
        };
        prop_assert!(registry.record_consumption(consumption1).is_ok());

        let consumption2 = SealConsumption {
            chain: ChainId::Bitcoin,
            seal_ref: seal2,
            right_id,
            block_height: 200,
            tx_hash: Hash::new([0xCD; 32]),
            recorded_at: 12346,
        };
        prop_assert!(registry.record_consumption(consumption2).is_ok());
    }
}

// ── Hash domain separation tests ────────────────────────────────────────────

proptest! {
    #[test]
    fn prop_tagged_hash_different_tags_produce_different_results(
        data in vec(any::<u8>(), 0..=128),
    ) {
        use csv_adapter_core::tagged_hash::csv_tagged_hash;

        let hash1 = csv_tagged_hash("right-id", &data);
        let hash2 = csv_tagged_hash("right-nullifier", &data);

        // Different tags must produce different hashes (except for the astronomically unlikely collision)
        prop_assert_ne!(hash1, hash2);
    }

    #[test]
    fn prop_tagged_hash_different_data_produce_different_results(
        data1 in vec(any::<u8>(), 1..=64),
        data2 in vec(any::<u8>(), 1..=64),
    ) {
        use csv_adapter_core::tagged_hash::csv_tagged_hash;

        prop_assume!(data1 != data2);

        let hash1 = csv_tagged_hash("right-id", &data1);
        let hash2 = csv_tagged_hash("right-id", &data2);

        prop_assert_ne!(hash1, hash2);
    }

    #[test]
    fn prop_tagged_hash_is_deterministic(
        tag in "[a-z-]{3,20}",
        data in vec(any::<u8>(), 0..=128),
    ) {
        use csv_adapter_core::tagged_hash::csv_tagged_hash;

        let hash1 = csv_tagged_hash(&tag, &data);
        let hash2 = csv_tagged_hash(&tag, &data);

        prop_assert_eq!(hash1, hash2);
    }

    #[test]
    fn prop_tagged_hash_differs_from_raw_sha256(
        data in vec(any::<u8>(), 1..=128),
    ) {
        use csv_adapter_core::tagged_hash::csv_tagged_hash;
        use sha2::{Digest, Sha256};

        let tagged = csv_tagged_hash("right-id", &data);

        let mut hasher = Sha256::new();
        hasher.update(&data);
        let raw: [u8; 32] = hasher.finalize().into();

        // Tagged hash must differ from raw SHA-256 (domain separation)
        prop_assert_ne!(tagged, raw);
    }
}
