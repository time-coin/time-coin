//! TIME-MSG v1 message encryption and decryption.
//!
//! Implements the XChaCha20-Poly1305 + X25519 ECDH + HKDF-SHA-256 scheme
//! specified in TIME-MSG-PROTOCOL-SPEC.md, matching the masternode implementation
//! exactly so envelopes are interoperable.

use chacha20poly1305::{aead::Aead, KeyInit, XChaCha20Poly1305};
use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
use hkdf::Hkdf;
use rand_core::{OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use x25519_dalek::{EphemeralSecret, PublicKey as X25519PublicKey, StaticSecret};

pub const MSG_VERSION: u8 = 1;
pub const MAX_TTL_SECONDS: u32 = 60 * 60 * 24 * 30; // 30 days
#[allow(dead_code)]
pub const MAX_BODY_BYTES: usize = 65_535;
pub const DEFAULT_TTL_SECONDS: u32 = 60 * 60 * 24 * 7; // 7 days

/// Custom serde helper for [u8; 64] signatures.
mod serde_sig64 {
    use serde::{Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &[u8; 64], s: S) -> Result<S::Ok, S::Error> {
        s.serialize_bytes(v)
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<[u8; 64], D::Error> {
        struct Visitor;
        impl<'de> serde::de::Visitor<'de> for Visitor {
            type Value = [u8; 64];
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "64 bytes")
            }
            fn visit_bytes<E: serde::de::Error>(self, v: &[u8]) -> Result<[u8; 64], E> {
                v.try_into()
                    .map_err(|_| E::custom("expected 64 bytes for signature"))
            }
            fn visit_byte_buf<E: serde::de::Error>(self, v: Vec<u8>) -> Result<[u8; 64], E> {
                v.try_into()
                    .map_err(|_| E::custom("expected 64 bytes for signature"))
            }
            fn visit_seq<A: serde::de::SeqAccess<'de>>(
                self,
                mut seq: A,
            ) -> Result<[u8; 64], A::Error> {
                let mut arr = [0u8; 64];
                for (i, slot) in arr.iter_mut().enumerate() {
                    *slot = seq
                        .next_element()?
                        .ok_or_else(|| serde::de::Error::invalid_length(i, &"64 bytes"))?;
                }
                Ok(arr)
            }
        }
        d.deserialize_bytes(Visitor)
    }
}

/// Plaintext message (never stored or transmitted unencrypted).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeMessage {
    pub version: u8,
    pub sender_pubkey: [u8; 32],
    pub recipient_addr: String,
    pub timestamp: i64,
    pub ttl_seconds: u32,
    pub flags: u8,
    pub thread_id: Option<[u8; 32]>,
    pub subject: Vec<u8>,
    pub body: Vec<u8>,
}

impl TimeMessage {
    #[allow(dead_code)]
    pub fn request_read_receipt(&self) -> bool {
        self.flags & 0x01 != 0
    }
}

/// Wire envelope stored on relay nodes.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeEnvelope {
    pub version: u8,
    pub msg_id: [u8; 32],
    pub recipient_addr_hash: [u8; 32],
    pub sender_pubkey: [u8; 32],
    pub ephemeral_pubkey: [u8; 32],
    pub nonce: [u8; 24],
    pub ciphertext_payload: Vec<u8>,
    #[serde(with = "serde_sig64")]
    pub sender_sig: [u8; 64],
    pub created_at: i64,
    pub ttl_seconds: u32,
    pub flags: u8,
}

impl TimeEnvelope {
    pub fn serialise(&self) -> Result<Vec<u8>, String> {
        serde_cbor::to_vec(self).map_err(|e| e.to_string())
    }

    pub fn deserialise(bytes: &[u8]) -> Result<Self, String> {
        serde_cbor::from_slice(bytes).map_err(|e| e.to_string())
    }

    #[allow(dead_code)]
    pub fn expires_at(&self) -> i64 {
        self.created_at + self.ttl_seconds as i64
    }
}

/// Derive an X25519 static secret from an Ed25519 signing key.
pub fn ed25519_privkey_to_x25519(ed_key: &SigningKey) -> StaticSecret {
    let hash = Sha512::digest(ed_key.to_bytes());
    let mut scalar = [0u8; 32];
    scalar.copy_from_slice(&hash[..32]);
    scalar[0] &= 248;
    scalar[31] &= 127;
    scalar[31] |= 64;
    StaticSecret::from(scalar)
}

/// Convert an Ed25519 public key (bytes) to X25519 Montgomery form.
pub fn ed25519_pubkey_to_x25519(ed_pubkey: &[u8; 32]) -> [u8; 32] {
    let point = curve25519_dalek::edwards::CompressedEdwardsY(*ed_pubkey)
        .decompress()
        .unwrap_or_else(|| {
            let hash: [u8; 32] = Sha256::digest(ed_pubkey.as_slice()).into();
            let fallback = curve25519_dalek::edwards::CompressedEdwardsY(hash);
            fallback
                .decompress()
                .unwrap_or_default()
        });
    point.to_montgomery().to_bytes()
}

