//! GyID 配置模块

use serde::{Deserialize, Serialize};
use crate::crypto::GyIdWeights;
use crate::geo::GeoPrecisionLevel;

/// GyID 配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GyIdConfig {
    /// 地理位置精度等级
    pub geo_level: GeoPrecisionLevel,
    /// GyID 权重配置
    pub weights: GyIdWeights,
    /// 是否启用头像
    pub enable_avatar: bool,
    /// 是否启用地理位置
    pub enable_geo: bool,
    /// GyID 版本
    pub version: String,
}

impl Default for GyIdConfig {
    fn default() -> Self {
        Self {
            geo_level: GeoPrecisionLevel::City,
            weights: GyIdWeights::default(),
            enable_avatar: true,
            enable_geo: true,
            version: "1.0".to_string(),
        }
    }
}

impl GyIdConfig {
    /// 创建新配置
    pub fn new() -> Self {
        Self::default()
    }
    
    /// 设置地理位置精度
    pub fn with_geo_level(mut self, level: GeoPrecisionLevel) -> Self {
        self.geo_level = level;
        self
    }
    
    /// 设置权重
    pub fn with_weights(mut self, weights: GyIdWeights) -> Self {
        self.weights = weights;
        self
    }
    
    /// 启用/禁用头像
    pub fn with_avatar(mut self, enabled: bool) -> Self {
        self.enable_avatar = enabled;
        self
    }
    
    /// 启用/禁用地理位置
    pub fn with_geo(mut self, enabled: bool) -> Self {
        self.enable_geo = enabled;
        self
    }
    
    /// 转换为 JSON 字符串
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }
    
    /// 从 JSON 字符串加载
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config() {
        let config = GyIdConfig::new()
            .with_geo_level(GeoPrecisionLevel::Exact)
            .with_avatar(true);
        
        let json = config.to_json().unwrap();
        println!("Config JSON: {}", json);
        
        let loaded = GyIdConfig::from_json(&json).unwrap();
        assert_eq!(loaded.geo_level, GeoPrecisionLevel::Exact);
    }
}
