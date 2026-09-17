// ═══════════════════════════════════════════════════════════════════════════
// 位置服务模块
// 高德地图 SDK 集成
// ═══════════════════════════════════════════════════════════════════════════

use crate::photo::GpsCoordinate;
use md5::{Digest as Md5Digest, Md5};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// 高德地图 API 配置
#[derive(Debug, Clone)]
pub struct AmapConfig {
    /// API Key
    pub api_key: String,
    /// 数字签名私钥
    pub secret_key: String,
    /// 服务器地址
    pub base_url: String,
}

impl Default for AmapConfig {
    fn default() -> Self {
        Self {
            // 从环境变量读取，未配置时为空字符串（API 调用会返回错误，但不崩溃）
            // 部署时请在 .env 中设置 AMAP_API_KEY / AMAP_SECRET_KEY
            api_key: std::env::var("AMAP_API_KEY").unwrap_or_default(),
            secret_key: std::env::var("AMAP_SECRET_KEY").unwrap_or_default(),
            base_url: "https://restapi.amap.com/v3".to_string(),
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

    // 按字典序拼接
    let query_string: String = sorted.iter()
        .map(|(k, v)| format!("{}={}", k, v))
        .collect::<Vec<_>>()
        .join("&");

    // MD5(私钥_query_string)
    let sign_input = format!("{}_{}", secret_key, query_string);
    let mut hasher = Md5::new();
    hasher.update(sign_input.as_bytes());
    let sig = hex::encode(hasher.finalize());

    format!("{}?{}&sig={}", base_url, query_string, sig)
}

/// 精确定位结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocationResult {
    /// 纬度
    pub latitude: f64,
    /// 经度
    pub longitude: f64,
    /// 精度 (米)
    pub accuracy: f64,
    /// 地址
    pub address: Option<String>,
    /// 国家
    pub country: Option<String>,
    /// 省份
    pub province: Option<String>,
    /// 城市
    pub city: Option<String>,
    /// 区/县
    pub district: Option<String>,
    /// 数据来源
    pub source: String,
}

/// 位置服务
pub struct LocationService {
    config: AmapConfig,
    client: reqwest::Client,
}

impl LocationService {
    /// 创建位置服务
    pub fn new(config: AmapConfig) -> Self {
        let client = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(3))
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("创建 HTTP 客户端失败");
        
        Self { config, client }
    }
    
    /// 使用默认配置创建
    #[allow(clippy::should_implement_trait)]
    pub fn default() -> Self {
        Self::new(AmapConfig::default())
    }
    
    /// 验证并增强 GPS 坐标
    pub async fn verify_and_enhance(
        &self,
        gps: GpsCoordinate,
    ) -> Result<LocationResult, LocationError> {
        log::info!("🗺️ 调用高德SDK验证定位: ({}, {})", 
            gps.latitude, gps.longitude);
        
        // 调用高德逆地理编码 API
        let result = self.reverse_geocode(gps.latitude, gps.longitude).await?;
        
        log::info!("✅ 高德定位完成: {} (精度: {}m)", 
            result.address.as_deref().unwrap_or("未知"), 
            result.accuracy);
        
        Ok(result)
    }
    
    /// 逆地理编码 - 通过坐标获取地址
    pub async fn reverse_geocode(
        &self,
        latitude: f64,
        longitude: f64,
    ) -> Result<LocationResult, LocationError> {
        let location_str = format!("{},{}", longitude, latitude); // 高德: 经度,纬度
        let mut params = BTreeMap::new();
        params.insert("location", location_str);
        params.insert("extensions", "base".to_string());
        params.insert("output", "JSON".to_string());

        let url = build_signed_url(
            &format!("{}/geocode/regeo", self.config.base_url),
            &self.config.api_key,
            &self.config.secret_key,
            params,
        );
        
        log::debug!("🌐 请求高德API(签名): {}", url);
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| LocationError::NetworkError(e.to_string()))?;
        
        if !response.status().is_success() {
            return Err(LocationError::ApiError(format!(
                "HTTP {}", response.status()
            )));
        }
        
        let api_response: AmapRegeoResponse = response
            .json()
            .await
            .map_err(|e| LocationError::ParseError(e.to_string()))?;
        
