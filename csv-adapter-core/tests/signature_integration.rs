//! Core signature structural validation tests

use csv_adapter_core::signature::{Signature, SignatureScheme, verify_signatures};

#[cfg(test)]
mod core_signature_tests {
    use super::*;

    #[test]
    fn test_secp256k1_valid_structure() {
        // Valid 64-byte signature
        let signature = vec![0x01; 64];
        let public_key = vec![0x02; 33]; // Compressed
        let message = vec![0xAB; 32];

        let sig = Signature::new(signature, public_key, message);
        assert!(sig.verify(SignatureScheme::Secp256k1).is_ok());
    }

    #[test]
    fn test_secp256k1_65_byte_signature() {
        // 65 bytes with recovery ID
        let signature = vec![0x01; 65];
        let public_key = vec![0x02; 33];
        let message = vec![0xAB; 32];

        let sig = Signature::new(signature, public_key, message);
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
        let signature = vec![0x01; 64];
        let public_key = vec![0xAB; 32];
        let message = b"test message".to_vec();

        let sig = Signature::new(signature, public_key, message);
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
        let sigs: Vec<Signature> = (0..3).map(|i| {
            let mut sig_bytes = vec![0u8; 64];
            sig_bytes[0] = i as u8;
            Signature::new(sig_bytes, vec![0x02; 33], vec![i; 32])
        }).collect();

        assert!(verify_signatures(&sigs, SignatureScheme::Secp256k1).is_ok());
    }

    #[test]
    fn test_verify_empty_signatures() {
        let sigs: Vec<Signature> = vec![];
        assert!(verify_signatures(&sigs, SignatureScheme::Secp256k1).is_err());
    }
}
