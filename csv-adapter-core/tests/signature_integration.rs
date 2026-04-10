//! Core signature structural validation tests

use csv_adapter_core::signature::{Signature, SignatureScheme, verify_signatures};

#[cfg(test)]
mod core_signature_tests {
    use super::*;

    #[test]
    fn test_secp256k1_valid_structure() {
        use secp256k1::{Secp256k1, SecretKey, Message};

        let secp = Secp256k1::new();
        let secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let message = [0xAB; 32];
        let msg = Message::from_digest_slice(&message).unwrap();
        let signature = secp.sign_ecdsa(&msg, &secret_key);
        let sig_bytes = signature.serialize_compact();
        let pubkey_bytes = public_key.serialize();

        let sig = Signature::new(sig_bytes.to_vec(), pubkey_bytes.to_vec(), message.to_vec());
        assert!(sig.verify(SignatureScheme::Secp256k1).is_ok());
    }

    #[test]
    fn test_secp256k1_65_byte_signature() {
        use secp256k1::{Secp256k1, SecretKey, Message};

        let secp = Secp256k1::new();
        let secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
        let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
        let message = [0xAB; 32];
        let msg = Message::from_digest_slice(&message).unwrap();
        let signature = secp.sign_ecdsa(&msg, &secret_key);
        let sig_bytes = signature.serialize_compact();
        // Build 65-byte signature: [recovery_id (1 byte)] + [r || s (64 bytes)]
        let mut sig_65 = vec![0u8; 65];
        sig_65[0] = 0; // recovery ID
        sig_65[1..].copy_from_slice(&sig_bytes);
        let pubkey_bytes = public_key.serialize();

        let sig = Signature::new(sig_65, pubkey_bytes.to_vec(), message.to_vec());
        assert!(sig.verify(SignatureScheme::Secp256k1).is_ok());
    }

    #[test]
    fn test_secp256k1_rejects_wrong_message_size() {
        let signature = vec![0x01; 64];
        let public_key = vec![0x02; 33];
        let message = vec![0xAB; 16]; // Wrong size

        let sig = Signature::new(signature, public_key, message);
        assert!(sig.verify(SignatureScheme::Secp256k1).is_err());
    }

    #[test]
    fn test_secp256k1_rejects_empty_public_key() {
        let signature = vec![0x01; 64];
        let public_key = vec![];
        let message = vec![0xAB; 32];

        let sig = Signature::new(signature, public_key, message);
        assert!(sig.verify(SignatureScheme::Secp256k1).is_err());
    }

    #[test]
    fn test_secp256k1_rejects_wrong_public_key_size() {
        let signature = vec![0x01; 64];
        let public_key = vec![0x02; 32]; // Wrong size
        let message = vec![0xAB; 32];

        let sig = Signature::new(signature, public_key, message);
        assert!(sig.verify(SignatureScheme::Secp256k1).is_err());
    }

    #[test]
    fn test_ed25519_valid_structure() {
        use ed25519_dalek::{SigningKey, Signer};
        use rand::rngs::OsRng;

        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        let message = b"test message".to_vec();
        let signature = signing_key.sign(&message);

        let sig = Signature::new(signature.to_bytes().to_vec(), verifying_key.to_bytes().to_vec(), message);
        assert!(sig.verify(SignatureScheme::Ed25519).is_ok());
    }

    #[test]
    fn test_ed25519_rejects_wrong_signature_size() {
        let signature = vec![0x01; 32]; // Wrong size
        let public_key = vec![0xAB; 32];
        let message = b"test message".to_vec();

        let sig = Signature::new(signature, public_key, message);
        assert!(sig.verify(SignatureScheme::Ed25519).is_err());
    }

    #[test]
    fn test_ed25519_rejects_wrong_public_key_size() {
        let signature = vec![0x01; 64];
        let public_key = vec![0xAB; 33]; // Wrong size
        let message = b"test message".to_vec();

        let sig = Signature::new(signature, public_key, message);
        assert!(sig.verify(SignatureScheme::Ed25519).is_err());
    }

    #[test]
    fn test_verify_multiple_signatures() {
        use secp256k1::{Secp256k1, SecretKey, Message};

        let secp = Secp256k1::new();
        let message = [0xAB; 32];
        let msg = Message::from_digest_slice(&message).unwrap();

        let sigs: Vec<Signature> = (0..3).map(|_| {
            let secret_key = SecretKey::new(&mut secp256k1::rand::thread_rng());
            let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
            let signature = secp.sign_ecdsa(&msg, &secret_key);
            let sig_bytes = signature.serialize_compact();
            let pubkey_bytes = public_key.serialize();
            Signature::new(sig_bytes.to_vec(), pubkey_bytes.to_vec(), message.to_vec())
        }).collect();

        assert!(verify_signatures(&sigs, SignatureScheme::Secp256k1).is_ok());
    }

    #[test]
    fn test_verify_empty_signatures() {
        let sigs: Vec<Signature> = vec![];
        assert!(verify_signatures(&sigs, SignatureScheme::Secp256k1).is_err());
    }
}