        if api_response.status != "1" {
            return Err(LocationError::ApiError(api_response.info.clone()));
        }
        
        let regeo = api_response.regeocode
            .ok_or_else(|| LocationError::ApiError("高德返回数据为空".to_string()))?;
        
        Ok(LocationResult {
            latitude,
            longitude,
            accuracy: 3.0, // 高德返回的精度约 3 米
            address: Some(format!(
                "{}{}{}{}",
                regeo.address_component.province,
                regeo.address_component.city,
                regeo.address_component.district,
                regeo.address_component.township,
            )),
            country: Some(regeo.address_component.nation),
            province: Some(regeo.address_component.province),
            city: Some(regeo.address_component.city),
            district: Some(regeo.address_component.district),
            source: "amap".to_string(),
        })
    }
    
    /// 地理编码 - 通过地址获取坐标
    #[allow(dead_code)]
    pub async fn geocode(&self, address: &str) -> Result<(f64, f64), LocationError> {
        let mut params = BTreeMap::new();
        params.insert("address", address.to_string());
        params.insert("output", "JSON".to_string());

        let url = build_signed_url(
            &format!("{}/geocode/geo", self.config.base_url),
            &self.config.api_key,
            &self.config.secret_key,
            params,
        );
        
        let response = self.client
            .get(&url)
            .send()
            .await
            .map_err(|e| LocationError::NetworkError(e.to_string()))?;
        
        let api_response: AmapGeoResponse = response
            .json()
            .await
            .map_err(|e| LocationError::ParseError(e.to_string()))?;
        
        if api_response.status != "1" || api_response.geocodes.is_empty() {
            return Err(LocationError::ApiError("地理编码失败".to_string()));
        }
        
        let location = &api_response.geocodes[0].location;
        let parts: Vec<&str> = location.split(',').collect();
        
        if parts.len() != 2 {
            return Err(LocationError::ParseError("无效的坐标格式".to_string()));
        }
        
        let longitude: f64 = parts[0].parse()
            .map_err(|_| LocationError::ParseError("经度解析失败".to_string()))?;
        let latitude: f64 = parts[1].parse()
            .map_err(|_| LocationError::ParseError("纬度解析失败".to_string()))?;
        
        Ok((latitude, longitude))
    }
    
    /// 计算两点之间的距离 (米)
    #[allow(dead_code)]
    pub fn distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
        let r = 6371000.0; // 地球半径 (米)
        
        let lat1_rad = lat1.to_radians();
        let lat2_rad = lat2.to_radians();
        let delta_lat = (lat2 - lat1).to_radians();
        let delta_lon = (lon2 - lon1).to_radians();
        
        let a = (delta_lat / 2.0).sin().powi(2) 
            + lat1_rad.cos() * lat2_rad.cos() * (delta_lon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().asin();
        
        r * c
    }
}

/// 高德逆地理编码响应
#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AmapRegeoResponse {
    status: String,
    info: String,
    regeocode: Option<AmapRegeoocode>,
}

#[derive(Debug, Deserialize)]
struct AmapRegeoocode {
    address_component: AmapAddressComponent,
    #[allow(dead_code)]
    formatted_address: String,
}

#[derive(Debug, Deserialize)]
struct AmapAddressComponent {
    nation: String,
    province: String,
    city: String,
    district: String,
    township: String,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AmapGeoResponse {
    status: String,
    info: String,
    geocodes: Vec<AmapGeocode>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct AmapGeocode {
    location: String,
}

/// 位置服务错误
#[derive(Debug, thiserror::Error)]
pub enum LocationError {
    #[error("网络错误: {0}")]
    NetworkError(String),
    
    #[error("API 错误: {0}")]
    ApiError(String),
    
    #[error("解析错误: {0}")]
    ParseError(String),
    
    #[allow(dead_code)]
    #[error("无效的坐标: {0}")]
    InvalidCoordinate(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_distance() {
        // 北京到上海约 1088 公里
        let d = LocationService::distance(39.9042, 116.4074, 31.2304, 121.4737);
        assert!((d / 1000.0 - 1088.0).abs() < 50.0);
    }
}
