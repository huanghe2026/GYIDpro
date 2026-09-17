//! Base58 encoding for human-readable output

use base58::{FromBase58, ToBase58};

/// Base58 encoder/decoder
pub struct Base58Encoder;

impl Base58Encoder {
    /// Encode bytes to Base58 string
    pub fn encode(data: &[u8]) -> String {
        data.to_base58()
    }
    
    /// Decode Base58 string to bytes
    pub fn decode(data: &str) -> Result<Vec<u8>, String> {
        data.from_base58().map_err(|e| format!("{:?}", e))
    }
    
    /// Encode with Bitcoin alphabet (no base64 chars)
    pub fn encode_check(data: &[u8]) -> String {
        use sha2::{Sha256, Digest};
        
        // Double SHA256 for checksum
        let mut hasher = Sha256::new();
        hasher.update(data);
        let first = hasher.finalize();
        
        let mut hasher2 = Sha256::new();
        hasher2.update(first);
        let checksum = &hasher2.finalize()[..4];
        
        // Append checksum
        let mut with_checksum = data.to_vec();
        with_checksum.extend_from_slice(checksum);
        
        Self::encode(&with_checksum)
    }
    
    /// Check and decode Base58 with checksum
    pub fn decode_check(data: &str) -> Result<Vec<u8>, String> {
        let decoded = Self::decode(data)?;
        
        if decoded.len() < 4 {
            return Err("Data too short for checksum".to_string());
        }
        
        let (payload, checksum) = decoded.split_at(decoded.len() - 4);
        
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(payload);
        let first = hasher.finalize();
        
        let mut hasher2 = Sha256::new();
        hasher2.update(first);
        let expected = &hasher2.finalize()[..4];
        
        if checksum != expected {
            return Err("Checksum mismatch".to_string());
        }
        
        Ok(payload.to_vec())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_base58_roundtrip() {
        let original = b"hello world";
        let encoded = Base58Encoder::encode(original);
        assert!(encoded.len() > 0);
        assert_ne!(encoded, "hello world");
        
        let decoded = Base58Encoder::decode(&encoded).unwrap();
        assert_eq!(decoded, original);
    }
    
    #[test]
    fn test_base58_check() {
        let data = b"GyID_test_data";
        let encoded = Base58Encoder::encode_check(data);
        let decoded = Base58Encoder::decode_check(&encoded).unwrap();
        assert_eq!(decoded, data);
    }
}
