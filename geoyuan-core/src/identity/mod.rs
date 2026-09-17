//! Identity module - GyID generation
//! 
//! Generates unique identifiers based on photo data, GPS coordinates, and timestamp

pub mod generator;
pub mod validator;

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// GyID - The unique identity derived from photo and location
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GyId {
    /// The GeoID string (e.g., "GeoID7xK9m2...")
    pub id: String,

    /// Full hash (32 bytes, hex encoded)
    pub hash: String,

    /// When this GeoID was created
    pub created_at: DateTime<Utc>,

    /// Source photo hash
    pub photo_hash: String,

    /// GPS coordinates used
    pub latitude: f64,
    pub longitude: f64,

    /// Geohash for map display
    pub geohash: String,

    /// GeoID version
    pub version: u8,
}

impl GyId {
    /// Create new GyId from string ID
    pub fn new(id: &str) -> Option<Self> {
        // Validate prefix
        if !id.starts_with("GyID") && !id.starts_with("TGyID") {
            return None;
        }

        Some(Self {
            id: id.to_string(),
            hash: String::new(),
            created_at: Utc::now(),
            photo_hash: String::new(),
            latitude: 0.0,
            longitude: 0.0,
            geohash: String::new(),
            version: 1,
        })
    }

    /// Get prefix for display (first 8 chars)
    pub fn prefix(&self) -> &str {
        &self.id[..8.min(self.id.len())]
    }

    /// Check if this is a testnet GyID
    pub fn is_testnet(&self) -> bool {
        self.id.starts_with("TGyID")
    }

    /// Get bytes representation for hashing
    pub fn as_bytes(&self) -> &[u8] {
        self.id.as_bytes()
    }
}

impl Default for GyId {
    fn default() -> Self {
        Self {
            id: "GyID000000000000000000000000000000000000000000000000000000000000".to_string(),
            hash: String::new(),
            created_at: Utc::now(),
            photo_hash: String::new(),
            latitude: 0.0,
            longitude: 0.0,
            geohash: String::new(),
            version: 1,
        }
    }
}

/// PartialEq based solely on the id string.
/// All other fields (created_at, hash, etc.) are metadata and do NOT affect identity equality.
impl PartialEq for GyId {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl PartialOrd for GyId {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

// Eq based on the unique id string (ignores f64 / DateTime fields)
impl Eq for GyId {}

// Manual Hash based on the unique id string (ignores f64 fields)
impl std::hash::Hash for GyId {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

impl Ord for GyId {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

/// Configuration for GeoID generation
#[derive(Debug, Clone)]
pub struct GeoIdConfig {
    /// AMap API key for high-precision positioning
    pub amap_api_key: Option<String>,
    
    /// Geohash precision (1-12)
    pub geohash_precision: usize,
    
    /// Salt for hash computation
    pub salt: [u8; 32],
    
    /// Version byte
    pub version: u8,
    
    /// Use testnet prefix
    pub testnet: bool,
}

impl Default for GeoIdConfig {
    fn default() -> Self {
        Self {
            amap_api_key: None,
            geohash_precision: 9,
            salt: [0u8; 32], // Will be randomized on first use
            version: 1,
            testnet: false,
        }
    }
}

impl GeoIdConfig {
    /// Generate random salt
    pub fn with_random_salt(mut self) -> Self {
        use rand::RngCore;
        rand::rngs::OsRng.fill_bytes(&mut self.salt);
        self
    }
    
    /// Set AMap API key
    pub fn with_amap_key(mut self, key: &str) -> Self {
        self.amap_api_key = Some(key.to_string());
        self
    }
    
    /// Enable testnet
    pub fn with_testnet(mut self) -> Self {
        self.testnet = true;
        self
    }
}

pub use generator::GyIdGenerator;
pub use validator::GyIdValidator;

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_geo_id_prefix() {
        let geo_id = GyId {
            id: "GeoID7xK9m2AbCdEfGh".to_string(),
            hash: "0".repeat(64),
            created_at: Utc::now(),
            photo_hash: "0".repeat(64),
            latitude: 39.9042,
            longitude: 116.4074,
            geohash: "wx4g0e6md5".to_string(),
            version: 1,
        };
        
        assert_eq!(geo_id.prefix(), "GeoID7xK");
    }
}
