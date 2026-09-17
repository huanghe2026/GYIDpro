//! GPS 定位模块
//!
//! 三平台精确定位实现：
//! - Windows: Windows.Devices.Geolocation WinRT API
//! - macOS: CoreLocationCLI 工具 / locationd plist 解析
//! - Linux: GeoClue2 D-Bus / gpsd NMEA 解析
//!
//! 优化版本支持：
//! - GPS 可用性快速检测（1秒超时）
//! - 无 GPS 设备时的快速降级
//! - 并行定位采集

use crate::{geo::GeoLocation, GyIdError, Result};

/// GPS 可用性状态
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum GpsAvailability {
    /// GPS 可用
    Available,
    /// 无 GPS 硬件
    NoHardware,
    /// 权限被拒绝
    PermissionDenied,
    /// 位置服务已禁用
    ServiceDisabled,
    /// 检测超时
    Timeout,
}

impl std::fmt::Display for GpsAvailability {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GpsAvailability::Available => write!(f, "GPS 可用"),
            GpsAvailability::NoHardware => write!(f, "无 GPS 硬件"),
            GpsAvailability::PermissionDenied => write!(f, "权限被拒绝"),
            GpsAvailability::ServiceDisabled => write!(f, "位置服务已禁用"),
            GpsAvailability::Timeout => write!(f, "检测超时"),
        }
    }
}

/// GPS 定位器
pub struct GpsLocator;

/// 从 GPS 数据创建 GeoLocation（不计算 H3，H3 在 collect_geo 中统一计算）
fn geo_from_gps(level: crate::geo::GeoPrecisionLevel, lat: f64, lon: f64) -> GeoLocation {
    GeoLocation {
        level,
        latitude: Some(lat),
        longitude: Some(lon),
        city: None,
        country: None,
        wifi_bssids: None,
        h3_cell: None,
        h3_resolution: None,
    }
}

impl GpsLocator {
    /// 获取 GPS 坐标
    ///
    /// 按精度从高到低依次尝试各种方式，任一方式成功即返回。
    /// 需要系统 GPS 支持和用户授权。
    pub async fn locate() -> Result<GeoLocation> {
        #[cfg(target_os = "windows")]
        {
            Self::locate_windows().await
        }

        #[cfg(target_os = "macos")]
        {
            Self::locate_macos().await
        }

        #[cfg(target_os = "linux")]
        {
            Self::locate_linux().await
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            Err(GyIdError::GeoError("当前平台不支持 GPS 定位".to_string()))
        }
    }

    /// 快速检测 GPS 可用性（1秒超时）
    ///
    /// 这个方法比 locate() 快得多，因为它：
    /// 1. 不等待完整的 GPS 定位
    /// 2. 只需确认 GPS 服务可用
    /// 3. 失败时立即返回，不会阻塞
    pub async fn check_availability() -> GpsAvailability {
        #[cfg(target_os = "windows")]
        {
            Self::check_windows().await
        }

        #[cfg(target_os = "macos")]
        {
            Self::check_macos().await
        }

        #[cfg(target_os = "linux")]
        {
            Self::check_linux().await
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            GpsAvailability::NoHardware
        }
    }

    /// 快速 GPS 定位（3秒超时）
    ///
    /// 比 locate() 更快的定位，牺牲一些精度换取速度。
    /// 适用于需要快速定位但不需要最高精度的场景。
    pub async fn locate_fast() -> Result<GeoLocation> {
        #[cfg(target_os = "windows")]
        {
            Self::locate_windows_fast().await
        }

        #[cfg(target_os = "macos")]
        {
            Self::locate_macos_fast().await
        }

        #[cfg(target_os = "linux")]
        {
            Self::locate_linux_fast().await
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            Err(GyIdError::GeoError("当前平台不支持 GPS 定位".to_string()))
        }
    }

    // ──────────────────────────────────────────────
    // Windows 实现
    // ──────────────────────────────────────────────

