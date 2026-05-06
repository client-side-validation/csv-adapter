//! Wallet encryption utilities.
//!
//! Provides AES-256-GCM encryption for wallet storage.

use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce, Key,
};
use csv_core::agent_types::{HasErrorSuggestion, FixAction, error_codes};
use zeroize::Zeroize;

/// Error type for encryption operations.
#[derive(Debug, thiserror::Error)]
pub enum EncryptionError {
    /// Encryption failed
    #[error("Encryption failed: {0}")]
    EncryptionFailed(String),
    /// Decryption failed
    #[error("Decryption failed: {0}")]
    DecryptionFailed(String),
    /// Invalid password
    #[error("Invalid password")]
    InvalidPassword,
}

impl HasErrorSuggestion for EncryptionError {
    fn error_code(&self) -> &'static str {
        match self {
            EncryptionError::EncryptionFailed(_) => error_codes::WALLET_ENCRYPTION_FAILED,
            EncryptionError::DecryptionFailed(_) => error_codes::WALLET_DECRYPTION_FAILED,
            EncryptionError::InvalidPassword => error_codes::WALLET_INVALID_PASSWORD,
        }
    }

    fn description(&self) -> String {
        self.to_string()
    }

    fn suggested_fix(&self) -> String {
        match self {
            EncryptionError::EncryptionFailed(_) => {
                "Wallet encryption failed. This may indicate a system issue. \
                 Try: 1) Restarting the application, 2) Using a different password, \
                 3) Checking available system memory.".to_string()
            }
            EncryptionError::DecryptionFailed(_) => {
                "Wallet decryption failed. The data may be corrupted or the \
                 wrong encryption parameters were used. Ensure you have the \
                 correct wallet file and try again.".to_string()
            }
            EncryptionError::InvalidPassword => {
                "Invalid password. The password you entered does not match \
                 the one used to encrypt this wallet. Check for: \
                 1) Typos or extra spaces, 2) Caps Lock, 3) Different keyboard layout. \
                 Passwords cannot be recovered - ensure you have your mnemonic backed up.".to_string()
            }
        }
    }

    fn docs_url(&self) -> String {
        error_codes::docs_url(self.error_code())
    }

    fn fix_action(&self) -> Option<FixAction> {
        match self {
            EncryptionError::InvalidPassword => {
                Some(FixAction::CheckState {
                    url: "https://docs.csv.dev/wallet/recovery".to_string(),
                    what: "Verify password or recover from mnemonic".to_string(),
                })
            }
            EncryptionError::EncryptionFailed(_) | EncryptionError::DecryptionFailed(_) => {
                Some(FixAction::Retry {
                    parameter_changes: std::collections::HashMap::from([
                        ("verify_memory".to_string(), "true".to_string()),
                    ]),
                })
            }
        }
    }
}

/// Encrypted wallet data.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EncryptedWallet {
    /// Encrypted data (base64)
    pub ciphertext: String,
    /// Nonce (base64)
    pub nonce: String,
    /// Salt for key derivation (base64)
    pub salt: String,
}

/// Encrypt data with a password.
pub fn encrypt(data: &[u8], password: &str) -> Result<EncryptedWallet, EncryptionError> {
    // Generate random nonce
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    
    // Derive key from password with random salt
    let salt = Aes256Gcm::generate_nonce(&mut OsRng);
    let key = derive_key(password, &salt);
    
    let cipher = Aes256Gcm::new(&key);
    
    let ciphertext = cipher
        .encrypt(&nonce, data)
        .map_err(|e| EncryptionError::EncryptionFailed(format!("AES-GCM encryption: {}", e)))?;
    
    Ok(EncryptedWallet {
        ciphertext: base64_encode(&ciphertext),
        nonce: base64_encode(&nonce),
        salt: base64_encode(&salt),
    })
}

/// Decrypt data with a password.
pub fn decrypt(encrypted: &EncryptedWallet, password: &str) -> Result<Vec<u8>, EncryptionError> {
    let ciphertext = base64_decode(&encrypted.ciphertext)
        .map_err(|e| EncryptionError::DecryptionFailed(format!("Invalid ciphertext: {}", e)))?;
    let nonce = base64_decode(&encrypted.nonce)
        .map_err(|e| EncryptionError::DecryptionFailed(format!("Invalid nonce: {}", e)))?;
    let salt = base64_decode(&encrypted.salt)
        .map_err(|e| EncryptionError::DecryptionFailed(format!("Invalid salt: {}", e)))?;
    
    let key = derive_key(password, &salt);
    let cipher = Aes256Gcm::new(&key);
    
    let nonce = Nonce::from_slice(&nonce);
    
    cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| EncryptionError::InvalidPassword)
}

/// Derive AES key from password and salt using simple SHA-256 iteration.
/// In production, use argon2 or scrypt.
fn derive_key(password: &str, salt: &[u8]) -> Key<Aes256Gcm> {
    use sha2::{Sha256, Digest};
    
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    hasher.update(salt);
    let result = hasher.finalize();
    
    // For AES-256 we need 32 bytes
    let mut key = Key::<Aes256Gcm>::default();
    key.copy_from_slice(&result[..]);
    key
}

/// Base64 encode.
fn base64_encode(data: &[u8]) -> String {
    use std::fmt::Write;
    let mut encoded = String::new();
    for byte in data {
        let _ = write!(encoded, "{:02x}", byte);
    }
    encoded
}

/// Base64 decode.
fn base64_decode(s: &str) -> Result<Vec<u8>, String> {
    if s.len() % 2 != 0 {
        return Err("Invalid hex string".to_string());
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i+2], 16).map_err(|e| e.to_string()))
        .collect()
}

/// Securely clear sensitive data.
pub fn secure_clear(data: &mut [u8]) {
    data.zeroize();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let data = b"test wallet data";
        let password = "secure_password";
        
        let encrypted = encrypt(data, password).unwrap();
        let decrypted = decrypt(&encrypted, password).unwrap();
        
        assert_eq!(data.to_vec(), decrypted);
    }

    #[test]
    fn test_wrong_password() {
        let data = b"test wallet data";
        let password = "correct_password";
        let wrong_password = "wrong_password";
        
        let encrypted = encrypt(data, password).unwrap();
        let result = decrypt(&encrypted, wrong_password);
        
        assert!(result.is_err());
    }
}
