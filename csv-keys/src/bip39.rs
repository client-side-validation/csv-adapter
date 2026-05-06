//! BIP-39 mnemonic phrase generation and recovery.
//!
//! This module provides secure mnemonic phrase handling for wallet backup
//! and recovery. Supports 12, 15, 18, 21, and 24 word mnemonics.

use crate::memory::Seed;
use std::str::FromStr;
use thiserror::Error;

/// Error type for BIP-39 operations.
#[derive(Debug, Error)]
pub enum Bip39Error {
    /// Invalid mnemonic phrase.
    #[error("Invalid mnemonic: {0}")]
    InvalidMnemonic(String),

    /// Invalid word count.
    #[error("Invalid word count: {0}. Must be 12, 15, 18, 21, or 24")]
    InvalidWordCount(usize),

    /// Checksum verification failed.
    #[error("Checksum verification failed")]
    ChecksumFailed,

    /// Internal error from bip39 crate.
    #[error("BIP-39 internal error: {0}")]
    Internal(String),
}

/// Type of mnemonic phrase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MnemonicType {
    /// 12 words (128 bits entropy).
    Words12,
    /// 15 words (160 bits entropy).
    Words15,
    /// 18 words (192 bits entropy).
    Words18,
    /// 21 words (224 bits entropy).
    Words21,
    /// 24 words (256 bits entropy).
    Words24,
}

impl MnemonicType {
    /// Get the number of words.
    pub fn word_count(&self) -> usize {
        match self {
            MnemonicType::Words12 => 12,
            MnemonicType::Words15 => 15,
            MnemonicType::Words18 => 18,
            MnemonicType::Words21 => 21,
            MnemonicType::Words24 => 24,
        }
    }

    /// Get the entropy size in bits.
    pub fn entropy_bits(&self) -> usize {
        match self {
            MnemonicType::Words12 => 128,
            MnemonicType::Words15 => 160,
            MnemonicType::Words18 => 192,
            MnemonicType::Words21 => 224,
            MnemonicType::Words24 => 256,
        }
    }

    /// Get the entropy size in bytes.
    pub fn entropy_bytes(&self) -> usize {
        self.entropy_bits() / 8
    }
}

impl Default for MnemonicType {
    fn default() -> Self {
        MnemonicType::Words24 // Most secure default
    }
}

/// A BIP-39 mnemonic phrase.
#[derive(Debug, Clone)]
pub struct Mnemonic {
    phrase: String,
    mnemonic_type: MnemonicType,
}

impl Mnemonic {
    /// Generate a new random mnemonic.
    ///
    /// # Example
    /// ```
    /// use csv_adapter_keystore::bip39::{Mnemonic, MnemonicType};
    ///
    /// let mnemonic = Mnemonic::generate(MnemonicType::Words12);
    /// assert_eq!(mnemonic.words().count(), 12);
    /// ```
    pub fn generate(mnemonic_type: MnemonicType) -> Self {
        let word_count = match mnemonic_type {
            MnemonicType::Words12 => 12,
            MnemonicType::Words15 => 15,
            MnemonicType::Words18 => 18,
            MnemonicType::Words21 => 21,
            MnemonicType::Words24 => 24,
        };

        let mnemonic = bip39::Mnemonic::generate_in(bip39::Language::English, word_count)
            .expect("Failed to generate mnemonic");
        let phrase = mnemonic.to_string();

        Self {
            phrase,
            mnemonic_type,
        }
    }

