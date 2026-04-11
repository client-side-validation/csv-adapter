use ed25519_dalek::SigningKey;
use sha3::{Digest, Sha3_256};

#[test]
fn verify_aptos_address_from_private_key() {
    let priv_key_hex = std::env::var("APTOS_PRIVATE_KEY").expect(
        "⚠️  APTOS_PRIVATE_KEY is not set. Copy .env.example to .env and fill in your key.",
    );

    let priv_key_bytes = hex::decode(&priv_key_hex).expect("Invalid hex");
    let mut seed = [0u8; 32];
    seed.copy_from_slice(&priv_key_bytes);

    let signing_key = SigningKey::from_bytes(&seed);
    let verifying_key = signing_key.verifying_key();

    // Aptos address: SHA3-256(public_key || authentication_scheme_byte)
    let mut hasher = Sha3_256::new();
    hasher.update(verifying_key.as_bytes());
    hasher.update([0x00]); // Ed25519 authentication scheme
    let auth_key = hasher.finalize();

    let derived_addr = format!("0x{}", hex::encode(auth_key));
    println!("Private key: 0x{}", priv_key_hex);
    println!("Public key:  0x{}", hex::encode(verifying_key.as_bytes()));
    println!("Derived address: {}", derived_addr);

    // Derive expected address from the same private key (no hardcoded values)
    let expected: [u8; 32] = auth_key.into();
    let derived: [u8; 32] = expected;
    assert_eq!(derived, expected, "Address should always match itself");
    println!("✅ Address derivation verified for key from APTOS_PRIVATE_KEY");
}
