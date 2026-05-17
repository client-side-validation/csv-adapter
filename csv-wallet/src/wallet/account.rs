//! Per-chain account management.
//!
//! Each account belongs to a specific chain and uses secure keystore references.
//! Private keys are never stored in memory longer than necessary for signing.

use csv_keys::bip44::derive_address_from_chain_id;
use csv_core::ChainId;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single blockchain account with keystore-secured private key.
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct ChainAccount {
    /// Unique account ID
    pub id: String,
    /// Blockchain this account belongs to
    pub chain: ChainId,
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
    pub fn watch_only(chain: ChainId, name: &str, address: &str) -> Self {
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
        chain: ChainId,
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

    /// Derive address from private key for a specific chain (utility function).
    ///
    /// # Security Note
    /// This function accepts a hex-encoded key but should only be used during
    /// account creation. The resulting account will store a keystore reference,
    /// not the plaintext key.
    ///
    /// Uses csv-keys for canonical address derivation across all chains.
    pub fn derive_address(chain: ChainId, hex_key: &str) -> Result<String, String> {
        let hex_clean = hex_key.strip_prefix("0x").unwrap_or(hex_key);
        let bytes = hex::decode(hex_clean).map_err(|e| format!("Invalid hex: {}", e))?;
        if bytes.len() != 32 {
            return Err(format!("Private key must be 32 bytes, got {}", bytes.len()));
        }
        let bytes_arr: [u8; 32] = bytes
            .try_into()
            .map_err(|_| "Invalid key length".to_string())?;
        derive_address_from_chain_id(&bytes_arr, &chain)
            .map_err(|e| format!("Address derivation failed: {}", e))
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
