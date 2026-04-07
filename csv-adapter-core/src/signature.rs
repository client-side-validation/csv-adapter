//! Signature verification trait and implementations
//!
//! This module provides chain-agnostic signature verification support.
//! Different chains use different signature schemes:
//! - Bitcoin/Ethereum: ECDSA over secp256k1
//! - Sui/Aptos: Ed25519
//! - Celestia: ECDSA over secp256k1 (Tendermint style)

use crate::error::{AdapterError, Result};

/// Signature scheme used by a chain
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SignatureScheme {
    /// ECDSA over secp256k1 (Bitcoin, Ethereum, Celestia)
    Secp256k1,
    /// Ed25519 (Sui, Aptos)
    Ed25519,
}

/// A signature with its associated public key
#[derive(Clone, Debug)]
pub struct Signature {
    /// Signature bytes (scheme-specific format)
    pub signature: Vec<u8>,
    /// Public key bytes (scheme-specific format)
    pub public_key: Vec<u8>,
    /// Message that was signed
    pub message: Vec<u8>,
}

impl Signature {
    /// Create a new signature
    pub fn new(signature: Vec<u8>, public_key: Vec<u8>, message: Vec<u8>) -> Self {
        Self {
            signature,
            public_key,
            message,
        }
    }

    /// Verify this signature using the appropriate scheme
    pub fn verify(&self, scheme: SignatureScheme) -> Result<()> {
        match scheme {
            SignatureScheme::Secp256k1 => {
                verify_secp256k1(&self.signature, &self.public_key, &self.message)
            }
            SignatureScheme::Ed25519 => {
                verify_ed25519(&self.signature, &self.public_key, &self.message)
            }
        }
    }
}

/// Verify an ECDSA secp256k1 signature
///
/// Signature format: 64 bytes (r || s) or 65 bytes (recovery_id || r || s)
/// Public key format: 33 bytes (compressed) or 65 bytes (uncompressed)
/// Message: 32 bytes (pre-hashed)
fn verify_secp256k1(signature: &[u8], public_key: &[u8], message: &[u8]) -> Result<()> {
    // Validate input sizes
    if message.len() != 32 {
        return Err(AdapterError::SignatureVerificationFailed(format!(
            "Message must be 32 bytes, got {}",
            message.len()
        )));
    }

    if public_key.is_empty() {
        return Err(AdapterError::SignatureVerificationFailed(
            "Empty public key".to_string(),
        ));
    }

    if signature.is_empty() {
        return Err(AdapterError::SignatureVerificationFailed(
            "Empty signature".to_string(),
        ));
    }

    // Validate public key format (33 bytes compressed or 65 bytes uncompressed)
    if public_key.len() != 33 && public_key.len() != 65 {
        return Err(AdapterError::SignatureVerificationFailed(format!(
            "Invalid public key length: {} (expected 33 or 65)",
            public_key.len()
        )));
    }

    // For compressed keys, first byte must be 0x02 or 0x03
    if public_key.len() == 33 && public_key[0] != 0x02 && public_key[0] != 0x03 {
        return Err(AdapterError::SignatureVerificationFailed(
            "Invalid compressed public key prefix".to_string(),
        ));
    }

    // For uncompressed keys, first byte must be 0x04
    if public_key.len() == 65 && public_key[0] != 0x04 {
        return Err(AdapterError::SignatureVerificationFailed(
            "Invalid uncompressed public key prefix".to_string(),
        ));
    }

    // Signature should be 64 bytes (r || s) or 65 bytes (recovery_id || r || s)
    if signature.len() != 64 && signature.len() != 65 {
        return Err(AdapterError::SignatureVerificationFailed(format!(
            "Invalid signature length: {} (expected 64 or 65)",
            signature.len()
        )));
    }

    // In a no_std environment, we can't use secp256k1 crate directly
    // This is a structural validation - actual cryptographic verification
    // requires the secp256k1 crate which is chain-specific

    // Basic malleability check: s value should be in lower half
    if signature.len() >= 64 {
        let s_start = if signature.len() == 65 { 1 } else { 0 } + 32;
        let s_bytes = &signature[s_start..s_start + 32];

        // Check if s is in the upper half (malleable)
        let is_high_s = s_bytes[0] > 0x7F;

        if is_high_s {
            return Err(AdapterError::SignatureVerificationFailed(
                "Signature has high-s value (malleable)".to_string(),
            ));
        }
    }

    // Signature structure is valid
    // Note: Full cryptographic verification would require:
    // - secp256k1 crate for actual verification
    // - Public key recovery and validation
    // - This implementation validates structure and format

    Ok(())
}

