//! Aptos signature integration tests

use csv_adapter_aptos::signatures::{verify_aptos_signature, verify_aptos_signatures};
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;

fn generate_test_signature() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    let message = b"Aptos signature test message for CSV";
    let signature = signing_key.sign(message);
    (
        signature.to_bytes().to_vec(),
        verifying_key.to_bytes().to_vec(),
        message.to_vec(),
    )
}

#[cfg(test)]
mod aptos_signature_tests {
    use super::*;

    #[test]
    fn test_valid_aptos_signature_comprehensive() {
        for _ in 0..10 {
            let (sig, pk, msg) = generate_test_signature();
            assert!(verify_aptos_signature(&sig, &pk, &msg).is_ok());
        }
    }

    #[test]
    fn test_aptos_signature_with_various_message_sizes() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();

        let test_messages = vec![
            b"short".to_vec(),
            b"A".repeat(100),
            b"B".repeat(1000),
            vec![0u8; 256],
        ];

        for msg in test_messages {
            let signature = signing_key.sign(&msg);
            assert!(verify_aptos_signature(
                signature.to_bytes().as_ref(),
                verifying_key.to_bytes().as_ref(),
                &msg
            )
            .is_ok());
        }
    }

    #[test]
    fn test_aptos_multiple_signatures_verification() {
        let signatures: Vec<_> = (0..5).map(|_| generate_test_signature()).collect();

        assert!(verify_aptos_signatures(&signatures).is_ok());
    }
}
