//! HD (Hierarchical Deterministic) wallet implementation.
//!
//! Handles mnemonic generation, seed derivation, and address generation
//! for multiple chains from a single seed.

use crate::wallet::metadata::WalletMetadata;
use csv_keys::bip39::{Mnemonic, MnemonicType};
use csv_core::ChainId;
use rand::Rng;
use serde::{Deserialize, Serialize};

// Re-export for convenience
pub use crate::wallet::metadata::BitcoinNetwork;

/// Extended wallet with metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtendedWallet {
    /// Wallet metadata
    pub metadata: WalletMetadata,
    /// Mnemonic phrase
    pub mnemonic: String,
    /// Seed bytes (serialized as hex)
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
}

fn serialize_seed<S: serde::Serializer>(seed: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
    let hex = hex::encode(seed);
    s.serialize_str(&hex)
}

fn deserialize_seed<'de, D: serde::Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
    use serde::de::Error;
    let hex = String::deserialize(d)?;
    hex::decode(&hex).map_err(D::Error::custom).map(|v| {
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&v);
        arr
    })
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
        })
    }

    /// Set Bitcoin network
    pub fn with_bitcoin_network(mut self, network: BitcoinNetwork) -> Self {
        self.bitcoin_network = network;
        self
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
        use secp256k1::Secp256k1;

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

        // Get the secret key and create XOnlyPublicKey via keypair
        let secret_key = child_key.private_key;
        let secret_key = bitcoin::secp256k1::SecretKey::from_slice(secret_key.as_ref())
            .map_err(|e| format!("Invalid secret key: {}", e))?;
        let keypair = bitcoin::secp256k1::Keypair::from_secret_key(&secp, &secret_key);
        let (xonly, _parity) = bitcoin::secp256k1::XOnlyPublicKey::from_keypair(&keypair);

        // Apply taproot tweak
        let (tweaked_pk, _parity) = xonly.tap_tweak(&secp, None);

        // Create P2TR address
        let address = Address::p2tr_tweaked(tweaked_pk, btc_network);

        Ok(address.to_string())
    }

    /// Get addresses for all chains.
    pub fn all_addresses(&self) -> Vec<(ChainId, String)> {
        use csv_keys::bip44::{derive_all_chain_keys, derive_address_from_key};

        let mut addresses = Vec::new();
        let keys = derive_all_chain_keys(&self.seed, 0);

        for (chain_id, key) in &keys {
            match derive_address_from_key(key.as_bytes(), chain_id) {
                Ok(address) => addresses.push((chain_id.clone(), address)),
                Err(e) => eprintln!("Error: Address derivation failed for {:?}: {}", chain_id, e),
            }
        }

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
