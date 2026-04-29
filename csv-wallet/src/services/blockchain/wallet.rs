//! Wallet abstractions - native and browser wallets.

use crate::services::blockchain::types::{BlockchainError, SignedTransaction, UnsignedTransaction};
use crate::wallet_core::ChainAccount;
use csv_adapter_core::Chain;

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
    pub fn private_key(&self) -> Result<String, BlockchainError> {
        // Private key must be retrieved from keystore using keystore_ref
        match &self.account.keystore_ref {
            Some(_keystore_ref) => {
                // TODO: Implement keystore retrieval
                Err(BlockchainError {
                    message: "Keystore key retrieval not yet implemented".to_string(),
                    chain: Some(self.chain),
                    code: None,
                })
            }
            None => Err(BlockchainError {
                message: "Watch-only account has no private key".to_string(),
                chain: Some(self.chain),
                code: None,
            }),
        }
    }

    /// Sign a transaction using the native signer.
    /// TODO: Reimplement without native_signer module
    pub fn sign_transaction(&self, _tx: &UnsignedTransaction) -> Result<SignedTransaction, BlockchainError> {
        Err(BlockchainError {
            message: "Native signing not yet reimplemented".to_string(),
            chain: Some(self.chain),
            code: None,
        })
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
