//! WiFi BSSID 定位模块

use crate::{GyIdError, Result, geo::{GeoLocation, GeoPrecisionLevel}};

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

/// WiFi 扫描结果：BSSID + RSSI(dBm)
#[derive(Debug, Clone)]
pub struct WifiScanResult {
    pub bssid: String,
    pub rssi: i32, // dBm, 通常为 -30 到 -90
}

/// WiFi BSSID 定位器
pub struct WifiLocator;

impl WifiLocator {
    /// 扫描周围 WiFi 网络，返回 BSSID 和信号强度
    pub fn scan() -> Result<Vec<WifiScanResult>> {
        #[cfg(target_os = "windows")]
        {
            Self::scan_windows()
        }
        
        #[cfg(target_os = "macos")]
        {
            Self::scan_macos()
        }
        
        #[cfg(target_os = "linux")]
        {
            Self::scan_linux()
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            Ok(vec![])
        }
    }

    /// 将百分比信号转换为 dBm (近似)
    /// Windows netsh 输出百分比 (0-100%)，MLS 需要 dBm (-30 到 -90)
    fn signal_percent_to_dbm(percent: i32) -> i32 {
        // 近似映射：100% → -30dBm, 0% → -90dBm
        // 公式: dBm = -30 - (100 - percent) * 0.6
        let dbm = -30 - ((100 - percent) as f32 * 0.6) as i32;
        dbm.clamp(-90, -30)
    }

