//! Multi-chain HD wallet.
//!
//! Extends csv_adapter_core with additional functionality for the UI application.

use csv_adapter_core::Chain;
use bip32::Mnemonic;
use serde::{Serialize, Deserialize};
use rand::RngCore;
use rand::rngs::OsRng;

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

/// Extended wallet with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedWallet {
    /// Wallet metadata
    pub metadata: WalletMetadata,
    /// Mnemonic phrase
    pub mnemonic: String,
    /// Seed bytes
    pub seed: [u8; 64],
    /// Whether the wallet is locked (encrypted)
    pub is_locked: bool,
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
        })
    }

    /// Get addresses for all chains.
    pub fn all_addresses(&self) -> Vec<(Chain, String)> {
        use secp256k1::{Secp256k1, SecretKey};
        use ed25519_dalek::SigningKey;
        use sha2::{Sha256, Digest};
        use sha3::Keccak256;
        use blake2::Blake2b;
        
        let mut addresses = Vec::new();
        
        // Bitcoin
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&self.seed[..32]);
        if let Ok(secret_key) = SecretKey::from_slice(&key_bytes) {
            let secp = Secp256k1::new();
            let pubkey = secret_key.public_key(&secp);
            addresses.push((Chain::Bitcoin, format!("bc1q{}", hex::encode(&pubkey.serialize()[1..21]))));
        }
        
        // Ethereum
        let mut key_bytes2 = [0u8; 32];
        key_bytes2.copy_from_slice(&self.seed[32..]);
        if let Ok(secret_key) = SecretKey::from_slice(&key_bytes2) {
            let secp = Secp256k1::new();
            let public_key = secret_key.public_key(&secp);
            let pubkey_bytes = public_key.serialize_uncompressed();
            let mut hasher = Keccak256::new();
            hasher.update(&pubkey_bytes[1..]);
            let hash = hasher.finalize();
            let mut address = [0u8; 20];
            address.copy_from_slice(&hash[12..]);
            addresses.push((Chain::Ethereum, format!("0x{}", hex::encode(address))));
        }
        
        // Sui
        let mut sui_key = [0u8; 32];
        sui_key.copy_from_slice(&self.seed[..32]);
        let sui_signing = SigningKey::from_bytes(&sui_key);
        let sui_verifying: ed25519_dalek::VerifyingKey = sui_signing.verifying_key();
        let mut hasher = Blake2b::new();
        hasher.update(&[0x00]);
        hasher.update(sui_verifying.as_bytes());
        let hash = hasher.finalize();
        addresses.push((Chain::Sui, format!("0x{}", hex::encode(&hash[..]))));
        
        // Aptos
        let mut aptos_key = [0u8; 32];
        aptos_key.copy_from_slice(&self.seed[32..]);
        let aptos_signing = SigningKey::from_bytes(&aptos_key);
        let aptos_verifying: ed25519_dalek::VerifyingKey = aptos_signing.verifying_key();
        let mut hasher = sha3::Sha3_256::new();
        hasher.update(aptos_verifying.as_bytes());
        hasher.update(&[0x00]);
        let hash = hasher.finalize();
        addresses.push((Chain::Aptos, format!("0x{}", hex::encode(&hash[..]))));
        
        addresses
    }

    /// Get address for a specific chain.
    pub fn address(&self, chain: Chain) -> String {
        let addresses = self.all_addresses();
        addresses.iter()
            .find(|(c, _)| *c == chain)
            .map(|(_, addr)| addr.clone())
            .unwrap_or_default()
    }
}

/// Generate a unique ID.
fn generate_uuid() -> String {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 16] = rng.gen();
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
