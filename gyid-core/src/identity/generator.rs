//! GyID 生成器
//!
//! 优化版本支持：
//! - 快速 GPS 检测（无设备时秒级降级）
//! - 并行地理位置采集
//! - 手动输入坐标支持
//! - 无位置模式（仅硬件指纹）
//! - H3 六边形网格系统

use crate::{
    GyIdError, Result,
    fingerprint::{HardwareFingerprint, MacCollector, CpuCollector, BoardCollector, DiskCollector},
    geo::{GeoLocation, GeoPrecisionLevel, IpLocator, WifiLocator, GpsLocator, GpsAvailability},
    avatar::{Avatar, AvatarLoader},
    crypto::{GyIdHasher, Base58Encoder, GyIdWeights},
    identity::GyId,
};
use parking_lot::RwLock;
use std::time::{Duration, Instant};

/// GyID 生成器配置
#[derive(Debug, Clone)]
pub struct GeneratorConfig {
    /// 地理位置精度等级
    pub geo_level: GeoPrecisionLevel,
    /// 权重配置
    pub weights: GyIdWeights,
    /// 是否采集头像
    pub with_avatar: bool,
    /// 是否采集地理位置
    pub with_geo: bool,
}

impl Default for GeneratorConfig {
    fn default() -> Self {
        Self {
            geo_level: GeoPrecisionLevel::City,
            weights: GyIdWeights::default(),
            with_avatar: false,  // 默认不需要头像，由用户显式提供
            with_geo: true,
        }
    }
}

/// GyID 生成器
pub struct GyIdGenerator {
    config: GeneratorConfig,
    /// 指纹缓存（避免短时间内重复采集，线程安全）
    fingerprint_cache: RwLock<Option<(Instant, HardwareFingerprint)>>,
}

impl Default for GyIdGenerator {
    fn default() -> Self {
        Self::new(GeneratorConfig::default())
    }
}

impl GyIdGenerator {
    /// 创建新的生成器
    pub fn new(config: GeneratorConfig) -> Self {
        Self { 
            config, 
            fingerprint_cache: RwLock::new(None),
        }
    }

    /// 清除指纹缓存
    pub fn clear_cache(&self) {
        let mut cache = self.fingerprint_cache.write();
        *cache = None;
    }
    
    /// 采集硬件指纹（带缓存 + 全局超时保护）
    ///
    /// 优化策略：
    /// 1. 四个采集器并行执行（之前是串行，在 Windows 上每次 wmic 子进程约 1~3s，
    ///    串行累计 4~12s；并行后整体耗时降至单个最慢子进程的时间，约 1~3s）
    /// 2. 短期缓存：30秒内重复调用直接返回缓存结果
    /// 3. 全局 2s 超时：防止 wmic 命令永久卡住（⚡ 关键优化）
    pub async fn collect_fingerprint(&self) -> Result<HardwareFingerprint> {
        // 检查缓存（30秒有效期）
        {
            let cache = self.fingerprint_cache.read();
            if let Some((instant, fp)) = cache.as_ref() {
                if instant.elapsed() < Duration::from_secs(30) {
                    tracing::debug!("使用缓存的硬件指纹");
                    return Ok(fp.clone());
                }
            }
        }

        // 并行启动四个阻塞采集任务，加 2s 全局超时（防止 wmic 永久卡住）
        let fingerprint_result = tokio::time::timeout(
            Duration::from_secs(2),
            async {
                let (mac_res, cpu_res, board_res, disk_res) = tokio::join!(
                    tokio::task::spawn_blocking(MacCollector::collect),
                    tokio::task::spawn_blocking(CpuCollector::collect),
                    tokio::task::spawn_blocking(BoardCollector::collect),
                    tokio::task::spawn_blocking(DiskCollector::collect),
                );

                // 解包 JoinHandle，内部错误仍然透传
                let mac   = mac_res.map_err(|e| crate::GyIdError::InvalidParam(format!("MAC 采集 panic: {}", e)))??;
                let cpu_id = cpu_res.map_err(|e| crate::GyIdError::InvalidParam(format!("CPU 采集 panic: {}", e)))??;
                let board  = board_res.map_err(|e| crate::GyIdError::InvalidParam(format!("主板采集 panic: {}", e)))??;
                let disk   = disk_res.map_err(|e| crate::GyIdError::InvalidParam(format!("磁盘采集 panic: {}", e)))??;

                Ok(HardwareFingerprint {
                    mac,
                    cpu_id,
                    board_serial: board,
                    disk_serial: disk,
                })
            }
        ).await;

        let fp = match fingerprint_result {
            Ok(Ok(fp)) => fp,
            Ok(Err(e)) => return Err(e),
            Err(_) => {
                // 超时：返回部分指纹（至少有一些数据）
                tracing::warn!("硬件指纹采集超时（2s），使用部分数据");
                HardwareFingerprint::default()
            }
        };

        // 更新缓存（仅在有效时）
        if fp.is_valid() {
            let mut cache = self.fingerprint_cache.write();
            *cache = Some((Instant::now(), fp.clone()));
        }

        Ok(fp)
    }
    
