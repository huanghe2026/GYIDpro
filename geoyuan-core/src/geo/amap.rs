//! AMap (高德地图) SDK integration
//! 
//! Provides high-precision positioning (1-3 meters) and reverse geocoding

use crate::{GeoYuanError, Result};
use crate::geo::{AMapLocation, GeoPoint, LocationType, AddressComponent};
use crate::photo::GpsCoordinates;
use md5::{Digest as Md5Digest, Md5};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// AMap API configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AMapConfig {
    /// AMap API Key (required for REST API calls)
    pub api_key: Option<String>,

    /// 数字签名私钥
    pub secret_key: Option<String>,
    
    /// Use native SDK (if available on platform)
    pub use_native_sdk: bool,
    
    /// Request timeout in seconds
    pub timeout_secs: u64,
}

impl Default for AMapConfig {
    fn default() -> Self {
        Self {
            api_key: None,
            secret_key: None,
            use_native_sdk: true,
            timeout_secs: 10,
        }
    }
}

/// 构建带数字签名的高德 API URL
///
/// 签名算法（高德官方）:
/// 1. 收集所有请求参数（不含 sig）
/// 2. 按 key 字典序排序
/// 3. 拼接为 key1=value1&key2=value2...
/// 4. sig = MD5(私钥 + "_" + 拼接字符串)
/// 5. 将 sig 追加到请求参数中
fn build_signed_url(base_url: &str, api_key: &str, secret_key: &str, params: BTreeMap<&str, String>) -> String {
    let mut sorted: BTreeMap<&str, String> = BTreeMap::new();
    sorted.insert("key", api_key.to_string());
    for (k, v) in params {
        sorted.insert(k, v);
    }

    let query_string: String = sorted.iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");

    let sign_input = format!("{}_{}", secret_key, query_string);
    let mut hasher = Md5::new();
    hasher.update(sign_input.as_bytes());
    let sig = hex::encode(hasher.finalize());

    format!("{}?{}&sig={}", base_url, query_string, sig)
}

/// AMap REST API client
pub struct AMapClient {
    config: AMapConfig,
    http_client: reqwest::Client,
}

