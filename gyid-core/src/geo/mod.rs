//! 地理位置模块
//!
//! 提供多精度地理位置获取：IP定位、WiFi BSSID定位、GPS定位
//!
//! 集成 Uber H3 六边形网格系统，支持 16 个精度级别
//! - H3 Level 4 (~252km²): 城市级别
//! - H3 Level 7 (~38km²):  区域级别
//! - H3 Level 9 (~0.74km²): 精确区域

mod ip;
mod gps;
mod wifi;
pub mod h3;
pub mod nominatim;

pub use ip::IpLocator;
pub use gps::{GpsLocator, GpsAvailability};
pub use wifi::WifiLocator;
pub use h3::{H3Grid, H3Cell};
pub use nominatim::{NominatimGeocoder, ReverseGeoResult};

use serde::{Deserialize, Serialize};

/// 地理位置精度等级
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GeoPrecisionLevel {
    /// L1: 城市级 (1-50km)
    #[default]
    City,
    /// L2: 区域级 (100m-1km)
    District,
    /// L3: 精确级 (10-30m)
    Exact,
    /// L4: 手动输入坐标（用户自行提供）
    Manual,
    /// L5: 仅使用硬件指纹，不采集位置
    None,
}

impl GeoPrecisionLevel {
    /// 获取等级名称
    pub fn name(&self) -> &'static str {
        match self {
            GeoPrecisionLevel::City => "城市级",
            GeoPrecisionLevel::District => "区域级",
            GeoPrecisionLevel::Exact => "精确级",
            GeoPrecisionLevel::Manual => "手动输入",
            GeoPrecisionLevel::None => "不使用位置",
        }
    }

    /// 获取等级描述
    pub fn description(&self) -> &'static str {
        match self {
            GeoPrecisionLevel::City => "基于 IP 定位，精度 1-50km",
            GeoPrecisionLevel::District => "基于 WiFi BSSID 定位，精度 100m-1km",
            GeoPrecisionLevel::Exact => "基于 GPS + WiFi 混合定位，精度 10-30m",
            GeoPrecisionLevel::Manual => "手动输入坐标，精度由您决定",
            GeoPrecisionLevel::None => "不使用位置信息，仅依赖硬件指纹",
        }
    }

    /// 获取等级精度描述
    pub fn precision(&self) -> &'static str {
        match self {
            GeoPrecisionLevel::City => "1-50km",
            GeoPrecisionLevel::District => "100m-1km",
            GeoPrecisionLevel::Exact => "10-30m",
            GeoPrecisionLevel::Manual => "用户决定",
            GeoPrecisionLevel::None => "无",
        }
    }

    /// 获取速度描述
    pub fn speed(&self) -> &'static str {
        match self {
            GeoPrecisionLevel::City => "快速",
            GeoPrecisionLevel::District => "中等",
            GeoPrecisionLevel::Exact => "较慢",
            GeoPrecisionLevel::Manual => "快速",
            GeoPrecisionLevel::None => "最快",
        }
    }

    /// 获取隐私等级描述
    pub fn privacy(&self) -> &'static str {
        match self {
            GeoPrecisionLevel::City => "高",
            GeoPrecisionLevel::District => "中",
            GeoPrecisionLevel::Exact => "低",
            GeoPrecisionLevel::Manual => "最高",
            GeoPrecisionLevel::None => "最高",
        }
    }
}

/// 地理位置数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoLocation {
    /// 精度等级
    pub level: GeoPrecisionLevel,
    /// 纬度
    pub latitude: Option<f64>,
    /// 经度
    pub longitude: Option<f64>,
    /// 城市名称
    pub city: Option<String>,
    /// 国家/地区
    pub country: Option<String>,
    /// WiFi BSSID 列表 (用于 L2/L3)
    pub wifi_bssids: Option<Vec<String>>,
    /// H3 单元格 ID（基于当前精度等级）
    #[serde(default)]
    pub h3_cell: Option<String>,
    /// H3 分辨率级别
    #[serde(default)]
    pub h3_resolution: Option<u8>,
}

