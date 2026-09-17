//! 存储模块
//!
//! 提供本地 SQLite 存储和链上锚定功能

mod local;
pub mod chain;
pub mod aptos;

pub use local::LocalStorage;
pub use chain::ChainAnchor;
pub use chain::Blockchain;

use serde::{Deserialize, Serialize};

/// 链上锚定数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GyIdAnchor {
    /// GyID 哈希值
    pub gyid_hash: String,
    /// 创建时间戳
    pub created_at: u64,
    /// 元数据哈希
    pub metadata_hash: String,
    /// 关联设备数量
    pub linked_count: u32,
    /// 链上交易哈希
    pub tx_hash: Option<String>,
    /// 区块高度
    pub block_height: Option<u64>,
    /// 目标链
    pub chain: Option<String>,
}

impl GyIdAnchor {
    /// 创建新的链上锚定
    pub fn new(gyid_hash: String, metadata_hash: String, linked_count: u32) -> Self {
        Self {
            gyid_hash,
            created_at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64,
            metadata_hash,
            linked_count,
            tx_hash: None,
            block_height: None,
            chain: None,
        }
    }

    /// 设置交易哈希
    pub fn with_tx_hash(mut self, tx_hash: String) -> Self {
        self.tx_hash = Some(tx_hash);
        self
    }

    /// 设置区块高度
    pub fn with_block_height(mut self, height: u64) -> Self {
        self.block_height = Some(height);
        self
    }

    /// 设置目标链
    pub fn with_chain(mut self, chain: &str) -> Self {
        self.chain = Some(chain.to_string());
        self
    }
}

/// 存储后端类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StorageBackend {
    /// 仅本地存储
    LocalOnly,
    /// 本地 + 链上锚定
    LocalWithChain,
}
