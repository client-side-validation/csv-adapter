//! Transaction signer for blockchain operations.
//!
//! This module delegates all signing operations to the csv-adapter facade,
//! which routes to the appropriate chain adapter implementing ChainSigner.
//!
//! Production Guarantee Plan compliant - no duplicate signing implementations.

use crate::services::blockchain::types::{BlockchainError, SignedTransaction, UnsignedTransaction};
use crate::services::blockchain::wallet::NativeWallet;
use csv_adapter::prelude::CsvClient;
use csv_adapter::StoreBackend;
use csv_adapter_core::Chain;

/// Transaction signer for different chains.
///
/// All signing operations are delegated to the csv-adapter facade,
/// which routes to chain adapters implementing the ChainSigner trait.
pub struct TransactionSigner {
    /// CSV adapter client for facade-based signing
    csv_client: Option<CsvClient>,
}

impl TransactionSigner {
    /// Create a new transaction signer.
    pub fn new() -> Self {
        Self { csv_client: None }
    }

    /// Sign a transaction for the specified chain using the facade.
    ///
    /// This delegates to the ChainSigner trait implementation in the
    /// appropriate chain adapter rather than performing local signing.
    pub async fn sign_transaction(
        &self,
        chain: Chain,
        tx_data: &UnsignedTransaction,
        signer: &NativeWallet,
    ) -> Result<SignedTransaction, BlockchainError> {
        web_sys::console::log_1(&format!("Signing transaction for {:?} via facade", chain).into());

        // Build CSV client with the requested chain
        let client = self.get_or_build_client(chain)?;

        // Get the chain facade which implements ChainSigner
        let facade = client.chain_facade();

        // Serialize transaction data for signing
        let tx_bytes = self.serialize_unsigned_tx(tx_data)?;

        // Get the signer's key identifier
        let key_id = signer.key_id()
            .map_err(|e| BlockchainError {
                message: format!("Failed to get signer key ID: {}", e),
                chain: Some(chain),
                code: Some(500),
            })?;

        // Delegate signing to the ChainSigner trait via the facade
        let signature = facade
            .sign_transaction(chain, &tx_bytes, &key_id)
            .await
            .map_err(|e| BlockchainError {
                message: format!("ChainSigner failed: {}", e),
                chain: Some(chain),
                code: Some(500),
            })?;

        // Build the signed transaction
        let tx_hash = self.compute_tx_hash(&signature);

        Ok(SignedTransaction {
            chain,
            raw_bytes: signature,
            tx_hash,
        })
    }

    /// Sign a Bitcoin anchor transaction via the facade.
    pub async fn sign_bitcoin_anchor(
        &self,
        unsigned_tx: &[u8],
        _private_key: &[u8],
        _utxo: &[u8],
        _address: &str,
    ) -> Result<Vec<u8>, BlockchainError> {
        web_sys::console::log_1(&"Signing Bitcoin anchor transaction via facade...".into());

        let client = self.get_or_build_client(Chain::Bitcoin)?;
        let facade = client.chain_facade();

        // Use a default key ID for now - should come from keystore
        let key_id = "default";

        facade
            .sign_transaction(Chain::Bitcoin, unsigned_tx, key_id)
            .await
            .map_err(|e| BlockchainError {
                message: format!("Bitcoin signing failed: {}", e),
                chain: Some(Chain::Bitcoin),
                code: Some(500),
            })
    }

    /// Sign an EVM-style transaction via the facade.
    pub async fn sign_evm_transaction(
        &self,
        tx_data: &UnsignedTransaction,
        signer: &NativeWallet,
    ) -> Result<SignedTransaction, BlockchainError> {
        web_sys::console::log_1(&"Signing EVM transaction via facade...".into());
        self.sign_transaction(Chain::Ethereum, tx_data, signer).await
    }

