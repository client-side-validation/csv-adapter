//! Adapter signature integration tests

use csv_adapter_bitcoin::signatures::{verify_bitcoin_signature, verify_bitcoin_signatures};
use rand::rngs::OsRng;
use secp256k1::{Secp256k1, SecretKey};

fn generate_test_signature() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let secp = Secp256k1::new();
    let secret_key = SecretKey::new(&mut OsRng);
    let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
    let message = [0xAB; 32];
    let msg = secp256k1::Message::from_slice(&message).unwrap();
    let signature = secp.sign_ecdsa(&msg, &secret_key);
    let sig_bytes = signature.serialize_compact();
    let pubkey_bytes = public_key.serialize();
    (sig_bytes.to_vec(), pubkey_bytes.to_vec(), message.to_vec())
}

#[cfg(test)]
mod bitcoin_signature_tests {
    use super::*;

    #[test]
    fn test_valid_bitcoin_signature_comprehensive() {
        for _ in 0..10 {
            let (sig, pk, msg) = generate_test_signature();
            assert!(verify_bitcoin_signature(&sig, &pk, &msg).is_ok());
        }
    }

    #[test]
    fn test_bitcoin_signature_edge_cases() {
        let (sig, pk, msg) = generate_test_signature();

        // Test minimum valid sizes
        assert_eq!(sig.len(), 64);
        assert_eq!(pk.len(), 33);
        assert_eq!(msg.len(), 32);
    }

    #[test]
    fn test_bitcoin_multiple_signatures_verification() {
        let signatures: Vec<_> = (0..5).map(|_| generate_test_signature()).collect();

        assert!(verify_bitcoin_signatures(&signatures).is_ok());
    }

    #[test]
    fn test_bitcoin_signature_malleability() {
        let (mut sig, pk, msg) = generate_test_signature();

        // Flip high bit of S
        sig[32] |= 0x80;

        assert!(verify_bitcoin_signature(&sig, &pk, &msg).is_err());
    }
}
