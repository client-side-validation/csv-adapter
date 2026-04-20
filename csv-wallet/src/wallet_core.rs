//! Per-chain account management.
//!
//! Each account belongs to a specific chain and has its own private key.
//! Multiple accounts per chain are supported.

use csv_adapter_core::Chain;
use rand::RngCore;
use rand::rngs::OsRng;
use secp256k1::{Secp256k1, SecretKey};
use ed25519_dalek::{SigningKey, VerifyingKey};
use sha3::Keccak256;
use blake2::Blake2b;
use sha2::Digest;
use serde::{Serialize, Deserialize};
use uuid::Uuid;

/// A single blockchain account with its own private key and address.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ChainAccount {
    /// Unique account ID
    pub id: String,
    /// Blockchain this account belongs to
    pub chain: Chain,
    /// User-friendly account name
    pub name: String,
    /// Hex-encoded private key (32 or 64 bytes depending on curve)
    pub private_key: String,
    /// Derived address for display
    pub address: String,
}

impl ChainAccount {
    /// Derive address from private key for a specific chain.
    pub fn derive_address(chain: Chain, hex_key: &str) -> Result<String, String> {
        let hex_clean = hex_key.strip_prefix("0x").unwrap_or(hex_key);
        let bytes = hex::decode(hex_clean)
            .map_err(|e| format!("Invalid hex: {}", e))?;

        match chain {
            Chain::Bitcoin => Self::derive_bitcoin_address(&bytes),
            Chain::Ethereum => Self::derive_ethereum_address(&bytes),
            Chain::Sui => Self::derive_sui_address(&bytes),
            Chain::Aptos => Self::derive_aptos_address(&bytes),
            _ => Err(format!("Unsupported chain: {:?}", chain)),
        }
    }

    /// Create a new account from a private key.
    pub fn from_private_key(chain: Chain, name: &str, hex_key: &str) -> Result<Self, String> {
        let address = Self::derive_address(chain, hex_key)?;
        Ok(Self {
            id: Uuid::new_v4().to_string(),
            chain,
            name: name.to_string(),
            private_key: hex_key.to_string(),
            address,
        })
    }

    // ===== Address derivation per chain =====

    fn derive_bitcoin_address(key_bytes: &[u8]) -> Result<String, String> {
        let key = if key_bytes.len() == 32 {
            key_bytes.to_vec()
        } else if key_bytes.len() == 64 {
            key_bytes[..32].to_vec()
        } else {
            return Err(format!("Invalid key length: {}", key_bytes.len()));
        };

        if let Ok(secret_key) = SecretKey::from_slice(&key) {
            let secp = Secp256k1::new();
            let pubkey = secret_key.public_key(&secp);
            let pubkey_bytes = pubkey.serialize();
            Ok(format!("bc1q{}", hex::encode(&pubkey_bytes[1..21])))
        } else {
            Err("Invalid secp256k1 key for Bitcoin".to_string())
        }
    }

    fn derive_ethereum_address(key_bytes: &[u8]) -> Result<String, String> {
        let key = if key_bytes.len() == 32 {
            key_bytes.to_vec()
        } else if key_bytes.len() == 64 {
            key_bytes[..32].to_vec()
        } else {
            return Err(format!("Invalid key length: {}", key_bytes.len()));
        };

        if let Ok(secret_key) = SecretKey::from_slice(&key) {
            let secp = Secp256k1::new();
            let public_key = secret_key.public_key(&secp);
            let pubkey_bytes = public_key.serialize_uncompressed();

            let mut hasher = Keccak256::new();
            hasher.update(&pubkey_bytes[1..]);
            let hash = hasher.finalize();

            Ok(format!("0x{}", hex::encode(&hash[12..])))
        } else {
            Err("Invalid secp256k1 key for Ethereum".to_string())
        }
    }

