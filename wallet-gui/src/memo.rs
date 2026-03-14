//! Encrypted memo support for TIME Coin wallet transactions.
//!
//! Memos are encrypted using ECDH key exchange (X25519) + AES-256-GCM so that
//! only the sender and recipient can decrypt them. Wire format matches the
//! masternode implementation exactly.
//!
//! ## Wire format (`encrypted_memo` bytes)
//!
//! ```text
//! [0]       version byte (0x01)
//! [1..33]   sender's Ed25519 public key (32 bytes)
//! [33..65]  recipient's Ed25519 public key (32 bytes)
//! [65..77]  AES-GCM nonce (12 bytes)
//! [77..]    AES-GCM ciphertext + 16-byte auth tag
//! ```

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use ed25519_dalek::SigningKey;
use sha2::{Digest, Sha256};
use x25519_dalek::{PublicKey as X25519Public, StaticSecret as X25519Secret};

const MEMO_VERSION: u8 = 0x01;
const MEMO_MAX_LEN: usize = 256;

/// Encrypt a memo so that both the sender and the recipient can decrypt it.
///
/// # Arguments
/// - `sender_key` – The sender's Ed25519 signing key (BIP-44 derived wallet key).
/// - `recipient_ed25519_pubkey` – The recipient's Ed25519 verifying key (32 bytes).
///   For self-sends (consolidation), pass the sender's own verifying key bytes.
/// - `plaintext` – The human-readable memo string (max 256 bytes).
///
/// # Returns
/// The encrypted memo blob ready to store in `Transaction::encrypted_memo`.
pub fn encrypt_memo(
    sender_key: &SigningKey,
    recipient_ed25519_pubkey: &[u8; 32],
    plaintext: &str,
) -> Result<Vec<u8>, MemoError> {
    if plaintext.is_empty() {
        return Err(MemoError::Empty);
    }
    if plaintext.len() > MEMO_MAX_LEN {
        return Err(MemoError::TooLong(plaintext.len()));
    }

    let sender_x25519 = ed25519_to_x25519_secret(sender_key);
    let recipient_x25519 = ed25519_to_x25519_public(recipient_ed25519_pubkey);

    let shared_secret = sender_x25519.diffie_hellman(&recipient_x25519);
    let enc_key = derive_aes_key(shared_secret.as_bytes());

    let cipher = Aes256Gcm::new_from_slice(&enc_key)
        .map_err(|_| MemoError::Encryption("AES key init failed".into()))?;

    let nonce_bytes: [u8; 12] = rand::Rng::random(&mut rand::rng());
    let nonce = Nonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|_| MemoError::Encryption("AES-GCM encrypt failed".into()))?;

    // Assemble wire format: version + sender_pubkey + recipient_pubkey + nonce + ciphertext
    let sender_pubkey = sender_key.verifying_key().to_bytes();
    let mut blob = Vec::with_capacity(1 + 32 + 32 + 12 + ciphertext.len());
    blob.push(MEMO_VERSION);
    blob.extend_from_slice(&sender_pubkey);
    blob.extend_from_slice(recipient_ed25519_pubkey);
    blob.extend_from_slice(&nonce_bytes);
    blob.extend_from_slice(&ciphertext);

    Ok(blob)
}

/// Convert an Ed25519 signing key to an X25519 static secret.
fn ed25519_to_x25519_secret(ed_key: &SigningKey) -> X25519Secret {
    use sha2::Sha512;
    let hash = Sha512::digest(ed_key.to_bytes());
    let mut x25519_bytes = [0u8; 32];
    x25519_bytes.copy_from_slice(&hash[..32]);
    X25519Secret::from(x25519_bytes)
}

/// Convert an Ed25519 public key (32 bytes) to an X25519 public key.
fn ed25519_to_x25519_public(ed_pubkey: &[u8; 32]) -> X25519Public {
    let ed_point = curve25519_dalek::edwards::CompressedEdwardsY(*ed_pubkey);
    if let Some(point) = ed_point.decompress() {
        let montgomery = point.to_montgomery();
        X25519Public::from(montgomery.to_bytes())
    } else {
        // Fallback: hash-based derivation (should never happen with valid keys)
        let hash = Sha256::digest(ed_pubkey);
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(&hash[..32]);
        X25519Public::from(bytes)
    }
}

/// Derive a 256-bit AES key from the ECDH shared secret using domain separation.
fn derive_aes_key(shared_secret: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(shared_secret);
    hasher.update(b"TIME-memo-v1");
    hasher.finalize().into()
}

#[derive(Debug, thiserror::Error)]
pub enum MemoError {
    #[error("memo is empty")]
    Empty,
    #[error("memo too long ({0} bytes, max {MEMO_MAX_LEN})")]
    TooLong(usize),
    #[error("encryption error: {0}")]
    Encryption(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    fn random_signing_key() -> SigningKey {
        let mut bytes = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::rng(), &mut bytes);
        SigningKey::from_bytes(&bytes)
    }

    #[test]
    fn test_encrypt_produces_valid_blob() {
        let sender = random_signing_key();
        let recipient = random_signing_key();
        let recipient_pub = recipient.verifying_key().to_bytes();

        let blob = encrypt_memo(&sender, &recipient_pub, "Hello TIME!").unwrap();

        assert_eq!(blob[0], MEMO_VERSION);
        assert!(blob.len() >= 1 + 32 + 32 + 12 + 16); // min: header + tag
    }

    #[test]
    fn test_empty_memo_rejected() {
        let sender = random_signing_key();
        let recipient = random_signing_key();
        let recipient_pub = recipient.verifying_key().to_bytes();

        assert!(matches!(
            encrypt_memo(&sender, &recipient_pub, ""),
            Err(MemoError::Empty)
        ));
    }

    #[test]
    fn test_too_long_memo_rejected() {
        let sender = random_signing_key();
        let recipient = random_signing_key();
        let recipient_pub = recipient.verifying_key().to_bytes();
        let long = "x".repeat(257);

        assert!(matches!(
            encrypt_memo(&sender, &recipient_pub, &long),
            Err(MemoError::TooLong(257))
        ));
    }

    #[test]
    fn test_self_send_encrypt() {
        let key = random_signing_key();
        let pub_bytes = key.verifying_key().to_bytes();

        let blob = encrypt_memo(&key, &pub_bytes, "UTXO Consolidation").unwrap();
        assert_eq!(blob[0], MEMO_VERSION);
        // sender and recipient pubkeys should be identical
        assert_eq!(&blob[1..33], &blob[33..65]);
    }
}
