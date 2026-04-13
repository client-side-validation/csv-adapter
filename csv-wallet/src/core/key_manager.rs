//! Key manager for multi-chain wallet.
//!
//! Handles key derivation and signing operations for all supported chains.

use csv_adapter_core::Chain;
use secp256k1::{Secp256k1, SecretKey, XOnlyPublicKey};
use ed25519_dalek::{SigningKey, VerifyingKey};
use sha2::{Sha256, Digest};
use sha3::Keccak256;
use blake2::Blake2b;

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

/// Key manager handling multi-chain key operations.
pub struct KeyManager {
    /// Wallet seed (64 bytes from BIP-39)
    seed: [u8; 64],
}

impl KeyManager {
    /// Create a new key manager from a seed.
    pub fn new(seed: [u8; 64]) -> Self {
        Self { seed }
    }

    /// Derive Bitcoin Taproot key pair (BIP-86: m/86'/0'/0'/0/0).
    pub fn derive_bitcoin_keys(&self) -> Result<(SecretKey, XOnlyPublicKey), KeyError> {
        // Simplified derivation - in production use full BIP-86 path
        let secp = Secp256k1::new();
        
        // Use first 32 bytes of seed as private key
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&self.seed[..32]);
        
        let secret_key = SecretKey::from_slice(&key_bytes)
            .map_err(|e| KeyError::DerivationError(format!("Invalid secret key: {}", e)))?;
        
        let x_only_pubkey = XOnlyPublicKey::from_secret_key(&secp, &secret_key);
        
        Ok((secret_key, x_only_pubkey))
    }

    /// Derive Ethereum key pair (m/44'/60'/0'/0/0).
    pub fn derive_ethereum_keys(&self) -> Result<(SecretKey, [u8; 20]), KeyError> {
        let secp = Secp256k1::new();
        
        // Use different portion of seed for Ethereum
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&self.seed[32..]);
        
        let secret_key = SecretKey::from_slice(&key_bytes)
            .map_err(|e| KeyError::DerivationError(format!("Invalid secret key: {}", e)))?;
        
        // Derive address from public key
        let public_key = secret_key.public_key(&secp);
        let pubkey_bytes = public_key.serialize_uncompressed();
        
        // Keccak256 of uncompressed public key (without 0x04 prefix)
        let mut hasher = Keccak256::new();
        hasher.update(&pubkey_bytes[1..]);
        let hash = hasher.finalize();
        
        // Last 20 bytes
        let mut address = [0u8; 20];
        address.copy_from_slice(&hash[12..]);
        
        Ok((secret_key, address))
    }

    /// Derive Sui key pair (ed25519).
    pub fn derive_sui_keys(&self) -> Result<(SigningKey, VerifyingKey), KeyError> {
        // Use first 32 bytes for ed25519
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&self.seed[..32]);
        
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key: VerifyingKey = signing_key.verifying_key();
        
        Ok((signing_key, verifying_key))
    }

    /// Derive Aptos key pair (ed25519).
    pub fn derive_aptos_keys(&self) -> Result<(SigningKey, VerifyingKey), KeyError> {
        // Use different portion for Aptos
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&self.seed[32..]);
        
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key: VerifyingKey = signing_key.verifying_key();
        
        Ok((signing_key, verifying_key))
    }

    /// Sign a message with the appropriate key for the given chain.
    pub fn sign(&self, chain: Chain, message: &[u8; 32]) -> Result<Vec<u8>, KeyError> {
        match chain {
            Chain::Bitcoin => self.sign_bitcoin(message),
            Chain::Ethereum => self.sign_ethereum(message),
            Chain::Sui => self.sign_sui(message),
            Chain::Aptos => self.sign_aptos(message),
        }
    }

    /// Sign with Bitcoin key (Schnorr).
    fn sign_bitcoin(&self, message: &[u8; 32]) -> Result<Vec<u8>, KeyError> {
        let (secret_key, _) = self.derive_bitcoin_keys()?;
        let secp = Secp256k1::new();
        
        // In production, use BIP-340 Schnorr signing
        // For now, return ECDSA signature
        let msg = secp256k1::Message::from_digest_slice(message)
            .map_err(|e| KeyError::SigningError(format!("Invalid message: {}", e)))?;
        
        let signature = secp.sign_ecdsa(&msg, &secret_key);
        Ok(signature.serialize_der().to_vec())
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

    /// Sign with Sui key (Ed25519).
    fn sign_sui(&self, message: &[u8; 32]) -> Result<Vec<u8>, KeyError> {
        let (signing_key, _) = self.derive_sui_keys()?;
        let signature = signing_key.sign(message);
        Ok(signature.to_bytes().to_vec())
    }

    /// Sign with Aptos key (Ed25519).
    fn sign_aptos(&self, message: &[u8; 32]) -> Result<Vec<u8>, KeyError> {
        let (signing_key, _) = self.derive_aptos_keys()?;
        let signature = signing_key.sign(message);
        Ok(signature.to_bytes().to_vec())
    }

    /// Format address for display.
    pub fn format_address(&self, chain: Chain) -> Result<String, KeyError> {
        match chain {
            Chain::Bitcoin => {
                let (_, pubkey) = self.derive_bitcoin_keys()?;
                // Simplified - in production convert to bech32m
                Ok(format!("bc1q{}", hex::encode(&pubkey.serialize()[0..20])))
            }
            Chain::Ethereum => {
                let (_, address) = self.derive_ethereum_keys()?;
                Ok(format!("0x{}", hex::encode(address)))
            }
            Chain::Sui => {
                let (_, verifying_key) = self.derive_sui_keys()?;
                // Sui address: BLAKE2b-256(0x00 || public_key)
                let mut hasher = Blake2b::new();
                hasher.update(&[0x00]);
                hasher.update(verifying_key.as_bytes());
                let hash = hasher.finalize();
                Ok(format!("0x{}", hex::encode(&hash[..])))
            }
            Chain::Aptos => {
                let (_, verifying_key) = self.derive_aptos_keys()?;
                // Aptos address: SHA3-256(public_key || 0x00)
                let mut hasher = sha3::Sha3_256::new();
                hasher.update(verifying_key.as_bytes());
                hasher.update(&[0x00]);
                let hash = hasher.finalize();
                Ok(format!("0x{}", hex::encode(&hash[..])))
            }
        }
    }
}
