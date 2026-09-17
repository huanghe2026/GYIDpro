//! IP 定位模块

use crate::{Result, geo::GeoLocation};

/// 全局复用的 HTTP Client（OnceLock 保证只初始化一次，节省每次 TCP 握手开销）
#[cfg(feature = "native")]
static HTTP_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();

#[cfg(feature = "native")]
fn get_http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent("GyID/0.1")
            .timeout(std::time::Duration::from_secs(5))
            .build()
            .expect("HTTP Client 初始化失败")
    })
}

/// IP 地址定位器
pub struct IpLocator;

impl IpLocator {
    /// 使用免费 IP 定位 API 获取位置
    ///
    /// - Native 平台: 使用复用的全局 reqwest::Client 调用 ip-api.com
    /// - WASM 平台: 返回空位置（浏览器中需要通过 Geolocation API 在 JS 层处理）
    pub async fn locate() -> Result<GeoLocation> {
        #[cfg(feature = "native")]
        return Self::locate_native().await;

        #[cfg(not(feature = "native"))]
        return Ok(GeoLocation::partial(
            crate::geo::GeoPrecisionLevel::City,
            None, None, None, None, None,
        ));
    }

    #[cfg(feature = "native")]
    async fn locate_native() -> Result<GeoLocation> {
        // 主源：ip-api.com（免费，无需 API Key）
        if let Some(loc) = Self::try_ip_api().await {
            tracing::debug!("IP 定位成功（ip-api.com）");
            return Ok(loc);
        }

        // 备用源：ipinfo.io（免费，100k/月）
        if let Some(loc) = Self::try_ipinfo().await {
            tracing::debug!("IP 定位成功（ipinfo.io 备用源）");
            return Ok(loc);
        }

        // 两源均失败，使用离线占位
        tracing::warn!("IP 定位失败，使用离线占位位置");
        Ok(Self::fallback_location())
    }

    /// 尝试 ip-api.com 定位
    #[cfg(feature = "native")]
    async fn try_ip_api() -> Option<GeoLocation> {
        let client = get_http_client();
        let response = client
            .get("https://ip-api.com/json/?fields=status,city,country,lat,lon")
            .send()
            .await
            .ok()?;

        let data: serde_json::Value = response.json().await.ok()?;

        if data["status"] == "fail" {
            return None;
        }

        let city = data["city"].as_str().map(String::from);
        let country = data["country"].as_str().map(String::from);
        let lat = data["lat"].as_f64();
        let lon = data["lon"].as_f64();

        Some(GeoLocation::partial(
            crate::geo::GeoPrecisionLevel::City,
            lat, lon, city, country, None,
        ))
    }

    /// 尝试 ipinfo.io 定位（备用源）
    #[cfg(feature = "native")]
    async fn try_ipinfo() -> Option<GeoLocation> {
        let client = get_http_client();
        let response = client
            .get("https://ipinfo.io/json")
            .send()
            .await
            .ok()?;

        let data: serde_json::Value = response.json().await.ok()?;

        // ipinfo.io 格式: { "city": "Beijing", "country": "CN", "loc": "39.9042,116.4074" }
        let city = data["city"].as_str().map(String::from);
        let country = data["country"].as_str().map(String::from);

        let (lat, lon) = data["loc"].as_str().and_then(|loc| {
            let mut parts = loc.splitn(2, ',');
            let lat: f64 = parts.next()?.parse().ok()?;
            let lon: f64 = parts.next()?.parse().ok()?;
            Some((lat, lon))
        }).map(|(la, lo)| (Some(la), Some(lo)))
          .unwrap_or((None, None));

        Some(GeoLocation::partial(
            crate::geo::GeoPrecisionLevel::City,
            lat, lon, city, country, None,
        ))
    }

    /// 离线降级位置 - 当网络不可用时使用
    fn fallback_location() -> GeoLocation {
        GeoLocation::partial(
            crate::geo::GeoPrecisionLevel::City,
            None, None, None, None, None,
        )
    }

    /// 同步版本（使用阻塞运行时，仅 native）
    #[cfg(all(feature = "native", feature = "sync"))]
    pub fn locate_sync() -> Result<GeoLocation> {
        use tokio::runtime::Runtime;

        let rt = Runtime::new().map_err(|e| GyIdError::GeoError(e.to_string()))?;
        rt.block_on(async {
            Self::locate().await
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(feature = "native")]
    #[tokio::test]
    async fn test_ip_locate() {
        let location = IpLocator::locate().await;
        println!("IP 定位结果: {:?}", location);
    }
}
