//! Hashing utilities using BLAKE3 and SHA256

use blake3::Hasher as Blake3Hasher;
use sha2::{Sha256, Digest};

/// Compute BLAKE3 hash
pub fn blake3_hash(data: &[u8]) -> String {
    let hash = blake3_hash_bytes(data);
    hex::encode(hash)
}

/// Compute BLAKE3 hash and return as bytes
pub fn blake3_hash_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Blake3Hasher::new();
    hasher.update(data);
    let hash = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_bytes());
    out
}

/// Compute BLAKE3 hash of multiple inputs
pub fn blake3_hash_many(inputs: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Blake3Hasher::new();
    for input in inputs {
        hasher.update(input);
    }
    let hash = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(hash.as_bytes());
    out
}

/// Compute SHA256 hash (for photo data)
pub fn sha256_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Compute SHA256 hash and return as bytes
pub fn sha256_hash_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(&hasher.finalize());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_blake3() {
        let data = b"hello world";
        let hash = blake3_hash(data);
        assert_eq!(hash.len(), 64); // 32 bytes = 64 hex chars
    }
    
    #[test]
    fn test_sha256() {
        let data = b"hello world";
        let hash = sha256_hash(data);
        assert_eq!(hash.len(), 64);
        // Known SHA256 of "hello world"
        assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    }
}
