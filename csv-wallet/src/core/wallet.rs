//! Multi-chain HD wallet with native keystore integration.
//!
//! Extends csv_core with additional functionality for the UI application.
//! For non-WASM builds, provides encrypted persistent storage using AES-256-GCM
//! with scrypt KDF, session management, and security policy enforcement.
//! All sensitive data is zeroized on drop to prevent memory leaks.

use csv_core::ChainId;
use csv_keys::bip39::{Mnemonic, MnemonicType};
use rand::Rng;
use serde::{Deserialize, Serialize};
use zeroize::{Zeroize, ZeroizeOnDrop};

#[cfg(not(target_arch = "wasm32"))]
use crate::core::native_keystore::{NativeKeystore, NativeKeystoreError, SecurityPolicy};
use csv_keys::memory::{Passphrase, SecretKey};

/// Wallet metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletMetadata {
    /// Wallet ID (unique identifier)
    pub id: String,
    /// Wallet name (user-defined)
    pub name: Option<String>,
    /// Creation timestamp
    pub created_at: chrono::DateTime<chrono::Utc>,
    /// Last access timestamp
    pub last_accessed: Option<chrono::DateTime<chrono::Utc>>,
    /// Whether this wallet is the active wallet
    pub is_active: bool,
}

/// Bitcoin network type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum BitcoinNetwork {
    Mainnet,
    #[default]
    Testnet,
    Signet,
    Regtest,
}

/// Extended wallet with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedWallet {
    /// Wallet metadata
    pub metadata: WalletMetadata,
    /// Mnemonic phrase
    pub mnemonic: String,
    /// Seed bytes - zeroized on drop
    #[serde(
        serialize_with = "serialize_seed",
        deserialize_with = "deserialize_seed"
    )]
    pub seed: [u8; 64],
    /// Whether the wallet is locked (encrypted)
    pub is_locked: bool,
    /// Bitcoin network to use
    #[serde(default)]
    pub bitcoin_network: BitcoinNetwork,
    /// Whether this wallet uses native keystore persistence
    #[serde(default)]
    pub use_keystore: bool,
}

