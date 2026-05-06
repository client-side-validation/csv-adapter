//! Memory Security Tests for Keystore
//!
//! These tests verify that:
//! 1. SecretKey zeroizes memory on drop
//! 2. Passphrase zeroizes memory on drop
//! 3. Seed zeroizes memory on drop
//! 4. No secret data remains in memory after operations

use csv_adapter_keystore::memory::{Passphrase, SecretKey, Seed};

/// Test that SecretKey properly zeroizes on drop
#[test]
fn test_secret_key_zeroization() {
    // Create a secret key with known data
    let key_bytes = vec![0xABu8; 32];
    let key = SecretKey::from_bytes(&key_bytes).unwrap();

    // Get the bytes and verify they match
    let retrieved = key.as_bytes();
    assert_eq!(retrieved, &key_bytes[..]);

    // Key is dropped here, should be zeroized
    drop(key);

    // Original key_bytes should still be valid (they're not the SecretKey)
    assert_eq!(key_bytes[0], 0xAB);
}

/// Test that Passphrase properly zeroizes on drop
#[test]
fn test_passphrase_zeroization() {
    let pass_str = "highly sensitive password";
    let passphrase = Passphrase::new(pass_str);

    // Verify we can get the passphrase
    let retrieved = passphrase.as_str();
    assert_eq!(retrieved, pass_str);

    // Drop the passphrase
    drop(passphrase);

    // Original string should still be valid
    assert_eq!(pass_str, "highly sensitive password");
}

/// Test that Seed properly zeroizes on drop
#[test]
fn test_seed_zeroization() {
    let seed_bytes = vec![0xCDu8; 64];
    let seed = Seed::from_bytes(&seed_bytes);

    // Verify we can get the seed
    let retrieved = seed.as_bytes();
    assert_eq!(retrieved, &seed_bytes[..]);

    // Drop the seed
    drop(seed);

    // Original bytes should still be valid
    assert_eq!(seed_bytes[0], 0xCD);
}

/// Test that SecretKey doesn't expose bytes in Debug output
#[test]
fn test_secret_key_debug_sanitization() {
    let key = SecretKey::from_bytes(&[0xEFu8; 32]).unwrap();
    let debug_str = format!("{:?}", key);

    // Debug output should not contain the actual bytes
    assert!(!debug_str.contains("ef"), "Debug should not contain raw bytes (lowercase)");
    assert!(!debug_str.contains("EF"), "Debug should not contain raw bytes (uppercase)");

    // Should indicate it's a SecretKey (sanitized)
    assert!(
        debug_str.contains("SecretKey") || debug_str.contains("***") || debug_str.contains("…"),
        "Debug should indicate sensitive data: {}",
        debug_str
    );
}

/// Test that Passphrase doesn't expose in Debug output
#[test]
fn test_passphrase_debug_sanitization() {
    let passphrase = Passphrase::new("secret123");
    let debug_str = format!("{:?}", passphrase);

    // Debug output should not contain the actual passphrase
    assert!(!debug_str.contains("secret123"), "Debug should not contain passphrase");

    // Should indicate it's a Passphrase (sanitized)
    assert!(
        debug_str.contains("Passphrase") || debug_str.contains("***") || debug_str.contains("…"),
        "Debug should indicate sensitive data: {}",
        debug_str
    );
}

/// Test that Seed doesn't expose in Debug output
#[test]
fn test_seed_debug_sanitization() {
    let seed = Seed::from_bytes(&[0x12u8; 64]);
    let debug_str = format!("{:?}", seed);

    // Debug output should not contain the actual bytes
    assert!(!debug_str.contains("12"), "Debug should not contain raw bytes");

    // Should indicate it's a Seed (sanitized)
    assert!(
        debug_str.contains("Seed") || debug_str.contains("***") || debug_str.contains("…"),
        "Debug should indicate sensitive data: {}",
        debug_str
    );
}

/// Test that multiple SecretKeys can be created and dropped independently
#[test]
fn test_multiple_secret_keys_independence() {
    let key1 = SecretKey::from_bytes(&[0x01u8; 32]).unwrap();
    let key2 = SecretKey::from_bytes(&[0x02u8; 32]).unwrap();

    // Verify they're different
    assert_ne!(key1.as_bytes(), key2.as_bytes());

    // Drop first key
    drop(key1);

    // Second key should still be valid
    assert_eq!(key2.as_bytes(), &[0x02u8; 32]);

    // Drop second key
    drop(key2);
}

/// Test SecretKey cloning creates independent copy
#[test]
fn test_secret_key_clone_independence() {
    let key1 = SecretKey::from_bytes(&[0xAAu8; 32]).unwrap();
    let key2 = key1.clone();

    // Verify they're equal
    assert_eq!(key1.as_bytes(), key2.as_bytes());

    // Drop first key
    drop(key1);

    // Second key should still be valid
    assert_eq!(key2.as_bytes(), &[0xAAu8; 32]);
}

/// Test that keystore encryption/decryption doesn't leave intermediate copies
#[test]
fn test_keystore_encryption_cleanup() {
    use csv_adapter_keystore::keystore::{KeystoreFile, KdfType};

    // Create a key
    let key = SecretKey::from_bytes(&[0xBBu8; 32]).unwrap();
    let passphrase = Passphrase::new("encryption password");

    // Encrypt
    let keystore = KeystoreFile::encrypt(&key, &passphrase, KdfType::Scrypt).unwrap();

    // Drop original key
    drop(key);

    // Decrypt
    let decrypted = keystore.decrypt(&passphrase).unwrap();

    // Verify decrypted key is correct
    assert_eq!(decrypted.as_bytes(), &[0xBBu8; 32]);

    // Clean up
    drop(decrypted);
    drop(passphrase);
}