impl AMapClient {
    /// Create new AMap client
    pub fn new(config: AMapConfig) -> Self {
        let timeout_secs = config.timeout_secs;
        Self {
            config,
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(timeout_secs))
                .build()
                .unwrap_or_default(),
        }
    }
    
    /// Create with default config
    pub fn new_with_key(api_key: &str) -> Self {
        Self::new(AMapConfig {
            api_key: Some(api_key.to_string()),
            ..Default::default()
        })
    }

    /// Create with API key and secret key
    pub fn new_with_keys(api_key: &str, secret_key: &str) -> Self {
        Self::new(AMapConfig {
            api_key: Some(api_key.to_string()),
            secret_key: Some(secret_key.to_string()),
            ..Default::default()
        })
    }
    
    /// Correct GPS coordinates using AMap's high-precision positioning
    pub async fn correct_coordinates(&self, gps: &GpsCoordinates) -> Result<AMapLocation> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| GeoYuanError::Geo("AMap API key not configured".to_string()))?;
        let secret_key = self.config.secret_key.as_ref()
            .ok_or_else(|| GeoYuanError::Geo("AMap secret key not configured".to_string()))?;
        
        let mut params = BTreeMap::new();
        params.insert("locations", format!("{},{}", gps.longitude, gps.latitude));
        params.insert("coordsys", "gps".to_string());
        params.insert("output", "json".to_string());

        let url = build_signed_url(
            "https://restapi.amap.com/v3/assistant/coordinate/convert",
            api_key,
            secret_key,
            params,
        );
        
        let response: AMapConvertResponse = self.http_client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;
        
        if response.status != "1" {
            return Err(GeoYuanError::Geo(format!("AMap convert error: {}", response.info)));
        }
        
        // Parse corrected coordinates
        let corrected = response.locations.split(',')
            .filter_map(|s| s.parse::<f64>().ok())
            .collect::<Vec<_>>();
        
        let corrected_point = if corrected.len() >= 2 {
            Some(GeoPoint {
                latitude: corrected[1],
                longitude: corrected[0],
                altitude: gps.altitude,
            })
        } else {
            None
        };
        
        // Calculate geohash
        let fallback = GeoPoint {
            latitude: gps.latitude,
            longitude: gps.longitude,
            altitude: gps.altitude,
        };
        let best_coords = corrected_point.as_ref().unwrap_or(&fallback);
        
        let geohash = super::geohash::encode(best_coords.latitude, best_coords.longitude, 9);
        
        Ok(AMapLocation {
            raw_gps: gps.clone(),
            corrected: corrected_point,
            geohash,
            address: None,
            accuracy: 2.0, // AMap claims 1-3m accuracy
            loc_type: LocationType::Gps,
        })
    }
    
    /// Reverse geocode coordinates to address
    pub async fn reverse_geocode(&self, lat: f64, lon: f64) -> Result<AddressComponent> {
        let api_key = self.config.api_key.as_ref()
            .ok_or_else(|| GeoYuanError::Geo("AMap API key not configured".to_string()))?;
        let secret_key = self.config.secret_key.as_ref()
            .ok_or_else(|| GeoYuanError::Geo("AMap secret key not configured".to_string()))?;
        
        let mut params = BTreeMap::new();
        params.insert("location", format!("{},{}", lon, lat));
        params.insert("extensions", "base".to_string());
        params.insert("output", "JSON".to_string());

        let url = build_signed_url(
            "https://restapi.amap.com/v3/geocode/regeo",
            api_key,
            secret_key,
            params,
        );
        
        let response: AMapRegeoResponse = self.http_client
            .get(&url)
            .send()
            .await?
            .json()
            .await?;
        
        if response.status != "1" {
            return Err(GeoYuanError::Geo(format!("AMap regeo error: {}", response.info)));
        }
        
        let regeocode = response.regeocode
            .ok_or_else(|| GeoYuanError::Geo("No regeocode data".to_string()))?;
        
        let comp = &regeocode.address_component;
        
        Ok(AddressComponent {
            country: Some(comp.country.clone().unwrap_or_default()),
            province: comp.province.clone(),
            city: comp.city.as_ref().and_then(|c| c.first().cloned()),
            district: comp.district.clone(),
            street: regeocode.street_number.as_ref().map(|s| s.street.clone()),
            street_number: regeocode.street_number.as_ref().map(|s| s.number.clone()),
            poi_name: regeocode.pois.first().map(|p| p.name.clone()),
        })
    }
    
    /// Get full location with address
    pub async fn get_full_location(&self, gps: &GpsCoordinates) -> Result<AMapLocation> {
        let mut location = self.correct_coordinates(gps).await?;
        
        // Try to get address
        let best = location.best_coordinates();
        if let Ok(address) = self.reverse_geocode(best.latitude, best.longitude).await {
            location.address = Some(address);
        }
        
        Ok(location)
    }
}

/// AMap coordinate conversion response
#[derive(Debug, Deserialize)]
struct AMapConvertResponse {
    status: String,
    info: String,
    locations: String,
}

/// AMap reverse geocode response
#[derive(Debug, Deserialize)]
struct AMapRegeoResponse {
    status: String,
    info: String,
    regeocode: Option<AMapRegeoCode>,
}

#[derive(Debug, Deserialize)]
struct AMapRegeoCode {
    address_component: AMapAddressComponent,
    street_number: Option<AMapStreetNumber>,
    pois: Vec<AMapPoi>,
}

#[derive(Debug, Deserialize)]
struct AMapAddressComponent {
    country: Option<String>,
    #[serde(rename = "province")]
    province: Option<String>,
    #[serde(rename = "city")]
    city: Option<Vec<String>>,
    #[serde(rename = "district")]
    district: Option<String>,
}

#[derive(Debug, Deserialize)]
struct AMapStreetNumber {
    street: String,
    number: String,
}

#[derive(Debug, Deserialize)]
struct AMapPoi {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[tokio::test]
    #[ignore] // Requires valid API key (set AMAP_API_KEY / AMAP_SECRET_KEY env vars)
    async fn test_amap_convert() {
        let key = match std::env::var("AMAP_API_KEY") {
            Ok(v) if !v.is_empty() => v,
            _ => {
                eprintln!("跳过：未设置 AMAP_API_KEY 环境变量");
                return;
            }
        };
        let secret = match std::env::var("AMAP_SECRET_KEY") {
            Ok(v) if !v.is_empty() => v,
            _ => {
                eprintln!("跳过：未设置 AMAP_SECRET_KEY 环境变量");
                return;
            }
        };
        let client = AMapClient::new_with_keys(&key, &secret);
        let gps = GpsCoordinates::new(39.9042, 116.4074);

        let result = client.correct_coordinates(&gps).await;
        assert!(result.is_ok());
    }
}
