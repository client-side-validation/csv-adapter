//! Key manager for multi-chain wallet.
//!
//! Handles key derivation and signing operations for all supported chains.
//! Supports both in-memory seed-based keys and persistent native keystore storage.
//! All sensitive data is zeroized on drop to prevent memory leaks.

use blake2::Blake2b;
use csv_core::ChainId;
use csv_core::mcp::{FixAction, HasErrorSuggestion, error_codes};
use ed25519_dalek::{Signer, SigningKey, VerifyingKey};
use secp256k1::{Keypair, Secp256k1, SecretKey, XOnlyPublicKey};
use sha2::Digest;
use sha3::Keccak256;
use zeroize::{Zeroize, ZeroizeOnDrop};

#[cfg(not(target_arch = "wasm32"))]
use crate::core::native_keystore::{NativeKeystore, NativeKeystoreError};
#[cfg(not(target_arch = "wasm32"))]
use csv_keys::memory::{Passphrase, SecretKey as MemorySecretKey};

/// Error type for key management operations.
#[derive(Debug, thiserror::Error)]
pub enum KeyError {
    /// Invalid key format
    #[error("Invalid key format: {0}")]
    InvalidKeyFormat(String),
    /// Derivation error
    #[error("Key derivation error: {0}")]
    DerivationError(String),
    /// Signing error
    #[error("Signing error: {0}")]
    SigningError(String),
}

impl HasErrorSuggestion for KeyError {
    fn error_code(&self) -> &'static str {
        match self {
            KeyError::InvalidKeyFormat(_) => error_codes::WALLET_KEY_INVALID_FORMAT,
            KeyError::DerivationError(_) => error_codes::WALLET_KEY_DERIVATION_FAILED,
            KeyError::SigningError(_) => error_codes::WALLET_SIGNING_FAILED,
        }
    }

    fn description(&self) -> String {
        self.to_string()
    }

    fn suggested_fix(&self) -> String {
        match self {
            KeyError::InvalidKeyFormat(_) => "Invalid key format. For seed phrases, ensure: \
                 1) 12 or 24 BIP-39 words, 2) Words from standard wordlist, \
                 3) Correct spelling. For private keys, ensure: \
                 1) 64 hex characters, 2) Valid for the target chain."
                .to_string(),
            KeyError::DerivationError(_) => "Key derivation failed. Check: \
                 1) The seed/mnemonic is valid, 2) The derivation path is correct, \
                 3) The target chain uses the correct curve (secp256k1 vs ed25519). \
                 Common paths: m/44'/60'/0'/0/0 (Ethereum), m/86'/0'/0'/0/0 (Bitcoin Taproot)."
                .to_string(),
            KeyError::SigningError(_) => "Signing operation failed. Ensure: \
                 1) The key is valid and complete, 2) The message format is correct, \
                 3) The signing algorithm matches the key type (ECDSA vs EdDSA)."
                .to_string(),
        }
    }

    fn docs_url(&self) -> String {
        error_codes::docs_url(self.error_code())
    }

    fn fix_action(&self) -> Option<FixAction> {
        match self {
            KeyError::InvalidKeyFormat(_) => Some(FixAction::CheckState {
                url: "https://docs.csv.dev/wallet/key-formats".to_string(),
                what: "Verify key format matches BIP-39 or hex private key".to_string(),
            }),
            KeyError::DerivationError(_) => Some(FixAction::CheckState {
                url: "https://docs.csv.dev/wallet/derivation-paths".to_string(),
                what: "Verify correct BIP-32 derivation path for target chain".to_string(),
            }),
            KeyError::SigningError(_) => Some(FixAction::Retry {
                parameter_changes: std::collections::HashMap::from([(
                    "verify_key_type".to_string(),
                    "true".to_string(),
                )]),
            }),
        }
    }
}

/// Key manager handling multi-chain key operations.
pub struct KeyManager {
    /// Wallet seed (64 bytes from BIP-39) - zeroized on drop
    seed: [u8; 64],
    /// Optional native keystore for persistent key storage
    #[cfg(not(target_arch = "wasm32"))]
    keystore: Option<NativeKeystore>,
}

impl Drop for KeyManager {
    fn drop(&mut self) {
        self.seed.zeroize();
    }
}

impl KeyManager {
    /// Create a new key manager from a seed.
    pub fn new(seed: [u8; 64]) -> Self {
        Self {
            seed,
            #[cfg(not(target_arch = "wasm32"))]
            keystore: None,
        }
    }

