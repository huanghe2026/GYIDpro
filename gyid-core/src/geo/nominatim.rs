//! 逆地理编码模块 (Reverse Geocoding)
//!
//! 使用 OpenStreetMap Nominatim 免费 API，将 (lat, lon) 坐标转换为
//! 可读的城市/国家信息。不需要 API Key，每秒限 1 次请求。
//!
//! API 文档: https://nominatim.openstreetmap.org/reverse

use crate::Result;

/// 逆地理编码结果
#[derive(Debug, Clone)]
pub struct ReverseGeoResult {
    /// 城市名称（优先 city，其次 town/village）
    pub city: Option<String>,
    /// 省/州
    pub state: Option<String>,
    /// 国家名称
    pub country: Option<String>,
    /// 国家代码（两位 ISO，如 "CN"）
    pub country_code: Option<String>,
}

/// 逆地理编码器（基于 OpenStreetMap Nominatim）
pub struct NominatimGeocoder;

#[cfg(feature = "native")]
static HTTP_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();

#[cfg(feature = "native")]
fn get_http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("GyID/0.4 (geoyuan.app; contact@geoyuan.app)")
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("Nominatim HTTP Client 初始化失败")
    })
}

impl NominatimGeocoder {
    /// 根据坐标查询城市/国家信息
    ///
    /// - 成功: 返回 `ReverseGeoResult`
    /// - 失败/超时: 返回 `Ok(None)`，不阻塞上层流程
    pub async fn reverse_geocode(lat: f64, lon: f64) -> Result<Option<ReverseGeoResult>> {
        #[cfg(feature = "native")]
        return Self::reverse_native(lat, lon).await;

        #[cfg(not(feature = "native"))]
        let _ = (lat, lon);
        #[cfg(not(feature = "native"))]
        return Ok(None);
    }

    #[cfg(feature = "native")]
    async fn reverse_native(lat: f64, lon: f64) -> Result<Option<ReverseGeoResult>> {
        let client = get_http_client();

        let url = format!(
            "https://nominatim.openstreetmap.org/reverse?lat={:.6}&lon={:.6}&format=json&accept-language=zh-CN,en",
            lat, lon
        );

        match client.get(&url).send().await {
            Ok(response) => {
                let data: serde_json::Value = match response.json().await {
                    Ok(v) => v,
                    Err(_) => return Ok(None),
                };

                let address = &data["address"];
                if address.is_null() {
                    return Ok(None);
                }

                // 按优先级提取城市名（Nominatim 中不同地区字段名不同）
                let city = address["city"]
                    .as_str()
                    .or_else(|| address["town"].as_str())
                    .or_else(|| address["village"].as_str())
                    .or_else(|| address["municipality"].as_str())
                    .or_else(|| address["county"].as_str())
                    .map(String::from);

                let state = address["state"]
                    .as_str()
                    .or_else(|| address["province"].as_str())
                    .map(String::from);

                let country = address["country"].as_str().map(String::from);

                let country_code = address["country_code"]
                    .as_str()
                    .map(|s| s.to_uppercase());

                Ok(Some(ReverseGeoResult {
                    city,
                    state,
                    country,
                    country_code,
                }))
            }
            Err(_) => {
                // 网络失败：静默返回 None，不中断 GyID 生成
                Ok(None)
            }
        }
    }

    /// 将逆地理编码结果填充到 GeoLocation 中（就地修改）
    ///
    /// 只在 city/country 为 None 时才填充（避免覆盖 IP 定位已有的城市名）
    pub async fn enrich_location(
        lat: f64,
        lon: f64,
        city: &mut Option<String>,
        country: &mut Option<String>,
    ) {
        // 如果两个字段都已有值，跳过网络请求
        if city.is_some() && country.is_some() {
            return;
        }

        if let Ok(Some(result)) = Self::reverse_geocode(lat, lon).await {
            if city.is_none() {
                *city = result.city;
            }
            if country.is_none() {
                *country = result.country;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn test_reverse_geocode_beijing() {
        // 北京天安门广场
        let result = NominatimGeocoder::reverse_geocode(39.9087, 116.3975).await;
        println!("逆地理编码结果（北京）: {:?}", result);
        // 不强制断言，因为网络可能不可用
    }

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn test_reverse_geocode_fallback() {
        // 大西洋中心（应当返回 None 或海洋信息）
        let result = NominatimGeocoder::reverse_geocode(0.0, -30.0).await;
        println!("逆地理编码结果（大西洋）: {:?}", result);
    }
}
