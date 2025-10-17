//! TIME Coin Cryptography
//! 
//! Signature generation and verification

use ed25519_dalek::{Signer, Verifier, SigningKey, VerifyingKey, Signature};
use sha2::{Sha256, Digest};
use rand::rngs::OsRng;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Invalid signature")]
    InvalidSignature,
    
    #[error("Invalid public key")]
    InvalidPublicKey,
    
    #[error("Invalid private key")]
    InvalidPrivateKey,
}

/// Key pair for signing transactions
#[derive(Clone)]
pub struct KeyPair {
    signing_key: SigningKey,
    verifying_key: VerifyingKey,
}

impl KeyPair {
    /// Generate new random keypair
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key = signing_key.verifying_key();
        
        Self {
            signing_key,
            verifying_key,
        }
    }
    
    /// Get public key as hex string (address)
    pub fn public_key_hex(&self) -> String {
        hex::encode(self.verifying_key.as_bytes())
    }
    
    /// Get private key as hex string
    pub fn private_key_hex(&self) -> String {
        hex::encode(self.signing_key.to_bytes())
    }
    
    /// Create keypair from private key hex
    pub fn from_private_key_hex(hex_str: &str) -> Result<Self, CryptoError> {
        let bytes = hex::decode(hex_str)
            .map_err(|_| CryptoError::InvalidPrivateKey)?;
        
        let key_bytes: [u8; 32] = bytes.try_into()
            .map_err(|_| CryptoError::InvalidPrivateKey)?;
        
        let signing_key = SigningKey::from_bytes(&key_bytes);
        let verifying_key = signing_key.verifying_key();
        
        Ok(Self {
            signing_key,
            verifying_key,
        })
    }
    
    /// Sign message
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.signing_key.sign(message).to_bytes().to_vec()
    }
    
    /// Verify signature
    pub fn verify(
        public_key_hex: &str,
        message: &[u8],
        signature_bytes: &[u8],
    ) -> Result<(), CryptoError> {
        let pub_key_bytes = hex::decode(public_key_hex)
            .map_err(|_| CryptoError::InvalidPublicKey)?;
        
        let pub_key_array: [u8; 32] = pub_key_bytes.try_into()
            .map_err(|_| CryptoError::InvalidPublicKey)?;
        
        let verifying_key = VerifyingKey::from_bytes(&pub_key_array)
            .map_err(|_| CryptoError::InvalidPublicKey)?;
        
        let sig_array: [u8; 64] = signature_bytes.try_into()
            .map_err(|_| CryptoError::InvalidSignature)?;
        
        let signature = Signature::from_bytes(&sig_array);
        
        verifying_key.verify(message, &signature)
            .map_err(|_| CryptoError::InvalidSignature)
    }
}

/// Hash data with SHA256
pub fn hash_sha256(data: &[u8]) -> Vec<u8> {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hasher.finalize().to_vec()
}

/// Hash data and return as hex string
pub fn hash_sha256_hex(data: &[u8]) -> String {
    hex::encode(hash_sha256(data))
}

/// Generate address from public key
pub fn public_key_to_address(public_key_hex: &str) -> String {
    format!("TIME1{}", &public_key_hex[..40])
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_keypair_generation() {
        let keypair = KeyPair::generate();
        let pub_key = keypair.public_key_hex();
        
        assert_eq!(pub_key.len(), 64); // 32 bytes = 64 hex chars
    }
    
    #[test]
    fn test_sign_and_verify() {
        let keypair = KeyPair::generate();
        let message = b"test message";
        
        let signature = keypair.sign(message);
        let result = KeyPair::verify(
            &keypair.public_key_hex(),
            message,
            &signature,
        );
        
        assert!(result.is_ok());
    }
    
    #[test]
    fn test_invalid_signature() {
        let keypair = KeyPair::generate();
        let message = b"test message";
        let wrong_message = b"wrong message";
        
        let signature = keypair.sign(message);
        let result = KeyPair::verify(
            &keypair.public_key_hex(),
            wrong_message,
            &signature,
        );
        
        assert!(result.is_err());
    }
    
    #[test]
    fn test_address_generation() {
        let keypair = KeyPair::generate();
        let address = public_key_to_address(&keypair.public_key_hex());
        
        assert!(address.starts_with("TIME1"));
    }
}
