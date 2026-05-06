//! Tagged hash with domain separation
//!
//! Implements BIP-340 style tagged hashing:
//! `tagged_hash(tag, data) = sha256(sha256(tag) || sha256(tag) || data)`
//!
//! This prevents cross-protocol hash collision attacks by ensuring
//! hashes computed in different contexts (MPC trees, commitments, DAGs)
//! are cryptographically separated.

use alloc::format;
use sha2::{Digest, Sha256};

/// The domain tag prefix for all CSV-related hashes
pub const CSV_TAG_PREFIX: &str = "urn:lnp-bp:csv:";

/// Compute a tagged hash with domain separation.
///
/// `tagged_hash(tag, data) = sha256(sha256(tag) || sha256(tag) || data)`
///
/// This matches BIP-340 (Taproot) tagged hashing, preventing
/// cross-protocol hash collision attacks.
pub fn tagged_hash(tag: &str, data: &[u8]) -> [u8; 32] {
    let tag_hash = {
        let mut hasher = Sha256::new();
        hasher.update(tag.as_bytes());
        hasher.finalize()
    };

    let mut hasher = Sha256::new();
    hasher.update(tag_hash);
    hasher.update(tag_hash);
    hasher.update(data);
    let result = hasher.finalize();

    let mut array = [0u8; 32];
    array.copy_from_slice(&result);
    array
}

/// Compute a tagged hash with the CSV domain prefix.
///
/// Convenience wrapper: `csv_tagged_hash(name, data) = tagged_hash("urn:lnp-bp:csv:" || name, data)`
pub fn csv_tagged_hash(name: &str, data: &[u8]) -> [u8; 32] {
    let full_tag = format!("{}{}", CSV_TAG_PREFIX, name);
    tagged_hash(&full_tag, data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tagged_hash_deterministic() {
        let h1 = tagged_hash("test", b"hello");
        let h2 = tagged_hash("test", b"hello");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_tagged_hash_different_tags() {
        let h1 = tagged_hash("tag1", b"hello");
        let h2 = tagged_hash("tag2", b"hello");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_tagged_hash_different_data() {
        let h1 = tagged_hash("test", b"hello");
        let h2 = tagged_hash("test", b"world");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_csv_tagged_hash() {
        let h1 = csv_tagged_hash("commitment", b"data");
        let h2 = csv_tagged_hash("commitment", b"data");
        assert_eq!(h1, h2);

        // Different name produces different hash
        let h3 = csv_tagged_hash("mpc", b"data");
        assert_ne!(h1, h3);
    }

    #[test]
    fn test_tagged_hash_not_same_as_raw_sha256() {
        let tagged = tagged_hash("test", b"data");
        let raw = {
            let mut hasher = Sha256::new();
            hasher.update(b"data");
            hasher.finalize()
        };
        // Tagged hash should NOT equal raw SHA-256
        let raw_arr: [u8; 32] = raw.into();
        assert_ne!(&tagged, &raw_arr);
    }
}