    #[cfg(target_os = "windows")]
    async fn locate_windows() -> Result<GeoLocation> {
        use windows::Devices::Geolocation::{Geolocator, PositionStatus};

        // 创建 Geolocator 实例
        let locator = Geolocator::new()
            .map_err(|e| GyIdError::GeoError(format!("创建 Geolocator 失败: {}", e)))?;

        // 检查位置服务状态
        let status = locator
            .LocationStatus()
            .map_err(|e| GyIdError::GeoError(format!("获取位置状态失败: {}", e)))?;

        match status {
            PositionStatus::Disabled | PositionStatus::NotAvailable => {
                return Err(GyIdError::PermissionDenied(
                    "Windows 位置服务已禁用，请在系统设置中启用".to_string(),
                ));
            }
            PositionStatus::NoData => {
                return Err(GyIdError::GeoError(
                    "GPS 暂无数据，请确认设备有 GPS 硬件并处于开阔区域".to_string(),
                ));
            }
            _ => {}
        }

        // 异步获取位置：IAsyncOperation 需要用 get() 阻塞完成（windows crate 不是 Future）
        // 使用 spawn_blocking 避免阻塞 tokio 线程
        let geopos = tokio::time::timeout(
            std::time::Duration::from_secs(15),
            tokio::task::spawn_blocking(move || {
                locator
                    .GetGeopositionAsync()
                    .map_err(|e| GyIdError::GeoError(format!("请求位置失败: {}", e)))?
                    .get()
                    .map_err(|e| GyIdError::GeoError(format!("等待定位结果失败: {}", e)))
            }),
        )
        .await
        .map_err(|_| {
            GyIdError::GeoError("GPS 定位超时（15s），请确认设备已开启位置服务".to_string())
        })?
        .map_err(|e| GyIdError::GeoError(format!("spawn_blocking 失败: {}", e)))??;

        let coord = geopos
            .Coordinate()
            .map_err(|e| GyIdError::GeoError(format!("读取坐标失败: {}", e)))?;
        let point = coord
            .Point()
            .map_err(|e| GyIdError::GeoError(format!("读取坐标点失败: {}", e)))?;
        let pos = point
            .Position()
            .map_err(|e| GyIdError::GeoError(format!("读取经纬度失败: {}", e)))?;

        Ok(geo_from_gps(
            crate::geo::GeoPrecisionLevel::Exact,
            pos.Latitude,
            pos.Longitude,
        ))
    }

    /// Windows 快速检测 GPS 可用性（1秒超时）
    #[cfg(target_os = "windows")]
    async fn check_windows() -> GpsAvailability {
        use windows::Devices::Geolocation::{Geolocator, PositionStatus};

        let locator = match Geolocator::new() {
            Ok(locator) => locator,
            Err(_) => return GpsAvailability::NoHardware,
        };

        // 检查位置服务状态
        let status = match locator.LocationStatus() {
            Ok(status) => status,
            Err(_) => return GpsAvailability::ServiceDisabled,
        };

        match status {
            PositionStatus::Disabled | PositionStatus::NotAvailable => {
                GpsAvailability::ServiceDisabled
            }
            PositionStatus::NoData => GpsAvailability::NoHardware,
            PositionStatus::Ready | PositionStatus::Initializing => GpsAvailability::Available,
            _ => GpsAvailability::Available,
        }
    }

