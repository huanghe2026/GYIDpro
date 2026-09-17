//! Geographic location module
//! 
//! Integrates with AMap (高德地图) SDK for high-precision positioning (1-3 meters)

pub mod amap;
pub mod geohash;
pub mod h3grid;

use serde::{Deserialize, Serialize};
use crate::photo::GpsCoordinates;

/// Precision level for geolocation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GeoPrecision {
    /// ~100 meters (IP-based)
    Low,
    /// ~10 meters (WiFi/Cell tower)
    Medium,
    /// ~1-3 meters (GPS + AMap correction)
    #[default]
    High,
}

/// High-precision location data from AMap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AMapLocation {
    /// Raw GPS coordinates
    pub raw_gps: GpsCoordinates,
    
    /// AMap-corrected coordinates (higher precision)
    pub corrected: Option<GeoPoint>,
    
    /// Geohash string for map display
    pub geohash: String,
    
    /// AMap address component
    pub address: Option<AddressComponent>,
    
    /// Positioning accuracy in meters
    pub accuracy: f64,
    
    /// Location type (GPS, Network, etc.)
    pub loc_type: LocationType,
}

/// Geographic point with high precision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoPoint {
    /// Latitude
    pub latitude: f64,
    
    /// Longitude
    pub longitude: f64,
    
    /// Altitude (if available)
    pub altitude: Option<f64>,
}

/// Type alias for backward compatibility
pub type GeoLocation = GeoPoint;

/// Address component from reverse geocoding
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddressComponent {
    /// Country
    pub country: Option<String>,
    
    /// Province/State
    pub province: Option<String>,
    
    /// City
    pub city: Option<String>,
    
    /// District/County
    pub district: Option<String>,
    
    /// Street
    pub street: Option<String>,
    
    /// Street number
    pub street_number: Option<String>,
    
    /// POI name (Point of Interest)
    pub poi_name: Option<String>,
}

/// Location type from AMap
#[derive(Debug, Clone, Copy, Serialize, Deserialize, Default)]
pub enum LocationType {
    /// GPS satellite positioning
    Gps,
    /// Network positioning (WiFi/Cell)
    Network,
    /// AMap offline positioning
    Offline,
    /// Unknown
    #[default]
    Unknown,
}

impl AMapLocation {
    /// Get the best available coordinates (prefer corrected)
    pub fn best_coordinates(&self) -> GeoPoint {
        self.corrected.clone().unwrap_or(GeoPoint {
            latitude: self.raw_gps.latitude,
            longitude: self.raw_gps.longitude,
            altitude: self.raw_gps.altitude,
        })
    }
    
    /// Calculate accuracy level
    pub fn accuracy_level(&self) -> GeoPrecision {
        if self.accuracy <= 3.0 {
            GeoPrecision::High
        } else if self.accuracy <= 10.0 {
            GeoPrecision::Medium
        } else {
            GeoPrecision::Low
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_accuracy_level() {
        let high = AMapLocation {
            raw_gps: GpsCoordinates::new(39.9042, 116.4074),
            corrected: None,
            geohash: String::new(),
            address: None,
            accuracy: 2.5,
            loc_type: LocationType::Gps,
        };
        assert_eq!(high.accuracy_level(), GeoPrecision::High);
        
        let low = AMapLocation {
            accuracy: 50.0,
            ..high
        };
        assert_eq!(low.accuracy_level(), GeoPrecision::Low);
    }
}
