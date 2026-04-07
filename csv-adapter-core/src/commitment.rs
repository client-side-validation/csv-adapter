//! Commitment types with canonical encoding (v2: MPC-aware)
//!
//! Commitments bind off-chain state transitions to the anchoring layer.
//!
//! v1: Single-protocol: `H(version || contract_id || prev_commitment || payload_hash || seal_id || domain_separator)`
//! v2: MPC-aware: `H(version || protocol_id || mpc_root || prev_commitment || payload_hash || seal_id || domain_separator)`

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::hash::Hash;
use crate::mpc::MpcTree;
use crate::seal::SealRef;

/// Current commitment version
pub const COMMITMENT_VERSION: u8 = 2;

/// v1 commitment (legacy, single-protocol)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitmentV1 {
    pub version: u8,
    pub contract_id: Hash,
    pub previous_commitment: Hash,
    pub transition_payload_hash: Hash,
    pub seal_id: Hash,
    pub domain_separator: [u8; 32],
}

/// v2 commitment (MPC-aware, multi-protocol)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommitmentV2 {
    pub version: u8,
    /// Protocol this commitment belongs to
    pub protocol_id: [u8; 32],
    /// MPC tree root (all protocols sharing this witness)
    pub mpc_root: Hash,
    /// Unique contract identifier
    pub contract_id: Hash,
    /// Previous commitment hash
    pub previous_commitment: Hash,
    /// Hash of the transition payload
    pub transition_payload_hash: Hash,
    /// Seal reference hash
    pub seal_id: Hash,
    /// Domain separator for chain-specific isolation
    pub domain_separator: [u8; 32],
}

/// A canonical commitment (unified enum for versioning)
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Commitment {
    /// v1: legacy single-protocol
    V1(CommitmentV1),
    /// v2: MPC-aware multi-protocol
    V2(CommitmentV2),
}

impl Commitment {
    /// Create a v2 (MPC-aware) commitment
    pub fn new(
        protocol_id: [u8; 32],
        mpc_tree: &MpcTree,
        contract_id: Hash,
        previous_commitment: Hash,
        transition_payload_hash: Hash,
        seal_ref: &SealRef,
        domain_separator: [u8; 32],
    ) -> Self {
        let seal_hash = {
            let mut hasher = Sha256::new();
            hasher.update(seal_ref.to_vec());
            let result = hasher.finalize();
            let mut array = [0u8; 32];
            array.copy_from_slice(&result);
            Hash::new(array)
        };

        let mpc_root = mpc_tree.root();

        Commitment::V2(CommitmentV2 {
            version: COMMITMENT_VERSION,
            protocol_id,
            mpc_root,
            contract_id,
            previous_commitment,
            transition_payload_hash,
            seal_id: seal_hash,
            domain_separator,
        })
    }

    /// Create a v1 (legacy) commitment for backwards compatibility
    pub fn v1(
        contract_id: Hash,
        previous_commitment: Hash,
        transition_payload_hash: Hash,
        seal_ref: &SealRef,
        domain_separator: [u8; 32],
    ) -> Self {
        let seal_hash = {
            let mut hasher = Sha256::new();
            hasher.update(seal_ref.to_vec());
            let result = hasher.finalize();
            let mut array = [0u8; 32];
            array.copy_from_slice(&result);
            Hash::new(array)
        };

        Commitment::V1(CommitmentV1 {
            version: 1,
            contract_id,
            previous_commitment,
            transition_payload_hash,
            seal_id: seal_hash,
            domain_separator,
        })
    }

    /// Compute the commitment hash
    pub fn hash(&self) -> Hash {
        let mut hasher = Sha256::new();
        self.hash_into(&mut hasher);
        let result = hasher.finalize();
        let mut array = [0u8; 32];
        array.copy_from_slice(&result);
        Hash::new(array)
    }