    /// Windows 快速 GPS 定位（3秒超时）
    #[cfg(target_os = "windows")]
    async fn locate_windows_fast() -> Result<GeoLocation> {
        use windows::Devices::Geolocation::{Geolocator, PositionStatus};

        let locator = Geolocator::new()
            .map_err(|e| GyIdError::GeoError(format!("创建 Geolocator 失败: {}", e)))?;

        let status = locator
            .LocationStatus()
            .map_err(|e| GyIdError::GeoError(format!("获取位置状态失败: {}", e)))?;

        match status {
            PositionStatus::Disabled | PositionStatus::NotAvailable => {
                return Err(GyIdError::PermissionDenied(
                    "Windows 位置服务已禁用".to_string(),
                ));
            }
            PositionStatus::NoData => {
                return Err(GyIdError::GeoError(
                    "GPS 暂无数据，请确认设备有 GPS 硬件并处于开阔区域".to_string(),
                ));
            }
            _ => {}
        }

        // 使用 3 秒超时（比默认 15 秒快很多）
        let geopos = tokio::time::timeout(
            std::time::Duration::from_secs(3),
            tokio::task::spawn_blocking(move || {
                locator
                    .GetGeopositionAsync()
                    .map_err(|e| GyIdError::GeoError(format!("请求位置失败: {}", e)))?
                    .get()
                    .map_err(|e| GyIdError::GeoError(format!("等待定位结果失败: {}", e)))
            }),
        )
        .await
        .map_err(|_| {
            GyIdError::GeoError("GPS 定位超时（3s），请确认设备已开启位置服务".to_string())
        })?
        .map_err(|e| GyIdError::GeoError(format!("spawn_blocking 失败: {}", e)))??;

        let coord = geopos
            .Coordinate()
            .map_err(|e| GyIdError::GeoError(format!("读取坐标失败: {}", e)))?;
        let point = coord
            .Point()
            .map_err(|e| GyIdError::GeoError(format!("读取坐标点失败: {}", e)))?;
        let pos = point
            .Position()
            .map_err(|e| GyIdError::GeoError(format!("读取经纬度失败: {}", e)))?;

        Ok(geo_from_gps(
            crate::geo::GeoPrecisionLevel::Exact,
            pos.Latitude,
            pos.Longitude,
        ))
    }

    // ──────────────────────────────────────────────
    // macOS 实现
    // ──────────────────────────────────────────────

    #[cfg(target_os = "macos")]
    async fn locate_macos() -> Result<GeoLocation> {
        // 优先使用 CoreLocationCLI（Homebrew 安装：brew install CoreLocationCLI）
        if let Ok(result) = Self::locate_macos_via_cli().await {
            return Ok(result);
        }
        // Fallback：尝试解析 locationd 缓存的 plist
        Self::locate_macos_via_plist().await
    }

    /// macOS 快速检测 GPS 可用性（1秒超时）
    #[cfg(target_os = "macos")]
    async fn check_macos() -> GpsAvailability {
        use tokio::process::Command;

        // 检查 CoreLocationCLI 是否可用
        let check = Command::new("which").arg("CoreLocationCLI").output().await;

        if check.map(|o| o.status.success()).unwrap_or(false) {
            GpsAvailability::Available
        } else {
            // 检查 locationd 服务
            let check_plist = Command::new("plutil")
                .args([
                    "-convert",
                    "json",
                    "-o",
                    "-",
                    "/var/db/locationd/Library/Caches/locationd/clients.plist",
                ])
                .output()
                .await;

            if check_plist.map(|o| o.status.success()).unwrap_or(false) {
                GpsAvailability::Available
            } else {
                GpsAvailability::NoHardware
            }
        }
    }

