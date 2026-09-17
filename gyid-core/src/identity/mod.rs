//! 身份生成核心模块
//!
//! 提供 GyID 生成和验证功能

mod generator;
mod validator;
mod config;

pub use generator::GyIdGenerator;
pub use generator::GeneratorConfig;
pub use validator::GyIdValidator;
pub use config::GyIdConfig;

use serde::{Deserialize, Serialize};

/// GyID 账号结构
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GyId {
    /// GyID 字符串 (Base58 编码)
    pub id: String,
    /// GyID 原始哈希
    pub hash: String,
    /// 创建时间戳 (UTC 毫秒)
    pub created_at: u64,
    /// 关联设备数量
    pub linked_devices: u32,
}

impl GyId {
    /// 创建新的 GyID
    pub fn new(id: String, hash: String, created_at: u64) -> Self {
        Self {
            id,
            hash,
            created_at,
            linked_devices: 0,
        }
    }

    /// GyID 格式验证
    pub fn is_valid_format(&self) -> bool {
        // Base58 格式: 32-48 个字符，只包含 A-Z, a-z, 0-9, -, _
        if self.id.len() < 32 || self.id.len() > 48 {
            return false;
        }
        self.id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gyid_format() {
        // "GyID" + 28字符 = 32字符
        let gyid = GyId::new(
            "GyID1234567890abcdefghijklmnopqr".to_string(),
            "abc123".to_string(),
            1711814400000,
        );
        assert!(gyid.is_valid_format());
    }

    #[test]
    fn test_gyid_invalid_format() {
        let gyid = GyId::new(
            "short".to_string(),
            "abc123".to_string(),
            1711814400000,
        );
        assert!(!gyid.is_valid_format());
    }
}