impl Default for GeoLocation {
    fn default() -> Self {
        Self {
            level: GeoPrecisionLevel::City,
            latitude: None,
            longitude: None,
            city: None,
            country: None,
            wifi_bssids: None,
            h3_cell: None,
            h3_resolution: None,
        }
    }
}

impl GeoLocation {
    /// 创建基础位置（不含 H3 单元格）
    /// 
    /// H3 单元格会在 collect_geo() 中自动计算
    pub fn partial(
        level: GeoPrecisionLevel,
        latitude: Option<f64>,
        longitude: Option<f64>,
        city: Option<String>,
        country: Option<String>,
        wifi_bssids: Option<Vec<String>>,
    ) -> Self {
        Self {
            level,
            latitude,
            longitude,
            city,
            country,
            wifi_bssids,
            h3_cell: None,
            h3_resolution: None,
        }
    }
    /// 生成用于 GyID 的地理位置哈希因子
    ///
    /// 优先使用 H3 单元格 ID，兼容降级到原始坐标
    pub fn to_hash_factor(&self) -> String {
        // 优先使用 H3 单元格 ID（如果有）
        if let (Some(cell_id), Some(res)) = (&self.h3_cell, self.h3_resolution) {
            return h3::to_hash_factor(cell_id, res);
        }

        // 降级：使用原始坐标
        match &self.level {
            GeoPrecisionLevel::City => {
                // 城市级：使用城市名称哈希
                self.city.as_deref().unwrap_or("unknown").to_string()
            }
            GeoPrecisionLevel::District => {
                // 区域级：使用粗精度坐标 + WiFi BSSID 前缀
                let coord = format!("{:.2},{:.2}",
                    self.latitude.unwrap_or(0.0),
                    self.longitude.unwrap_or(0.0)
                );
                if let Some(bssids) = &self.wifi_bssids {
                    let prefix = bssids.first()
                        .map(|b| &b[..b.len().min(8)])
                        .unwrap_or("00000000");
                    format!("{}:{}", coord, prefix)
                } else {
                    coord
                }
            }
            GeoPrecisionLevel::Exact => {
                // 精确级：使用完整坐标
                format!("{:.6},{:.6}",
                    self.latitude.unwrap_or(0.0),
                    self.longitude.unwrap_or(0.0)
                )
            }
            GeoPrecisionLevel::Manual => {
                // 手动输入：使用用户提供的坐标（最高精度）
                format!("manual:{:.8},{:.8}",
                    self.latitude.unwrap_or(0.0),
                    self.longitude.unwrap_or(0.0)
                )
            }
            GeoPrecisionLevel::None => {
                // 不使用位置：返回空标记（将在 GyID 计算中被忽略）
                String::new()
            }
        }
    }

    /// 从手动输入创建位置
    pub fn from_manual(latitude: f64, longitude: f64) -> Self {
        let mut loc = Self {
            level: GeoPrecisionLevel::Manual,
            latitude: Some(latitude),
            longitude: Some(longitude),
            city: None,
            country: None,
            wifi_bssids: None,
            h3_cell: None,
            h3_resolution: None,
        };
        // 自动计算 H3 单元格
        loc.calculate_h3_cell();
        loc
    }

    /// 计算 H3 单元格（基于当前精度等级）
    pub fn calculate_h3_cell(&mut self) {
        if let (Some(lat), Some(lon)) = (self.latitude, self.longitude) {
            let resolution = match self.level {
                GeoPrecisionLevel::City => 4,     // ~252km²
                GeoPrecisionLevel::District => 7, // ~38km²
                GeoPrecisionLevel::Exact => 9,     // ~0.74km² (约 860 米)
                GeoPrecisionLevel::Manual => 9,   // 手动输入默认使用精确级
                GeoPrecisionLevel::None => return, // 不使用位置
            };

            if let Some(cell_id) = H3Grid::latlng_to_cell(lat, lon, resolution) {
                self.h3_cell = Some(cell_id);
                self.h3_resolution = Some(resolution);
            }
        }
    }

    /// 设置 H3 单元格（用于测试或手动指定）
    pub fn with_h3_cell(mut self, cell_id: String, resolution: u8) -> Self {
        self.h3_cell = Some(cell_id);
        self.h3_resolution = Some(resolution);
        self
    }
}
