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
    use secp256k1::{Secp256k1, PublicKey, Message, ecdsa};

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

    // Signature should be 64 bytes (r || s) or 65 bytes (recovery_id || r || s)
    if signature.len() != 64 && signature.len() != 65 {
        return Err(AdapterError::SignatureVerificationFailed(format!(
            "Invalid signature length: {} (expected 64 or 65)",
            signature.len()
        )));
    }

    // Parse public key
    let pubkey = PublicKey::from_slice(public_key)
        .map_err(|e| AdapterError::SignatureVerificationFailed(format!(
            "Invalid public key: {}", e
        )))?;

    // Parse signature
    let sig = if signature.len() == 64 {
        ecdsa::Signature::from_compact(signature)
            .map_err(|e| AdapterError::SignatureVerificationFailed(format!(
                "Invalid signature format: {}", e
            )))?
    } else {
        // 65 bytes: skip recovery ID
        ecdsa::Signature::from_compact(&signature[1..])
            .map_err(|e| AdapterError::SignatureVerificationFailed(format!(
                "Invalid signature format: {}", e
            )))?
    };

    // Parse message
    let msg = Message::from_digest_slice(message)
        .map_err(|e| AdapterError::SignatureVerificationFailed(format!(
            "Invalid message: {}", e
        )))?;

    // Perform actual cryptographic verification
    let secp = Secp256k1::verification_only();
    secp.verify_ecdsa(&msg, &sig, &pubkey)
        .map_err(|e| AdapterError::SignatureVerificationFailed(format!(
            "Signature verification failed: {}", e
        )))?;

    Ok(())
}