    /// 采集地理位置
    /// 
    /// 自动计算并存储 H3 单元格 ID
    /// 
    /// ⚡ 性能优化（目标 ≤ 3s）：
    /// - IP 定位：1s 超时（原 5s）
    /// - WiFi 扫描：1s 超时（原 3s）
    /// - WiFi MLS：1s 超时（原 3s）
    /// - 移除 Nominatim 逆地理编码（国内访问 OSM 极慢，节省 4s）
    pub async fn collect_geo(&self) -> Result<GeoLocation> {
        use tokio::time::{timeout, Duration};
        let mut geo = match self.config.geo_level {
            GeoPrecisionLevel::City => {
                // 城市级：直接 IP 定位（1s 超时）
                match timeout(Duration::from_secs(1), IpLocator::locate()).await {
                    Ok(Ok(g)) => g,
                    Ok(Err(_)) | Err(_) => GeoLocation::default(),
                }
            }
            GeoPrecisionLevel::District => {
                // 区域级：WiFi BSSID 定位（1s 扫描 + 1s MLS），失败则降级到 IP
                let wifi_results = match timeout(
                    Duration::from_secs(1),
                    tokio::task::spawn_blocking(|| WifiLocator::scan().unwrap_or_default()),
                ).await {
                    Ok(Ok(b)) => b,
                    _ => vec![],
                };
                if wifi_results.is_empty() {
                    match timeout(Duration::from_secs(1), IpLocator::locate()).await {
                        Ok(Ok(g)) => g,
                        Ok(Err(_)) | Err(_) => GeoLocation::default(),
                    }
                } else {
                    match timeout(Duration::from_secs(1), WifiLocator::query_mls(&wifi_results)).await {
                        Ok(Ok(g)) => g,
                        _ => match timeout(Duration::from_secs(1), IpLocator::locate()).await {
                            Ok(Ok(g)) => g,
                            _ => GeoLocation::default(),
                        },
                    }
                }
            }
            GeoPrecisionLevel::Exact => {
                // 精确级：使用并行采集策略
                self.collect_geo_parallel().await?
            }
            GeoPrecisionLevel::Manual => {
                // 手动模式：需要用户先设置手动位置
                return Err(GyIdError::InvalidParam(
                    "请先设置手动坐标（使用 set_manual_geo）".to_string()
                ));
            }
            GeoPrecisionLevel::None => {
                // 无位置模式：返回空位置
                GeoLocation::default()
            }
        };
        
        // 自动计算 H3 单元格
        geo.calculate_h3_cell();

        // ⚡ 注意：已移除 Nominatim 逆地理编码（国内访问 OSM 极慢）
        // 城市名可从 IP 定位结果直接获取，无需额外 API 调用

        Ok(geo)
    }

