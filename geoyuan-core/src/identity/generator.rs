//! GyID generator
//! 
//! Generates unique GyIDs from photo data and GPS coordinates

use super::{GyId, GeoIdConfig};
use crate::{Result, photo::PhotoMetadata, geo::AMapLocation};
use crate::crypto::{blake3_hash_bytes, encoder::Base58Encoder};
use crate::geo::geohash;
use chrono::Utc;

/// GyID Generator
pub struct GyIdGenerator {
    config: GeoIdConfig,
}

impl GyIdGenerator {
    /// Create a new generator with config
    pub fn new(config: GeoIdConfig) -> Self {
        Self { config }
    }
    
    /// Create with default config
    pub fn new_default() -> Self {
        Self::new(GeoIdConfig::default().with_random_salt())
    }
    
    /// Generate GyID from photo metadata
    pub async fn generate(&self, photo: &PhotoMetadata) -> Result<GyId> {
        // Require GPS coordinates
        let gps = photo.gps.as_ref()
            .ok_or_else(|| crate::GeoYuanError::Identity("Photo has no GPS data".to_string()))?;
        
        // Get high-precision location if AMap key is configured
        let location = if let Some(ref key) = self.config.amap_api_key {
            let client = crate::geo::amap::AMapClient::new_with_key(key);
            client.correct_coordinates(gps).await.ok()
        } else {
            None
        };
        
        // Use location data
        let (lat, lon, geohash_str) = if let Some(ref loc) = location {
            let coords = loc.best_coordinates();
            (coords.latitude, coords.longitude, loc.geohash.clone())
        } else {
            // Fall back to raw GPS
            let gh = geohash::encode(gps.latitude, gps.longitude, self.config.geohash_precision);
            (gps.latitude, gps.longitude, gh)
        };
        
        // Build hash input
        let hash_input = self.build_hash_input(photo, lat, lon);
        
        // Compute BLAKE3 hash
        let hash = blake3_hash_bytes(&hash_input);
        
        // Encode to Base58 with prefix
        let base58_hash = Base58Encoder::encode_check(&hash);
        let id = if self.config.testnet {
            format!("TGyID{}", &base58_hash[..20])
        } else {
            format!("GyID{}", &base58_hash[..22])
        };
        
        Ok(GyId {
            id,
            hash: hex::encode(hash),
            created_at: Utc::now(),
            photo_hash: photo.image_hash.clone(),
            latitude: lat,
            longitude: lon,
            geohash: geohash_str,
            version: self.config.version,
        })
    }
    
    /// Generate from AMap location directly
    pub fn generate_from_location(&self, photo_hash: &str, location: &AMapLocation) -> GyId {
        let coords = location.best_coordinates();
        
        // Build hash input
        let hash_input = self.build_hash_input_raw(
            photo_hash,
            coords.latitude,
            coords.longitude,
        );
        
        // Compute BLAKE3 hash
        let hash = blake3_hash_bytes(&hash_input);
        
        // Encode to Base58 with prefix
        let base58_hash = Base58Encoder::encode_check(&hash);
        let id = if self.config.testnet {
            format!("TGyID{}", &base58_hash[..20])
        } else {
            format!("GyID{}", &base58_hash[..22])
        };
        
        GyId {
            id,
            hash: hex::encode(hash),
            created_at: Utc::now(),
            photo_hash: photo_hash.to_string(),
            latitude: coords.latitude,
            longitude: coords.longitude,
            geohash: location.geohash.clone(),
            version: self.config.version,
        }
    }
    
    /// Build hash input from photo metadata
    fn build_hash_input(&self, photo: &PhotoMetadata, lat: f64, lon: f64) -> Vec<u8> {
        use std::io::Write;
        
        let mut buf = Vec::new();
        
        // Version
        buf.write_all(&[self.config.version]).unwrap();
        
        // Photo hash (SHA256)
        buf.write_all(hex::decode(&photo.image_hash).unwrap().as_slice()).unwrap();
        
        // Latitude (f64)
        buf.write_all(&lat.to_le_bytes()).unwrap();
        
        // Longitude (f64)
        buf.write_all(&lon.to_le_bytes()).unwrap();
        
        // Timestamp
        let ts = photo.capture_time.unwrap_or_else(|| Utc::now().timestamp());
        buf.write_all(&ts.to_le_bytes()).unwrap();
        
        // Salt
        buf.write_all(&self.config.salt).unwrap();
        
        buf
    }
    
    /// Build hash input from raw values
    fn build_hash_input_raw(&self, photo_hash: &str, lat: f64, lon: f64) -> Vec<u8> {
        use std::io::Write;
        
        let mut buf = Vec::new();
        
        // Version
        buf.write_all(&[self.config.version]).unwrap();
        
        // Photo hash
        buf.write_all(hex::decode(photo_hash).unwrap().as_slice()).unwrap();
        
        // Latitude
        buf.write_all(&lat.to_le_bytes()).unwrap();
        
        // Longitude
        buf.write_all(&lon.to_le_bytes()).unwrap();
        
        // Timestamp (current time)
        buf.write_all(&Utc::now().timestamp().to_le_bytes()).unwrap();
        
        // Salt
        buf.write_all(&self.config.salt).unwrap();
        
        buf
    }
    
    /// Get config reference
    pub fn config(&self) -> &GeoIdConfig {
        &self.config
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::photo::GpsCoordinates;
    
    #[tokio::test]
    async fn test_generate_from_photo() {
        let generator = GyIdGenerator::new_default();
        
        let photo = PhotoMetadata {
            image_hash: "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9".to_string(),
            filename: Some("test.jpg".to_string()),
            file_size: 1024,
            dimensions: Some((1920, 1080)),
            gps: Some(GpsCoordinates::new(39.9042, 116.4074)),
            capture_time: Some(1710000000),
            make: Some("Canon".to_string()),
            model: Some("EOS R5".to_string()),
        };
        
        let gy_id = generator.generate(&photo).await.unwrap();
        
        assert!(gy_id.id.starts_with("GyID"));
        assert_eq!(gy_id.hash.len(), 64);
        assert_eq!(gy_id.photo_hash, photo.image_hash);
        assert!((gy_id.latitude - 39.9042).abs() < 0.001);
    }
    
    #[test]
    fn test_generate_from_location() {
        let generator = GyIdGenerator::new_default();
        
        let photo_hash = "b94d27b9934d3e08a52e52d7da7dabfac484efe37a5380ee9088f7ace2efcde9";
        
        let location = crate::geo::AMapLocation {
            raw_gps: GpsCoordinates::new(39.9042, 116.4074),
            corrected: Some(crate::geo::GeoPoint {
                latitude: 39.9043,
                longitude: 116.4075,
                altitude: None,
            }),
            geohash: "wx4g0e6md5".to_string(),
            address: None,
            accuracy: 2.0,
            loc_type: crate::geo::LocationType::Gps,
        };
        
        let gy_id = generator.generate_from_location(photo_hash, &location);
        
        assert!(gy_id.id.starts_with("GyID"));
        assert_eq!(gy_id.latitude, 39.9043);
    }
}
