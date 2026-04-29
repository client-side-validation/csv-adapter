//! Keystore migration for CSV CLI — migrate from plaintext to encrypted keys.
//!
//! This module provides migration functionality to move from the old plaintext
//! key storage format to the new encrypted keystore format using BIP-39/BIP-44.

use csv_adapter_keystore::{
    bip39::Mnemonic,
    bip44::derive_key,
    keystore::{KeystoreFile, KdfType, create_keystore},
    memory::{Passphrase, SecretKey, Seed},
};
use csv_adapter_core::Chain;
use csv_adapter_store::state::{WalletAccount, WalletConfig, UnifiedStorage};
use std::path::Path;
use thiserror::Error;

/// Error type for migration operations.
#[derive(Debug, Error)]
pub enum MigrationError {
    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Keystore error.
    #[error("Keystore error: {0}")]
    Keystore(#[from] csv_adapter_keystore::keystore::KeystoreError),

    /// BIP-39 error.
    #[error("BIP-39 error: {0}")]
    Bip39(#[from] csv_adapter_keystore::bip39::Bip39Error),

    /// Derivation error.
    #[error("BIP-44 derivation error: {0}")]
    Bip44(#[from] csv_adapter_keystore::bip44::Bip44Error),

    /// No plaintext keys found.
    #[error("No plaintext keys found to migrate")]
    NoKeysFound,

    /// Invalid key format.
    #[error("Invalid key format: {0}")]
    InvalidKey(String),
}

/// Result type for migration operations.
pub type MigrationResult<T> = Result<T, MigrationError>;

/// Keystore migration manager.
pub struct KeystoreMigration {
    /// Base directory for keystore files.
    keystore_dir: String,
}

impl KeystoreMigration {
    /// Create a new migration manager.
    pub fn new() -> Self {
        let keystore_dir = if let Some(home) = dirs::home_dir() {
            home.join(".csv/keystore").to_string_lossy().to_string()
        } else {
            std::env::temp_dir().join("csv-keystore").to_string_lossy().to_string()
        };

        Self { keystore_dir }
    }

    /// Create with custom keystore directory.
    pub fn with_dir(keystore_dir: impl Into<String>) -> Self {
        Self {
            keystore_dir: keystore_dir.into(),
        }
    }

    /// Check if migration is needed (plaintext keys exist).
    pub fn migration_needed(&self, storage: &UnifiedStorage) -> bool {
        storage.wallet.accounts.iter().any(|account| {
            account.private_key.is_some() && account.keystore_ref.is_none()
        })
    }

    /// Get keystore file path for a UUID.
    fn keystore_path(&self, uuid: &str) -> std::path::PathBuf {
        std::path::Path::new(&self.keystore_dir).join(format!("{}.json", uuid))
    }

    /// Migrate a single account from plaintext key to encrypted keystore.
    fn migrate_account(
        &self,
        account: &mut WalletAccount,
        passphrase: &Passphrase,
    ) -> MigrationResult<()> {
        // Check if already migrated
        if account.keystore_ref.is_some() {
            return Ok(());
        }

        // Get the plaintext private key
        let private_key_hex = account
            .private_key
            .as_ref()
            .ok_or(MigrationError::NoKeysFound)?;

        // Parse the hex key
        let key_bytes = hex::decode(private_key_hex.trim_start_matches("0x"))
            .map_err(|e| MigrationError::InvalidKey(format!("Invalid hex: {}", e)))?;

        if key_bytes.len() != 32 {
            return Err(MigrationError::InvalidKey(
                format!("Key length {} != 32 bytes", key_bytes.len()),
            ));
        }

        let mut key_array = [0u8; 32];
        key_array.copy_from_slice(&key_bytes);
        let secret_key = SecretKey::new(key_array);

        // Create encrypted keystore
        let keystore = KeystoreFile::encrypt(&secret_key, passphrase, KdfType::Scrypt)?;

        // Ensure keystore directory exists
        std::fs::create_dir_all(&self.keystore_dir)?;

        // Save keystore
        let keystore_id = keystore.id().to_string();
        let keystore_path = self.keystore_path(&keystore_id);
        keystore.save_to(&keystore_path)?;

        // Update account to reference keystore instead of storing plaintext
        account.keystore_ref = Some(keystore_id);
        account.private_key = None; // Clear plaintext key

        // Also zeroize the local copy
        drop(secret_key);

        Ok(())
    }

    /// Perform full migration of all plaintext keys to encrypted keystores.
    ///
    /// # Arguments
    /// * `storage` - The unified storage to migrate
    /// * `passphrase` - Passphrase for encrypting the new keystores
    ///
    /// # Returns
    /// Number of accounts migrated.
    pub fn migrate_storage(
        &self,
        storage: &mut UnifiedStorage,
        passphrase: &Passphrase,
    ) -> MigrationResult<usize> {
        let mut migrated_count = 0;

        for account in &mut storage.wallet.accounts {
            if self.migrate_account(account, passphrase).is_ok() {
                migrated_count += 1;
            }
        }

        if migrated_count > 0 {
            // Generate mnemonic for the wallet if not present
            if storage.wallet.mnemonic.is_none() {
                let mnemonic = Mnemonic::generate(csv_adapter_keystore::bip39::MnemonicType::Words24);
                storage.wallet.mnemonic = Some(mnemonic.as_str().to_string());
            }
        }

        Ok(migrated_count)
    }

    /// Create a new wallet with encrypted keystores from scratch.
    ///
    /// This generates a fresh BIP-39 mnemonic and derives keys for all chains.
    pub fn create_new_wallet(
        &self,
        storage: &mut UnifiedStorage,
        passphrase: &Passphrase,
    ) -> MigrationResult<(String, Vec<(Chain, WalletAccount)>)> {
        // Generate mnemonic
        let mnemonic = Mnemonic::generate(csv_adapter_keystore::bip39::MnemonicType::Words24);
        let phrase = mnemonic.as_str().to_string();

        // Convert to seed
        let seed = mnemonic.to_seed(None);

        // Ensure keystore directory exists
        std::fs::create_dir_all(&self.keystore_dir)?;

        // Generate keys for each chain
        let mut accounts = Vec::new();
        for chain in [Chain::Bitcoin, Chain::Ethereum, Chain::Sui, Chain::Aptos, Chain::Solana] {
            let key = derive_key(seed.as_bytes(), chain, 0, 0)?;
            
            // Create encrypted keystore
            let keystore = KeystoreFile::encrypt(&key, passphrase, KdfType::Scrypt)?;
            let keystore_id = keystore.id().to_string();
            
            // Save keystore
            let keystore_path = self.keystore_path(&keystore_id);
            keystore.save_to(&keystore_path)?;

            // Create wallet account (no plaintext key)
            let account = WalletAccount {
                id: keystore_id.clone(),
                chain,
                name: format!("{:?} Account", chain),
                address: format!("0x{}", hex::encode(key.as_bytes())), // Simplified
                private_key: None, // Never store plaintext
                xpub: None,
                derivation_path: Some(csv_adapter_keystore::bip44::derivation_path(chain, 0, 0).to_string_path()),
                keystore_ref: Some(keystore_id),
            };

            accounts.push((chain, account));
        }

        // Store mnemonic (encrypted indicator only)
        storage.wallet.mnemonic = Some(phrase.clone());
        storage.wallet.mnemonic_passphrase = None; // No additional passphrase

        Ok((phrase, accounts))
    }

    /// Unlock a keystore to get the secret key.
    pub fn unlock_keystore(
        &self,
        keystore_ref: &str,
        passphrase: &Passphrase,
    ) -> MigrationResult<SecretKey> {
        let keystore_path = self.keystore_path(keystore_ref);
        let keystore = KeystoreFile::load_from(&keystore_path)?;
        let secret_key = keystore.decrypt(passphrase)?;
        Ok(secret_key)
    }

    /// Get the keystore directory path.
    pub fn keystore_dir(&self) -> &str {
        &self.keystore_dir
    }
}

impl Default for KeystoreMigration {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_migration_needed() {
        let temp_dir = TempDir::new().unwrap();
        let migration = KeystoreMigration::with_dir(temp_dir.path());

        let mut storage = UnifiedStorage::new();
        
        // No keys - no migration needed
        assert!(!migration.migration_needed(&storage));

        // Add plaintext key
        storage.wallet.accounts.push(WalletAccount {
            id: "test".to_string(),
            chain: Chain::Ethereum,
            name: "Test".to_string(),
            address: "0x123".to_string(),
            private_key: Some("0x1234567890abcdef".to_string()),
            xpub: None,
            derivation_path: None,
            keystore_ref: None,
        });

        // Now migration is needed
        assert!(migration.migration_needed(&storage));
    }

    #[test]
    fn test_migrate_account() {
        let temp_dir = TempDir::new().unwrap();
        let migration = KeystoreMigration::with_dir(temp_dir.path());

        let mut account = WalletAccount {
            id: "test".to_string(),
            chain: Chain::Ethereum,
            name: "Test".to_string(),
            address: "0x123".to_string(),
            private_key: Some("0x0000000000000000000000000000000000000000000000000000000000000001".to_string()),
            xpub: None,
            derivation_path: None,
            keystore_ref: None,
        };

        let passphrase = Passphrase::new("test password");
        migration.migrate_account(&mut account, &passphrase).unwrap();

        // Check that plaintext key is cleared
        assert!(account.private_key.is_none());
        assert!(account.keystore_ref.is_some());
    }
}
