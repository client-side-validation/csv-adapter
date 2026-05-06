//! A 32-byte cryptographic hash used throughout the CSV protocol.
//!
//! This type wraps a fixed-size `[u8; 32]` array and provides safe conversion
//! between byte slices, hex strings, and the internal representation. All
//! hashing in CSV uses SHA-256 with domain separation to prevent cross-protocol
//! replay attacks (see `crate::tagged_hash`).
//!
//! # Examples
//!
//! ```
//! use csv_adapter_core::Hash;
//!
//! // Create from bytes
//! let h = Hash::new([0xAB; 32]);
//!
//! // Convert to hex
//! let hex = h.to_hex();
//! assert!(hex.starts_with("abab"));
//!
//! // Parse from hex
//! let parsed = Hash::from_hex(&hex).unwrap();
//! assert_eq!(h, parsed);
//! ```

use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};

/// A 32-byte hash value.
///
/// This is the fundamental building block for commitments, right IDs,
/// seal references, and all cryptographic operations in CSV.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Hash([u8; 32]);

impl Hash {
    /// Creates a new [`struct@Hash`] from exactly 32 bytes.
    ///
    /// # Panics
    /// This method does not panic — it accepts any `[u8; 32]`. For fallible
    /// construction from a slice, use [`Hash::try_from`].
    #[inline]
    pub const fn new(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Returns a hash of all zeros. Useful as a sentinel value.
    #[inline]
    pub const fn zero() -> Self {
        Self([0u8; 32])
    }

    /// Returns a reference to the underlying 32-byte array.
    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }

    /// Returns a mutable reference to the underlying 32-byte array.
    #[inline]
    pub fn as_bytes_mut(&mut self) -> &mut [u8; 32] {
        &mut self.0
    }

    /// Returns the hash as a byte slice.
    #[inline]
    pub fn as_slice(&self) -> &[u8] {
        &self.0
    }

    /// Consumes the hash and returns the inner byte array.
    #[inline]
    pub fn into_inner(self) -> [u8; 32] {
        self.0
    }

    /// Returns a new [`Vec<u8>`] containing the hash bytes.
    ///
    /// This allocates. For a borrowed slice, use [`Self::as_slice`].
    #[inline]
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.to_vec()
    }

    /// Returns the hash as a lowercase hex string without the `0x` prefix.
    ///
    /// The returned string is always 64 characters long.
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Parses a [`struct@Hash`] from a hex string.
    ///
    /// The input may optionally start with `0x` or `0X`. The remaining
    /// characters must be valid hex digits representing exactly 32 bytes.
    ///
    /// # Errors
    /// Returns [`HashParseError`] if the input is not valid hex or does not
    /// represent exactly 32 bytes.
    pub fn from_hex(s: &str) -> Result<Self, HashParseError> {
        let s = s
            .strip_prefix("0x")
            .or_else(|| s.strip_prefix("0X"))
            .unwrap_or(s);
        let bytes = hex::decode(s).map_err(|e| HashParseError::InvalidHex(e.to_string()))?;
        if bytes.len() != 32 {
            return Err(HashParseError::WrongLength {
                expected: 32,
                got: bytes.len(),
            });
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(&bytes);
        Ok(Self(arr))
    }
}

// ============================================================================
// Trait Implementations
// ============================================================================

impl AsRef<[u8]> for Hash {
    #[inline]
    fn as_ref(&self) -> &[u8] {
        &self.0
    }
}

impl AsRef<[u8; 32]> for Hash {
    #[inline]
    fn as_ref(&self) -> &[u8; 32] {
        &self.0
    }
}

impl From<[u8; 32]> for Hash {
    #[inline]
    fn from(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }
}

impl From<&[u8; 32]> for Hash {
    #[inline]
    fn from(bytes: &[u8; 32]) -> Self {
        Self(*bytes)
    }
}

impl TryFrom<&[u8]> for Hash {
    type Error = HashParseError;

    fn try_from(bytes: &[u8]) -> Result<Self, Self::Error> {
        if bytes.len() != 32 {
            return Err(HashParseError::WrongLength {
                expected: 32,
                got: bytes.len(),
            });
        }
        let mut arr = [0u8; 32];
        arr.copy_from_slice(bytes);
        Ok(Self(arr))
    }
}

impl FromStr for Hash {
    type Err = HashParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::from_hex(s)
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Show first 8 hex chars + ellipsis for compact display
        if f.alternate() {
            write!(f, "0x{}", self.to_hex())
        } else {
            write!(f, "0x{}…", &self.to_hex()[..8])
        }
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash(0x{})", self.to_hex())
    }
}

impl Default for Hash {
    #[inline]
    fn default() -> Self {
        Self::zero()
    }
}

// ============================================================================
// Error Types
// ============================================================================

/// Errors that can occur when parsing a [`struct@Hash`] from a string or byte slice.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[allow(missing_docs)]
pub enum HashParseError {
    /// The input string is not valid hexadecimal.
    #[error("invalid hex: {0}")]
    InvalidHex(String),

    /// The decoded bytes are not exactly 32 bytes.
    #[error("expected 32 bytes, got {got}")]
    WrongLength { expected: usize, got: usize },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_new() {
        let h = Hash::new([1u8; 32]);
        assert_eq!(h.as_bytes(), &[1u8; 32]);
    }

    #[test]
    fn test_hash_zero() {
        let h = Hash::zero();
        assert_eq!(h.as_bytes(), &[0u8; 32]);
    }

    #[test]
    fn test_hash_hex_roundtrip() {
        let h = Hash::new([0xAB; 32]);
        let hex = h.to_hex();
        let parsed = Hash::from_hex(&hex).unwrap();
        assert_eq!(h, parsed);
    }

    #[test]
    fn test_hash_from_hex_with_prefix() {
        let h = Hash::from_hex("0xabcdef").unwrap_err();
        assert!(matches!(h, HashParseError::WrongLength { .. }));
    }

    #[test]
    fn test_hash_display() {
        let h = Hash::new([0xAB; 32]);
        let display = format!("{}", h);
        assert!(display.starts_with("0x"));
        assert!(display.contains("…"));
    }

    #[test]
    fn test_hash_display_altern() {
        let h = Hash::new([0xAB; 32]);
        let display = format!("{:#}", h);
        assert_eq!(display.len(), 66); // "0x" + 64 hex chars
    }

    #[test]
    fn test_hash_debug() {
        let h = Hash::new([0xAB; 32]);
        let debug = format!("{:?}", h);
        assert!(debug.starts_with("Hash(0x"));
    }

    #[test]
    fn test_hash_from_str() {
        let h: Hash = "abababababababababababababababababababababababababababababababab"
            .parse()
            .unwrap();
        assert_eq!(
            h.to_hex(),
            "abababababababababababababababababababababababababababababababab"
        );
    }
}