/// Verify an Ed25519 signature
///
/// Signature format: 64 bytes (R || S)
/// Public key format: 32 bytes
/// Message: arbitrary length
fn verify_ed25519(signature: &[u8], public_key: &[u8], message: &[u8]) -> Result<()> {
    use ed25519_dalek::{VerifyingKey, Signature, Verifier};

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

    // Parse public key
    let verifying_key = VerifyingKey::from_bytes(public_key.try_into().unwrap())
        .map_err(|e| AdapterError::SignatureVerificationFailed(format!(
            "Invalid Ed25519 public key: {}", e
        )))?;

    // Parse signature
    let sig_bytes_arr: [u8; 64] = signature.try_into().unwrap();
    let sig = Signature::from_bytes(&sig_bytes_arr);

    // Perform actual cryptographic verification
    verifying_key.verify(message, &sig)
        .map_err(|e| AdapterError::SignatureVerificationFailed(format!(
            "Ed25519 signature verification failed: {}", e
        )))?;

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
    fn test_secp256k1_valid_signature() {
        use secp256k1::{Secp256k1, SecretKey, Message};

        let secp = Secp256k1::new();
        let secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let message = [0xCD; 32];
        let msg = Message::from_digest_slice(&message).unwrap();
        let signature = secp.sign_ecdsa(&msg, &secret_key);
        let sig_bytes = signature.serialize_compact();
        let pubkey_bytes = public_key.serialize();

        let sig = Signature::new(sig_bytes.to_vec(), pubkey_bytes.to_vec(), message.to_vec());
        assert!(sig.verify(SignatureScheme::Secp256k1).is_ok());
    }

    #[test]
    fn test_secp256k1_invalid_signature_fails() {
        use secp256k1::{Secp256k1, SecretKey, Message};

        let secp = Secp256k1::new();
        let secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let pubkey_bytes = public_key.serialize();

        // Wrong message
        let message = [0xCD; 32];
        let different_message = [0xAB; 32];
        let msg = Message::from_digest_slice(&message).unwrap();
        let signature = secp.sign_ecdsa(&msg, &secret_key);
        let sig_bytes = signature.serialize_compact();

        let sig = Signature::new(sig_bytes.to_vec(), pubkey_bytes.to_vec(), different_message.to_vec());
        assert!(sig.verify(SignatureScheme::Secp256k1).is_err());
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
    fn test_secp256k1_tampered_signature() {
        use secp256k1::{Secp256k1, SecretKey, Message};

        let secp = Secp256k1::new();
        let secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let message = [0xCD; 32];
        let msg = Message::from_digest_slice(&message).unwrap();
        let signature = secp.sign_ecdsa(&msg, &secret_key);
        let mut sig_bytes = signature.serialize_compact();
        // Tamper with signature
        sig_bytes[0] ^= 0xFF;
        let pubkey_bytes = public_key.serialize();

        let sig = Signature::new(sig_bytes.to_vec(), pubkey_bytes.to_vec(), message.to_vec());
        assert!(sig.verify(SignatureScheme::Secp256k1).is_err());
    }

    #[test]
    fn test_ed25519_valid_signature() {
        use ed25519_dalek::{SigningKey, Signer, VerifyingKey};
        use ed25519_dalek::Signature as DalekSignature;
        use rand::rngs::OsRng;

        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key: VerifyingKey = signing_key.verifying_key();
        let message = b"This is a test message for Ed25519 verification";
        let signature: DalekSignature = signing_key.sign(message);

        let sig = Signature::new(signature.to_bytes().to_vec(), verifying_key.to_bytes().to_vec(), message.to_vec());
        assert!(sig.verify(SignatureScheme::Ed25519).is_ok());
    }

    #[test]
    fn test_ed25519_invalid_signature_fails() {
        use ed25519_dalek::{SigningKey, Signer, VerifyingKey};
        use ed25519_dalek::Signature as DalekSignature;
        use rand::rngs::OsRng;

        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key: VerifyingKey = signing_key.verifying_key();
        let message = b"Original message";
        let different_message = b"Different message";
        let signature: DalekSignature = signing_key.sign(message);

        let sig = Signature::new(signature.to_bytes().to_vec(), verifying_key.to_bytes().to_vec(), different_message.to_vec());
        assert!(sig.verify(SignatureScheme::Ed25519).is_err());
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
    fn test_ed25519_tampered_signature() {
        use ed25519_dalek::{SigningKey, Signer, VerifyingKey};
        use ed25519_dalek::Signature as DalekSignature;
        use rand::rngs::OsRng;

        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key: VerifyingKey = signing_key.verifying_key();
        let message = b"Test message";
        let signature: DalekSignature = signing_key.sign(message);
        let mut sig_bytes = signature.to_bytes();
        // Tamper with signature
        sig_bytes[0] ^= 0xFF;

        let sig = Signature::new(sig_bytes.to_vec(), verifying_key.to_bytes().to_vec(), message.to_vec());
        assert!(sig.verify(SignatureScheme::Ed25519).is_err());
    }

    #[test]
    fn test_verify_signatures_multiple() {
        use secp256k1::{Secp256k1, SecretKey, Message};

        let secp = Secp256k1::new();
        let message = [0xCD; 32];
        let msg = Message::from_digest_slice(&message).unwrap();

        // Create 3 valid secp256k1 signatures with different keys
        let mut sigs = Vec::new();
        for _ in 0..3 {
            let secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
            let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
            let signature = secp.sign_ecdsa(&msg, &secret_key);
            let sig_bytes = signature.serialize_compact();
            let pubkey_bytes = public_key.serialize();
            sigs.push(Signature::new(sig_bytes.to_vec(), pubkey_bytes.to_vec(), message.to_vec()));
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
        use secp256k1::{Secp256k1, SecretKey, Message};

        let secp = Secp256k1::new();
        let message = [0xCD; 32];
        let msg = Message::from_digest_slice(&message).unwrap();

        // First signature is valid
        let secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let signature = secp.sign_ecdsa(&msg, &secret_key);
        let sig_bytes = signature.serialize_compact();
        let pubkey_bytes = public_key.serialize();
        let mut sigs = vec![Signature::new(sig_bytes.to_vec(), pubkey_bytes.to_vec(), message.to_vec())];

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
