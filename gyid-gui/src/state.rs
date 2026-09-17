// src/state.rs — GyID GUI 状态管理
// 所有非 UI 的状态、设置、历史记录管理逻辑

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// GyID 生成状态（Rust 端）
#[derive(Debug, Clone, PartialEq)]
pub enum GenStatus {
    Idle,
    CollectingHardware,
    CollectingGeo,
    Hashing,
    Done(String),   // GyID 字符串
    Error(String),  // 错误信息
    Cancelled,
    Paused,
    Timeout(String),
}

impl Default for GenStatus {
    fn default() -> Self {
        Self::Idle
    }
}

impl GenStatus {
    pub fn is_running(&self) -> bool {
        matches!(
            self,
            GenStatus::CollectingHardware
                | GenStatus::CollectingGeo
                | GenStatus::Hashing
                | GenStatus::Paused
        )
    }

    pub fn as_slint_str(&self) -> &'static str {
        match self {
            GenStatus::Idle => "idle",
            GenStatus::CollectingHardware => "collecting_hw",
            GenStatus::CollectingGeo => "collecting_geo",
            GenStatus::Hashing => "hashing",
            GenStatus::Done(_) => "done",
            GenStatus::Error(_) => "error",
            GenStatus::Cancelled => "cancelled",
            GenStatus::Paused => "paused",
            GenStatus::Timeout(_) => "timeout",
        }
    }
}

/// 地理精度等级
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum GeoPrecision {
    City,
    District,
    Exact,
}

impl Default for GeoPrecision {
    fn default() -> Self {
        Self::City
    }
}

impl GeoPrecision {
    pub fn as_str(&self) -> &'static str {
        match self {
            GeoPrecision::City => "city",
            GeoPrecision::District => "district",
            GeoPrecision::Exact => "exact",
        }
    }
    pub fn from_str(s: &str) -> Self {
        match s {
            "city" => GeoPrecision::City,
            "district" => GeoPrecision::District,
            "exact" => GeoPrecision::Exact,
            _ => GeoPrecision::City,
        }
    }
    pub fn label(&self) -> &'static str {
        match self {
            GeoPrecision::City => "城市级 (IP定位)",
            GeoPrecision::District => "区域级 (WiFi)",
            GeoPrecision::Exact => "精确级 (GPS)",
        }
    }
}

/// 历史记录条目
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub gyid: String,
    pub created_at: String,
    pub geo_level: String,
    pub has_avatar: bool,
}

/// 应用设置（持久化到本地 JSON）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    pub geo_precision: GeoPrecision,
    pub use_avatar: bool,
    pub avatar_path: Option<String>,
    pub weight_hardware: f32,
    pub weight_geo: f32,
    pub weight_time: f32,
    pub weight_avatar: f32,
    pub dark_mode: bool,
    pub last_gyid: Option<String>,
    pub last_generated_at: Option<String>,
    pub gyid_history: Vec<HistoryEntry>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            geo_precision: GeoPrecision::City,
            use_avatar: false,
            avatar_path: None,
            weight_hardware: 0.35,
            weight_geo: 0.25,
            weight_time: 0.15,
            weight_avatar: 0.25,
            dark_mode: true,
            last_gyid: None,
            last_generated_at: None,
            gyid_history: Vec::new(),
        }
    }
}

/// 历史记录条目（Slint UI 用）
#[derive(Debug, Clone)]
pub struct HistoryItem {
    pub id: String,
    pub time: String,
    pub geo_level: String,
}

impl AppSettings {
    /// 配置文件目录
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("gyid")
    }

    fn settings_path() -> PathBuf {
        Self::config_dir().join("gui_settings.json")
    }

    /// 加载本地设置
    pub fn load() -> anyhow::Result<Self> {
        let path = Self::settings_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(s) = serde_json::from_str::<AppSettings>(&content) {
                    return Ok(s);
                }
            }
        }
        Ok(Self::default())
    }

    /// 保存设置到本地
    pub fn save(&self) -> anyhow::Result<()> {
        let dir = Self::config_dir();
        std::fs::create_dir_all(&dir)?;
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(Self::settings_path(), json)?;
        Ok(())
    }

    /// 验证权重之和
    pub fn weights_valid(&self) -> bool {
        let sum = self.weight_hardware + self.weight_geo + self.weight_time + self.weight_avatar;
        (sum - 1.0f32).abs() < 0.05
    }

    /// 归一化权重
    pub fn normalize_weights(&mut self) {
        let sum = self.weight_hardware + self.weight_geo + self.weight_time + self.weight_avatar;
        if sum > 0.0 && (sum - 1.0).abs() > 0.01 {
            self.weight_hardware /= sum;
            self.weight_geo /= sum;
            self.weight_time /= sum;
            self.weight_avatar /= sum;
        }
    }

    /// 添加 GyID 到历史记录
    pub fn add_to_history(&mut self, gyid: String, geo_level: String, has_avatar: bool) {
        let now = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();

        // 去重：移除相同 GyID
        self.gyid_history.retain(|e| e.gyid != gyid);

        // 添加到开头
        self.gyid_history.insert(0, HistoryEntry {
            gyid: gyid.clone(),
            created_at: now.clone(),
            geo_level,
            has_avatar,
        });

        // 限制 20 条
        if self.gyid_history.len() > 20 {
            self.gyid_history.truncate(20);
        }

        self.last_gyid = Some(gyid);
        self.last_generated_at = Some(now);
    }

    /// 获取用于 UI 显示的历史记录
    pub fn history(&self) -> Vec<HistoryItem> {
        self.gyid_history.iter().map(|e| HistoryItem {
            id: e.gyid.clone(),
            time: e.created_at.clone(),
            geo_level: e.geo_level.clone(),
        }).collect()
    }
}

/// 线程安全的应用状态包装器
pub struct AppState {
    pub settings: Mutex<AppSettings>,
    pub generation_status: Mutex<GenStatus>,
    pub cancel_flag: Mutex<bool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            settings: Mutex::new(AppSettings::default()),
            generation_status: Mutex::new(GenStatus::Idle),
            cancel_flag: Mutex::new(false),
        }
    }
}

impl AppState {
    pub fn new(initial: AppSettings) -> Self {
        Self {
            settings: Mutex::new(initial),
            generation_status: Mutex::new(GenStatus::Idle),
            cancel_flag: Mutex::new(false),
        }
    }

    pub fn save_settings(&self) -> anyhow::Result<()> {
        self.settings.lock().unwrap().save()
    }

    pub fn get_gyid_generator_config(&self) -> gyid_core::identity::GeneratorConfig {
        let s = self.settings.lock().unwrap();
        let precision = match s.geo_precision {
            GeoPrecision::City => gyid_core::geo::GeoPrecisionLevel::City,
            GeoPrecision::District => gyid_core::geo::GeoPrecisionLevel::District,
            GeoPrecision::Exact => gyid_core::geo::GeoPrecisionLevel::Exact,
        };
        gyid_core::identity::GeneratorConfig {
            geo_level: precision,
            weights: gyid_core::crypto::GyIdWeights::default(),
            with_avatar: s.use_avatar,
            with_geo: true,
        }
    }
}

/// 生成结果（从异步任务传回）
#[derive(Debug, Clone)]
pub struct GenerateResult {
    pub gyid: Option<String>,
    pub error: Option<String>,
    pub status: GenStatus,
}