    fn derive_sui_address(key_bytes: &[u8]) -> Result<String, String> {
        let key = if key_bytes.len() == 32 {
            key_bytes.to_vec()
        } else if key_bytes.len() == 64 {
            key_bytes[..32].to_vec()
        } else {
            return Err(format!("Invalid key length: {}", key_bytes.len()));
        };

        if key.len() == 32 {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&key);
            let signing_key = SigningKey::from_bytes(&arr);
            let verifying_key: VerifyingKey = signing_key.verifying_key();

            let mut hasher = Blake2b::new();
            hasher.update([0x00]);
            hasher.update(verifying_key.as_bytes());
            let hash: [u8; 32] = hasher.finalize().into();

            Ok(format!("0x{}", hex::encode(&hash[..])))
        } else {
            Err("Invalid ed25519 key for Sui".to_string())
        }
    }

    fn derive_aptos_address(key_bytes: &[u8]) -> Result<String, String> {
        let key = if key_bytes.len() == 32 {
            key_bytes.to_vec()
        } else if key_bytes.len() == 64 {
            key_bytes[..32].to_vec()
        } else {
            return Err(format!("Invalid key length: {}", key_bytes.len()));
        };

        if key.len() == 32 {
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&key);
            let signing_key = SigningKey::from_bytes(&arr);
            let verifying_key: VerifyingKey = signing_key.verifying_key();

            let mut hasher = sha3::Sha3_256::new();
            hasher.update(verifying_key.as_bytes());
            hasher.update([0x00]);
            let hash: [u8; 32] = hasher.finalize().into();

            Ok(format!("0x{}", hex::encode(&hash[..])))
        } else {
            Err("Invalid ed25519 key for Aptos".to_string())
        }
    }
}

/// Complete wallet data — collection of per-chain accounts.
#[derive(Clone, Serialize, Deserialize, Default)]
pub struct WalletData {
    /// All accounts (multiple per chain allowed)
    pub accounts: Vec<ChainAccount>,
    /// Last selected account ID
    pub selected_account_id: Option<String>,
}

impl WalletData {
    /// Add an account.
    pub fn add_account(&mut self, account: ChainAccount) {
        self.accounts.push(account);
    }

    /// Remove an account by ID.
    pub fn remove_account(&mut self, id: &str) -> bool {
        let len_before = self.accounts.len();
        self.accounts.retain(|a| a.id != id);
        self.accounts.len() < len_before
    }

    /// Get accounts for a specific chain.
    pub fn accounts_for_chain(&self, chain: Chain) -> Vec<&ChainAccount> {
        self.accounts.iter().filter(|a| a.chain == chain).collect()
    }

    /// Get accounts count for a chain.
    pub fn account_count_for_chain(&self, chain: Chain) -> usize {
        self.accounts.iter().filter(|a| a.chain == chain).count()
    }

    /// Get account by ID.
    pub fn get_account(&self, id: &str) -> Option<&ChainAccount> {
        self.accounts.iter().find(|a| a.id == id)
    }

    /// Get mutable account by ID.
    pub fn get_account_mut(&mut self, id: &str) -> Option<&mut ChainAccount> {
        self.accounts.iter_mut().find(|a| a.id == id)
    }

    /// Get total account count.
    pub fn total_accounts(&self) -> usize {
        self.accounts.len()
    }

    /// Check if wallet has any accounts.
    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }

    /// Select an account.
    pub fn select_account(&mut self, id: String) {
        self.selected_account_id = Some(id);
    }

    /// Clear selection.
    pub fn clear_selection(&mut self) {
        self.selected_account_id = None;
    }

    /// Export as JSON string.
    pub fn to_json(&self) -> Result<String, String> {
        serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize: {}", e))
    }

    /// Import from JSON string.
    pub fn from_json(json: &str) -> Result<Self, String> {
        serde_json::from_str(json)
            .map_err(|e| format!("Failed to parse JSON: {}", e))
    }

    /// Generate a random hex key for testing (32 bytes).
    pub fn generate_test_key() -> String {
        let mut bytes = [0u8; 32];
        OsRng.fill_bytes(&mut bytes);
        hex::encode(bytes)
    }
}

/// Helper: truncate address for display.
pub fn truncate_address(addr: &str, chars: usize) -> String {
    if addr.len() <= chars * 2 + 2 {
        addr.to_string()
    } else {
        format!("{}...{}", &addr[..chars + 2], &addr[addr.len() - chars..])
    }
}
