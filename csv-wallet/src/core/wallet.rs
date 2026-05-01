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

/// Bitcoin network type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BitcoinNetwork {
    Mainnet,
    Testnet,
    Signet,
    Regtest,
}

impl Default for BitcoinNetwork {
    fn default() -> Self {
        BitcoinNetwork::Testnet
    }
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
    /// Bitcoin network to use
    #[serde(default)]
    pub bitcoin_network: BitcoinNetwork,
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
            bitcoin_network: BitcoinNetwork::default(),
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
            bitcoin_network: BitcoinNetwork::default(),
        })
    }

    /// Set Bitcoin network
    pub fn with_bitcoin_network(mut self, network: BitcoinNetwork) -> Self {
        self.bitcoin_network = network;
        self
    }

    /// Derive a proper Taproot (P2TR) address using BIP-86
    fn derive_taproot_address(&self, account_index: u32, address_index: u32) -> Result<String, String> {
        use secp256k1::{Secp256k1, KeyPair, XOnlyPublicKey};
        use bitcoin::{
            bip32::{DerivationPath, ExtendedPrivKey},
            Address, Network as BitcoinNetworkType,
            key::TapTweak,
        };

        // Map our network to Bitcoin network type
        let btc_network = match self.bitcoin_network {
            BitcoinNetwork::Mainnet => BitcoinNetworkType::Bitcoin,
            BitcoinNetwork::Testnet => BitcoinNetworkType::Testnet,
            BitcoinNetwork::Signet => BitcoinNetworkType::Signet,
            BitcoinNetwork::Regtest => BitcoinNetworkType::Regtest,
        };

        // Create extended private key from seed
        let secp = Secp256k1::new();
        let master_key = ExtendedPrivKey::new_master(btc_network, &self.seed)
            .map_err(|e| format!("Failed to create master key: {}", e))?;

        // BIP-86 path: m/86'/coin_type'/account'/change/address_index
        // coin_type: 0 for mainnet, 1 for testnet/signet/regtest
        let coin_type = match self.bitcoin_network {
            BitcoinNetwork::Mainnet => 0,
            _ => 1,
        };

        let path_str = format!(
            "m/86'/{coin_type}'/{account_index}'/0/{address_index}"
        );
        
        let path: DerivationPath = path_str
            .parse()
            .map_err(|e| format!("Invalid derivation path: {}", e))?;

        // Derive child key
        let child_key = master_key
            .derive_priv(&secp, &path)
            .map_err(|e| format!("Key derivation failed: {}", e))?;

        // Get the secret key
        let secret_key = child_key.private_key;
        let key_pair = KeyPair::from_secret_key(&secp, &secret_key);
        let (xonly, _parity) = XOnlyPublicKey::from_keypair(&key_pair);

        // Apply taproot tweak
        let (tweaked_pk, _parity) = xonly.tap_tweak(&secp, None);

        // Create P2TR address
        let address = Address::p2tr_tweaked(tweaked_pk, btc_network);

        Ok(address.to_string())
    }

    /// Get addresses for all chains.
    pub fn all_addresses(&self) -> Vec<(Chain, String)> {
        use secp256k1::{Secp256k1, SecretKey};
        use ed25519_dalek::SigningKey;
        use sha2::{Sha256, Digest};
        use sha3::Keccak256;
        use blake2::Blake2b;

        let mut addresses = Vec::new();

        // Bitcoin - derive proper Taproot (P2TR) address
        match self.derive_taproot_address(0, 0) {
            Ok(address) => {
                addresses.push((Chain::Bitcoin, address));
            }
            Err(e) => {
                // Log error but don't add a sample address
                eprintln!("Error: Bitcoin address derivation failed: {}", e);
                // Don't add invalid placeholder - let caller handle the missing address
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

        // Solana
        let mut solana_key = [0u8; 32];
        solana_key.copy_from_slice(&self.seed[..32]);
        let solana_signing = SigningKey::from_bytes(&solana_key);
        let solana_verifying: ed25519_dalek::VerifyingKey = solana_signing.verifying_key();
        addresses.push((Chain::Solana, bs58::encode(solana_verifying.as_bytes()).into_string()));

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
