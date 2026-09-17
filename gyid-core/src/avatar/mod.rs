//! 头像处理模块
//!
//! 提供头像图片加载、预处理和哈希计算

mod loader;
mod hasher;

pub use loader::AvatarLoader;
pub use hasher::AvatarHasher;

use serde::{Deserialize, Serialize};

/// 头像数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Avatar {
    /// 原始图片路径
    pub path: Option<String>,
    /// 图片数据哈希 (用于 GyID 计算)
    pub hash: String,
    /// 图片尺寸 (宽, 高)
    pub dimensions: Option<(u32, u32)>,
    /// 图片格式
    pub format: Option<String>,
}

impl Avatar {
    /// 生成用于 GyID 的头像哈希因子
    pub fn to_hash_factor(&self) -> String {
        self.hash.clone()
    }
}
