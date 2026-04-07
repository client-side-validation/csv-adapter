//! Celestia signature integration tests

use csv_adapter_celestia::signatures::{verify_celestia_signature, verify_celestia_signatures};
use secp256k1::{Secp256k1, SecretKey};
use rand::rngs::OsRng;

fn generate_test_signature() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let secp = Secp256k1::new();
    let secret_key = SecretKey::new(&mut OsRng);
    let public_key = secp256k1::PublicKey::from_secret_key(&secp, &secret_key);
    let message = [0xEF; 32];
    let msg = secp256k1::Message::from_slice(&message).unwrap();
    let signature = secp.sign_ecdsa(&msg, &secret_key);
    let sig_bytes = signature.serialize_compact();
    let pubkey_bytes = public_key.serialize();
    (sig_bytes.to_vec(), pubkey_bytes.to_vec(), message.to_vec())
}

#[cfg(test)]
mod celestia_signature_tests {
    use super::*;

    #[test]
    fn test_valid_celestia_signature_comprehensive() {
        for _ in 0..10 {
            let (sig, pk, msg) = generate_test_signature();
            assert!(verify_celestia_signature(&sig, &pk, &msg).is_ok());
        }
    }

    #[test]
    fn test_celestia_tendermint_format() {
        // Tendermint uses compressed public keys
        let (sig, pk, msg) = generate_test_signature();

        // Verify compressed public key (33 bytes)
        assert_eq!(pk.len(), 33);
        assert!(pk[0] == 0x02 || pk[0] == 0x03);

        // Verify signature is 64 bytes
        assert_eq!(sig.len(), 64);

        assert!(verify_celestia_signature(&sig, &pk, &msg).is_ok());
    }

    #[test]
    fn test_celestia_multiple_signatures_verification() {
        let signatures: Vec<_> = (0..5)
            .map(|_| generate_test_signature())
            .collect();

        assert!(verify_celestia_signatures(&signatures).is_ok());
    }
}