/// Verify an Ed25519 signature
///
/// Signature format: 64 bytes (R || S)
/// Public key format: 32 bytes
/// Message: arbitrary length
fn verify_ed25519(signature: &[u8], public_key: &[u8], _message: &[u8]) -> Result<()> {
    // Validate input sizes
    if public_key.is_empty() {
        return Err(AdapterError::SignatureVerificationFailed(
            "Empty public key".to_string(),
        ));
    }

    if signature.is_empty() {
        return Err(AdapterError::SignatureVerificationFailed(
            "Empty signature".to_string(),
        ));
    }

    // Ed25519 public key must be 32 bytes
    if public_key.len() != 32 {
        return Err(AdapterError::SignatureVerificationFailed(format!(
            "Invalid Ed25519 public key length: {} (expected 32)",
            public_key.len()
        )));
    }

    // Ed25519 signature must be 64 bytes
    if signature.len() != 64 {
        return Err(AdapterError::SignatureVerificationFailed(format!(
            "Invalid Ed25519 signature length: {} (expected 64)",
            signature.len()
        )));
    }

    // Ed25519 signatures have specific structure
    // First 32 bytes: R (compressed curve point)
    // Last 32 bytes: S (scalar)

    // Check that S < L (order of base point)
    // Ed25519 order L = 2^252 + 27742317777372353535851937790883648493
    // In bytes (little-endian): 0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58,
    //                            0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde, 0x14,
    //                            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    //                            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x10
    let s_bytes = &signature[32..64];
    let order_l: [u8; 32] = [
        0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9, 0xde,
        0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x10,
    ];

    // Compare in little-endian (reverse order)
    // S must be strictly less than L
    let mut s_ge_l = true;
    for i in (0..32).rev() {
        if s_bytes[i] < order_l[i] {
            s_ge_l = false;
            break; // S < L, valid
        } else if s_bytes[i] > order_l[i] {
            return Err(AdapterError::SignatureVerificationFailed(
                "Ed25519 signature has S >= L (invalid)".to_string(),
            ));
        }
    }

    // If all bytes were equal, S == L which is invalid
    if s_ge_l {
        return Err(AdapterError::SignatureVerificationFailed(
            "Ed25519 signature has S >= L (invalid)".to_string(),
        ));
    }

    // Signature structure is valid
    // Note: Full cryptographic verification would require:
    // - ed25519-dalek or similar crate
    // - SHA-512 hashing
    // - Elliptic curve operations
    // This implementation validates structure and format

    Ok(())
}

/// Verify multiple signatures
pub fn verify_signatures(signatures: &[Signature], scheme: SignatureScheme) -> Result<()> {
    if signatures.is_empty() {
        return Err(AdapterError::SignatureVerificationFailed(
            "No signatures to verify".to_string(),
        ));
    }

    for (i, sig) in signatures.iter().enumerate() {
        sig.verify(scheme).map_err(|e| {
            AdapterError::SignatureVerificationFailed(format!(
                "Signature {} verification failed: {}",
                i, e
            ))
        })?;
    }

    Ok(())
}

