//! Commitment type with canonical encoding (MPC-aware, multi-protocol)
//!
//! Commitments bind off-chain state transitions to the anchoring layer.
//! Each commitment is a node in a **commitment chain** — a sequence of
//! linked state transitions that clients validate independently of the blockchain.
//!
//! ## Stability Guarantee
//!
//! **Only V2 is supported.** V1 was removed to prevent silent divergence
//! between clients. All commitments must use the V2 format. This format
//! will not change without a version bump and backward-compatible migration.

use alloc::vec::Vec;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::hash::Hash;
use crate::commit_mux::MpcTree;
use crate::seal::SealPoint;
use crate::tagged_hash::csv_tagged_hash;

/// Current commitment format version.
///
/// This constant is the authoritative version number. Any commitment
/// with `version != COMMITMENT_VERSION` should be rejected.
pub const COMMITMENT_VERSION: u8 = 2;

/// A V2 commitment binding state to an anchor.
///
/// A commitment is the core data structure in CSV. It captures everything
/// needed to verify a state transition:
///
/// - **Which protocol** it belongs to (`protocol_id`)
/// - **What state** it represents (`mpc_root`, `contract_id`)
/// - **Where it came from** (`previous_commitment`)
/// - **What changed** (`transition_payload_hash`)
/// - **What seal was consumed** (`seal_id`)
/// - **Which chain context** it's valid in (`domain_separator`)
///
/// ## Commitment Chain
///
/// Commitments form a linked chain: each commitment references the hash
/// of the previous one via `previous_commitment`. Clients validate the
/// entire chain from genesis to the current state without querying the
/// blockchain for each step.
///
/// ## Collision Resistance
///
/// Each field is hashed with a unique domain tag (e.g., `"commitment-version"`,
/// `"commitment-protocol-id"`) using [`csv_tagged_hash`]. This prevents
/// cross-field and cross-protocol collision attacks where an attacker could
/// rearrange field bytes to forge a valid-looking commitment.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Commitment {
    /// Format version. Currently always [`COMMITMENT_VERSION`].
    pub version: u8,

    /// Protocol identifier this commitment belongs to.
    ///
    /// This is a 32-byte namespace that isolates commitments from different
    /// protocols sharing the same anchoring layer. For example:
    /// - Bitcoin adapter: `b"CSV-BTC-\x00\x00..."` (magic bytes padded)
    /// - Ethereum adapter: `b"CSV-ETH-\x00\x00..."`
    pub protocol_id: [u8; 32],

    /// Merkle root of the MPC tree for multi-protocol witnessing.
    ///
    /// When multiple protocols share a single witness (e.g., a threshold
    /// signature spanning Bitcoin and Ethereum), this root commits to
    /// all participating protocols' individual roots.
    ///
    /// For single-protocol use (the common case), this is set to a
    /// deterministic empty root: `SHA256("csv-empty-mpc-root")`.
    pub mpc_root: Hash,

    /// Unique contract/right identifier.
    ///
    /// For NFTs, this is the token ID. For credentials, this is the
    /// credential hash. For assets, this is the asset identifier.
    ///
    /// Must be unique within the protocol namespace.
    pub contract_id: Hash,

    /// Hash of the previous commitment in the chain.
    ///
    /// For the first commitment (genesis), this is typically `Hash::zero()`.
    /// For subsequent commitments, this is `previous_commitment.hash()`.
    ///
    /// This forms the commitment chain that clients validate independently.
    pub previous_commitment: Hash,

    /// SHA-256 hash of the state transition payload.
    ///
    /// This commits to the actual data being transitioned (e.g., new owner,
    /// metadata update, transfer details). Clients must verify this hash
    /// matches the payload they expect.
    pub transition_payload_hash: Hash,

    /// SHA-256 hash of the consumed seal reference.
    ///
    /// This binds the commitment to the specific seal that was consumed
    /// to authorize this transition. The seal reference includes the
    /// chain-specific identifier (UTXO txid, object ID, etc.) and the
    /// seal's nonce/value.
    pub seal_id: Hash,

    /// Domain separator for chain-specific commitment isolation.
    ///
    /// Prevents cross-chain replay attacks by ensuring a commitment
    /// created on one chain cannot be used on another. Typically
    /// constructed as `H(chain_id || network || protocol_version)`.
    pub domain_separator: [u8; 32],
}

