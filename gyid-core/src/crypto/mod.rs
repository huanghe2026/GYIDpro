//! 加密模块
//!
//! 提供哈希运算和编码工具

mod hasher;
mod encoder;

pub use hasher::GyIdHasher;
pub use encoder::Base58Encoder;

use serde::{Deserialize, Serialize};

/// GyID 配置权重
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GyIdWeights {
    /// 硬件指纹权重
    pub hardware: f32,
    /// 地理位置权重
    pub geo: f32,
    /// 时间戳权重
    pub timestamp: f32,
    /// 头像权重
    pub avatar: f32,
}

impl Default for GyIdWeights {
    fn default() -> Self {
        Self {
            hardware: 0.35,
            geo: 0.25,
            timestamp: 0.15,
            avatar: 0.25,
        }
    }
}

impl GyIdWeights {
    /// 验证权重总和是否等于 1.0
    pub fn is_valid(&self) -> bool {
        let sum = self.hardware + self.geo + self.timestamp + self.avatar;
        (sum - 1.0).abs() < 0.001
    }

    /// 从权重生成因子数据
    pub fn to_factor_bytes(&self) -> [u8; 16] {
        // 将四个 f32 权重打包成 16 字节
        let hw = (self.hardware * 1000.0) as u16;
        let gw = (self.geo * 1000.0) as u16;
        let tw = (self.timestamp * 1000.0) as u16;
        let aw = (self.avatar * 1000.0) as u16;
        
        [
            (hw >> 8) as u8, hw as u8,
            (gw >> 8) as u8, gw as u8,
            (tw >> 8) as u8, tw as u8,
            (aw >> 8) as u8, aw as u8,
            0, 0, 0, 0, 0, 0, 0, 0,
        ]
    }
}
