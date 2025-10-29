use ed25519_dalek::{SigningKey, VerifyingKey, Signature, Signer, Verifier};
use rand::rngs::OsRng;
use serde::{Serialize, Deserialize};
use std::fmt;

#[derive(Debug)]
pub enum KeypairError {
    GenerationError,
    SignatureError,
    VerificationError,
    SerializationError,
}

impl fmt::Display for KeypairError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            KeypairError::GenerationError => write!(f, "Key generation failed"),
            KeypairError::SignatureError => write!(f, "Invalid signature"),
            KeypairError::VerificationError => write!(f, "Signature verification failed"),
            KeypairError::SerializationError => write!(f, "Serialization failed"),
        }
    }
}

impl std::error::Error for KeypairError {}

#[derive(Clone, Serialize, Deserialize)]
pub struct Keypair {
    #[serde(with = "signing_key_serde")]
    signing_key: SigningKey,
}

mod signing_key_serde {
    use super::*;
    use serde::{Deserializer, Serializer};

    pub fn serialize<S>(key: &SigningKey, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_bytes(&key.to_bytes())
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SigningKey, D::Error>
    where
        D: Deserializer<'de>,
    {
        let bytes: Vec<u8> = serde::Deserialize::deserialize(deserializer)?;
        if bytes.len() != 32 {
            return Err(serde::de::Error::custom("Invalid key length"));
        }
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&bytes);
        Ok(SigningKey::from_bytes(&key_bytes))
    }
}

impl Keypair {
    pub fn generate() -> Result<Self, KeypairError> {
        let signing_key = SigningKey::generate(&mut OsRng);
        Ok(Keypair { signing_key })
    }

    pub fn from_bytes(bytes: &[u8; 32]) -> Result<Self, KeypairError> {
        let signing_key = SigningKey::from_bytes(bytes);
        Ok(Keypair { signing_key })
    }

    pub fn from_secret_key(secret_key: &[u8]) -> Result<Self, KeypairError> {
        if secret_key.len() != 32 {
            return Err(KeypairError::GenerationError);
        }
        let mut bytes = [0u8; 32];
        bytes.copy_from_slice(secret_key);
        Self::from_bytes(&bytes)
    }

    pub fn from_hex(hex: &str) -> Result<Self, KeypairError> {
        let bytes = hex::decode(hex).map_err(|_| KeypairError::SerializationError)?;
        if bytes.len() != 32 {
            return Err(KeypairError::GenerationError);
        }
        let mut key_bytes = [0u8; 32];
        key_bytes.copy_from_slice(&bytes);
        Self::from_bytes(&key_bytes)
    }

    pub fn to_bytes(&self) -> [u8; 32] {
        self.signing_key.to_bytes()
    }

    pub fn secret_key_bytes(&self) -> [u8; 32] {
        self.to_bytes()
    }

    pub fn secret_key_hex(&self) -> String {
        hex::encode(self.to_bytes())
    }

    pub fn public_key(&self) -> VerifyingKey {
        self.signing_key.verifying_key()
    }

    pub fn public_key_bytes(&self) -> [u8; 32] {
        self.public_key().to_bytes()
    }

    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.signing_key.sign(message).to_bytes().to_vec()
    }

    pub fn verify(&self, message: &[u8], signature: &[u8]) -> Result<(), KeypairError> {
        if signature.len() != 64 {
            return Err(KeypairError::SignatureError);
        }
        let mut sig_bytes = [0u8; 64];
        sig_bytes.copy_from_slice(signature);
        let sig = Signature::from_bytes(&sig_bytes);
        
        self.public_key()
            .verify(message, &sig)
            .map_err(|_| KeypairError::VerificationError)
    }

    pub fn verify_with_public_key(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<(), KeypairError> {
        verify_signature(public_key, message, signature)
    }
}

pub fn verify_signature(public_key: &[u8], message: &[u8], signature: &[u8]) -> Result<(), KeypairError> {
    if public_key.len() != 32 {
        return Err(KeypairError::VerificationError);
    }
    if signature.len() != 64 {
        return Err(KeypairError::SignatureError);
    }
    
    let mut pk_bytes = [0u8; 32];
    pk_bytes.copy_from_slice(public_key);
    let verifying_key = VerifyingKey::from_bytes(&pk_bytes)
        .map_err(|_| KeypairError::VerificationError)?;
    
    let mut sig_bytes = [0u8; 64];
    sig_bytes.copy_from_slice(signature);
    let sig = Signature::from_bytes(&sig_bytes);
    
    verifying_key
        .verify(message, &sig)
        .map_err(|_| KeypairError::VerificationError)
}

pub fn keypair_from_seed(seed: &[u8]) -> Result<Keypair, KeypairError> {
    if seed.len() < 32 {
        return Err(KeypairError::GenerationError);
    }
    
    let mut secret_bytes = [0u8; 32];
    secret_bytes.copy_from_slice(&seed[..32]);
    
    Keypair::from_bytes(&secret_bytes)
}
