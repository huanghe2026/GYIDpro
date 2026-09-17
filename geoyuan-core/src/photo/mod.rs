//! Photo module - EXIF parsing and image hashing
//! 
//! This module handles:
//! - Reading and parsing EXIF data from images
//! - Extracting GPS coordinates from EXIF
//! - Computing SHA256 hash of image data

pub mod exif;
pub mod hasher;

use serde::{Deserialize, Serialize};
use std::path::Path;

/// Photo metadata extracted from EXIF
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoMetadata {
    /// SHA256 hash of the raw image bytes
    pub image_hash: String,
    
    /// Original filename
    pub filename: Option<String>,
    
    /// File size in bytes
    pub file_size: u64,
    
    /// Image dimensions (width, height)
    pub dimensions: Option<(u32, u32)>,
    
    /// GPS coordinates if available
    pub gps: Option<GpsCoordinates>,
    
    /// Original capture timestamp from EXIF
    pub capture_time: Option<i64>,
    
    /// Camera make
    pub make: Option<String>,
    
    /// Camera model
    pub model: Option<String>,
}

/// GPS coordinates
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpsCoordinates {
    /// Latitude (-90 to 90)
    pub latitude: f64,
    
    /// Longitude (-180 to 180)
    pub longitude: f64,
    
    /// Altitude in meters (optional)
    pub altitude: Option<f64>,
    
    /// GPS timestamp (Unix timestamp)
    pub timestamp: Option<i64>,
}

impl GpsCoordinates {
    /// Create new GPS coordinates
    pub fn new(latitude: f64, longitude: f64) -> Self {
        Self {
            latitude,
            longitude,
            altitude: None,
            timestamp: None,
        }
    }
    
    /// Calculate distance to another coordinate (Haversine formula)
    pub fn distance_to(&self, other: &GpsCoordinates) -> f64 {
        const EARTH_RADIUS: f64 = 6371000.0; // meters
        
        let lat1 = self.latitude.to_radians();
        let lat2 = other.latitude.to_radians();
        let delta_lat = (other.latitude - self.latitude).to_radians();
        let delta_lon = (other.longitude - self.longitude).to_radians();
        
        let a = (delta_lat / 2.0).sin().powi(2) 
            + lat1.cos() * lat2.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().asin();
        
        EARTH_RADIUS * c
    }
}

/// Parse photo and extract metadata
pub async fn parse_photo<P: AsRef<Path>>(path: P) -> crate::Result<PhotoMetadata> {
    let path = path.as_ref();
    
    // Read file bytes
    let bytes = tokio::fs::read(path).await?;
    let file_size = bytes.len() as u64;
    
    // Compute image hash
    let image_hash = hasher::sha256(&bytes);
    
    // Parse EXIF
    let exif_data = exif::parse_exif(&bytes)?;
    
    // Extract metadata
    let filename = path.file_name()
        .and_then(|n| n.to_str())
        .map(|s| s.to_string());
    
    let dimensions = image::image_dimensions(path).ok();
    
    Ok(PhotoMetadata {
        image_hash,
        filename,
        file_size,
        dimensions,
        gps: exif_data.gps,
        capture_time: exif_data.timestamp,
        make: exif_data.make,
        model: exif_data.model,
    })
}

/// Parse photo from bytes (in memory)
pub fn parse_photo_bytes(bytes: &[u8]) -> crate::Result<PhotoMetadata> {
    // Compute image hash
    let image_hash = hasher::sha256(bytes);
    let file_size = bytes.len() as u64;
    
    // Parse EXIF
    let exif_data = exif::parse_exif_from_slice(bytes)?;
    
    Ok(PhotoMetadata {
        image_hash,
        filename: None,
        file_size,
        dimensions: None,
        gps: exif_data.gps,
        capture_time: exif_data.timestamp,
        make: exif_data.make,
        model: exif_data.model,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gps_distance() {
        // Beijing (39.9042, 116.4074)
        let beijing = GpsCoordinates::new(39.9042, 116.4074);
        
        // Tiananmen Square (39.9073, 116.3972) - about 1.2km away
        let tiananmen = GpsCoordinates::new(39.9073, 116.3972);
        
        let distance = beijing.distance_to(&tiananmen);
        assert!(distance > 800.0 && distance < 1400.0, "Distance should be ~935m-1.2km, got {}m", distance);
    }
}
