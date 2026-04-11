//! Sui signature integration tests

use csv_adapter_sui::signatures::{verify_sui_signature, verify_sui_signatures};
use ed25519_dalek::{Signer, SigningKey};
use rand::rngs::OsRng;

fn generate_test_signature() -> (Vec<u8>, Vec<u8>, Vec<u8>) {
    let mut csprng = OsRng;
    let signing_key = SigningKey::generate(&mut csprng);
    let verifying_key = signing_key.verifying_key();
    let message = b"Sui signature test message for CSV adapter";
    let signature = signing_key.sign(message);
    (
        signature.to_bytes().to_vec(),
        verifying_key.to_bytes().to_vec(),
        message.to_vec(),
    )
}

#[cfg(test)]
mod sui_signature_tests {
    use super::*;

    #[test]
    fn test_valid_sui_signature_comprehensive() {
        for _ in 0..10 {
            let (sig, pk, msg) = generate_test_signature();
            assert!(verify_sui_signature(&sig, &pk, &msg).is_ok());
        }
    }

    #[test]
    fn test_sui_signature_with_various_message_sizes() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();

        // Test with different message sizes
        let test_messages = vec![
            b"short".to_vec(),
            b"A".repeat(100),
            b"B".repeat(1000),
            vec![0u8; 256],
        ];

        for msg in test_messages {
            let signature = signing_key.sign(&msg);
            assert!(verify_sui_signature(
                signature.to_bytes().as_ref(),
                verifying_key.to_bytes().as_ref(),
                &msg
            )
            .is_ok());
        }
    }

    #[test]
    fn test_sui_multiple_signatures_verification() {
        let signatures: Vec<_> = (0..5).map(|_| generate_test_signature()).collect();

        assert!(verify_sui_signatures(&signatures).is_ok());
    }

    #[test]
    fn test_sui_signature_determinism() {
        // Ed25519 is deterministic - same key + message = same signature
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let message = b"deterministic test";

        let sig1 = signing_key.sign(message);
        let sig2 = signing_key.sign(message);

        assert_eq!(sig1.to_bytes(), sig2.to_bytes());
    }
}
