//! Per-chain account management.
//!
//! Each account belongs to a specific chain and uses secure keystore references.
//! Private keys are never stored in memory longer than necessary for signing.

use blake2::Blake2b;
use csv_core::Chain;
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use rand::RngCore;
use secp256k1::{Secp256k1, SecretKey};
use serde::{Deserialize, Serialize};
use sha2::Digest;
use sha3::Keccak256;
use uuid::Uuid;

/// A single blockchain account with keystore-secured private key.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ChainAccount {
    /// Unique account ID
    pub id: String,
    /// Blockchain this account belongs to
    pub chain: Chain,
    /// User-friendly account name
    pub name: String,
    /// Keystore reference (UUID) - points to encrypted key in browser storage
    /// Never store the actual private key here!
    pub keystore_ref: Option<String>,
    /// Derived address for display
    pub address: String,
    /// Balance in native token (BTC, ETH, SUI, APT, etc.)
    /// Stored as raw chain-native units (satoshis, wei, lamports, MIST, octas)
    /// Not serialized - fetched dynamically from blockchain
    #[serde(default, skip_serializing)]
    pub balance_raw: u64,
    /// BIP-44 derivation path (if HD wallet)
    pub derivation_path: Option<String>,
}

impl ChainAccount {
    /// Create a new account from an address (for watch-only accounts).
    pub fn watch_only(chain: Chain, name: &str, address: &str) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            chain,
            name: name.to_string(),
            keystore_ref: None,
            address: address.to_string(),
            balance_raw: 0,
            derivation_path: None,
        }
    }

    /// Check if this is a watch-only account (no keystore reference).
    pub fn is_watch_only(&self) -> bool {
        self.keystore_ref.is_none()
    }

    /// Create account from keystore reference (secure, no plaintext key).
    pub fn from_keystore(
        chain: Chain,
        name: &str,
        address: &str,
        keystore_ref: &str,
        derivation_path: Option<&str>,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            chain,
            name: name.to_string(),
            keystore_ref: Some(keystore_ref.to_string()),
            address: address.to_string(),
            balance_raw: 0,
            derivation_path: derivation_path.map(|s| s.to_string()),
        }
    }
}

impl ChainAccount {
    /// Derive address from private key for a specific chain (utility function).
    ///
    /// # Security Note
    /// This function accepts a hex-encoded key but should only be used during
    /// account creation. The resulting account will store a keystore reference,
    /// not the plaintext key.
    pub fn derive_address(chain: Chain, hex_key: &str) -> Result<String, String> {
        let hex_clean = hex_key.strip_prefix("0x").unwrap_or(hex_key);
        let bytes = hex::decode(hex_clean).map_err(|e| format!("Invalid hex: {}", e))?;

        match chain {
            Chain::Bitcoin => Self::derive_bitcoin_address(&bytes),
            Chain::Ethereum => Self::derive_ethereum_address(&bytes),
            Chain::Sui => Self::derive_sui_address(&bytes),
            Chain::Aptos => Self::derive_aptos_address(&bytes),
            Chain::Solana => Self::derive_solana_address(&bytes),
            _ => Err(format!("Unsupported chain: {:?}", chain)),
        }
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

        if key.len() == 32 {
            use bitcoin::{
                key::TapTweak,
                secp256k1::{Keypair, Secp256k1, SecretKey, XOnlyPublicKey},
                Address, Network,
            };

            let secret_key =
                SecretKey::from_slice(&key).map_err(|e| format!("Invalid secret key: {}", e))?;
            let secp = Secp256k1::new();
            let keypair = Keypair::from_secret_key(&secp, &secret_key);
            let (xonly_pubkey, _parity) = XOnlyPublicKey::from_keypair(&keypair);
            let (tweaked_pubkey, _parity) = xonly_pubkey.tap_tweak(&secp, None);
            let address = Address::p2tr_tweaked(tweaked_pubkey, Network::Testnet);
            Ok(address.to_string())
        } else {
            Err("Invalid key length for Bitcoin address derivation".to_string())
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

    fn derive_solana_address(key_bytes: &[u8]) -> Result<String, String> {
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

            Ok(bs58::encode(verifying_key.as_bytes()).into_string())
        } else {
            Err("Invalid ed25519 key for Solana".to_string())
        }
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