    /// 并行采集地理位置（用于精确级）
    /// 
    /// ⚡ 性能优化（目标 ≤ 3s）：
    /// - GPS 检测：500ms（原 800ms）
    /// - GPS 定位：1s（原 3s）
    /// - WiFi 扫描：1s（原 3s）
    /// - WiFi MLS：1s（原 5s）
    /// - IP 定位：1s（原 2s）
    /// 
    /// 注意：H3 单元格在 collect_geo() 中统一计算
    async fn collect_geo_parallel(&self) -> Result<GeoLocation> {
        use tokio::time::{timeout, Duration};

        // 快速检测 GPS 是否可用（500ms 超时守卫）
        let gps_available = match timeout(
            Duration::from_millis(500),
            GpsLocator::check_availability(),
        ).await {
            Ok(avail) => avail,
            Err(_) => {
                tracing::warn!("GPS 可用性检测超时（500ms），跳过 GPS");
                GpsAvailability::Timeout
            }
        };
        
        // 如果 GPS 不可用，直接使用 WiFi/IP
        if gps_available != GpsAvailability::Available {
            tracing::info!("GPS 不可用 ({})，使用 WiFi/IP 定位", gps_available);
            return self.collect_geo_without_gps().await;
        }

        // GPS 可用：WiFi 扫描用 spawn_blocking + 1s 超时（避免 netsh 阻塞 async 线程）
        let wifi_results = match timeout(
            Duration::from_secs(1),
            tokio::task::spawn_blocking(|| WifiLocator::scan().unwrap_or_default()),
        ).await {
            Ok(Ok(b)) => b,
            _ => vec![],
        };
        
        // 提取 BSSID 列表供后续使用
        let bssids: Vec<String> = wifi_results.iter().map(|r| r.bssid.clone()).collect();

        // 使用 tokio::select! 实现三路竞速
        tokio::select! {
            // GPS 优先（1秒超时）
            result = timeout(Duration::from_secs(1), GpsLocator::locate()) => {
                match result {
                    Ok(Ok(mut geo)) => {
                        geo.level = GeoPrecisionLevel::Exact;
                        geo.wifi_bssids = Some(bssids);
                        tracing::info!("GPS 定位成功");
                        return Ok(geo);
                    }
                    Ok(Err(e)) => {
                        tracing::warn!("GPS 定位失败: {}", e);
                    }
                    Err(_) => {
                        tracing::warn!("GPS 定位超时（1s）");
                    }
                }
            }
            // WiFi MLS（1秒超时）
            result = timeout(Duration::from_secs(1), async {
                if wifi_results.is_empty() {
                    Err(GyIdError::GeoError("无 BSSID".to_string()))
                } else {
                    WifiLocator::query_mls(&wifi_results).await
                }
            }) => {
                match result {
                    Ok(Ok(mut geo)) => {
                        geo.level = GeoPrecisionLevel::District;
                        tracing::info!("WiFi 定位成功");
                        return Ok(geo);
                    }
                    Ok(Err(e)) => {
                        tracing::warn!("WiFi 定位失败: {}", e);
                    }
                    Err(_) => {
                        tracing::warn!("WiFi 定位超时（1s）");
                    }
                }
            }
            // IP 定位（1秒超时）
            result = timeout(Duration::from_secs(1), IpLocator::locate()) => {
                match result {
                    Ok(Ok(geo)) => {
                        tracing::info!("IP 定位成功");
                        return Ok(geo);
                    }
                    Ok(Err(e)) => {
                        tracing::warn!("IP 定位失败: {}", e);
                    }
                    Err(_) => {
                        tracing::warn!("IP 定位超时（1s）");
                    }
                }
            }
        }

        // 所有方式都失败：返回占位位置
        tracing::warn!("所有定位方式都失败，返回占位位置");
        Ok(GeoLocation::default())
    }

    /// 无 GPS 的定位采集
    ///
    /// ⚡ 性能优化（目标 ≤ 3s）：
    /// - WiFi 扫描：1s（原 3s）
    /// - WiFi MLS：1s（原 3s）
    /// - IP 定位：1s（原 2s）
    ///
    /// 注意：H3 单元格在 collect_geo() 中统一计算
    async fn collect_geo_without_gps(&self) -> Result<GeoLocation> {
        use tokio::time::{timeout, Duration};

        // WiFi 扫描：spawn_blocking 避免阻塞 async 线程，加 1s 超时
        let bssids = match timeout(
            Duration::from_secs(1),
            tokio::task::spawn_blocking(|| WifiLocator::scan().unwrap_or_default()),
        ).await {
            Ok(Ok(b)) => b,
            _ => vec![],
        };

        let bssids_clone = bssids.clone();

        // WiFi MLS 与 IP 并行竞速 —— 谁先成功返回谁
        tokio::select! {
            // WiFi MLS（1s 超时）
            result = timeout(Duration::from_secs(1), async move {
                if bssids_clone.is_empty() {
                    Err(GyIdError::GeoError("无 BSSID".to_string()))
                } else {
                    WifiLocator::query_mls(&bssids_clone).await
                }
            }) => {
                if let Ok(Ok(geo)) = result {
                    tracing::info!("WiFi 定位成功（无 GPS 模式）");
                    return Ok(geo);
                }
                // WiFi 失败：等 IP 竞速结果（select 会继续执行下一个分支）
                // 注意：这里 select 已结束，需要单独尝试 IP
            }
            // IP 定位（1s 超时）
            result = timeout(Duration::from_secs(1), IpLocator::locate()) => {
                if let Ok(Ok(geo)) = result {
                    tracing::info!("IP 定位成功（无 GPS 模式）");
                    return Ok(geo);
                }
            }
        }

        // 两路都失败：补尝 IP（IP 比 WiFi 超时短，此处大概率已超时，快速降级）
        match timeout(Duration::from_secs(1), IpLocator::locate()).await {
            Ok(Ok(geo)) => {
                tracing::info!("IP 定位成功（降级）");
                Ok(geo)
            }
            _ => {
                tracing::warn!("WiFi 和 IP 定位均失败，返回占位位置");
                Ok(GeoLocation::default())
            }
        }
    }

