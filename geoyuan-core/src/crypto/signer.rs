//! Ed25519 digital signatures
//! 
//! Uses ed25519-dalek for fast, secure signatures

use ed25519_dalek::{Signer, SigningKey, Verifier, VerifyingKey};
pub use ed25519_dalek::Signature;
use rand::rngs::OsRng;

/// Ed25519 key pair for GeoYuan identity
#[derive(Debug, Clone)]
pub struct KeyPair {
    secret: [u8; 32],
    public: VerifyingKey,
}

impl KeyPair {
    /// Generate a new random key pair
    pub fn generate() -> Self {
        let signing_key = SigningKey::generate(&mut OsRng);
        let secret = signing_key.to_bytes();
        let public = signing_key.verifying_key();
        
        Self { secret, public }
    }
    
    /// Create from secret key bytes
    pub fn from_secret(secret_bytes: &[u8; 32]) -> Result<Self, &'static str> {
        let signing_key = SigningKey::from_bytes(secret_bytes);
        let public = signing_key.verifying_key();
        Ok(Self { secret: *secret_bytes, public })
    }
    
    /// Get public key as hex string
    pub fn public_hex(&self) -> String {
        hex::encode(self.public.as_bytes())
    }
    
    /// Get public key as bytes
    pub fn public_bytes(&self) -> [u8; 32] {
        *self.public.as_bytes()
    }

    /// Get secret key bytes (for persistence)
    pub fn secret_bytes(&self) -> [u8; 32] {
        self.secret
    }
    
    /// Sign data
    pub fn sign(&self, message: &[u8]) -> Signature {
        let signing_key = SigningKey::from_bytes(&self.secret);
        signing_key.sign(message)
    }
    
    /// Verify signature
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        self.public.verify(message, signature).is_ok()
    }
}

/// Ed25519 signature wrapper for serialization
#[derive(Debug, Clone)]
pub struct SignatureBytes(pub [u8; 64]);

impl SignatureBytes {
    /// Create from raw bytes
    pub fn from_bytes(bytes: [u8; 64]) -> Self {
        Self(bytes)
    }
    
    /// Convert to bytes
    pub fn to_bytes(&self) -> [u8; 64] {
        self.0
    }
    
    /// Convert to hex string
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
    
    /// Create from hex string
    pub fn from_hex(hex_str: &str) -> Result<Self, String> {
        let bytes = hex::decode(hex_str)
            .map_err(|e| e.to_string())?
            .try_into()
            .map_err(|_| "Invalid signature hex length".to_string())?;
        Ok(Self(bytes))
    }
}

impl KeyPair {
    /// Verify signature using public key bytes
    pub fn verify_signature(message: &[u8], signature: &[u8; 64], public_key: &[u8; 32]) -> bool {
        let Ok(public) = VerifyingKey::from_bytes(public_key) else {
            return false;
        };
        
        let sig = Signature::from_bytes(signature);
        
        public.verify(message, &sig).is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_keypair_generate() {
        let keypair = KeyPair::generate();
        assert_eq!(keypair.public_bytes().len(), 32);
    }
    
    #[test]
    fn test_sign_verify() {
        let keypair = KeyPair::generate();
        let message = b"Hello, GeoYuan!";
        
        let signature = keypair.sign(message);
        assert!(keypair.verify(message, &signature));
        
        // Verify with wrong message fails
        assert!(!keypair.verify(b"Wrong message", &signature));
    }
    
    #[test]
    fn test_signature_hex() {
        let keypair = KeyPair::generate();
        let message = b"Test message";
        let signature = keypair.sign(message);
        
        let sig_bytes = SignatureBytes::from_bytes(signature.to_bytes());
        let hex = sig_bytes.to_hex();
        assert_eq!(hex.len(), 128);
        
        let recovered = SignatureBytes::from_hex(&hex).unwrap();
        assert_eq!(recovered.to_bytes(), sig_bytes.to_bytes());
    }
}
