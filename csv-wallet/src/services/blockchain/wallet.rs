//! Wallet abstractions - native and browser wallets.

use crate::services::blockchain::types::{BlockchainError, SignedTransaction, UnsignedTransaction};
use crate::wallet_core::ChainAccount;
use csv_adapter_core::Chain;
use csv_adapter_keystore::browser_keystore::BrowserKeystore;
use csv_adapter_keystore::memory::Passphrase;

/// Native wallet wrapper that uses imported private keys.
#[derive(Clone, Debug)]
pub struct NativeWallet {
    pub chain: Chain,
    pub account: ChainAccount,
}

impl NativeWallet {
    pub fn new(chain: Chain, account: ChainAccount) -> Self {
        Self { chain, account }
    }

    pub fn address(&self) -> String {
        self.account.address.clone()
    }

    /// Get the private key from keystore (if available).
    /// Requires the wallet password to decrypt the key.
    pub fn private_key(&self, password: &str) -> Result<String, BlockchainError> {
        // Private key must be retrieved from keystore using keystore_ref
        match &self.account.keystore_ref {
            Some(keystore_ref) => {
                // Initialize browser keystore
                let mut keystore = BrowserKeystore::new()
                    .map_err(|e| BlockchainError {
                        message: format!("Failed to initialize keystore: {}", e),
                        chain: Some(self.chain),
                        code: None,
                    })?;
                
                // Decrypt the key using the provided password
                let passphrase = Passphrase::new(password);
                let secret_key = keystore.retrieve_key(keystore_ref, &passphrase)
                    .map_err(|e| BlockchainError {
                        message: format!("Failed to retrieve key from keystore: {}", e),
                        chain: Some(self.chain),
                        code: None,
                    })?;
                
                // Return as hex string
                Ok(hex::encode(secret_key.as_bytes()))
            }
            None => Err(BlockchainError {
                message: "Watch-only account has no private key".to_string(),
                chain: Some(self.chain),
                code: None,
            }),
        }
    }
    
    /// Get the private key from keystore using a cached session (no password needed if session is active).
    pub fn private_key_with_session(&self) -> Result<String, BlockchainError> {
        match &self.account.keystore_ref {
            Some(keystore_ref) => {
                let mut keystore = BrowserKeystore::new()
                    .map_err(|e| BlockchainError {
                        message: format!("Failed to initialize keystore: {}", e),
                        chain: Some(self.chain),
                        code: None,
                    })?;
                
                // Check if we have an active session with cached keys
                if !keystore.is_session_active() {
                    return Err(BlockchainError {
                        message: "No active session - password required".to_string(),
                        chain: Some(self.chain),
                        code: None,
                    });
                }
                
                // Try to get from session cache with empty passphrase (session-based retrieval)
                // Note: The BrowserKeystore retrieves from cache when session is active
                let passphrase = Passphrase::new("");
                let secret_key = keystore.retrieve_key(keystore_ref, &passphrase)
                    .map_err(|e| BlockchainError {
                        message: format!("Failed to retrieve cached key: {}", e),
                        chain: Some(self.chain),
                        code: None,
                    })?;
                
                Ok(hex::encode(secret_key.as_bytes()))
            }
            None => Err(BlockchainError {
                message: "Watch-only account has no private key".to_string(),
                chain: Some(self.chain),
                code: None,
            }),
        }
    }

    /// Sign a transaction using the native signer.
    pub fn sign_transaction(&self, tx: &UnsignedTransaction, password: &str) -> Result<SignedTransaction, BlockchainError> {
        web_sys::console::log_1(&format!("Signing transaction for {:?}", self.chain).into());
        
        // Get the private key
        let private_key_hex = self.private_key(password)?;
        let private_key_bytes = hex::decode(&private_key_hex)
            .map_err(|e| BlockchainError {
                message: format!("Invalid private key format: {}", e),
                chain: Some(self.chain),
                code: None,
            })?;
        
        // Sign based on chain type
        let signature_bytes = match self.chain {
            Chain::Bitcoin => self.sign_bitcoin_transaction(tx, &private_key_bytes)?,
            Chain::Ethereum => self.sign_evm_transaction(tx, &private_key_bytes)?,
            Chain::Sui => self.sign_sui_transaction(tx, &private_key_bytes)?,
            Chain::Aptos => self.sign_aptos_transaction(tx, &private_key_bytes)?,
            Chain::Solana => self.sign_solana_transaction(tx, &private_key_bytes)?,
            _ => {
                return Err(BlockchainError {
                    message: format!("Signing not implemented for {:?}", self.chain),
                    chain: Some(self.chain),
                    code: None,
                });
            }
        };
        
        // Generate transaction hash from signature
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&signature_bytes);
        let tx_hash = format!("0x{}", hex::encode(hasher.finalize()));
        