    /// macOS 快速 GPS 定位（3秒超时）
    #[cfg(target_os = "macos")]
    async fn locate_macos_fast() -> Result<GeoLocation> {
        use tokio::process::Command;
        use tokio::time::{timeout, Duration};

        // CoreLocationCLI 输出格式：<lat> <lon>
        let output = timeout(
            Duration::from_secs(3), // 3秒超时（比默认10秒快）
            Command::new("CoreLocationCLI")
                .args(["-once", "-format", "%latitude %longitude"])
                .output(),
        )
        .await
        .map_err(|_| GyIdError::GeoError("CoreLocationCLI 超时（3s）".to_string()))?
        .map_err(|_| GyIdError::GeoError("CoreLocationCLI 未安装".to_string()))?;

        if !output.status.success() {
            return Err(GyIdError::GeoError("CoreLocationCLI 返回错误".to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.trim().split_whitespace().collect();
        if parts.len() < 2 {
            return Err(GyIdError::GeoError(
                "CoreLocationCLI 输出格式无法解析".to_string(),
            ));
        }

        let lat: f64 = parts[0]
            .parse()
            .map_err(|_| GyIdError::GeoError("纬度解析失败".to_string()))?;
        let lon: f64 = parts[1]
            .parse()
            .map_err(|_| GyIdError::GeoError("经度解析失败".to_string()))?;

        if lat == 0.0 && lon == 0.0 {
            return Err(GyIdError::GeoError(
                "CoreLocationCLI 返回零坐标".to_string(),
            ));
        }

        Ok(geo_from_gps(crate::geo::GeoPrecisionLevel::Exact, lat, lon))
    }

    #[cfg(target_os = "macos")]
    async fn locate_macos_via_cli() -> Result<GeoLocation> {
        use tokio::process::Command;
        use tokio::time::{timeout, Duration};

        // CoreLocationCLI 输出格式：<lat> <lon>
        let output = timeout(
            Duration::from_secs(10),
            Command::new("CoreLocationCLI")
                .args(["-once", "-format", "%latitude %longitude"])
                .output(),
        )
        .await
        .map_err(|_| GyIdError::GeoError("CoreLocationCLI 超时".to_string()))?
        .map_err(|_| GyIdError::GeoError("CoreLocationCLI 未安装".to_string()))?;

        if !output.status.success() {
            return Err(GyIdError::GeoError("CoreLocationCLI 返回错误".to_string()));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parts: Vec<&str> = stdout.trim().split_whitespace().collect();
        if parts.len() < 2 {
            return Err(GyIdError::GeoError(
                "CoreLocationCLI 输出格式无法解析".to_string(),
            ));
        }

        let lat: f64 = parts[0]
            .parse()
            .map_err(|_| GyIdError::GeoError("纬度解析失败".to_string()))?;
        let lon: f64 = parts[1]
            .parse()
            .map_err(|_| GyIdError::GeoError("经度解析失败".to_string()))?;

        if lat == 0.0 && lon == 0.0 {
            return Err(GyIdError::GeoError(
                "CoreLocationCLI 返回零坐标".to_string(),
            ));
        }

        Ok(geo_from_gps(crate::geo::GeoPrecisionLevel::Exact, lat, lon))
    }

    /// 通过解析 locationd 的缓存 plist 获取最近一次的坐标（需要 root 权限）
    #[cfg(target_os = "macos")]
    async fn locate_macos_via_plist() -> Result<GeoLocation> {
        use tokio::process::Command;

        // macOS Ventura 以后路径
        let paths = [
            "/var/db/locationd/Library/Caches/locationd/clients.plist",
            "/private/var/db/locationd/Library/Caches/locationd/clients.plist",
        ];

        for path in &paths {
            let output = Command::new("plutil")
                .args(["-convert", "json", "-o", "-", path])
                .output()
                .await;

            if let Ok(out) = output {
                if out.status.success() {
                    let json_str = String::from_utf8_lossy(&out.stdout);
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str) {
                        // 解析其中的 Latitude / Longitude
                        let lat = val
                            .as_object()
                            .and_then(|m| m.values().next())
                            .and_then(|v| v.get("Latitude"))
                            .and_then(|v| v.as_f64());
                        let lon = val
                            .as_object()
                            .and_then(|m| m.values().next())
                            .and_then(|v| v.get("Longitude"))
                            .and_then(|v| v.as_f64());

                        if let (Some(lat), Some(lon)) = (lat, lon) {
                            if lat != 0.0 || lon != 0.0 {
                                return Ok(geo_from_gps(
                                    crate::geo::GeoPrecisionLevel::Exact,
                                    lat,
                                    lon,
                                ));
                            }
                        }
                    }
                }
            }
        }

        Err(GyIdError::PermissionDenied(
            "macOS 位置服务不可用，请安装 CoreLocationCLI 或授予 root 权限".to_string(),
        ))
    }

    // ──────────────────────────────────────────────
    // Linux 实现
    // ──────────────────────────────────────────────

    #[cfg(target_os = "linux")]
    async fn locate_linux() -> Result<GeoLocation> {
        // 方案1：GeoClue2 D-Bus（桌面系统首选）
        if let Ok(result) = Self::locate_linux_geoclue().await {
            return Ok(result);
        }
        // 方案2：gpsd + NMEA 解析（嵌入式 / 服务器）
        Self::locate_linux_gpsd().await
    }

    /// Linux 快速检测 GPS 可用性（1秒超时）
    #[cfg(target_os = "linux")]
    async fn check_linux() -> GpsAvailability {
        use tokio::process::Command;

        // 检查 GeoClue2 服务
        let geoclue_check = Command::new("gdbus")
            .args([
                "call",
                "--system",
                "--dest",
                "org.freedesktop.GeoClue2",
                "--object-path",
                "/org/freedesktop/GeoClue2/Manager",
                "--method",
                "org.freedesktop.GeoClue2.Manager.GetClient",
            ])
            .output()
            .await;

        if let Ok(out) = geoclue_check {
            if out.status.success() {
                return GpsAvailability::Available;
            }
        }

        // 检查 gpsd 服务
        let gpsd_check = Command::new("pgrep").arg("-x").arg("gpsd").output().await;

        if let Ok(out) = gpsd_check {
            if out.status.success() {
                return GpsAvailability::Available;
            }
        }

        GpsAvailability::NoHardware
    }

    /// Linux 快速 GPS 定位（3秒超时）
    #[cfg(target_os = "linux")]
    async fn locate_linux_fast() -> Result<GeoLocation> {
        use tokio::{
            io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
            net::TcpStream,
            time::{timeout, Duration},
        };

        // 尝试 gpsd（更快）
        let gpsd_result = timeout(Duration::from_secs(3), Self::locate_linux_gpsd_inner()).await;

        if let Ok(Ok(loc)) = gpsd_result {
            return Ok(loc);
        }

        // Fallback: GeoClue2（更慢但更通用）
        Self::locate_linux_geoclue_fast().await
    }

    /// GeoClue2 快速定位（3秒超时）
    #[cfg(target_os = "linux")]
    async fn locate_linux_geoclue_fast() -> Result<GeoLocation> {
        use tokio::process::Command;
        use tokio::time::{timeout, Duration};

        let output = timeout(
            Duration::from_secs(3),
            Command::new("gdbus")
                .args([
                    "call",
                    "--system",
                    "--dest",
                    "org.freedesktop.GeoClue2",
                    "--object-path",
                    "/org/freedesktop/GeoClue2/Manager",
                    "--method",
                    "org.freedesktop.GeoClue2.Manager.GetClient",
                ])
                .output(),
        )
        .await
        .map_err(|_| GyIdError::GeoError("GeoClue2 检测超时（3s）".to_string()))?
        .map_err(|e| GyIdError::GeoError(format!("gdbus 调用失败: {}", e)))?;

        if !output.status.success() {
            return Err(GyIdError::GeoError("GeoClue2 服务不可用".to_string()));
        }

        let client_path_raw = String::from_utf8_lossy(&output.stdout);
        let client_path = client_path_raw
            .trim()
            .trim_matches(|c| c == '(' || c == ')')
            .trim()
            .trim_matches('\'')
            .to_string();

        if client_path.is_empty() {
            return Err(GyIdError::GeoError(
                "GeoClue2 未返回 client path".to_string(),
            ));
        }

        // 设置 DesktopId
        let _ = Command::new("gdbus")
            .args([
                "call",
                "--system",
                "--dest",
                "org.freedesktop.GeoClue2",
                "--object-path",
                &client_path,
                "--method",
                "org.freedesktop.DBus.Properties.Set",
                "org.freedesktop.GeoClue2.Client",
                "DesktopId",
                "<\'gyid-cli\'>",
            ])
            .output()
            .await;

        // 启动定位
        let _ = Command::new("gdbus")
            .args([
                "call",
                "--system",
                "--dest",
                "org.freedesktop.GeoClue2",
                "--object-path",
                &client_path,
                "--method",
                "org.freedesktop.GeoClue2.Client.Start",
            ])
            .output()
            .await;

        // 等待信号（最多 3 秒）
        for i in 0..3 {
            tokio::time::sleep(Duration::from_secs(1)).await;

            let loc_output = Command::new("gdbus")
                .args([
                    "call",
                    "--system",
                    "--dest",
                    "org.freedesktop.GeoClue2",
                    "--object-path",
                    &client_path,
                    "--method",
                    "org.freedesktop.DBus.Properties.Get",
                    "org.freedesktop.GeoClue2.Client",
                    "Location",
                ])
                .output()
                .await;

            if let Ok(loc_out) = loc_output {
                let loc_str = String::from_utf8_lossy(&loc_out.stdout);
                let loc_path = loc_str
                    .trim()
                    .trim_matches(|c: char| c == '(' || c == ')')
                    .trim()
                    .trim_matches('<')
                    .trim_matches('>')
                    .trim()
                    .trim_matches(|c: char| c == '\'' || c == '"')
                    .to_string();

                if loc_path.contains("/Location/") {
                    if let Some((lat, lon)) = Self::geoclue_read_coord(&loc_path, "Latitude")
                        .await
                        .ok()
                        .zip(Self::geoclue_read_coord(&loc_path, "Longitude").await.ok())
                    {
                        let _ = Command::new("gdbus")
                            .args([
                                "call",
                                "--system",
                                "--dest",
                                "org.freedesktop.GeoClue2",
                                "--object-path",
                                &client_path,
                                "--method",
                                "org.freedesktop.GeoClue2.Client.Stop",
                            ])
                            .output()
                            .await;

                        return Ok(geo_from_gps(crate::geo::GeoPrecisionLevel::Exact, lat, lon));
                    }
                }
            }
        }

        Err(GyIdError::GeoError("GeoClue2 定位超时（3s）".to_string()))
    }

    /// gpsd 定位内部实现（不带超时）
    #[cfg(target_os = "linux")]
    async fn locate_linux_gpsd_inner() -> Result<GeoLocation> {
        use std::time::Duration;
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
        use tokio::net::TcpStream;

        let stream = TcpStream::connect("127.0.0.1:2947")
            .await
            .map_err(|e| GyIdError::GeoError(format!("gpsd 连接失败: {}", e)))?;

        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();

        writer
            .write_all(b"?WATCH={\"enable\":true,\"json\":true}\n")
            .await
            .map_err(|e| GyIdError::GeoError(format!("发送 WATCH 命令失败: {}", e)))?;

        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        loop {
            if tokio::time::Instant::now() > deadline {
                break;
            }

            match tokio::time::timeout(Duration::from_secs(1), lines.next_line()).await {
                Ok(Ok(Some(line))) => {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                        if val.get("class").and_then(|v| v.as_str()) == Some("TPV") {
                            let lat = val.get("lat").and_then(|v| v.as_f64());
                            let lon = val.get("lon").and_then(|v| v.as_f64());
                            let mode = val.get("mode").and_then(|v| v.as_i64()).unwrap_or(0);

                            if mode >= 2 {
                                if let (Some(lat), Some(lon)) = (lat, lon) {
                                    return Ok(geo_from_gps(
                                        crate::geo::GeoPrecisionLevel::Exact,
                                        lat,
                                        lon,
                                    ));
                                }
                            }
                        }
                    }
                }
                _ => {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        }

        Err(GyIdError::GeoError("gpsd 定位超时".to_string()))
    }

    /// 通过 gdbus 工具调用 GeoClue2 D-Bus 接口
    #[cfg(target_os = "linux")]
    async fn locate_linux_geoclue() -> Result<GeoLocation> {
        use tokio::process::Command;
        use tokio::time::{timeout, Duration};

        // 检查 gdbus 是否可用
        let check = Command::new("which").arg("gdbus").output().await;
        if check.map(|o| !o.status.success()).unwrap_or(true) {
            return Err(GyIdError::GeoError("gdbus 不可用".to_string()));
        }

        // 通过 gdbus 调用 GeoClue2 Manager.GetClient
        // 注意：完整 D-Bus 鉴权需要通过 Polkit，这里使用 geoclue 的 CLI 工具封装
        let output = timeout(
            Duration::from_secs(15),
            Command::new("gdbus")
                .args([
                    "call",
                    "--system",
                    "--dest",
                    "org.freedesktop.GeoClue2",
                    "--object-path",
                    "/org/freedesktop/GeoClue2/Manager",
                    "--method",
                    "org.freedesktop.GeoClue2.Manager.GetClient",
                ])
                .output(),
        )
        .await
        .map_err(|_| GyIdError::GeoError("GeoClue2 调用超时".to_string()))?
        .map_err(|e| GyIdError::GeoError(format!("gdbus 调用失败: {}", e)))?;

        if !output.status.success() {
            return Err(GyIdError::GeoError("GeoClue2 服务不可用".to_string()));
        }

        // 解析返回的 client object path，例如 ('/org/freedesktop/GeoClue2/Client/1',)
        let client_path_raw = String::from_utf8_lossy(&output.stdout);
        let client_path = client_path_raw
            .trim()
            .trim_matches(|c| c == '(' || c == ')')
            .trim()
            .trim_matches('\'')
            .to_string();

        if client_path.is_empty() {
            return Err(GyIdError::GeoError(
                "GeoClue2 未返回 client path".to_string(),
            ));
        }

        // 设置 DesktopId
        let _ = Command::new("gdbus")
            .args([
                "call",
                "--system",
                "--dest",
                "org.freedesktop.GeoClue2",
                "--object-path",
                &client_path,
                "--method",
                "org.freedesktop.DBus.Properties.Set",
                "org.freedesktop.GeoClue2.Client",
                "DesktopId",
                "<'gyid-cli'>",
            ])
            .output()
            .await;

        // 启动定位
        let _ = Command::new("gdbus")
            .args([
                "call",
                "--system",
                "--dest",
                "org.freedesktop.GeoClue2",
                "--object-path",
                &client_path,
                "--method",
                "org.freedesktop.GeoClue2.Client.Start",
            ])
            .output()
            .await
            .map_err(|e| GyIdError::GeoError(format!("启动定位失败: {}", e)))?;

        // 等待信号（简化：轮询 Location 属性，最多等 10 秒）
        for _ in 0..10 {
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;

            let loc_output = Command::new("gdbus")
                .args([
                    "call",
                    "--system",
                    "--dest",
                    "org.freedesktop.GeoClue2",
                    "--object-path",
                    &client_path,
                    "--method",
                    "org.freedesktop.DBus.Properties.Get",
                    "org.freedesktop.GeoClue2.Client",
                    "Location",
                ])
                .output()
                .await;

            if let Ok(loc_out) = loc_output {
                let loc_str = String::from_utf8_lossy(&loc_out.stdout);
                // 输出类似：(<objectpath '/org/freedesktop/GeoClue2/Location/1'>,)
                let loc_path = loc_str
                    .trim()
                    .trim_matches(|c: char| c == '(' || c == ')')
                    .trim()
                    .trim_matches('<')
                    .trim_matches('>')
                    .trim()
                    .trim_matches(|c: char| c == '\'' || c == '"')
                    .to_string();

                if loc_path.contains("/Location/") {
                    // 读取 Latitude 和 Longitude
                    if let Some((lat, lon)) = Self::geoclue_read_coord(&loc_path, "Latitude")
                        .await
                        .ok()
                        .zip(Self::geoclue_read_coord(&loc_path, "Longitude").await.ok())
                    {
                        // 停止定位
                        let _ = Command::new("gdbus")
                            .args([
                                "call",
                                "--system",
                                "--dest",
                                "org.freedesktop.GeoClue2",
                                "--object-path",
                                &client_path,
                                "--method",
                                "org.freedesktop.GeoClue2.Client.Stop",
                            ])
                            .output()
                            .await;

                        return Ok(geo_from_gps(crate::geo::GeoPrecisionLevel::Exact, lat, lon));
                    }
                }
            }
        }

        Err(GyIdError::GeoError(
            "GeoClue2 定位超时，未获取到有效坐标".to_string(),
        ))
    }

    /// 从 GeoClue2 Location 对象读取单个坐标属性
    #[cfg(target_os = "linux")]
    async fn geoclue_read_coord(location_path: &str, prop: &str) -> Result<f64> {
        use tokio::process::Command;

        let output = Command::new("gdbus")
            .args([
                "call",
                "--system",
                "--dest",
                "org.freedesktop.GeoClue2",
                "--object-path",
                location_path,
                "--method",
                "org.freedesktop.DBus.Properties.Get",
                "org.freedesktop.GeoClue2.Location",
                prop,
            ])
            .output()
            .await
            .map_err(|e| GyIdError::GeoError(format!("读取 {} 失败: {}", prop, e)))?;

        if !output.status.success() {
            return Err(GyIdError::GeoError(format!("{} 属性读取失败", prop)));
        }

        // 输出格式：(<double 39.9042>,)
        let raw = String::from_utf8_lossy(&output.stdout);
        let value_str = raw
            .trim()
            .trim_matches(|c: char| c == '(' || c == ')')
            .trim()
            .trim_matches('<')
            .trim_matches('>')
            .trim()
            .trim_start_matches("double ")
            .trim();

        value_str
            .parse::<f64>()
            .map_err(|_| GyIdError::GeoError(format!("{} 解析失败: {}", prop, value_str)))
    }

    /// 通过 gpsd 的 TCP JSON 接口读取 NMEA 数据
    #[cfg(target_os = "linux")]
    async fn locate_linux_gpsd() -> Result<GeoLocation> {
        use tokio::{
            io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
            net::TcpStream,
            time::{timeout, Duration},
        };

        // gpsd 默认监听 2947 端口
        let stream = timeout(Duration::from_secs(3), TcpStream::connect("127.0.0.1:2947"))
            .await
            .map_err(|_| GyIdError::GeoError("gpsd 连接超时".to_string()))?
            .map_err(|e| GyIdError::GeoError(format!("gpsd 未运行 ({})", e)))?;

        let (reader, mut writer) = stream.into_split();
        let mut lines = BufReader::new(reader).lines();

        // 发送 WATCH 命令启用 JSON 报文
        writer
            .write_all(b"?WATCH={\"enable\":true,\"json\":true}\n")
            .await
            .map_err(|e| GyIdError::GeoError(format!("发送 WATCH 命令失败: {}", e)))?;

        // 读取报文，找到 TPV（位置）报文
        let deadline = tokio::time::Instant::now() + Duration::from_secs(15);
        loop {
            if tokio::time::Instant::now() > deadline {
                break;
            }

            match timeout(Duration::from_secs(2), lines.next_line()).await {
                Ok(Ok(Some(line))) => {
                    if let Ok(val) = serde_json::from_str::<serde_json::Value>(&line) {
                        if val.get("class").and_then(|v| v.as_str()) == Some("TPV") {
                            let lat = val.get("lat").and_then(|v| v.as_f64());
                            let lon = val.get("lon").and_then(|v| v.as_f64());
                            let mode = val.get("mode").and_then(|v| v.as_i64()).unwrap_or(0);

                            // mode 2 = 2D fix, mode 3 = 3D fix
                            if mode >= 2 {
                                if let (Some(lat), Some(lon)) = (lat, lon) {
                                    return Ok(geo_from_gps(
                                        crate::geo::GeoPrecisionLevel::Exact,
                                        lat,
                                        lon,
                                    ));
                                }
                            }
                        }
                    }
                }
                _ => {
                    tokio::time::sleep(Duration::from_millis(200)).await;
                }
            }
        }

        Err(GyIdError::GeoError(
            "gpsd 定位超时，未获取到有效 GPS 定位（确认 GPS 天线可见天空）".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要 GPS 硬件或位置服务授权
    async fn test_gps_locate() {
        match GpsLocator::locate().await {
            Ok(loc) => {
                println!(
                    "GPS 定位成功: lat={:?}, lon={:?}",
                    loc.latitude, loc.longitude
                );
                assert!(loc.latitude.is_some());
                assert!(loc.longitude.is_some());
            }
            Err(e) => {
                println!("GPS 定位失败（可能无硬件/未授权）: {}", e);
            }
        }
    }
}