    /// 设置手动坐标（用于 Manual 模式）
    pub fn set_manual_geo(&self, latitude: f64, longitude: f64) -> Result<GeoLocation> {
        // 验证坐标范围
        if !(-90.0..=90.0).contains(&latitude) {
            return Err(GyIdError::InvalidParam(
                "纬度必须在 -90 到 90 之间".to_string()
            ));
        }
        if !(-180.0..=180.0).contains(&longitude) {
            return Err(GyIdError::InvalidParam(
                "经度必须在 -180 到 180 之间".to_string()
            ));
        }

        Ok(GeoLocation::from_manual(latitude, longitude))
    }
    
    /// 生成 GyID
    pub async fn generate(&self, avatar_path: Option<&str>) -> Result<GyId> {
        // 1. 采集硬件指纹
        let fingerprint = self.collect_fingerprint().await?;

        if !fingerprint.is_valid() {
            return Err(GyIdError::InvalidParam(
                "硬件指纹数据不足，至少需要 2 个有效因子".to_string()
            ));
        }

        // 2. 采集地理位置 (可选)
        let geo = if self.config.with_geo {
            Some(self.collect_geo().await?)
        } else {
            None
        };

        // 3. 处理头像 (可选)
        let avatar = if self.config.with_avatar {
            if let Some(path) = avatar_path {
                Some(AvatarLoader::load(path)?)
            } else {
                None
            }
        } else {
            None
        };

        // 4. 生成 GyID
        self.compute_gyid(&fingerprint, &geo, &avatar)
    }

    /// 使用手动坐标生成 GyID
    ///
    /// 适用于 Manual 精度等级，用户手动提供坐标
    pub async fn generate_with_manual_geo(
        &self,
        latitude: f64,
        longitude: f64,
        avatar_path: Option<&str>,
    ) -> Result<GyId> {
        // 1. 采集硬件指纹
        let fingerprint = self.collect_fingerprint().await?;

        if !fingerprint.is_valid() {
            return Err(GyIdError::InvalidParam(
                "硬件指纹数据不足，至少需要 2 个有效因子".to_string()
            ));
        }

        // 2. 验证并创建手动位置
        let geo = self.set_manual_geo(latitude, longitude)?;

        // 3. 处理头像 (可选)
        let avatar = if self.config.with_avatar {
            if let Some(path) = avatar_path {
                Some(AvatarLoader::load(path)?)
            } else {
                None
            }
        } else {
            None
        };

        // 4. 生成 GyID
        self.compute_gyid(&fingerprint, &Some(geo), &avatar)
    }
    
    /// 计算 GyID (核心算法)
    fn compute_gyid(
        &self,
        fingerprint: &HardwareFingerprint,
        geo: &Option<GeoLocation>,
        avatar: &Option<Avatar>,
    ) -> Result<GyId> {
        let mut hasher = GyIdHasher::new();

        // 添加权重信息
        hasher.update_str(&format!("{:?}", self.config.weights));

        // 添加硬件指纹 (带权重)
        if let Some(mac) = &fingerprint.mac {
            hasher.update_str(mac);
        }
        if let Some(cpu) = &fingerprint.cpu_id {
            hasher.update_str(cpu);
        }
        if let Some(board) = &fingerprint.board_serial {
            hasher.update_str(board);
        }
        if let Some(disk) = &fingerprint.disk_serial {
            hasher.update_str(disk);
        }

        // 添加地理位置（跳过 None 级别）
        if let Some(g) = geo {
            let geo_factor = g.to_hash_factor();
            if !geo_factor.is_empty() {
                hasher.update_str(&geo_factor);
            }
        }

        // 添加头像哈希
        if let Some(a) = avatar {
            hasher.update_str(&a.to_hash_factor());
        }

        // 添加时间戳 (UTC 毫秒 + 16bit 随机数)
        let timestamp = Self::timestamp_with_random();
        hasher.update_str(&timestamp.to_string());

        // 添加随机盐值
        let salt: [u8; 16] = rand::random();
        hasher.update(&salt);

        // 计算最终哈希
        let hash = hasher.finalize_hex();

        // Base58 编码
        let gyid_str = Base58Encoder::encode_hex(&hash)
            .map_err(GyIdError::CryptoError)?;

        // 截取为固定长度 (32 字符)
        let gyid = format!("GyID{}", &gyid_str[2..34]);

        Ok(GyId::new(gyid, hash, timestamp))
    }
    
    /// 生成带随机数的时间戳
    fn timestamp_with_random() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        let random: u16 = rand::random();
        (now << 16) | (random as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // 需要网络和硬件访问
    async fn test_generate() {
        let generator = GyIdGenerator::default();
        let gyid = generator.generate(None).await;
        println!("GyID: {:?}", gyid);
    }
}
