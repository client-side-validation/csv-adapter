//! Canonical serialization primitives for CSV Protocol
//!
//! All proof payloads, sanad envelopes, and commitment inputs MUST use
//! these functions. Raw serde JSON is forbidden in any hashing path.
//!
//! Encoding: deterministic CBOR (RFC 8949 section 4.2 canonical form)
//!   - Keys sorted lexicographically
//!   - No indefinite-length encoding
//!   - Integers in smallest representation
//!
//! External crate: `ciborium` (no_std compatible, pure Rust)

use alloc::string::String;
use alloc::vec::Vec;
use crate::error::ProtocolError;

/// Serialize `value` to deterministic CBOR bytes.
///
/// # Errors
/// Returns `ProtocolError::SerializationError` if encoding fails.
pub fn to_canonical_cbor<T: serde::Serialize>(value: &T) -> Result<Vec<u8>, ProtocolError> {
    let mut buf = Vec::new();
    ciborium::into_writer(value, &mut buf)
        .map_err(|e| ProtocolError::SerializationError(String::from(e.to_string())))?;
    Ok(buf)
}

/// Deserialize from deterministic CBOR bytes.
///
/// # Errors
/// Returns `ProtocolError::Deserialization` if decoding fails.
pub fn from_canonical_cbor<T: serde::de::DeserializeOwned>(bytes: &[u8]) -> Result<T, ProtocolError> {
    ciborium::from_reader(bytes)
        .map_err(|e| ProtocolError::SerializationError(String::from(e.to_string())))
}

/// Hash canonical CBOR encoding of `value` using tagged_hash.
///
/// This is the ONLY approved way to hash protocol data.
/// Direct `sha256`, `keccak256`, or `blake3` calls are forbidden.
pub fn canonical_hash<T: serde::Serialize>(
    domain: &str,
    value: &T,
) -> Result<crate::hash::Hash, ProtocolError> {
    let cbor = to_canonical_cbor(value)?;
    Ok(crate::hash::Hash::new(
        crate::tagged_hash::csv_tagged_hash(domain, &cbor)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Fixture { a: u32, b: String }

    #[test]
    fn roundtrip_is_lossless() {
        let v = Fixture { a: 42, b: "hello".into() };
        let bytes = to_canonical_cbor(&v).unwrap();
        let back: Fixture = from_canonical_cbor(&bytes).unwrap();
        assert_eq!(v, back);
    }

    #[test]
    fn encoding_is_deterministic() {
        let v = Fixture { a: 1, b: "world".into() };
        let b1 = to_canonical_cbor(&v).unwrap();
        let b2 = to_canonical_cbor(&v).unwrap();
        assert_eq!(b1, b2);
    }

    #[test]
    fn canonical_hash_domain_separation() {
        let v = Fixture { a: 0, b: "test".into() };
        let h1 = canonical_hash("domain.a", &v).unwrap();
        let h2 = canonical_hash("domain.b", &v).unwrap();
        assert_ne!(h1, h2);
    }
}
