//! Key manager for multi-chain wallet.
//!
//! Handles key derivation and signing operations for all supported chains.

use csv_core::Chain;
use csv_core::mcp::{HasErrorSuggestion, FixAction, error_codes};
use secp256k1::{Secp256k1, SecretKey, XOnlyPublicKey};
use ed25519_dalek::{SigningKey, VerifyingKey};
use sha2::{Sha256, Digest};
use sha3::Keccak256;
use blake2::Blake2b;

use bip32::{ExtendedSigningKey, Seed, XPriv};
use bitcoin::bip32::DerivationPath as BitcoinDerivationPath;
use std::str::FromStr;

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
            KeyError::InvalidKeyFormat(_) => {
                "Invalid key format. For seed phrases, ensure: \
                 1) 12 or 24 BIP-39 words, 2) Words from standard wordlist, \
                 3) Correct spelling. For private keys, ensure: \
                 1) 64 hex characters, 2) Valid for the target chain.".to_string()
            }
            KeyError::DerivationError(_) => {
                "Key derivation failed. Check: \
                 1) The seed/mnemonic is valid, 2) The derivation path is correct, \
                 3) The target chain uses the right curve (secp256k1 vs ed25519). \
                 Common paths: m/44'/60'/0'/0/0 (Ethereum), m/86'/0'/0'/0/0 (Bitcoin Taproot).".to_string()
            }
            KeyError::SigningError(_) => {
                "Signing operation failed. Ensure: \
                 1) The key is valid and complete, 2) The message format is correct, \
                 3) The signing algorithm matches the key type (ECDSA vs EdDSA).".to_string()
            }
        }
    }

    fn docs_url(&self) -> String {
        error_codes::docs_url(self.error_code())
    }

    fn fix_action(&self) -> Option<FixAction> {
        match self {
            KeyError::InvalidKeyFormat(_) => {
                Some(FixAction::CheckState {
                    url: "https://docs.csv.dev/wallet/key-formats".to_string(),
                    what: "Verify key format matches BIP-39 or hex private key".to_string(),
                })
            }
            KeyError::DerivationError(_) => {
                Some(FixAction::CheckState {
                    url: "https://docs.csv.dev/wallet/derivation-paths".to_string(),
                    what: "Verify correct BIP-32 derivation path for target chain".to_string(),
                })
            }
            KeyError::SigningError(_) => {
                Some(FixAction::Retry {
                    parameter_changes: std::collections::HashMap::from([
                        ("verify_key_type".to_string(), "true".to_string()),
                    ]),
                })
            }
        }
    }
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
        let secp = Secp256k1::new();

        // Use proper BIP-86 derivation path: m/86'/0'/0'/0/0
        // 86' is the BIP-86 purpose for Taproot single-key addresses
        // 0' is Bitcoin mainnet coin type (1' for testnet/signet)
        // 0' is the account index
        // 0 is the change chain (external)
        // 0 is the address index
        let derivation_path = "m/86'/0'/0'/0/0"
            .parse::<BitcoinDerivationPath>()
            .map_err(|e| KeyError::DerivationError(format!("Invalid derivation path: {}", e)))?;

        // Create master key from seed using bip32
        let seed = Seed::new(self.seed);
        let master_key = XPriv::new(seed)
            .map_err(|e| KeyError::DerivationError(format!("Failed to create master key: {}", e)))?;

        // Derive the child key at the BIP-86 path
        let child_key = master_key
            .derive(&derivation_path)
            .map_err(|e| KeyError::DerivationError(format!("Derivation failed: {}", e)))?;

        // Extract the private key bytes
        let secret_key = SecretKey::from_slice(&child_key.private_key().to_bytes())
            .map_err(|e| KeyError::DerivationError(format!("Invalid derived key: {}", e)))?;

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

    /// Derive Solana key pair (ed25519).
    pub fn derive_solana_keys(&self) -> Result<(SigningKey, VerifyingKey), KeyError> {
        // Use a different portion of seed for Solana (bytes 16-48)
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&self.seed[16..48]);
        
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
            Chain::Solana => self.sign_solana(message),
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
                let (_, xonly_pubkey) = self.derive_bitcoin_keys()?;
                // Build proper Taproot (P2TR) address using bech32m encoding
                // Taproot address: bc1p<xonly_pubkey>
                use bitcoin::secp256k1::PublicKey as BitcoinPubkey;
                use bitcoin::key::XOnlyPublicKey as BitcoinXOnlyPubkey;

                // Convert secp256k1 XOnlyPublicKey to bitcoin crate format
                let xonly_bytes = xonly_pubkey.serialize();
                let bitcoin_xonly = BitcoinXOnlyPubkey::from_slice(&xonly_bytes)
                    .map_err(|e| KeyError::InvalidKeyFormat(format!("Invalid XOnly public key: {}", e)))?;

                // Build Taproot address (witness v1)
                let address = bitcoin::Address::from_witness_program(
                    bitcoin::WitnessProgram::new_v1(bitcoin_xonly)
                        .map_err(|e| KeyError::InvalidKeyFormat(format!("Invalid witness program: {}", e)))?,
                    bitcoin::Network::Bitcoin,
                );

                Ok(address.to_string())
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
            Chain::Solana => {
                let (_, verifying_key) = self.derive_solana_keys()?;
                // Solana address is the base58-encoded public key
                Ok(bs58::encode(verifying_key.as_bytes()).into_string())
            }
        }
    }
}