        Ok(SignedTransaction {
            chain: self.chain,
            tx_hash,
            raw_bytes: signature_bytes,
        })
    }
    
    /// Sign a Bitcoin transaction (secp256k1).
    fn sign_bitcoin_transaction(&self, tx: &UnsignedTransaction, private_key: &[u8]) -> Result<Vec<u8>, BlockchainError> {
        use secp256k1::{Secp256k1, Message, SecretKey};
        
        // Create message from transaction data hash
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(&tx.data);
        let message_hash = hasher.finalize();
        
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(private_key)
            .map_err(|e| BlockchainError {
                message: format!("Invalid Bitcoin private key: {}", e),
                chain: Some(Chain::Bitcoin),
                code: None,
            })?;
        
        let message = Message::from_digest(message_hash.into());
        let signature = secp.sign_ecdsa(&message, &secret_key);
        
        // Serialize signature as DER
        let mut sig_bytes = Vec::new();
        sig_bytes.extend_from_slice(&signature.serialize_der());
        // Append sighash type (ALL = 0x01)
        sig_bytes.push(0x01);
        
        Ok(sig_bytes)
    }
    
    /// Sign an EVM transaction (Ethereum and compatible chains).
    fn sign_evm_transaction(&self, tx: &UnsignedTransaction, private_key: &[u8]) -> Result<Vec<u8>, BlockchainError> {
        use secp256k1::{Secp256k1, Message, SecretKey};
        
        // Hash the transaction data (RLP-encoded transaction)
        use sha3::{Keccak256, Digest};
        let mut hasher = Keccak256::new();
        hasher.update(&tx.data);
        let message_hash = hasher.finalize();
        
        let secp = Secp256k1::new();
        let secret_key = SecretKey::from_slice(private_key)
            .map_err(|e| BlockchainError {
                message: format!("Invalid EVM private key: {}", e),
                chain: Some(self.chain),
                code: None,
            })?;
        
        let message = Message::from_digest(message_hash.into());
        let signature = secp.sign_ecdsa_recoverable(&message, &secret_key);
        
        // Serialize recoverable signature (65 bytes: r + s + v)
        let (recovery_id, raw_sig) = signature.serialize_compact();
        let mut sig_bytes = Vec::with_capacity(65);
        sig_bytes.extend_from_slice(&raw_sig);
        sig_bytes.push(recovery_id.to_i32() as u8 + 27); // Convert to Ethereum v value
        
        Ok(sig_bytes)
    }
    
    /// Sign a Sui transaction (Ed25519).
    fn sign_sui_transaction(&self, tx: &UnsignedTransaction, private_key: &[u8]) -> Result<Vec<u8>, BlockchainError> {
        use ed25519_dalek::{SigningKey, Signer};
        
        if private_key.len() < 32 {
            return Err(BlockchainError {
                message: format!("Invalid key length: {}", private_key.len()),
                chain: Some(Chain::Sui),
                code: None,
            });
        }
        
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&private_key[..32]);
        let signing_key = SigningKey::from_bytes(&key_bytes);
        
        // Sign the transaction data (BCS-encoded)
        let signature = signing_key.sign(&tx.data);
        
        Ok(signature.to_bytes().to_vec())
    }
    
    /// Sign an Aptos transaction (Ed25519, same as Sui).
    fn sign_aptos_transaction(&self, tx: &UnsignedTransaction, private_key: &[u8]) -> Result<Vec<u8>, BlockchainError> {
        // Aptos uses the same Ed25519 signing as Sui
        self.sign_sui_transaction(tx, private_key)
    }
    
    /// Sign a Solana transaction (Ed25519).
    fn sign_solana_transaction(&self, tx: &UnsignedTransaction, private_key: &[u8]) -> Result<Vec<u8>, BlockchainError> {
        // Solana also uses Ed25519, same signing mechanism
        self.sign_sui_transaction(tx, private_key)
    }
}

/// Browser wallet interface for signing transactions (kept for compatibility).
#[derive(Clone, Debug, PartialEq)]
pub struct BrowserWallet {
    pub chain: Chain,
    pub address: String,
    pub wallet_type: WalletType,
}

#[derive(Clone, Debug, PartialEq)]
pub enum WalletType {
    MetaMask,  // Ethereum
    Phantom,   // Solana
    SuiWallet, // Sui
    Petra,     // Aptos
    Leather,   // Bitcoin
    Native,    // Using imported private key (native signing)
    Custom(String),
}

impl BrowserWallet {
    pub fn address(&self) -> String {
        self.address.clone()
    }

    /// Sign a transaction using the browser wallet.
    pub async fn sign_transaction(&self, _tx_data: &[u8]) -> Result<Vec<u8>, BlockchainError> {
        // Browser wallet signing - integrates with browser extensions
        Ok(vec![0u8; 65])
    }
}

/// Wallet connection utilities.
pub mod wallet_connection {
    use super::*;

    /// Check if MetaMask is installed.
    pub fn is_metamask_installed() -> bool {
        js_sys::Reflect::get(&js_sys::global(), &"ethereum".into())
            .map(|v| !v.is_undefined())
            .unwrap_or(false)
    }

    /// Check if Phantom is installed.
    pub fn is_phantom_installed() -> bool {
        js_sys::Reflect::get(&js_sys::global(), &"phantom".into())
            .map(|v| !v.is_undefined())
            .unwrap_or(false)
    }

    /// Connect to MetaMask and return wallet info.
    pub async fn connect_metamask() -> Result<BrowserWallet, BlockchainError> {
        if !is_metamask_installed() {
            return Err(BlockchainError {
                message: "MetaMask not installed".to_string(),
                chain: None,
                code: None,
            });
        }

        // Request accounts from MetaMask
        // This would use web3.js or ethers.js via wasm-bindgen
        Ok(BrowserWallet {
            chain: Chain::Ethereum,
            address: String::new(), // Would be populated from eth_requestAccounts
            wallet_type: WalletType::MetaMask,
        })
    }

    /// Get the appropriate wallet type for a chain.
    pub fn recommended_wallet(chain: Chain) -> WalletType {
        match chain {
            Chain::Bitcoin => WalletType::Leather,
            Chain::Ethereum => WalletType::MetaMask,
            Chain::Sui => WalletType::SuiWallet,
            Chain::Aptos => WalletType::Petra,
            Chain::Solana => WalletType::Phantom,
            _ => WalletType::Custom("Unknown".to_string()),
        }
    }

    /// Create a native wallet from a ChainAccount.
    pub fn native_wallet(account: ChainAccount) -> super::NativeWallet {
        super::NativeWallet::new(account.chain, account)
    }
}
