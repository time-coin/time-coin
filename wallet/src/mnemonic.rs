//! BIP-39 Mnemonic implementation for TIME Coin wallet
//!
//! This module provides industry-standard mnemonic phrase support for
//! deterministic key generation and wallet recovery.

use crate::keypair::{Keypair, KeypairError};
use bip39::{Language, Mnemonic};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum MnemonicError {
    #[error("Invalid mnemonic phrase: {0}")]
    InvalidMnemonic(String),

    #[error("Keypair generation error: {0}")]
    KeypairError(#[from] KeypairError),

    #[error("Invalid word count: {0} (must be 12, 15, 18, 21, or 24)")]
    InvalidWordCount(usize),
}

/// Generate a new random mnemonic phrase with the specified number of words.
///
/// # Arguments
/// * `word_count` - Number of words in the mnemonic (12, 15, 18, 21, or 24)
///
/// # Returns
/// * `Result<String, MnemonicError>` - The mnemonic phrase as a space-separated string
///
/// # Example
/// ```
/// use wallet::mnemonic::generate_mnemonic;
///
/// let mnemonic = generate_mnemonic(12).unwrap();
/// assert_eq!(mnemonic.split_whitespace().count(), 12);
/// ```
pub fn generate_mnemonic(word_count: usize) -> Result<String, MnemonicError> {
    // Validate word count
    if ![12, 15, 18, 21, 24].contains(&word_count) {
        return Err(MnemonicError::InvalidWordCount(word_count));
    }

    // Generate random mnemonic
    // Note: bip39 crate uses entropy bits: 12 words = 128 bits, 24 words = 256 bits
    let mnemonic = match word_count {
        12 => Mnemonic::generate(12).map_err(|e| MnemonicError::InvalidMnemonic(e.to_string()))?,
        15 => Mnemonic::generate(15).map_err(|e| MnemonicError::InvalidMnemonic(e.to_string()))?,
        18 => Mnemonic::generate(18).map_err(|e| MnemonicError::InvalidMnemonic(e.to_string()))?,
        21 => Mnemonic::generate(21).map_err(|e| MnemonicError::InvalidMnemonic(e.to_string()))?,
        24 => Mnemonic::generate(24).map_err(|e| MnemonicError::InvalidMnemonic(e.to_string()))?,
        _ => unreachable!(),
    };

    Ok(mnemonic.to_string())
}

/// Validate a mnemonic phrase
///
/// # Arguments
/// * `phrase` - The mnemonic phrase to validate
///
/// # Returns
/// * `Result<(), MnemonicError>` - Ok if valid, error otherwise
pub fn validate_mnemonic(phrase: &str) -> Result<(), MnemonicError> {
    Mnemonic::parse_in(Language::English, phrase)
        .map_err(|e| MnemonicError::InvalidMnemonic(e.to_string()))?;
    Ok(())
}

/// Derive a keypair from a mnemonic phrase
///
/// # Arguments
/// * `phrase` - The mnemonic phrase (space-separated words)
/// * `passphrase` - Optional passphrase for additional security (use "" for none)
///
/// # Returns
/// * `Result<Keypair, MnemonicError>` - The derived keypair
///
/// # Example
/// ```
/// use wallet::mnemonic::{generate_mnemonic, mnemonic_to_keypair};
///
/// let mnemonic = generate_mnemonic(12).unwrap();
/// let keypair = mnemonic_to_keypair(&mnemonic, "").unwrap();
/// ```
pub fn mnemonic_to_keypair(phrase: &str, passphrase: &str) -> Result<Keypair, MnemonicError> {
    // Parse mnemonic
    let mnemonic = Mnemonic::parse_in(Language::English, phrase)
        .map_err(|e| MnemonicError::InvalidMnemonic(e.to_string()))?;

    // Convert to seed (512 bits / 64 bytes)
    let seed = mnemonic.to_seed(passphrase);

    // For Ed25519, we need 32 bytes for the private key
    // We'll use the first 32 bytes of the seed, or hash it if we want deterministic derivation
    // Using SHA-256 to derive a 32-byte key from the 64-byte seed
    let mut hasher = Sha256::new();
    hasher.update(&seed[..]);
    let key_bytes = hasher.finalize();

    let mut key_array = [0u8; 32];
    key_array.copy_from_slice(&key_bytes[..32]);

    Keypair::from_bytes(&key_array).map_err(MnemonicError::KeypairError)
}

/// Mnemonic wrapper for convenience
#[derive(Debug, Clone)]
pub struct MnemonicPhrase {
    phrase: String,
}

impl MnemonicPhrase {
    /// Generate a new random mnemonic
    pub fn generate(word_count: usize) -> Result<Self, MnemonicError> {
        let phrase = generate_mnemonic(word_count)?;
        Ok(Self { phrase })
    }

    /// Create from an existing phrase
    pub fn from_phrase(phrase: &str) -> Result<Self, MnemonicError> {
        validate_mnemonic(phrase)?;
        Ok(Self {
            phrase: phrase.to_string(),
        })
    }

    /// Get the phrase as a string
    pub fn phrase(&self) -> &str {
        &self.phrase
    }

    /// Get word count
    pub fn word_count(&self) -> usize {
        self.phrase.split_whitespace().count()
    }

    /// Derive a keypair from this mnemonic
    pub fn to_keypair(&self, passphrase: &str) -> Result<Keypair, MnemonicError> {
        mnemonic_to_keypair(&self.phrase, passphrase)
    }
}

impl std::fmt::Display for MnemonicPhrase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.phrase)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_mnemonic_12_words() {
        let mnemonic = generate_mnemonic(12).unwrap();
        let words: Vec<&str> = mnemonic.split_whitespace().collect();
        assert_eq!(words.len(), 12);
    }

    #[test]
    fn test_generate_mnemonic_24_words() {
        let mnemonic = generate_mnemonic(24).unwrap();
        let words: Vec<&str> = mnemonic.split_whitespace().collect();
        assert_eq!(words.len(), 24);
    }

    #[test]
    fn test_invalid_word_count() {
        let result = generate_mnemonic(13);
        assert!(result.is_err());
        match result {
            Err(MnemonicError::InvalidWordCount(13)) => {}
            _ => panic!("Expected InvalidWordCount error"),
        }
    }

    #[test]
    fn test_validate_mnemonic() {
        let mnemonic = generate_mnemonic(12).unwrap();
        assert!(validate_mnemonic(&mnemonic).is_ok());
    }

    #[test]
    fn test_invalid_mnemonic() {
        let result =
            validate_mnemonic("invalid word word word word word word word word word word word");
        assert!(result.is_err());
    }

    #[test]
    fn test_mnemonic_to_keypair() {
        let mnemonic = generate_mnemonic(12).unwrap();
        let keypair = mnemonic_to_keypair(&mnemonic, "").unwrap();

        // Verify we can get public key
        let _public_key = keypair.public_key_bytes();
    }

    #[test]
    fn test_mnemonic_deterministic() {
        // Same mnemonic should produce same keypair
        let test_mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";

        let keypair1 = mnemonic_to_keypair(test_mnemonic, "").unwrap();
        let keypair2 = mnemonic_to_keypair(test_mnemonic, "").unwrap();

        assert_eq!(keypair1.public_key_bytes(), keypair2.public_key_bytes());
        assert_eq!(keypair1.secret_key_bytes(), keypair2.secret_key_bytes());
    }

    #[test]
    fn test_mnemonic_with_passphrase() {
        let mnemonic = generate_mnemonic(12).unwrap();

        // Different passphrases should produce different keypairs
        let keypair1 = mnemonic_to_keypair(&mnemonic, "").unwrap();
        let keypair2 = mnemonic_to_keypair(&mnemonic, "password").unwrap();

        assert_ne!(keypair1.public_key_bytes(), keypair2.public_key_bytes());
    }

    #[test]
    fn test_mnemonic_phrase_wrapper() {
        let phrase = MnemonicPhrase::generate(12).unwrap();
        assert_eq!(phrase.word_count(), 12);

        let keypair = phrase.to_keypair("").unwrap();
        let _public_key = keypair.public_key_bytes();
    }

    #[test]
    fn test_mnemonic_phrase_from_string() {
        let mnemonic_str = generate_mnemonic(12).unwrap();
        let phrase = MnemonicPhrase::from_phrase(&mnemonic_str).unwrap();
        assert_eq!(phrase.phrase(), mnemonic_str);
    }
}