/// Encrypt a plaintext message into a `TimeEnvelope`.
///
/// `sender_key`       – wallet's Ed25519 signing key (index 0 by default)
/// `recipient_pubkey` – recipient's Ed25519 verifying key (32 bytes)
/// `recipient_addr`   – recipient's TIME1 address string (used for routing hash)
/// `body`             – plaintext UTF-8 message body
/// `subject`          – optional subject line
/// `ttl_seconds`      – time-to-live (default `DEFAULT_TTL_SECONDS`)
/// `flags`            – bit 0 = request read receipt, bit 1 = reply allowed
pub fn encrypt_envelope(
    sender_key: &SigningKey,
    recipient_pubkey: &[u8; 32],
    recipient_addr: &str,
    body: &str,
    subject: &str,
    ttl_seconds: u32,
    flags: u8,
) -> Result<TimeEnvelope, String> {
    // Build plaintext TimeMessage
    let plaintext = TimeMessage {
        version: MSG_VERSION,
        sender_pubkey: sender_key.verifying_key().to_bytes(),
        recipient_addr: recipient_addr.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        ttl_seconds: ttl_seconds.min(MAX_TTL_SECONDS),
        flags,
        thread_id: None,
        subject: subject.as_bytes().to_vec(),
        body: body.as_bytes().to_vec(),
    };
    let plaintext_bytes = serde_cbor::to_vec(&plaintext).map_err(|e| e.to_string())?;

    // Ephemeral X25519 keypair
    let ephemeral_secret = EphemeralSecret::random_from_rng(OsRng);
    let ephemeral_pubkey = X25519PublicKey::from(&ephemeral_secret);

    // Recipient X25519 pubkey
    let recipient_x25519 = X25519PublicKey::from(ed25519_pubkey_to_x25519(recipient_pubkey));

    // ECDH
    let shared = ephemeral_secret.diffie_hellman(&recipient_x25519);

    // Random 24-byte nonce
    let mut nonce_bytes = [0u8; 24];
    OsRng.fill_bytes(&mut nonce_bytes);

    // HKDF-SHA256: salt = nonce, info = "time-msg-v1"
    let hk = Hkdf::<Sha256>::new(Some(&nonce_bytes), shared.as_bytes());
    let mut sym_key = [0u8; 32];
    hk.expand(b"time-msg-v1", &mut sym_key)
        .map_err(|e| e.to_string())?;

    // XChaCha20-Poly1305 encrypt
    let cipher = XChaCha20Poly1305::new_from_slice(&sym_key).map_err(|e| e.to_string())?;
    let nonce = chacha20poly1305::XNonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext_bytes.as_ref())
        .map_err(|e| e.to_string())?;

    // msg_id = SHA-256(ciphertext)
    let msg_id: [u8; 32] = Sha256::digest(&ciphertext).into();

    // recipient_addr_hash = SHA-256(address bytes)
    let recipient_addr_hash: [u8; 32] = Sha256::digest(recipient_addr.as_bytes()).into();

    // Sign: msg_id || recipient_addr_hash || nonce || ciphertext
    let mut sig_input = Vec::with_capacity(32 + 32 + 24 + ciphertext.len());
    sig_input.extend_from_slice(&msg_id);
    sig_input.extend_from_slice(&recipient_addr_hash);
    sig_input.extend_from_slice(&nonce_bytes);
    sig_input.extend_from_slice(&ciphertext);
    let sig: [u8; 64] = sender_key.sign(&sig_input).to_bytes();

    Ok(TimeEnvelope {
        version: MSG_VERSION,
        msg_id,
        recipient_addr_hash,
        sender_pubkey: sender_key.verifying_key().to_bytes(),
        ephemeral_pubkey: ephemeral_pubkey.to_bytes(),
        nonce: nonce_bytes,
        ciphertext_payload: ciphertext,
        sender_sig: sig,
        created_at: chrono::Utc::now().timestamp(),
        ttl_seconds: ttl_seconds.min(MAX_TTL_SECONDS),
        flags,
    })
}