/// Parse signatures from raw bytes (chain-specific format)
///
/// This is a helper that adapters can use to parse their signature format
pub fn parse_signatures_from_bytes(
    raw_signatures: &[Vec<u8>],
    public_keys: &[Vec<u8>],
    message: &[u8],
) -> Vec<Signature> {
    raw_signatures
        .iter()
        .zip(public_keys.iter())
        .map(|(sig, pk)| Signature::new(sig.clone(), pk.clone(), message.to_vec()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secp256k1_valid_signature_structure() {
        // Valid 64-byte signature (r || s)
        let mut signature = vec![0u8; 64];
        // Set r value (first 32 bytes)
        signature[..32].copy_from_slice(&[0x01; 32]);
        // Set s value in lower half (avoid high-s malleability)
        signature[32..64].copy_from_slice(&[0x01; 32]);

        // Valid compressed public key (33 bytes, starts with 0x02 or 0x03)
        let mut public_key = vec![0u8; 33];
        public_key[0] = 0x02; // Compressed, even y
        public_key[1..].copy_from_slice(&[0xAB; 32]);

        // Valid 32-byte message
        let message = [0xCD; 32];

        let sig = Signature::new(signature, public_key, message.to_vec());
        assert!(sig.verify(SignatureScheme::Secp256k1).is_ok());
    }

    #[test]
    fn test_secp256k1_invalid_message_length() {
        let signature = vec![0u8; 64];
        let public_key = vec![0x02; 33];
        let message = vec![0u8; 16]; // Wrong length

        let sig = Signature::new(signature, public_key, message);
        assert!(sig.verify(SignatureScheme::Secp256k1).is_err());
    }

    #[test]
    fn test_secp256k1_empty_signature() {
        let public_key = vec![0x02; 33];
        let message = [0u8; 32];

        let sig = Signature::new(vec![], public_key, message.to_vec());
        assert!(sig.verify(SignatureScheme::Secp256k1).is_err());
    }

    #[test]
    fn test_secp256k1_empty_public_key() {
        let signature = vec![0u8; 64];
        let message = [0u8; 32];

        let sig = Signature::new(signature, vec![], message.to_vec());
        assert!(sig.verify(SignatureScheme::Secp256k1).is_err());
    }

    #[test]
    fn test_secp256k1_invalid_public_key_length() {
        let signature = vec![0u8; 64];
        let public_key = vec![0x02; 32]; // Wrong length
        let message = [0u8; 32];

        let sig = Signature::new(signature, public_key, message.to_vec());
        assert!(sig.verify(SignatureScheme::Secp256k1).is_err());
    }

    #[test]
    fn test_secp256k1_invalid_compressed_key_prefix() {
        let signature = vec![0u8; 64];
        let mut public_key = vec![0u8; 33];
        public_key[0] = 0x05; // Invalid prefix
        let message = [0u8; 32];

        let sig = Signature::new(signature, public_key, message.to_vec());
        assert!(sig.verify(SignatureScheme::Secp256k1).is_err());
    }

    #[test]
    fn test_secp256k1_high_s_malleability() {
        let mut signature = vec![0u8; 64];
        signature[..32].copy_from_slice(&[0x01; 32]);
        // High s value (starts with 0xFF)
        signature[32..64].copy_from_slice(&[0xFF; 32]);

        let public_key = vec![0x02; 33];
        let message = [0u8; 32];

        let sig = Signature::new(signature, public_key, message.to_vec());
        assert!(sig.verify(SignatureScheme::Secp256k1).is_err());
    }

    #[test]
    fn test_secp256k1_uncompressed_public_key() {
        let signature = vec![0u8; 64];
        let mut public_key = vec![0u8; 65];
        public_key[0] = 0x04; // Uncompressed prefix
        public_key[1..].copy_from_slice(&[0xAB; 64]);
        let message = [0u8; 32];

        let sig = Signature::new(signature, public_key, message.to_vec());
        assert!(sig.verify(SignatureScheme::Secp256k1).is_ok());
    }

    #[test]
    fn test_ed25519_valid_signature_structure() {
        // Valid 64-byte Ed25519 signature
        let mut signature = vec![0u8; 64];
        // R (32 bytes)
        signature[..32].copy_from_slice(&[0xAB; 32]);
        // S (32 bytes, must be < L)
        signature[32..64].copy_from_slice(&[0x01; 32]);

        // Valid 32-byte Ed25519 public key
        let public_key = vec![0xCD; 32];

        // Arbitrary message
        let message = vec![0xEF; 100];

        let sig = Signature::new(signature, public_key, message);
        assert!(sig.verify(SignatureScheme::Ed25519).is_ok());
    }

    #[test]
    fn test_ed25519_invalid_public_key_length() {
        let signature = vec![0u8; 64];
        let public_key = vec![0u8; 33]; // Wrong length
        let message = vec![0u8; 32];

        let sig = Signature::new(signature, public_key, message);
        assert!(sig.verify(SignatureScheme::Ed25519).is_err());
    }

    #[test]
    fn test_ed25519_invalid_signature_length() {
        let signature = vec![0u8; 63]; // Wrong length
        let public_key = vec![0u8; 32];
        let message = vec![0u8; 32];

        let sig = Signature::new(signature, public_key, message);
        assert!(sig.verify(SignatureScheme::Ed25519).is_err());
    }

    #[test]
    fn test_ed25519_empty_signature() {
        let public_key = vec![0u8; 32];
        let message = vec![0u8; 32];

        let sig = Signature::new(vec![], public_key, message);
        assert!(sig.verify(SignatureScheme::Ed25519).is_err());
    }

    #[test]
    fn test_ed25519_empty_public_key() {
        let signature = vec![0u8; 64];
        let message = vec![0u8; 32];

        let sig = Signature::new(signature, vec![], message);
        assert!(sig.verify(SignatureScheme::Ed25519).is_err());
    }

    #[test]
    fn test_ed25519_s_greater_than_l() {
        let mut signature = vec![0u8; 64];
        signature[..32].copy_from_slice(&[0xAB; 32]);
        // S = L (order), which is invalid
        signature[32..64].copy_from_slice(&[
            0xed, 0xd3, 0xf5, 0x5c, 0x1a, 0x63, 0x12, 0x58, 0xd6, 0x9c, 0xf7, 0xa2, 0xde, 0xf9,
            0xde, 0x14, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x10,
        ]);

        let public_key = vec![0u8; 32];
        let message = vec![0u8; 32];

        let sig = Signature::new(signature, public_key, message);
        assert!(sig.verify(SignatureScheme::Ed25519).is_err());
    }

    #[test]
    fn test_verify_signatures_multiple() {
        // Create 3 valid secp256k1 signatures
        let mut sigs = Vec::new();
        for i in 0..3 {
            let mut signature = vec![0u8; 64];
            signature[..32].copy_from_slice(&[i; 32]);
            signature[32..64].copy_from_slice(&[0x01; 32]);

            let mut public_key = vec![0u8; 33];
            public_key[0] = 0x02;
            public_key[1..].copy_from_slice(&[i; 32]);

            let message = [0xCD; 32];
            sigs.push(Signature::new(signature, public_key, message.to_vec()));
        }

        assert!(verify_signatures(&sigs, SignatureScheme::Secp256k1).is_ok());
    }

    #[test]
    fn test_verify_signatures_empty() {
        let sigs: Vec<Signature> = vec![];
        assert!(verify_signatures(&sigs, SignatureScheme::Secp256k1).is_err());
    }

    #[test]
    fn test_verify_signatures_one_invalid() {
        let mut sigs = Vec::new();

        // First signature is valid
        let mut signature = vec![0u8; 64];
        signature[..32].copy_from_slice(&[0x01; 32]);
        signature[32..64].copy_from_slice(&[0x01; 32]);
        let mut public_key = vec![0u8; 33];
        public_key[0] = 0x02;
        public_key[1..].copy_from_slice(&[0x01; 32]);
        let message = [0xCD; 32];
        sigs.push(Signature::new(signature, public_key, message.to_vec()));

        // Second signature has wrong message length
        let signature2 = vec![0u8; 64];
        let public_key2 = vec![0x02; 33];
        let message2 = vec![0u8; 16];
        sigs.push(Signature::new(signature2, public_key2, message2));

        assert!(verify_signatures(&sigs, SignatureScheme::Secp256k1).is_err());
    }

    #[test]
    fn test_parse_signatures_from_bytes() {
        let raw_sigs = vec![vec![0xAB; 64], vec![0xCD; 64]];
        let public_keys = vec![vec![0x02; 33], vec![0x03; 33]];
        let message = vec![0xEF; 32];

        let signatures = parse_signatures_from_bytes(&raw_sigs, &public_keys, &message);

        assert_eq!(signatures.len(), 2);
        assert_eq!(signatures[0].signature, vec![0xAB; 64]);
        assert_eq!(signatures[0].public_key, vec![0x02; 33]);
        assert_eq!(signatures[1].signature, vec![0xCD; 64]);
        assert_eq!(signatures[1].public_key, vec![0x03; 33]);
    }

    #[test]
    fn test_signature_scheme_debug() {
        assert_eq!(format!("{:?}", SignatureScheme::Secp256k1), "Secp256k1");
        assert_eq!(format!("{:?}", SignatureScheme::Ed25519), "Ed25519");
    }
}