    /// Restore a mnemonic from a phrase string.
    ///
    /// # Arguments
    /// * `phrase` - The mnemonic phrase (space-separated words)
    ///
    /// # Errors
    /// Returns `Bip39Error::InvalidMnemonic` if the phrase is invalid.
    pub fn from_phrase(phrase: &str) -> Result<Self, Bip39Error> {
        let phrase = phrase.trim().to_lowercase();
        let word_count = phrase.split_whitespace().count();

        let mnemonic_type = match word_count {
            12 => MnemonicType::Words12,
            15 => MnemonicType::Words15,
            18 => MnemonicType::Words18,
            21 => MnemonicType::Words21,
            24 => MnemonicType::Words24,
            n => return Err(Bip39Error::InvalidWordCount(n)),
        };

        // Validate the mnemonic
        bip39::Mnemonic::from_str(&phrase)
            .map_err(|e| Bip39Error::InvalidMnemonic(e.to_string()))?;

        Ok(Self {
            phrase,
            mnemonic_type,
        })
    }

    /// Get the phrase as a string slice.
    pub fn as_str(&self) -> &str {
        &self.phrase
    }

    /// Get an iterator over the words.
    pub fn words(&self) -> impl Iterator<Item = &str> {
        self.phrase.split_whitespace()
    }

    /// Get the number of words.
    pub fn word_count(&self) -> usize {
        self.mnemonic_type.word_count()
    }

    /// Get the mnemonic type.
    pub fn mnemonic_type(&self) -> MnemonicType {
        self.mnemonic_type
    }

    /// Convert to a BIP-39 seed with optional passphrase.
    ///
    /// # Arguments
    /// * `passphrase` - Optional passphrase for additional security
    ///
    /// # Returns
    /// A 64-byte seed for HD wallet derivation.
    pub fn to_seed(&self, passphrase: Option<&str>) -> Seed {
        let bip39_mnemonic =
            bip39::Mnemonic::from_str(&self.phrase).expect("Valid mnemonic should parse");

        let seed_bytes = bip39_mnemonic.to_seed(passphrase.unwrap_or(""));
        Seed::new(seed_bytes)
    }

    /// Validate that the mnemonic phrase is correct.
    pub fn validate(&self) -> Result<(), Bip39Error> {
        bip39::Mnemonic::from_str(&self.phrase)
            .map_err(|e| Bip39Error::InvalidMnemonic(e.to_string()))?;
        Ok(())
    }
}

impl AsRef<str> for Mnemonic {
    fn as_ref(&self) -> &str {
        &self.phrase
    }
}

/// Generate a new mnemonic phrase with the default type (24 words).
pub fn generate_mnemonic() -> Mnemonic {
    Mnemonic::generate(MnemonicType::default())
}

/// Validate a mnemonic phrase without creating a `Mnemonic` object.
pub fn validate_mnemonic(phrase: &str) -> Result<MnemonicType, Bip39Error> {
    let mnemonic = Mnemonic::from_phrase(phrase)?;
    Ok(mnemonic.mnemonic_type())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_mnemonic() {
        let mnemonic = Mnemonic::generate(MnemonicType::Words12);
        assert_eq!(mnemonic.word_count(), 12);

        let mnemonic = Mnemonic::generate(MnemonicType::Words24);
        assert_eq!(mnemonic.word_count(), 24);
    }

    #[test]
    fn test_from_phrase() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let mnemonic = Mnemonic::from_phrase(phrase);
        assert!(mnemonic.is_ok());
        assert_eq!(mnemonic.unwrap().word_count(), 12);
    }

    #[test]
    fn test_invalid_word_count() {
        let phrase = "abandon abandon abandon"; // Only 3 words
        let result = Mnemonic::from_phrase(phrase);
        assert!(matches!(result, Err(Bip39Error::InvalidWordCount(3))));
    }

    #[test]
    fn test_to_seed() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let mnemonic = Mnemonic::from_phrase(phrase).unwrap();
        let seed = mnemonic.to_seed(None);
        assert_eq!(seed.as_bytes().len(), 64);
    }

    #[test]
    fn test_validate() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let mnemonic = Mnemonic::from_phrase(phrase).unwrap();
        assert!(mnemonic.validate().is_ok());
    }

    #[test]
    fn test_generate_mnemonic_function() {
        let mnemonic = generate_mnemonic();
        assert_eq!(mnemonic.word_count(), 24); // Default is 24 words
    }
}