impl Commitment {
    /// Create a commitment
    ///
    /// This creates a V2 (MPC-aware) commitment. All adapters should use this
    /// constructor. The `protocol_id` should uniquely identify the protocol
    /// (e.g., "CSV-BTC-" for Bitcoin, "CSV-ETH-" for Ethereum).
    pub fn new(
        protocol_id: [u8; 32],
        mpc_tree: &MpcTree,
        contract_id: Hash,
        previous_commitment: Hash,
        transition_payload_hash: Hash,
        seal_ref: &SealPoint,
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

        Self {
            version: COMMITMENT_VERSION,
            protocol_id,
            mpc_root,
            contract_id,
            previous_commitment,
            transition_payload_hash,
            seal_id: seal_hash,
            domain_separator,
        }
    }

    /// Create a commitment without MPC tree (for simple single-protocol use)
    ///
    /// This is a convenience constructor that uses a default empty MPC root.
    /// For multi-protocol use cases, use [`Commitment::new`] instead.
    pub fn simple(
        contract_id: Hash,
        previous_commitment: Hash,
        transition_payload_hash: Hash,
        seal_ref: &SealPoint,
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

        // Extract protocol_id from domain separator (first 4 bytes)
        let mut protocol_id = [0u8; 32];
        protocol_id[..4].copy_from_slice(&domain_separator[..4]);

        // Use empty MPC root for single-protocol mode
        let mpc_root = {
            let mut hasher = Sha256::new();
            hasher.update(b"csv-empty-mpc-root");
            let result = hasher.finalize();
            let mut array = [0u8; 32];
            array.copy_from_slice(&result);
            Hash::new(array)
        };

        Self {
            version: COMMITMENT_VERSION,
            protocol_id,
            mpc_root,
            contract_id,
            previous_commitment,
            transition_payload_hash,
            seal_id: seal_hash,
            domain_separator,
        }
    }

    /// Backwards-compatible alias for [`Commitment::simple`].
    #[deprecated(since = "0.3.0", note = "Use `Commitment::simple` instead")]
    // Note: The `since` field must be a string literal per Rust's `deprecated` attribute.
    // It cannot use the generated constant directly. However, this is okay because
    // the deprecation marker itself rarely changes - only the crate version does.
    pub fn v1(
        contract_id: Hash,
        previous_commitment: Hash,
        transition_payload_hash: Hash,
        seal_ref: &SealPoint,
        domain_separator: [u8; 32],
    ) -> Self {
        Self::simple(
            contract_id,
            previous_commitment,
            transition_payload_hash,
            seal_ref,
            domain_separator,
        )
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
        // Use tagged hashing for each field to prevent cross-protocol collisions
        hasher.update(csv_tagged_hash("commitment-version", &[self.version]));
        hasher.update(csv_tagged_hash("commitment-protocol-id", &self.protocol_id));
        hasher.update(csv_tagged_hash(
            "commitment-mpc-root",
            self.mpc_root.as_bytes(),
        ));
        hasher.update(csv_tagged_hash(
            "commitment-contract-id",
            self.contract_id.as_bytes(),
        ));
        hasher.update(csv_tagged_hash(
            "commitment-prev",
            self.previous_commitment.as_bytes(),
        ));
        hasher.update(csv_tagged_hash(
            "commitment-payload",
            self.transition_payload_hash.as_bytes(),
        ));
        hasher.update(csv_tagged_hash("commitment-seal", self.seal_id.as_bytes()));
        hasher.update(csv_tagged_hash("commitment-domain", &self.domain_separator));
    }

    /// Get the version
    pub fn version(&self) -> u8 {
        self.version
    }

    /// Get the contract ID
    pub fn contract_id(&self) -> Hash {
        self.contract_id
    }

    /// Get the seal ID hash
    pub fn seal_id(&self) -> Hash {
        self.seal_id
    }

    /// Get the domain separator
    pub fn domain_separator(&self) -> [u8; 32] {
        self.domain_separator
    }

    /// Serialize commitment using canonical encoding
    pub fn to_canonical_bytes(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(1 + 32 * 7);
        out.push(self.version);
        out.extend_from_slice(&self.protocol_id);
        out.extend_from_slice(self.mpc_root.as_bytes());
        out.extend_from_slice(self.contract_id.as_bytes());
        out.extend_from_slice(self.previous_commitment.as_bytes());
        out.extend_from_slice(self.transition_payload_hash.as_bytes());
        out.extend_from_slice(self.seal_id.as_bytes());
        out.extend_from_slice(&self.domain_separator);
        out
    }

    /// Deserialize commitment from canonical bytes
    ///
    /// **Only V2 format is supported.** Legacy V1 format was removed.
    pub fn from_canonical_bytes(bytes: &[u8]) -> Result<Self, &'static str> {
        if bytes.is_empty() {
            return Err("Empty commitment bytes");
        }

        let version = bytes[0];
        if version != COMMITMENT_VERSION {
            return Err("Unsupported commitment version");
        }