/// Decrypt a `TimeEnvelope` using the recipient's Ed25519 key.
/// Verifies the sender's signature before decrypting.
pub fn decrypt_envelope(
    recipient_key: &SigningKey,
    envelope: &TimeEnvelope,
) -> Result<TimeMessage, String> {
    // Verify sender signature
    let sender_vk = VerifyingKey::from_bytes(&envelope.sender_pubkey)
        .map_err(|_| "Invalid sender pubkey".to_string())?;
    let mut sig_input = Vec::new();
    sig_input.extend_from_slice(&envelope.msg_id);
    sig_input.extend_from_slice(&envelope.recipient_addr_hash);
    sig_input.extend_from_slice(&envelope.nonce);
    sig_input.extend_from_slice(&envelope.ciphertext_payload);
    let sig = ed25519_dalek::Signature::from_bytes(&envelope.sender_sig);
    sender_vk
        .verify(&sig_input, &sig)
        .map_err(|_| "Signature verification failed".to_string())?;

    // Verify msg_id integrity
    let computed_id: [u8; 32] = Sha256::digest(&envelope.ciphertext_payload).into();
    if computed_id != envelope.msg_id {
        return Err("msg_id integrity check failed".to_string());
    }

    // Recipient X25519 static secret
    let recipient_x25519 = ed25519_privkey_to_x25519(recipient_key);

    // ECDH with ephemeral pubkey
    let ephemeral_pub = X25519PublicKey::from(envelope.ephemeral_pubkey);
    let shared = recipient_x25519.diffie_hellman(&ephemeral_pub);

    // HKDF-SHA256: salt = nonce, info = "time-msg-v1"
    let hk = Hkdf::<Sha256>::new(Some(&envelope.nonce), shared.as_bytes());
    let mut sym_key = [0u8; 32];
    hk.expand(b"time-msg-v1", &mut sym_key)
        .map_err(|e| e.to_string())?;

    // Decrypt
    let cipher = XChaCha20Poly1305::new_from_slice(&sym_key).map_err(|e| e.to_string())?;
    let nonce = chacha20poly1305::XNonce::from_slice(&envelope.nonce);
    let plaintext = cipher
        .decrypt(nonce, envelope.ciphertext_payload.as_ref())
        .map_err(|_| "Decryption failed".to_string())?;

    serde_cbor::from_slice(&plaintext).map_err(|e| e.to_string())
}

/// Flag value for an unencrypted pubkey-request envelope.
pub const FLAG_PUBKEY_REQUEST: u8 = 0x04;

/// Build an unencrypted pubkey-request envelope addressed to `recipient_addr`.
///
/// We don't yet know the recipient's pubkey, so no ECDH / encryption is
/// performed.  The `ciphertext_payload` is just the sender's TIME address in
/// UTF-8 (so the recipient knows who to respond to).  The `sender_pubkey` field
/// already carries the sender's Ed25519 key, giving the recipient everything
/// they need to send encrypted replies once they register their own key.
pub fn create_pubkey_request(
    sender_key: &SigningKey,
    sender_address: &str,
    recipient_addr: &str,
) -> Result<TimeEnvelope, String> {
    let sender_pubkey = sender_key.verifying_key().to_bytes();
    let recipient_addr_hash: [u8; 32] = Sha256::digest(recipient_addr.as_bytes()).into();

    // Payload = sender's address in UTF-8 (plaintext — no encryption).
    let payload = sender_address.as_bytes().to_vec();
    let msg_id: [u8; 32] = Sha256::digest(&payload).into();

    // Nonce and ephemeral key are zeroed — there is no ECDH.
    let nonce = [0u8; 24];
    let ephemeral_pubkey = [0u8; 32];

    // Sign: msg_id ‖ recipient_addr_hash ‖ nonce ‖ payload
    let mut sig_input = Vec::with_capacity(32 + 32 + 24 + payload.len());
    sig_input.extend_from_slice(&msg_id);
    sig_input.extend_from_slice(&recipient_addr_hash);
    sig_input.extend_from_slice(&nonce);
    sig_input.extend_from_slice(&payload);
    let sender_sig = sender_key.sign(&sig_input).to_bytes();

    Ok(TimeEnvelope {
        version: MSG_VERSION,
        msg_id,
        recipient_addr_hash,
        sender_pubkey,
        ephemeral_pubkey,
        nonce,
        ciphertext_payload: payload,
        sender_sig,
        created_at: chrono::Utc::now().timestamp(),
        ttl_seconds: DEFAULT_TTL_SECONDS,
        flags: FLAG_PUBKEY_REQUEST,
    })
}

/// Try to extract the sender's TIME address from a pubkey-request envelope.
/// Returns `(sender_address, sender_pubkey_hex)`.
pub fn read_pubkey_request(envelope: &TimeEnvelope) -> Option<(String, String)> {
    if envelope.flags & FLAG_PUBKEY_REQUEST == 0 {
        return None;
    }
    let sender_address = std::str::from_utf8(&envelope.ciphertext_payload)
        .ok()?
        .to_string();
    let sender_pubkey_hex = hex::encode(envelope.sender_pubkey);
    Some((sender_address, sender_pubkey_hex))
}