    /// Sign a Sui transaction via the facade.
    pub async fn sign_sui_transaction(
        &self,
        tx_bytes: &[u8],
        signer: &NativeWallet,
    ) -> Result<Vec<u8>, BlockchainError> {
        web_sys::console::log_1(&"Signing Sui transaction via facade...".into());

        let client = self.get_or_build_client(Chain::Sui)?;
        let facade = client.chain_facade();

        let key_id = signer.key_id()
            .map_err(|e| BlockchainError {
                message: format!("Failed to get key ID: {}", e),
                chain: Some(Chain::Sui),
                code: Some(500),
            })?;

        facade
            .sign_transaction(Chain::Sui, tx_bytes, &key_id)
            .await
            .map_err(|e| BlockchainError {
                message: format!("Sui signing failed: {}", e),
                chain: Some(Chain::Sui),
                code: Some(500),
            })
    }

    /// Sign an Aptos transaction via the facade.
    pub async fn sign_aptos_transaction(
        &self,
        tx_bytes: &[u8],
        signer: &NativeWallet,
    ) -> Result<Vec<u8>, BlockchainError> {
        web_sys::console::log_1(&"Signing Aptos transaction via facade...".into());

        let client = self.get_or_build_client(Chain::Aptos)?;
        let facade = client.chain_facade();

        let key_id = signer.key_id()
            .map_err(|e| BlockchainError {
                message: format!("Failed to get key ID: {}", e),
                chain: Some(Chain::Aptos),
                code: Some(500),
            })?;

        facade
            .sign_transaction(Chain::Aptos, tx_bytes, &key_id)
            .await
            .map_err(|e| BlockchainError {
                message: format!("Aptos signing failed: {}", e),
                chain: Some(Chain::Aptos),
                code: Some(500),
            })
    }

    /// Sign a Solana transaction via the facade.
    pub async fn sign_solana_transaction(
        &self,
        tx_bytes: &[u8],
        signer: &NativeWallet,
    ) -> Result<Vec<u8>, BlockchainError> {
        web_sys::console::log_1(&"Signing Solana transaction via facade...".into());

        let client = self.get_or_build_client(Chain::Solana)?;
        let facade = client.chain_facade();

        let key_id = signer.key_id()
            .map_err(|e| BlockchainError {
                message: format!("Failed to get key ID: {}", e),
                chain: Some(Chain::Solana),
                code: Some(500),
            })?;

        facade
            .sign_transaction(Chain::Solana, tx_bytes, &key_id)
            .await
            .map_err(|e| BlockchainError {
                message: format!("Solana signing failed: {}", e),
                chain: Some(Chain::Solana),
                code: Some(500),
            })
    }

    /// Get or build a CsvClient for the specified chain.
    fn get_or_build_client(&self, chain: Chain) -> Result<CsvClient, BlockchainError> {
        CsvClient::builder()
            .with_chain(chain)
            .with_store_backend(StoreBackend::InMemory)
            .build()
            .map_err(|e| BlockchainError {
                message: format!("Failed to build CSV client: {}", e),
                chain: Some(chain),
                code: Some(500),
            })
    }

    /// Serialize unsigned transaction to bytes.
    fn serialize_unsigned_tx(&self, tx: &UnsignedTransaction) -> Result<Vec<u8>, BlockchainError> {
        // Simple serialization - in production this would be proper RLP/BCS encoding
        // The chain adapter handles the actual serialization format
        let mut bytes = Vec::new();
        bytes.extend_from_slice(tx.from.as_bytes());
        bytes.extend_from_slice(tx.to.as_bytes());
        bytes.extend_from_slice(&tx.value.to_be_bytes());
        bytes.extend_from_slice(&tx.data);
        if let Some(nonce) = tx.nonce {
            bytes.extend_from_slice(&nonce.to_be_bytes());
        }
        Ok(bytes)
    }

    /// Compute transaction hash from signature.
    fn compute_tx_hash(&self, signature: &[u8]) -> String {
        // Simple hash computation - in production use proper hashing
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(signature);
        format!("0x{}", hex::encode(hasher.finalize()))
    }
}

impl Default for TransactionSigner {
    fn default() -> Self {
        Self::new()
    }
}
