//! Image hashing module
//! 
//! Computes SHA256 hash of image data for GeoID generation

use sha2::{Sha256, Digest};

/// Compute SHA256 hash of image bytes
pub fn sha256(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Compute SHA256 hash and return as bytes
pub fn sha256_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Compute SHA256 hash of image pixels only (excluding metadata)
/// This is more expensive but produces a hash based only on visual content
pub fn sha256_pixels(image_path: &str) -> Result<String, crate::GeoYuanError> {
    let img = image::open(image_path)
        .map_err(|e| crate::GeoYuanError::Photo(e.to_string()))?;
    
    // Convert to RGB
    let rgb = img.to_rgb8();
    let pixels = rgb.as_raw();
    
    Ok(sha256(pixels))
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_sha256() {
        let data = b"hello world";
        let hash = sha256(data);
        assert_eq!(hash.len(), 64);
        assert_eq!(hash, "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9");
    }
    
    #[test]
    fn test_sha256_bytes() {
        let data = b"test";
        let hash = sha256_bytes(data);
        assert_eq!(hash.len(), 32);
    }
}
