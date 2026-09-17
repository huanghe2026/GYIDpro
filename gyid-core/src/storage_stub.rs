//! Storage stub for WASM targets
//!
//! 在 WASM 平台上，SQLite 不可用，提供类型占位符保持 API 兼容性

use serde::{Deserialize, Serialize};

/// 链上锚定数据（WASM 平台仅提供数据结构，不实际存储）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GyIdAnchor {
    pub gyid_hash: String,
    pub created_at: u64,
    pub metadata_hash: String,
    pub linked_count: u32,
    pub tx_hash: Option<String>,
    pub block_height: Option<u64>,
}

impl GyIdAnchor {
    pub fn new(gyid_hash: String, metadata_hash: String, linked_count: u32) -> Self {
        Self {
            gyid_hash,
            created_at: 0, // WASM 不需要精确时间戳
            metadata_hash,
            linked_count,
            tx_hash: None,
            block_height: None,
        }
    }
}