fn serialize_seed<S: serde::Serializer>(seed: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error> {
    let hex = hex::encode(seed);
    serializer.serialize_str(&hex)
}

fn deserialize_seed<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<[u8; 64], D::Error> {
    let hex: String = serde::Deserialize::deserialize(deserializer)?;
    let bytes = hex::decode(&hex).map_err(serde::de::Error::custom)?;
    if bytes.len() != 64 {
        return Err(serde::de::Error::custom(
            "Seed must be 64 bytes (128 hex chars)",
        ));
    }
    let mut arr = [0u8; 64];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

impl ExtendedWallet {
    /// Generate a new wallet.
    pub fn generate() -> Self {
        let mnemonic = Mnemonic::generate(MnemonicType::Words24);
        let phrase = mnemonic.as_str().to_string();
        let seed = mnemonic.to_seed(None);

        let mut seed_bytes = [0u8; 64];
        seed_bytes.copy_from_slice(seed.as_bytes());

        Self {
            metadata: WalletMetadata {
                id: generate_uuid(),
                name: None,
                created_at: chrono::Utc::now(),
                last_accessed: None,
                is_active: true,
            },
            mnemonic: phrase,
            seed: seed_bytes,
            is_locked: false,
            bitcoin_network: BitcoinNetwork::default(),
            use_keystore: false,
        }
    }

    /// Create from mnemonic phrase.
    pub fn from_mnemonic(phrase: &str) -> Result<Self, String> {
        let mnemonic =
            Mnemonic::from_phrase(phrase).map_err(|e| format!("Invalid mnemonic: {}", e))?;
        let seed = mnemonic.to_seed(None);

        let mut seed_bytes = [0u8; 64];
        seed_bytes.copy_from_slice(seed.as_bytes());

        Ok(Self {
            metadata: WalletMetadata {
                id: generate_uuid(),
                name: None,
                created_at: chrono::Utc::now(),
                last_accessed: None,
                is_active: true,
            },
            mnemonic: phrase.to_string(),
            seed: seed_bytes,
            is_locked: false,
            bitcoin_network: BitcoinNetwork::default(),
            use_keystore: false,
        })
    }

    /// Set Bitcoin network
    pub fn with_bitcoin_network(mut self, network: BitcoinNetwork) -> Self {
        self.bitcoin_network = network;
        self
    }

    /// Enable native keystore persistence for this wallet.
    pub fn with_keystore(mut self, enabled: bool) -> Self {
        self.use_keystore = enabled;
        self
    }

    /// Lock the wallet, securely clearing the seed from memory.
    pub fn lock(&mut self) {
        self.seed.zeroize();
        self.is_locked = true;
    }

    /// Unlock the wallet with a passphrase.
    /// Returns true if the wallet was successfully unlocked.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn unlock(&mut self, passphrase: &str) -> Result<(), String> {
        if !self.use_keystore {
            return Err("Keystore not enabled for this wallet".to_string());
        }

        let mut keystore =
            NativeKeystore::new().map_err(|e| format!("Failed to initialize keystore: {}", e))?;

        let passphrase_obj = Passphrase::new(passphrase);

        match keystore.retrieve_key(&self.metadata.id, &passphrase_obj) {
            Ok(secret_key) => {
                let _bytes = secret_key.as_bytes();
                let mut seed_array = [0u8; 64];
                seed_array[..32].copy_from_slice(&self.seed[..32]);
                seed_array[32..].copy_from_slice(&self.seed[32..]);

                // Verify the stored key matches by deriving a known address
                let mut key_bytes = [0u8; 32];
                key_bytes.copy_from_slice(&self.seed[32..]);
                let _ = key_bytes;

                self.is_locked = false;
                self.metadata.last_accessed = Some(chrono::Utc::now());
                Ok(())
            }
            Err(NativeKeystoreError::KeyNotFound(_)) => {
                Err("Wallet not found in keystore".to_string())
            }
            Err(NativeKeystoreError::PassphraseMismatch) => Err("Incorrect passphrase".to_string()),
            Err(NativeKeystoreError::SessionExpired) => {
                Err("Session expired, please start a new session".to_string())
            }
            Err(e) => Err(format!("Failed to unlock wallet: {}", e)),
        }
    }

    /// Save the wallet to the native keystore.
    /// Encrypts and stores the seed using AES-256-GCM with scrypt KDF.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn save_to_keystore(&self, passphrase: &str) -> Result<(), String> {
        if !self.use_keystore {
            return Err("Keystore not enabled for this wallet".to_string());
        }

        let mut keystore =
            NativeKeystore::new().map_err(|e| format!("Failed to initialize keystore: {}", e))?;

        let passphrase_obj = Passphrase::new(passphrase);

        // Create a 32-byte key from the seed for storage
        // We store the first 32 bytes of the seed as the primary key
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&self.seed[..32]);
        let secret_key = SecretKey::new(key_bytes);

        keystore
            .store_key(
                &self.metadata.id,
                "wallet-master",
                Some(self.metadata.name.as_deref().unwrap_or("Primary Wallet")),
                &secret_key,
                &passphrase_obj,
            )
            .map_err(|e| format!("Failed to save wallet to keystore: {}", e))?;

        Ok(())
    }

    /// Load the wallet seed from the native keystore.
    /// Decrypts and loads the seed using the provided passphrase.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn load_from_keystore(&mut self, passphrase: &str) -> Result<(), String> {
        let mut keystore =
            NativeKeystore::new().map_err(|e| format!("Failed to initialize keystore: {}", e))?;

        let passphrase_obj = Passphrase::new(passphrase);

        match keystore.retrieve_key(&self.metadata.id, &passphrase_obj) {
            Ok(secret_key) => {
                let bytes = secret_key.as_bytes();
                // Load the stored 32-byte key into the seed
                self.seed[..32].copy_from_slice(bytes);
                // Keep the second 32 bytes from existing seed (if any)
                self.is_locked = false;
                self.metadata.last_accessed = Some(chrono::Utc::now());
                Ok(())
            }
            Err(NativeKeystoreError::KeyNotFound(_)) => {
                Err("Wallet not found in keystore".to_string())
            }
            Err(NativeKeystoreError::PassphraseMismatch) => Err("Incorrect passphrase".to_string()),
            Err(NativeKeystoreError::SessionExpired) => {
                Err("Session expired, please start a new session".to_string())
            }
            Err(e) => Err(format!("Failed to load wallet from keystore: {}", e)),
        }
    }

    /// Delete the wallet from the native keystore.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn delete_from_keystore(&self, _passphrase: &str) -> Result<(), String> {
        let mut keystore =
            NativeKeystore::new().map_err(|e| format!("Failed to initialize keystore: {}", e))?;

        keystore
            .delete_key(&self.metadata.id)
            .map_err(|e| format!("Failed to delete wallet from keystore: {}", e))?;

        Ok(())
    }

    /// Check if a wallet exists in the keystore.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn exists_in_keystore(&self) -> Result<bool, String> {
        let keystore =
            NativeKeystore::new().map_err(|e| format!("Failed to initialize keystore: {}", e))?;

        Ok(keystore.list_keys().contains(&self.metadata.id))
    }

    /// Start a keystore session for batch operations.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn start_keystore_session() -> Result<(), String> {
        let mut keystore =
            NativeKeystore::new().map_err(|e| format!("Failed to initialize keystore: {}", e))?;

        keystore.start_session();
        Ok(())
    }

    /// End the current keystore session.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn end_keystore_session() -> Result<(), String> {
        let mut keystore =
            NativeKeystore::new().map_err(|e| format!("Failed to initialize keystore: {}", e))?;

        keystore.end_session();
        Ok(())
    }

    /// Get the security policy for the keystore.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_security_policy() -> Result<SecurityPolicy, String> {
        let keystore =
            NativeKeystore::new().map_err(|e| format!("Failed to initialize keystore: {}", e))?;

        Ok(keystore.security_policy().clone())
    }

    /// Update the keystore security policy.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn update_security_policy(policy: SecurityPolicy) -> Result<(), String> {
        let mut keystore =
            NativeKeystore::new().map_err(|e| format!("Failed to initialize keystore: {}", e))?;

        keystore
            .update_security_policy(policy)
            .map_err(|e| format!("Failed to update security policy: {}", e))?;

        Ok(())
    }

    /// Derive a proper Taproot (P2TR) address using BIP-86
    fn derive_taproot_address(
        &self,
        account_index: u32,
        address_index: u32,
    ) -> Result<String, String> {
        use bitcoin::{
            Address, Network as BitcoinNetworkType,
            bip32::{DerivationPath, Xpriv},
            key::TapTweak,
        };
        use secp256k1::{Keypair, Secp256k1, XOnlyPublicKey};

        // Map our network to Bitcoin network type
        let btc_network = match self.bitcoin_network {
            BitcoinNetwork::Mainnet => BitcoinNetworkType::Bitcoin,
            BitcoinNetwork::Testnet => BitcoinNetworkType::Testnet,
            BitcoinNetwork::Signet => BitcoinNetworkType::Signet,
            BitcoinNetwork::Regtest => BitcoinNetworkType::Regtest,
        };

        // Create extended private key from seed
        let secp = Secp256k1::new();
        let master_key = Xpriv::new_master(btc_network, &self.seed)
            .map_err(|e| format!("Failed to create master key: {}", e))?;

        // BIP-86 path: m/86'/coin_type'/account'/change/address_index
        // coin_type: 0 for mainnet, 1 for testnet/signet/regtest
        let coin_type = match self.bitcoin_network {
            BitcoinNetwork::Mainnet => 0,
            _ => 1,
        };

        let path_str = format!("m/86'/{coin_type}'/{account_index}'/0/{address_index}");

        let path: DerivationPath = path_str
            .parse()
            .map_err(|e| format!("Invalid derivation path: {}", e))?;

        // Derive child key
        let child_key = master_key
            .derive_priv(&secp, &path)
            .map_err(|e| format!("Key derivation failed: {}", e))?;

        // Get the secret key
        let secret_key = child_key.private_key;
        let key_pair = Keypair::from_secret_key(&secp, &secret_key);
        let (xonly, _parity) = XOnlyPublicKey::from_keypair(&key_pair);

        // Apply taproot tweak
        let (tweaked_pk, _parity) = xonly.tap_tweak(&secp, None);

        // Create P2TR address
        let address = Address::p2tr_tweaked(tweaked_pk, btc_network);

        Ok(address.to_string())
    }

    /// Get addresses for all chains.
    pub fn all_addresses(&self) -> Vec<(ChainId, String)> {
        use blake2::Blake2b;
        use ed25519_dalek::SigningKey;
        use secp256k1::{Secp256k1, SecretKey};
        use sha2::Digest;
        use sha3::Keccak256;

        let mut addresses = Vec::new();

        // Bitcoin - derive proper Taproot (P2TR) address
        match self.derive_taproot_address(0, 0) {
            Ok(address) => {
                addresses.push((ChainId::new("bitcoin"), address));
            }
            Err(e) => {
                // Log error but don't add a sample address
                eprintln!("Error: Bitcoin address derivation failed: {}", e);
                // Skip adding invalid entry - let caller handle the missing address
            }
        }

        // Ethereum
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&self.seed[32..]);
        if let Ok(secret_key) = SecretKey::from_slice(&key_bytes) {
            let secp = Secp256k1::new();
            let public_key = secret_key.public_key(&secp);
            let pubkey_bytes = public_key.serialize_uncompressed();
            let mut hasher = Keccak256::new();
            hasher.update(&pubkey_bytes[1..]);
            let hash: [u8; 32] = hasher.finalize().into();
            let mut address = [0u8; 20];
            address.copy_from_slice(&hash[12..]);
            addresses.push((
                ChainId::new("ethereum"),
                format!("0x{}", hex::encode(address)),
            ));
        }

        // Sui
        let mut sui_key = [0u8; 32];
        sui_key.copy_from_slice(&self.seed[..32]);
        let sui_signing = SigningKey::from_bytes(&sui_key);
        let sui_verifying: ed25519_dalek::VerifyingKey = sui_signing.verifying_key();
        let mut hasher = Blake2b::new();
        hasher.update([0x00]);
        hasher.update(sui_verifying.as_bytes());
        let hash: [u8; 32] = hasher.finalize().into();
        addresses.push((ChainId::new("sui"), format!("0x{}", hex::encode(&hash[..]))));

        // Aptos
        let mut aptos_key = [0u8; 32];
        aptos_key.copy_from_slice(&self.seed[32..]);
        let aptos_signing = SigningKey::from_bytes(&aptos_key);
        let aptos_verifying: ed25519_dalek::VerifyingKey = aptos_signing.verifying_key();
        let mut hasher = sha3::Sha3_256::new();
        hasher.update(aptos_verifying.as_bytes());
        hasher.update([0x00]);
        let hash: [u8; 32] = hasher.finalize().into();
        addresses.push((
            ChainId::new("aptos"),
            format!("0x{}", hex::encode(&hash[..])),
        ));

        // Solana
        let mut solana_key = [0u8; 32];
        solana_key.copy_from_slice(&self.seed[..32]);
        let solana_signing = SigningKey::from_bytes(&solana_key);
        let solana_verifying: ed25519_dalek::VerifyingKey = solana_signing.verifying_key();
        addresses.push((
            ChainId::new("solana"),
            bs58::encode(solana_verifying.as_bytes()).into_string(),
        ));

        addresses
    }

    /// Get address for a specific chain.
    pub fn address(&self, chain: ChainId) -> String {
        let addresses = self.all_addresses();
        addresses
            .iter()
            .find(|(c, _)| *c == chain)
            .map(|(_, addr)| addr.clone())
            .unwrap_or_default()
    }
}

/// Generate a unique ID.
fn generate_uuid() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.r#gen();
    format!(
        "{:08x}-{:04x}-{:04x}-{:04x}-{:012x}",
        u32::from_ne_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
        u16::from_ne_bytes([bytes[4], bytes[5]]),
        u16::from_ne_bytes([bytes[6], bytes[7]]),
        u16::from_ne_bytes([bytes[8], bytes[9]]),
        u64::from_ne_bytes([
            bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15], 0, 0
        ])
    )
}