    fn hash_into(&self, hasher: &mut Sha256) {
        match self {
            Commitment::V1(v1) => {
                hasher.update([v1.version]);
                hasher.update(v1.contract_id.as_bytes());
                hasher.update(v1.previous_commitment.as_bytes());
                hasher.update(v1.transition_payload_hash.as_bytes());
                hasher.update(v1.seal_id.as_bytes());
                hasher.update(v1.domain_separator);
            }
            Commitment::V2(v2) => {
                hasher.update([v2.version]);
                hasher.update(&v2.protocol_id);
                hasher.update(v2.mpc_root.as_bytes());
                hasher.update(v2.contract_id.as_bytes());
                hasher.update(v2.previous_commitment.as_bytes());
                hasher.update(v2.transition_payload_hash.as_bytes());
                hasher.update(v2.seal_id.as_bytes());
                hasher.update(v2.domain_separator);
            }
        }
    }

    /// Get the version
    pub fn version(&self) -> u8 {
        match self {
            Commitment::V1(v1) => v1.version,
            Commitment::V2(v2) => v2.version,
        }
    }

    /// Get the contract ID
    pub fn contract_id(&self) -> Hash {
        match self {
            Commitment::V1(v1) => v1.contract_id,
            Commitment::V2(v2) => v2.contract_id,
        }
    }

    /// Get the seal ID hash
    pub fn seal_id(&self) -> Hash {
        match self {
            Commitment::V1(v1) => v1.seal_id,
            Commitment::V2(v2) => v2.seal_id,
        }
    }

    /// Get the domain separator
    pub fn domain_separator(&self) -> [u8; 32] {
        match self {
            Commitment::V1(v1) => v1.domain_separator,
            Commitment::V2(v2) => v2.domain_separator,
        }
    }