        // V2 format: version(1) + protocol_id(32) + mpc_root(32) + contract_id(32) +
        //             previous_commitment(32) + payload_hash(32) + seal_id(32) + domain_separator(32)
        let min_len = 1 + 32 * 7;
        if bytes.len() < min_len {
            return Err("Commitment bytes too short");
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

        Ok(Self {
            version,
            protocol_id,
            mpc_root: Hash::new(mpc_root),
            contract_id: Hash::new(contract_id),
            previous_commitment: Hash::new(previous_commitment),
            transition_payload_hash: Hash::new(transition_payload_hash),
            seal_id: Hash::new(seal_id),
            domain_separator,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::commit_mux::MpcTree;

    fn test_commitment_commitment() -> Commitment {
        Commitment::simple(
            Hash::new([1u8; 32]),
            Hash::new([2u8; 32]),
            Hash::new([3u8; 32]),
            &SealPoint::new(vec![4u8; 16], Some(42)).unwrap(),
            [5u8; 32],
        )
    }

    fn test_mpc_commitment() -> Commitment {
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
            &SealPoint::new(vec![4u8; 16], Some(42)).unwrap(),
            [5u8; 32],
        )
    }

    // ─────────────────────────────────────────────
    // Basic commitment tests
    // ─────────────────────────────────────────────

    #[test]
    fn test_commitment_creation() {
        let c = test_commitment_commitment();
        assert_eq!(c.version(), COMMITMENT_VERSION);
    }

    #[test]
    fn test_commitment_hash_deterministic() {
        let c1 = test_commitment_commitment();
        let c2 = test_commitment_commitment();
        assert_eq!(c1.hash(), c2.hash());
    }

    #[test]
    fn test_commitment_canonical_roundtrip() {
        let c = test_commitment_commitment();
        let bytes = c.to_canonical_bytes();
        let restored = Commitment::from_canonical_bytes(&bytes).unwrap();
        assert_eq!(c.hash(), restored.hash());
    }

    // ─────────────────────────────────────────────
    // MPC-aware commitment tests
    // ─────────────────────────────────────────────

    #[test]
    fn test_mpc_creation() {
        let c = test_mpc_commitment();
        assert_eq!(c.version(), COMMITMENT_VERSION);
    }

    #[test]
    fn test_mpc_hash_deterministic() {
        let c1 = test_mpc_commitment();
        let c2 = test_mpc_commitment();
        assert_eq!(c1.hash(), c2.hash());
    }

    #[test]
    fn test_mpc_canonical_roundtrip() {
        let c = test_mpc_commitment();
        let bytes = c.to_canonical_bytes();
        let restored = Commitment::from_canonical_bytes(&bytes).unwrap();
        assert_eq!(c.hash(), restored.hash());
    }

    #[test]
    fn test_mpc_contains_mpc_root() {
        let protocol_id = [10u8; 32];
        let mpc_tree = MpcTree::from_pairs(&[
            (protocol_id, Hash::new([20u8; 32])),
            ([20u8; 32], Hash::new([30u8; 32])),
        ]);
        let _expected_root = mpc_tree.root();

        let seal = SealPoint::new(vec![4u8; 16], Some(42)).unwrap();
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
    fn test_mpc_differs_by_protocol_id() {
        let mpc_tree = MpcTree::from_pairs(&[([10u8; 32], Hash::new([20u8; 32]))]);
        let seal = SealPoint::new(vec![4u8; 16], Some(42)).unwrap();

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
    fn test_commitment_v2_different_hashes() {
        let v1 = test_commitment_commitment();
        let v2 = test_mpc_commitment();
        assert_ne!(v1.hash(), v2.hash());
    }

    #[test]
    fn test_commitment_v2_same_contract_different_versions() {
        let v1 = test_commitment_commitment();
        let v2 = test_mpc_commitment();
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
        let v1 = test_commitment_commitment();
        assert_eq!(v1.contract_id(), Hash::new([1u8; 32]));
        assert_eq!(v1.domain_separator(), [5u8; 32]);

        let v2 = test_mpc_commitment();
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
    fn test_from_bytes_unsupported_version() {
        assert!(Commitment::from_canonical_bytes(&[1, 0, 0]).is_err()); // V1 no longer supported
    }

    #[test]
    fn test_from_bytes_too_short() {
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
        let seal = SealPoint::new(vec![0xDD; 16], Some(1)).unwrap();
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
        assert_eq!(commitment_a.mpc_root, mpc_tree.root());

        // Generate MPC proof for protocol A
        let proof = mpc_tree.prove(proto_a).unwrap();
        assert!(proof.verify(&mpc_tree.root()));
    }
}