    /// Create a new key manager from a seed with native keystore support.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn new_with_keystore(seed: [u8; 64]) -> Result<Self, KeyError> {
        Ok(Self {
            seed,
            keystore: Some(NativeKeystore::new().map_err(|e| {
                KeyError::DerivationError(format!("Failed to initialize keystore: {}", e))
            })?),
        })
    }

    /// Store a derived key in the native keystore.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn store_key_in_keystore(
        &mut self,
        key_id: &str,
        chain: &str,
        label: Option<&str>,
        secret_key: &[u8; 32],
        passphrase: &Passphrase,
    ) -> Result<(), KeyError> {
        let keystore = self.keystore.as_mut().ok_or_else(|| {
            KeyError::DerivationError("Native keystore not available".to_string())
        })?;

        let memory_key = MemorySecretKey::new(*secret_key);
        keystore
            .store_key(key_id, chain, label, &memory_key, passphrase)
            .map_err(|e| match e {
                NativeKeystoreError::Encryption(msg) => {
                    KeyError::DerivationError(format!("Encryption failed: {}", msg))
                }
                NativeKeystoreError::Filesystem(msg) => {
                    KeyError::DerivationError(format!("Filesystem error: {}", msg))
                }
                NativeKeystoreError::KeyNotFound(id) => KeyError::InvalidKeyFormat(id),
                _ => KeyError::DerivationError(format!("Keystore error: {}", e)),
            })
    }

    /// Retrieve a stored key from the native keystore.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn retrieve_key_from_keystore(
        &mut self,
        key_id: &str,
        passphrase: &Passphrase,
    ) -> Result<[u8; 32], KeyError> {
        let keystore = self.keystore.as_mut().ok_or_else(|| {
            KeyError::DerivationError("Native keystore not available".to_string())
        })?;

        let secret_key = keystore
            .retrieve_key(key_id, passphrase)
            .map_err(|e| match e {
                NativeKeystoreError::KeyNotFound(id) => KeyError::InvalidKeyFormat(id),
                NativeKeystoreError::PassphraseMismatch => {
                    KeyError::InvalidKeyFormat("Incorrect passphrase".to_string())
                }
                NativeKeystoreError::Encryption(msg) => {
                    KeyError::DerivationError(format!("Encryption error: {}", msg))
                }
                NativeKeystoreError::Filesystem(msg) => {
                    KeyError::DerivationError(format!("Filesystem error: {}", msg))
                }
                _ => KeyError::DerivationError(format!("Keystore error: {}", e)),
            })?;

        Ok(*secret_key.as_bytes())
    }

    /// Check if keystore is available.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn has_keystore(&self) -> bool {
        self.keystore.is_some()
    }

    /// List all stored key IDs in the keystore.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn list_keystore_keys(&self) -> Result<Vec<String>, KeyError> {
        let keystore = self.keystore.as_ref().ok_or_else(|| {
            KeyError::DerivationError("Native keystore not available".to_string())
        })?;

        Ok(keystore.list_keys())
    }

    /// Delete a key from the keystore.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn delete_key_from_keystore(&mut self, key_id: &str) -> Result<(), KeyError> {
        let keystore = self.keystore.as_mut().ok_or_else(|| {
            KeyError::DerivationError("Native keystore not available".to_string())
        })?;

        keystore
            .delete_key(key_id)
            .map_err(|e| KeyError::DerivationError(format!("Failed to delete key: {}", e)))
    }

    /// Derive Bitcoin Taproot key pair.
    pub fn derive_bitcoin_keys(&self) -> Result<(SecretKey, XOnlyPublicKey), KeyError> {
        let secp = Secp256k1::new();

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&self.seed[..32]);

        let secret_key = SecretKey::from_slice(&key_bytes)
            .map_err(|e| KeyError::DerivationError(format!("Invalid secret key: {}", e)))?;

        let _public_key = secret_key.public_key(&secp);
        let (x_only_pubkey, _) =
            XOnlyPublicKey::from_keypair(&Keypair::from_secret_key(&secp, &secret_key));

        Ok((secret_key, x_only_pubkey))
    }

    /// Derive Ethereum key pair.
    pub fn derive_ethereum_keys(&self) -> Result<(SecretKey, [u8; 20]), KeyError> {
        let secp = Secp256k1::new();

        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&self.seed[32..]);

        let secret_key = SecretKey::from_slice(&key_bytes)
            .map_err(|e| KeyError::DerivationError(format!("Invalid secret key: {}", e)))?;

        let public_key = secret_key.public_key(&secp);
        let pubkey_bytes = public_key.serialize_uncompressed();

        let mut hasher = Keccak256::new();
        hasher.update(&pubkey_bytes[1..]);
        let hash = hasher.finalize();

        let mut address = [0u8; 20];
        address.copy_from_slice(&hash[12..]);

        Ok((secret_key, address))
    }

    /// Derive Sui key pair (ed25519).
    pub fn derive_sui_keys(&self) -> Result<(SigningKey, VerifyingKey), KeyError> {
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&self.seed[..32]);

        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key: VerifyingKey = signing_key.verifying_key();

        Ok((signing_key, verifying_key))
    }

    /// Derive Aptos key pair (ed25519).
    pub fn derive_aptos_keys(&self) -> Result<(SigningKey, VerifyingKey), KeyError> {
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&self.seed[32..]);

        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key: VerifyingKey = signing_key.verifying_key();

        Ok((signing_key, verifying_key))
    }

    /// Derive Solana key pair (ed25519).
    pub fn derive_solana_keys(&self) -> Result<(SigningKey, VerifyingKey), KeyError> {
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&self.seed[16..48]);

        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key: VerifyingKey = signing_key.verifying_key();

        Ok((signing_key, verifying_key))
    }

    /// Sign a message with the appropriate key for the given chain.
    pub fn sign(&self, chain: &ChainId, message: &[u8; 32]) -> Result<Vec<u8>, KeyError> {
        match chain.as_str() {
            "ethereum" => self.sign_ethereum(message),
            "sui" => self.sign_ed25519(message, || self.derive_sui_keys().map(|(sk, _)| sk)),
            "aptos" => self.sign_ed25519(message, || self.derive_aptos_keys().map(|(sk, _)| sk)),
            "solana" => self.sign_ed25519(message, || self.derive_solana_keys().map(|(sk, _)| sk)),
            _ => self.sign_ethereum(message),
        }
    }

    /// Sign with Ethereum key (ECDSA).
    fn sign_ethereum(&self, message: &[u8; 32]) -> Result<Vec<u8>, KeyError> {
        let (secret_key, _) = self.derive_ethereum_keys()?;
        let secp = Secp256k1::new();

        let msg = secp256k1::Message::from_digest_slice(message)
            .map_err(|e| KeyError::SigningError(format!("Invalid message: {}", e)))?;

        let signature = secp.sign_ecdsa(&msg, &secret_key);
        Ok(signature.serialize_der().to_vec())
    }

    /// Sign with Ed25519 key.
    fn sign_ed25519<F>(&self, message: &[u8; 32], key_fn: F) -> Result<Vec<u8>, KeyError>
    where
        F: FnOnce() -> Result<SigningKey, KeyError>,
    {
        let signing_key = key_fn()?;
        let signature: ed25519_dalek::Signature = signing_key.sign(message);
        Ok(signature.to_bytes().to_vec())
    }

    /// Format address for display.
    pub fn format_address(&self, chain: &ChainId) -> Result<String, KeyError> {
        match chain.as_str() {
            "bitcoin" => {
                let (_, xonly_pubkey) = self.derive_bitcoin_keys()?;
                Ok(hex::encode(xonly_pubkey.serialize()))
            }
            "ethereum" => {
                let (_, address) = self.derive_ethereum_keys()?;
                Ok(format!("0x{}", hex::encode(address)))
            }
            "sui" => {
                let (_, verifying_key) = self.derive_sui_keys()?;
                let mut hasher = Blake2b::new();
                hasher.update([0x00]);
                hasher.update(verifying_key.as_bytes());
                let hash: [u8; 32] = hasher.finalize().into();
                Ok(format!("0x{}", hex::encode(&hash[..])))
            }
            "aptos" => {
                let (_, verifying_key) = self.derive_aptos_keys()?;
                let mut hasher = sha3::Sha3_256::new();
                hasher.update(verifying_key.as_bytes());
                hasher.update([0x00]);
                let hash: [u8; 32] = hasher.finalize().into();
                Ok(format!("0x{}", hex::encode(&hash[..])))
            }
            "solana" => {
                let (_, verifying_key) = self.derive_solana_keys()?;
                Ok(bs58::encode(verifying_key.as_bytes()).into_string())
            }
            _ => {
                let (_, address) = self.derive_ethereum_keys()?;
                Ok(format!("0x{}", hex::encode(address)))
            }
        }
    }
}