    /// Serialize commitment using canonical encoding
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        match self {
            Commitment::V1(v1) => {
                let mut out = Vec::with_capacity(1 + 32 * 5);
                out.push(v1.version);
                out.extend_from_slice(v1.contract_id.as_bytes());
                out.extend_from_slice(v1.previous_commitment.as_bytes());
                out.extend_from_slice(v1.transition_payload_hash.as_bytes());
                out.extend_from_slice(v1.seal_id.as_bytes());
                out.extend_from_slice(&v1.domain_separator);
                out
            }
            Commitment::V2(v2) => {
                let mut out = Vec::with_capacity(1 + 32 * 7);
                out.push(v2.version);
                out.extend_from_slice(&v2.protocol_id);
                out.extend_from_slice(v2.mpc_root.as_bytes());
                out.extend_from_slice(v2.contract_id.as_bytes());
                out.extend_from_slice(v2.previous_commitment.as_bytes());
                out.extend_from_slice(v2.transition_payload_hash.as_bytes());
                out.extend_from_slice(v2.seal_id.as_bytes());
                out.extend_from_slice(&v2.domain_separator);
                out
            }
        }
    }

    /// Deserialize commitment from canonical bytes
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.is_empty() {
            return Err("Empty commitment bytes");
        }

        let version = bytes[0];
        match version {
            1 => {
                if bytes.len() < 161 {
                    return Err("V1 commitment bytes too short");
                }
                let mut contract_id = [0u8; 32];
                contract_id.copy_from_slice(&bytes[1..33]);
                let mut previous_commitment = [0u8; 32];
                previous_commitment.copy_from_slice(&bytes[33..65]);
                let mut transition_payload_hash = [0u8; 32];
                transition_payload_hash.copy_from_slice(&bytes[65..97]);
                let mut seal_id = [0u8; 32];
                seal_id.copy_from_slice(&bytes[97..129]);
                let mut domain_separator = [0u8; 32];
                domain_separator.copy_from_slice(&bytes[129..161]);

                Ok(Commitment::V1(CommitmentV1 {
                    version: 1,
                    contract_id: Hash::new(contract_id),
                    previous_commitment: Hash::new(previous_commitment),
                    transition_payload_hash: Hash::new(transition_payload_hash),
                    seal_id: Hash::new(seal_id),
                    domain_separator,
                }))
            }
            2 => {
                if bytes.len() < 225 {
                    return Err("V2 commitment bytes too short");
                }
                let mut protocol_id = [0u8; 32];
                protocol_id.copy_from_slice(&bytes[1..33]);
                let mut mpc_root = [0u8; 32];
                mpc_root.copy_from_slice(&bytes[33..65]);
                let mut contract_id = [0u8; 32];
                contract_id.copy_from_slice(&bytes[65..97]);
                let mut previous_commitment = [0u8; 32];
                previous_commitment.copy_from_slice(&bytes[97..129]);
                let mut transition_payload_hash = [0u8; 32];
                transition_payload_hash.copy_from_slice(&bytes[129..161]);
                let mut seal_id = [0u8; 32];
                seal_id.copy_from_slice(&bytes[161..193]);
                let mut domain_separator = [0u8; 32];
                domain_separator.copy_from_slice(&bytes[193..225]);

                Ok(Commitment::V2(CommitmentV2 {
                    version: 2,
                    protocol_id,
                    mpc_root: Hash::new(mpc_root),
                    contract_id: Hash::new(contract_id),
                    previous_commitment: Hash::new(previous_commitment),
                    transition_payload_hash: Hash::new(transition_payload_hash),
                    seal_id: Hash::new(seal_id),
                    domain_separator,
                }))
            }
            _ => Err("Unknown commitment version"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mpc::MpcTree;

    fn test_v1_commitment() -> Commitment {
        Commitment::v1(
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            &SealRef::new(vec![4u8; 16], Some(42)).unwrap(),
            [5u8; 32],
        )
    }

    fn test_v2_commitment() -> Commitment {
        let protocol_id = [10u8; 32];
        let mpc_tree = MpcTree::from_pairs(&[
            (protocol_id, Hash::new([20u8; 32])),
            ([20u8; 32], Hash::new([30u8; 32])),
        ]);
        Commitment::new(
            protocol_id,
            &mpc_tree,
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            &SealRef::new(vec![4u8; 16], Some(42)).unwrap(),
            [5u8; 32],
        )
    }

    // ─────────────────────────────────────────────
    // V1 tests (backwards compat)
    // ─────────────────────────────────────────────

    #[test]
    fn test_v1_creation() {
        let c = test_v1_commitment();
        assert_eq!(c.version(), 1);
    }

    #[test]
    fn test_v1_hash_deterministic() {
        let c1 = test_v1_commitment();
        let c2 = test_v1_commitment();
        assert_eq!(c1.hash(), c2.hash());
    }

    #[test]
    fn test_v1_canonical_roundtrip() {
        let c = test_v1_commitment();
        let bytes = c.to_canonical_bytes();
        let restored = Commitment::from_canonical_bytes(&bytes).unwrap();
        assert_eq!(c.hash(), restored.hash());
    }

    // ─────────────────────────────────────────────
    // V2 tests (MPC-aware)
    // ─────────────────────────────────────────────

    #[test]
    fn test_v2_creation() {
        let c = test_v2_commitment();
        assert_eq!(c.version(), 2);
    }

    #[test]
    fn test_v2_hash_deterministic() {
        let c1 = test_v2_commitment();
        let c2 = test_v2_commitment();
        assert_eq!(c1.hash(), c2.hash());
    }

    #[test]
    fn test_v2_canonical_roundtrip() {
        let c = test_v2_commitment();
        let bytes = c.to_canonical_bytes();
        let restored = Commitment::from_canonical_bytes(&bytes).unwrap();
        assert_eq!(c.hash(), restored.hash());
    }

    #[test]
    fn test_v2_contains_mpc_root() {
        let protocol_id = [10u8; 32];
        let mpc_tree = MpcTree::from_pairs(&[
            (protocol_id, Hash::new([20u8; 32])),
            ([20u8; 32], Hash::new([30u8; 32])),
        ]);
        let _expected_root = mpc_tree.root();

        let seal = SealRef::new(vec![4u8; 16], Some(42)).unwrap();
        let c = Commitment::new(
            protocol_id,
            &mpc_tree,
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            &seal,
            [5u8; 32],
        );

        // The commitment hash should differ from a commitment without this MPC root
        let different_tree = MpcTree::from_pairs(&[(protocol_id, Hash::new([99u8; 32]))]);
        let c_different = Commitment::new(
            protocol_id,
            &different_tree,
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            &seal,
            [5u8; 32],
        );

        assert_ne!(c.hash(), c_different.hash());
    }

    #[test]
    fn test_v2_differs_by_protocol_id() {
        let mpc_tree = MpcTree::from_pairs(&[([10u8; 32], Hash::new([20u8; 32]))]);
        let seal = SealRef::new(vec![4u8; 16], Some(42)).unwrap();

        let c1 = Commitment::new(
            [10u8; 32],
            &mpc_tree,
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            &seal,
            [5u8; 32],
        );

        let c2 = Commitment::new(
            [11u8; 32],
            &mpc_tree,
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            &seal,
            [5u8; 32],
        );

        assert_ne!(c1.hash(), c2.hash());
    }

    // ─────────────────────────────────────────────
    // Version interop tests
    // ─────────────────────────────────────────────

    #[test]
    fn test_v1_v2_different_hashes() {
        let v1 = test_v1_commitment();
        let v2 = test_v2_commitment();
        assert_ne!(v1.hash(), v2.hash());
    }

    #[test]
    fn test_v1_v2_same_contract_different_versions() {
        let v1 = test_v1_commitment();
        let v2 = test_v2_commitment();
        // Both reference same contract ID
        assert_eq!(v1.contract_id(), v2.contract_id());
        // But different structure → different hashes
        assert_ne!(v1.hash(), v2.hash());
    }

    // ─────────────────────────────────────────────
    // Accessor tests
    // ─────────────────────────────────────────────

    #[test]
    fn test_commitment_accessors() {
        let v1 = test_v1_commitment();
        assert_eq!(v1.contract_id(), Hash::new([1u8; 32]));
        assert_eq!(v1.domain_separator(), [5u8; 32]);

        let v2 = test_v2_commitment();
        assert_eq!(v2.contract_id(), Hash::new([1u8; 32]));
        assert_eq!(v2.domain_separator(), [5u8; 32]);
    }

    // ─────────────────────────────────────────────
    // Deserialization error tests
    // ─────────────────────────────────────────────

    #[test]
    fn test_from_bytes_empty() {
        assert!(Commitment::from_canonical_bytes(&[]).is_err());
    }

    #[test]
    fn test_from_bytes_unknown_version() {
        let mut bytes = vec![99u8];
        bytes.resize(225, 0);
        assert!(Commitment::from_canonical_bytes(&bytes).is_err());
    }

    #[test]
    fn test_from_bytes_v1_too_short() {
        assert!(Commitment::from_canonical_bytes(&[1, 0, 0]).is_err());
    }

    #[test]
    fn test_from_bytes_v2_too_short() {
        assert!(Commitment::from_canonical_bytes(&[2, 0, 0]).is_err());
    }

    // ─────────────────────────────────────────────
    // MPC integration test
    // ─────────────────────────────────────────────

    #[test]
    fn test_commitment_with_multi_protocol_mpc() {
        // Simulate 3 protocols sharing one witness
        let proto_a = [0xAA; 32];
        let proto_b = [0xBB; 32];
        let proto_c = [0xCC; 32];

        let mpc_tree = MpcTree::from_pairs(&[
            (proto_a, Hash::new([1u8; 32])),
            (proto_b, Hash::new([2u8; 32])),
            (proto_c, Hash::new([3u8; 32])),
        ]);

        // Protocol A's commitment
        let seal = SealRef::new(vec![0xDD; 16], Some(1)).unwrap();
        let commitment_a = Commitment::new(
            proto_a,
            &mpc_tree,
            Hash::new([10u8; 32]),
            Hash::zero(),
            Hash::new([11u8; 32]),
            &seal,
            [0xEE; 32],
        );

        // Verify the MPC root in the commitment matches the tree root
        if let Commitment::V2(v2) = &commitment_a {
            assert_eq!(v2.mpc_root, mpc_tree.root());
        } else {
            panic!("Expected V2 commitment");
        }

        // Generate MPC proof for protocol A
        let proof = mpc_tree.prove(proto_a).unwrap();
        assert!(proof.verify(&mpc_tree.root()));
    }
}
