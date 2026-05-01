//! Solana wallet implementation for CSV

use solana_sdk::{
    pubkey::Pubkey,
    signature::{Keypair, Signature, Signer},
    transaction::Transaction,
};

use crate::error::{SolanaError, SolanaResult};
use crate::types::SolanaAnchorRef;

/// Solana program wallet
pub struct ProgramWallet {
    /// Keypair
    pub keypair: Keypair,
    /// Anchor reference
    pub anchor_ref: Option<SolanaAnchorRef>,
}

impl ProgramWallet {
    /// Create new program wallet
    pub fn new() -> SolanaResult<Self> {
        let keypair = Keypair::new();
        Ok(Self {
            keypair,
            anchor_ref: None,
        })
    }

    /// Create from keypair
    pub fn from_keypair(keypair: Keypair) -> Self {
        Self {
            keypair,
            anchor_ref: None,
        }
    }

    /// Get public key
    pub fn pubkey(&self) -> Pubkey {
        self.keypair.pubkey()
    }

    /// Get anchor reference
    pub fn anchor_ref(&self) -> Option<&SolanaAnchorRef> {
        self.anchor_ref.as_ref()
    }

    /// Set anchor reference
    pub fn set_anchor_ref(&mut self, anchor_ref: SolanaAnchorRef) {
        self.anchor_ref = Some(anchor_ref);
    }

    /// Sign transaction
    pub fn sign_transaction(&self, transaction: &mut Transaction) -> SolanaResult<()> {
        // Use a default hash for now - would need actual recent blockhash
        use solana_sdk::hash::Hash;
        let default_hash = Hash::default();
        transaction.partial_sign(&[&self.keypair], default_hash);
        Ok(())
    }

    /// Sign message
    pub fn sign_message(&self, message: &[u8]) -> Signature {
        self.keypair.sign_message(message)
    }

    /// Verify signature
    pub fn verify_signature(&self, message: &[u8], signature: &Signature) -> bool {
        // Use the signature's verify method with pubkey bytes
        let pubkey_bytes = self.keypair.pubkey().to_bytes();
        signature.verify(&pubkey_bytes, message)
    }

    /// Verify data with signature bytes
    pub fn verify(&self, message: &[u8], sig_bytes: &[u8; 64]) -> bool {
        let signature = Signature::from(*sig_bytes);
        self.verify_signature(message, &signature)
    }

    /// Serialize keypair
    pub fn serialize_keypair(&self) -> SolanaResult<Vec<u8>> {
        Ok(self.keypair.to_bytes().to_vec())
    }

    /// Deserialize keypair
    pub fn deserialize_keypair(data: &[u8]) -> SolanaResult<Self> {
        if data.len() != 64 {
            return Err(SolanaError::Wallet(
                "Invalid keypair data length".to_string(),
            ));
        }

        // Take first 32 bytes as the secret key
        let secret_key: [u8; 32] = data[..32]
            .try_into()
            .map_err(|_| SolanaError::Wallet("Invalid secret key data".to_string()))?;

        let keypair = Keypair::new_from_array(secret_key);
        Ok(Self::from_keypair(keypair))
    }
}

/// Wallet error type
#[derive(Debug, thiserror::Error)]
pub enum WalletError {
    #[error("Key error: {0}")]
    KeyError(String),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Invalid signature")]
    InvalidSignature,
    #[error("Transaction error: {0}")]
    TransactionError(String),
}

impl From<WalletError> for SolanaError {
    fn from(err: WalletError) -> Self {
        SolanaError::Wallet(err.to_string())
    }
}
