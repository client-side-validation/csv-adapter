//! Ethereum signature integration tests

use csv_adapter_ethereum::signatures::{verify_ethereum_signature, verify_ethereum_signatures};
use secp256k1::{Secp256k1, SecretKey};
use rand::rngs::OsRng;

fn generate_test_signature() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let secp = Secp256k1::new();
    let secret_key = SecretKey::new(&mut OsRng);
    let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
    let message = [0xCD; 32];
    let msg = secp256k1::Message::from_slice(&message).unwrap();
    let signature = secp.sign_ecdsa(&msg, &secret_key);
    let sig_bytes = signature.serialize_compact();
    let pubkey_bytes = public_key.serialize();
    (sig_bytes.to_vec(), pubkey_bytes.to_vec(), message.to_vec())
}

#[cfg(test)]
mod ethereum_signature_tests {
    use super::*;

    #[test]
    fn test_valid_ethereum_signature_comprehensive() {
        for _ in 0..10 {
            let (sig, pk, msg) = generate_test_signature();
            assert!(verify_ethereum_signature(&sig, &pk, &msg).is_ok());
        }
    }

    #[test]
    fn test_ethereum_signature_with_recovery_id() {
        let (mut sig, pk, msg) = generate_test_signature();

        // Add recovery ID (65 bytes total)
        sig.push(0x00);
        assert!(verify_ethereum_signature(&sig, &pk, &msg).is_ok());
    }

    #[test]
    fn test_ethereum_multiple_signatures_verification() {
        let signatures: Vec<_> = (0..5)
            .map(|_| generate_test_signature())
            .collect();

        assert!(verify_ethereum_signatures(&signatures).is_ok());
    }

    #[test]
    fn test_ethereum_signature_low_s_value() {
        // Ethereum requires low-S signatures (BIP-62)
        let (sig, pk, msg) = generate_test_signature();

        // The signature should already be low-S from secp256k1 crate
        assert!(verify_ethereum_signature(&sig, &pk, &msg).is_ok());
    }
}
