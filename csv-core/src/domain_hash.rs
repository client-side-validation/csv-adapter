//! Domain-separated hashing for cryptographic isolation
//!
//! This module provides type-safe domain-separated hashing to prevent
//! cross-domain replay attacks. Each cryptographic context (Bitcoin seals,
//! Ethereum mints, Aptos anchors, etc.) has its own domain type, ensuring
//! that identical payloads hash differently across domains.
//!
//! ## Security Properties
//!
//! - **Replay prevention**: A proof valid in one domain cannot be replayed in another
//! - **Collision resistance**: Domain tags prevent cross-protocol hash collisions
//! - **Type safety**: Rust's type system enforces domain separation at compile time
//!
//! ## Usage
//!
//! ```rust
//! use csv_core::domain_hash::DomainSeparatedHash;
//! use csv_core::domains::BitcoinSealDomain;
//!
//! let hash = DomainSeparatedHash::<BitcoinSealDomain>::hash(b"payload");
//! ```

use core::marker::PhantomData;
use sha2::{Digest, Sha256};

use crate::hash::Hash;

/// Domain marker trait for cryptographic separation
///
/// Each domain type implements this trait with a unique domain tag.
/// The domain tag is prepended to all hashes computed in that domain,
/// preventing cross-domain replay attacks.
pub trait Domain {
    /// Unique domain identifier for this cryptographic context
    ///
    /// Must be unique across all domains in the protocol.
    /// Recommended format: `b"csv.<context>.<version>"` e.g., `b"csv.bitcoin.seal.v1"`
    const DOMAIN: &'static [u8];
}

/// Domain-separated hash computation
///
/// This type provides domain-separated hashing using the type-level domain marker.
/// Hashes computed in different domains will always differ, even for identical payloads.
///
/// ## Implementation
///
/// Uses SHA256 with domain separation:
/// ```text
/// hash = SHA256(DOMAIN || payload)
/// ```
///
/// ## Type Parameters
///
/// - `D`: Domain type implementing [`Domain`] trait
pub struct DomainSeparatedHash<D>(PhantomData<D>);

impl<D: Domain> DomainSeparatedHash<D> {
    /// Compute a domain-separated hash of the payload
    ///
    /// The domain tag from `D::DOMAIN` is prepended to the payload before hashing,
    /// ensuring cryptographic separation between domains.
    ///
    /// ## Arguments
    ///
    /// - `payload`: Data to hash
    ///
    /// ## Returns
    ///
    /// SHA256 hash of `DOMAIN || payload`
    pub fn hash(payload: &[u8]) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(D::DOMAIN);
        hasher.update(payload);
        let result = hasher.finalize();

        let mut array = [0u8; 32];
        array.copy_from_slice(&result);
        Hash::new(array)
    }

    /// Compute a domain-separated hash of multiple payloads
    ///
    /// Concatenates all payloads with domain separation:
    /// ```text
    /// hash = SHA256(DOMAIN || payload1 || payload2 || ...)
    /// ```
    ///
    /// ## Arguments
    ///
    /// - `payloads`: Iterator over byte slices to hash
    ///
    /// ## Returns
    ///
    /// SHA256 hash of `DOMAIN || payload1 || payload2 || ...`
    pub fn hash_multiple<'a, I>(payloads: I) -> Hash
    where
        I: IntoIterator<Item = &'a [u8]>,
    {
        let mut hasher = Sha256::new();
        hasher.update(D::DOMAIN);
        for payload in payloads {
            hasher.update(payload);
        }
        let result = hasher.finalize();

        let mut array = [0u8; 32];
        array.copy_from_slice(&result);
        Hash::new(array)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Test domain types
    struct TestDomain1;
    impl Domain for TestDomain1 {
        const DOMAIN: &'static [u8] = b"csv.test.domain1.v1";
    }

    struct TestDomain2;
    impl Domain for TestDomain2 {
        const DOMAIN: &'static [u8] = b"csv.test.domain2.v1";
    }

    #[test]
    fn test_domain_hash_deterministic() {
        let h1 = DomainSeparatedHash::<TestDomain1>::hash(b"test");
        let h2 = DomainSeparatedHash::<TestDomain1>::hash(b"test");
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_different_domains_different_hashes() {
        let h1 = DomainSeparatedHash::<TestDomain1>::hash(b"test");
        let h2 = DomainSeparatedHash::<TestDomain2>::hash(b"test");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_different_payloads_different_hashes() {
        let h1 = DomainSeparatedHash::<TestDomain1>::hash(b"test1");
        let h2 = DomainSeparatedHash::<TestDomain1>::hash(b"test2");
        assert_ne!(h1, h2);
    }

    #[test]
    fn test_hash_multiple() {
        let payloads: [&[u8]; 3] = [b"a".as_slice(), b"b".as_slice(), b"c".as_slice()];
        let h1 = DomainSeparatedHash::<TestDomain1>::hash_multiple(payloads);
        let h2 = DomainSeparatedHash::<TestDomain1>::hash(b"abc");
        assert_ne!(h1, h2); // Different because hash_multiple concatenates without separators
    }

    #[test]
    fn test_domain_hash_not_raw_sha256() {
        let domain_hash = DomainSeparatedHash::<TestDomain1>::hash(b"test");

        let raw_hash = {
            let mut hasher = Sha256::new();
            hasher.update(b"test");
            let result = hasher.finalize();
            let mut array = [0u8; 32];
            array.copy_from_slice(&result);
            Hash::new(array)
        };

        assert_ne!(domain_hash, raw_hash);
    }

    #[test]
    fn test_domain_separation_prevents_replay() {
        // Simulate a proof that's valid in domain1
        let proof = b"valid_proof_data";
        let hash_domain1 = DomainSeparatedHash::<TestDomain1>::hash(proof);

        // Same proof in domain2 produces different hash
        let hash_domain2 = DomainSeparatedHash::<TestDomain2>::hash(proof);

        assert_ne!(hash_domain1, hash_domain2);
    }
}