    #[cfg(target_os = "windows")]
    fn scan_windows() -> Result<Vec<WifiScanResult>> {
        use std::process::Command;
        
        // 使用 netsh 扫描 WiFi 网络
        let output = Command::new("netsh")
            .args(["wlan", "show", "networks", "mode=bssid"])
            .output();
        
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let mut results = Vec::new();
                let mut current_bssid: Option<String> = None;
                let mut current_signal: Option<i32> = None;
                
                for line in stdout.lines() {
                    let line = line.trim();
                    
                    // 格式: BSSID 1                 : A4:C3:F0:85:1D:22
                    if line.starts_with("BSSID") {
                        // 保存上一个 BSSID（如果有）
                        if let (Some(bssid), Some(signal)) = (current_bssid.take(), current_signal.take()) {
                            results.push(WifiScanResult { bssid, rssi: signal });
                        }
                        
                        // 解析新的 BSSID
                        if let Some(colon_space) = line.find(": ") {
                            let bssid = line[colon_space + 2..].trim().to_string();
                            // 验证 MAC 格式（17 字符，含 5 个 ':'）
                            if bssid.len() == 17 && bssid.chars().filter(|&c| c == ':').count() == 5 {
                                current_bssid = Some(bssid);
                            }
                        }
                    }
                    // 格式: Signal             : 99%
                    else if line.starts_with("Signal") && current_bssid.is_some() {
                        if let Some(percent_pos) = line.find(':') {
                            let signal_str = line[percent_pos + 1..].trim();
                            // 提取数字部分（去掉 %）
                            let percent: i32 = signal_str
                                .trim_end_matches('%')
                                .trim()
                                .parse()
                                .unwrap_or(50);
                            current_signal = Some(Self::signal_percent_to_dbm(percent));
                        }
                    }
                }
                
                // 保存最后一个 BSSID
                if let (Some(bssid), Some(signal)) = (current_bssid, current_signal) {
                    results.push(WifiScanResult { bssid, rssi: signal });
                }
                
                // 按信号强度排序（最强的在前）
                results.sort_by(|a, b| b.rssi.cmp(&a.rssi));
                
                Ok(results)
            }
            Err(e) => Err(GyIdError::PermissionDenied(
                format!("无法扫描 WiFi 网络: {}", e)
            )),
        }
    }

    #[cfg(target_os = "macos")]
    fn scan_macos() -> Result<Vec<WifiScanResult>> {
        use std::process::Command;
        
        let output = Command::new("/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport")
            .args(["-s", "-i"])
            .output();
        
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let mut results = Vec::new();
                
                for line in stdout.lines().skip(1) {
                    // 格式: SSID BSSID RSSI CHANNEL HT CC SECURITY
                    // 示例: MyWifi 00:11:22:33:44:55 -65 6  Y  -- WPA2
                    let parts: Vec<&str> = line.split_whitespace().collect();
                    if parts.len() >= 3 {
                        let bssid = parts[1].to_string();
                        // airport 直接输出 dBm（负数）
                        let rssi: i32 = parts[2].parse().unwrap_or(-70);
                        
                        // 去重检查
                        if !results.iter().any(|r: &WifiScanResult| r.bssid == bssid) {
                            results.push(WifiScanResult { bssid, rssi });
                        }
                    }
                }
                
                // 按信号强度排序
                results.sort_by(|a, b| b.rssi.cmp(&a.rssi));
                
                Ok(results)
            }
            Err(e) => Err(GyIdError::PermissionDenied(
                format!("无法扫描 WiFi 网络: {}", e)
            )),
        }
    }

    #[cfg(target_os = "linux")]
    fn scan_linux() -> Result<Vec<WifiScanResult>> {
        use std::process::Command;
        
        // 尝试 nmcli 获取 BSSID 和 SIGNAL
        let output = Command::new("nmcli")
            .args(["-t", "-f", "BSSID,SIGNAL", "device", "wifi", "list"])
            .output();
        
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let mut results = Vec::new();
                
                // nmcli -t 输出格式: BSSID:SIGNAL（百分比）
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split(':').collect();
                    if parts.len() >= 2 {
                        let bssid = parts[0].to_string();
                        let signal_pct: i32 = parts[parts.len() - 1].parse().unwrap_or(50);
                        let rssi = Self::signal_percent_to_dbm(signal_pct);
                        
                        if !results.iter().any(|r: &WifiScanResult| r.bssid == bssid) {
                            results.push(WifiScanResult { bssid, rssi });
                        }
                    }
                }
                
                results.sort_by(|a, b| b.rssi.cmp(&a.rssi));
                Ok(results)
            }
            Err(_) => {
                // 备选：使用 iwlist（无法获取信号强度，固定 -70）
                let output = Command::new("sudo")
                    .args(["iwlist", "scan"])
                    .output();
                
                match output {
                    Ok(out) => {
                        let stdout = String::from_utf8_lossy(&out.stdout);
                        let mut results = Vec::new();
                        for line in stdout.lines() {
                            if line.contains("Address:") {
                                if let Some(pos) = line.find("Address:") {
                                    let bssid = line[pos + 9..].trim().to_string();
                                    if !results.iter().any(|r: &WifiScanResult| r.bssid == bssid) {
                                        results.push(WifiScanResult { bssid, rssi: -70 });
                                    }
                                }
                            }
                        }
                        results.sort_by(|a, b| b.rssi.cmp(&a.rssi));
                        Ok(results)
                    }
                    Err(_) => Err(GyIdError::PermissionDenied(
                        "无法扫描 WiFi 网络，需要 sudo 权限".to_string()
                    )),
                }
            }
        }
    }

    /// 使用 Mozilla Location Service 查询 BSSID 位置
    ///
    /// 注意：需要互联网连接（仅 native 平台）
    pub async fn query_mls(scan_results: &[WifiScanResult]) -> Result<GeoLocation> {
        #[cfg(not(feature = "native"))]
        {
            let bssids: Vec<String> = scan_results.iter().map(|r| r.bssid.clone()).collect();
            return Ok(GeoLocation::partial(
                GeoPrecisionLevel::District,
                None, None, None, None, Some(bssids),
            ));
        }

        #[cfg(feature = "native")]
        {
        if scan_results.is_empty() {
            return Err(GyIdError::GeoError("没有 BSSID 数据".to_string()));
        }

        let client = get_http_client();

        // MLS API 格式：传入真实信号强度提升三角定位精度
        let body = serde_json::json!({
            "wifiAccessPoints": scan_results.iter().take(20).map(|r| {
                serde_json::json!({
                    "macAddress": r.bssid.replace(":", ""),
                    "signalStrength": r.rssi
                })
            }).collect::<Vec<_>>()
        });

        match client
            .post("https://location.services.mozilla.com/v1/geolocate")
            .json(&body)
            .send()
            .await
        {
            Ok(response) => {
                let data: serde_json::Value = response
                    .json()
                    .await
                    .map_err(|e| GyIdError::GeoError(format!("JSON 解析失败: {}", e)))?;

                if data["location"].is_null() {
                    // MLS 无法定位，使用离线占位
                    let bssids: Vec<String> = scan_results.iter().map(|r| r.bssid.clone()).collect();
                    return Ok(GeoLocation::partial(
                        GeoPrecisionLevel::District,
                        None, None, None, None, Some(bssids),
                    ));
                }

                let lat = data["location"]["lat"].as_f64();
                let lng = data["location"]["lng"].as_f64();
                // 注意：MLS API 响应不含 city/country 字段，
                // 城市名将由上层调用方通过反向地理编码补充

                let bssids: Vec<String> = scan_results.iter().map(|r| r.bssid.clone()).collect();
                Ok(GeoLocation::partial(
                    GeoPrecisionLevel::District,
                    lat, lng,
                    None,  // city: MLS 不提供，由 Nominatim 补充
                    None,  // country: 同上
                    Some(bssids),
                ))
            }
            Err(_) => {
                // 网络请求失败，使用离线占位位置
                let bssids: Vec<String> = scan_results.iter().map(|r| r.bssid.clone()).collect();
                Ok(GeoLocation::partial(
                    GeoPrecisionLevel::District,
                    None, None, None, None, Some(bssids),
                ))
            }
        }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wifi_scan() {
        let bssids = WifiLocator::scan();
        println!("WiFi BSSID 列表: {:?}", bssids);
    }
}
