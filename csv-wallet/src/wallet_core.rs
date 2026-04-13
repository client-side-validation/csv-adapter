//! Core wallet functionality.

use csv_adapter_core::Chain;
use bip32::Mnemonic;
use rand::RngCore;
use rand::rngs::OsRng;
use secp256k1::{Secp256k1, SecretKey};
use ed25519_dalek::{SigningKey, VerifyingKey};
use sha3::Keccak256;
use blake2::Blake2b;
use sha2::Digest;
use serde::{Serialize, Deserialize};

/// Extended wallet with multi-chain support.
#[derive(Clone, Serialize, Deserialize)]
pub struct ExtendedWallet {
    /// Mnemonic phrase
    pub mnemonic: String,
    /// Seed bytes
    #[serde(with = "seed_serde")]
    pub seed: [u8; 64],
}

// Custom serde for [u8; 64]
mod seed_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    
    pub fn serialize<S>(seed: &[u8; 64], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        seed.to_vec().serialize(serializer)
    }
    
    pub fn deserialize<'de, D>(deserializer: D) -> Result<[u8; 64], D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<u8> = Vec::deserialize(deserializer)?;
        if vec.len() != 64 {
            return Err(serde::de::Error::custom("Expected 64 bytes"));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&vec);
        Ok(arr)
    }
}

impl ExtendedWallet {
    /// Generate a new wallet.
    pub fn generate() -> Self {
        let mut entropy = [0u8; 32];
        OsRng.fill_bytes(&mut entropy);
        let mnemonic = Mnemonic::from_entropy(entropy, bip32::Language::English);
        let phrase = mnemonic.phrase().to_string();
        let seed = mnemonic.to_seed("");
        
        let mut seed_bytes = [0u8; 64];
        seed_bytes.copy_from_slice(seed.as_bytes());
        
        Self {
            mnemonic: phrase,
            seed: seed_bytes,
        }
    }

    /// Create from mnemonic phrase.
    pub fn from_mnemonic(phrase: &str) -> Result<Self, String> {
        let mnemonic = Mnemonic::new(phrase, bip32::Language::English)
            .map_err(|e| format!("Invalid mnemonic: {}", e))?;
        let seed = mnemonic.to_seed("");

        let mut seed_bytes = [0u8; 64];
        seed_bytes.copy_from_slice(seed.as_bytes());

        Ok(Self {
            mnemonic: phrase.to_string(),
            seed: seed_bytes,
        })
    }

    /// Create from hex-encoded private key.
    /// Supports secp256k1 (64 hex chars) or ed25519 (64 hex chars).
    pub fn from_private_key(hex_key: &str) -> Result<Self, String> {
        let hex_clean = hex_key.strip_prefix("0x").unwrap_or(hex_key);
        let bytes = hex::decode(hex_clean)
            .map_err(|e| format!("Invalid hex: {}", e))?;

        let seed = if bytes.len() == 32 {
            // 32-byte private key — use as seed directly
            let mut seed = [0u8; 64];
            // For secp256k1: use key as first 32 bytes, hash for remaining
            use sha2::{Sha256, Digest};
            seed[..32].copy_from_slice(&bytes);
            let hash = Sha256::digest(&bytes);
            seed[32..].copy_from_slice(&hash);
            seed
        } else if bytes.len() == 64 {
            // 64-byte key — use as seed directly
            let mut seed = [0u8; 64];
            seed.copy_from_slice(&bytes);
            seed
        } else {
            return Err(format!("Invalid key length: expected 32 or 64 bytes, got {}", bytes.len()));
        };

        Ok(Self {
            mnemonic: "[imported from private key]".to_string(),
            seed,
        })
    }

    /// Derive Bitcoin address.
    fn derive_bitcoin_address(&self) -> String {
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&self.seed[..32]);
        
        if let Ok(secret_key) = SecretKey::from_slice(&key_bytes) {
            let secp = Secp256k1::new();
            let pubkey = secret_key.public_key(&secp);
            let pubkey_bytes = pubkey.serialize();
            format!("bc1q{}", hex::encode(&pubkey_bytes[1..21]))
        } else {
            "Invalid key".to_string()
        }
    }

    /// Derive Ethereum address.
    fn derive_ethereum_address(&self) -> String {
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&self.seed[32..]);
        
        if let Ok(secret_key) = SecretKey::from_slice(&key_bytes) {
            let secp = Secp256k1::new();
            let public_key = secret_key.public_key(&secp);
            let pubkey_bytes = public_key.serialize_uncompressed();
            
            let mut hasher = Keccak256::new();
            hasher.update(&pubkey_bytes[1..]);
            let hash = hasher.finalize();
            
            format!("0x{}", hex::encode(&hash[12..]))
        } else {
            "Invalid key".to_string()
        }
    }

    /// Derive Sui address.
    fn derive_sui_address(&self) -> String {
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&self.seed[..32]);
        
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key: VerifyingKey = signing_key.verifying_key();
        
        let mut hasher = Blake2b::new();
        hasher.update(&[0x00]);
        hasher.update(verifying_key.as_bytes());
        let hash: [u8; 32] = hasher.finalize().into();
        
        format!("0x{}", hex::encode(&hash[..]))
    }

    /// Derive Aptos address.
    fn derive_aptos_address(&self) -> String {
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&self.seed[32..]);
        
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key: VerifyingKey = signing_key.verifying_key();
        
        let mut hasher = sha3::Sha3_256::new();
        hasher.update(verifying_key.as_bytes());
        hasher.update(&[0x00]);
        let hash: [u8; 32] = hasher.finalize().into();
        
        format!("0x{}", hex::encode(&hash[..]))
    }

    /// Get addresses for all chains.
    pub fn all_addresses(&self) -> Vec<(Chain, String)> {
        vec![
            (Chain::Bitcoin, self.derive_bitcoin_address()),
            (Chain::Ethereum, self.derive_ethereum_address()),
            (Chain::Sui, self.derive_sui_address()),
            (Chain::Aptos, self.derive_aptos_address()),
        ]
    }

    /// Get address for specific chain.
    pub fn address(&self, chain: Chain) -> String {
        self.all_addresses()
            .into_iter()
            .find(|(c, _)| *c == chain)
            .map(|(_, addr)| addr)
            .unwrap_or_default()
    }
}
