//! SDK 数据模型
//!
//! 跨平台统一的数据结构，设计为可序列化为 JSON（便于 FFI 传递）

use serde::{Deserialize, Serialize};

/// GyID 生成选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateOptions {
    /// 地理位置精度等级
    /// 可选值: "city" | "district" | "exact"
    /// 默认: "city"
    pub geo_level: Option<String>,

    /// 是否采集地理位置（默认 true）
    pub with_geo: Option<bool>,

    /// 头像图片路径（可选，本地平台）
    pub avatar_path: Option<String>,

    /// 头像图片字节数据（可选，用于 WASM/移动端）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avatar_bytes: Option<Vec<u8>>,

    /// 权重配置（可选，默认 hardware=0.35, geo=0.25, timestamp=0.15, avatar=0.25）
    pub weights: Option<WeightsOptions>,
}

impl Default for GenerateOptions {
    fn default() -> Self {
        Self {
            geo_level: Some("city".to_string()),
            with_geo: Some(true),
            avatar_path: None,
            avatar_bytes: None,
            weights: None,
        }
    }
}

/// 权重配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeightsOptions {
    pub hardware: f32,
    pub geo: f32,
    pub timestamp: f32,
    pub avatar: f32,
}

/// GyID 生成结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GyIdResult {
    /// GyID 字符串（Base58 编码，以 "GyID" 为前缀）
    pub id: String,
    /// 原始哈希值（十六进制）
    pub hash: String,
    /// 创建时间戳（毫秒，从 created_at 字段右移 16 位得到）
    pub created_at_ms: i64,
    /// 关联设备数量
    pub linked_devices: u32,
    /// 生成平台
    pub platform: String,
    /// SDK 版本
    pub sdk_version: String,
}

impl GyIdResult {
    /// 将结果序列化为 JSON 字符串
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// 从 JSON 字符串反序列化
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// 设备关联选项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkOptions {
    /// 授权码（主设备传 "gen" 生成，从设备传实际的授权码）
    pub auth_code: String,
    /// 主设备 GyID（从设备模式需要）
    pub master_gyid: Option<String>,
}

/// 设备关联结果
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LinkResult {
    /// 本机 GyID
    pub local_gyid: String,
    /// 主设备 GyID
    pub master_gyid: String,
    /// 关联 ID
    pub link_id: String,
    /// 关联时间戳（毫秒）
    pub linked_at_ms: i64,
    /// 是否为新生成的 GyID
    pub is_new_gyid: bool,
}

/// 授权码生成结果（主设备模式）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthCodeResult {
    /// 生成的授权码
    pub auth_code: String,
    /// 主设备 GyID
    pub master_gyid: String,
    /// 有效期（秒，默认 600s = 10 分钟）
    pub expires_in_secs: u64,
    /// 建议的从设备完成命令
    pub complete_command: String,
}

/// 关联设备列表
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceList {
    /// 主设备 GyID
    pub master_gyid: String,
    /// 关联设备列表
    pub devices: Vec<DeviceInfo>,
}

/// 单个关联设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceInfo {
    /// 从设备 GyID
    pub linked_gyid: String,
    /// 关联 ID
    pub link_id: String,
    /// 关联时间戳（毫秒）
    pub linked_at_ms: i64,
    /// 是否活跃
    pub active: bool,
}

/// SDK 版本信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SdkVersion {
    pub version: String,
    pub platform: String,
    pub features: Vec<String>,
}

/// 获取当前运行平台标识
pub fn current_platform() -> &'static str {
    #[cfg(target_arch = "wasm32")]
    return "web-wasm";
    #[cfg(target_os = "android")]
    return "android";
    #[cfg(target_os = "ios")]
    return "ios";
    #[cfg(target_os = "windows")]
    return "windows";
    #[cfg(target_os = "macos")]
    return "macos";
    #[cfg(target_os = "linux")]
    return "linux";
    #[cfg(not(any(
        target_arch = "wasm32",
        target_os = "android",
        target_os = "ios",
        target_os = "windows",
        target_os = "macos",
        target_os = "linux"
    )))]
    return "unknown";
}
