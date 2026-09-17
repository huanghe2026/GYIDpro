// ═══════════════════════════════════════════════════════════════════════════
// 应用状态模块
// ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use geoyuan_core::device::DeviceLink;
use crate::achievement::AchievementManager;
use crate::energy::{EnergyData, TaskManager};

/// 铸造位置记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MintLocation {
    /// GyID
    pub gyid: String,
    /// 城市名称
    pub city: String,
    /// 地址
    pub address: String,
    /// 纬度
    pub lat: f64,
    /// 经度
    pub lon: f64,
    /// 铸造时间戳
    pub timestamp: u64,
}

/// 应用状态
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppState {
    /// 当前 GyID
    pub gy_id: Option<String>,
    /// 当前钱包地址
    pub wallet_address: Option<String>,
    /// 照片路径
    pub photo_path: Option<String>,
    /// Geoyuan 余额
    pub balance: f64,
    /// 是否已初始化
    pub initialized: bool,
    /// 网络类型
    pub network: NetworkType,
    /// 设置
    pub settings: AppSettings,
    /// 交易历史
    pub transactions: Vec<TransactionRecord>,
    /// 关联的 GyID 列表
    pub linked_gyids: Vec<String>,
    /// 关联设备记录（持久化）
    #[serde(default)]
    pub linked_devices: Vec<DeviceLink>,
    /// 铸造位置记录（用于探索页）
    #[serde(default)]
    pub mint_locations: Vec<MintLocation>,
    /// 节点状态
    pub node_status: NodeStatus,
    /// Ed25519 私钥（hex），首次生成时创建并持久化
    #[serde(default)]
    pub secret_key_hex: Option<String>,

    /// 成就管理器
    #[serde(default)]
    pub achievement_manager: AchievementManager,

    /// 能量数据
    #[serde(default)]
    pub energy_data: EnergyData,

    /// 每日任务管理器
    #[serde(default)]
    pub task_manager: TaskManager,
}

/// 交易记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransactionRecord {
    /// 交易哈希
    pub tx_hash: String,
    /// 交易类型
    pub tx_type: TransactionType,
    /// 金额 (GY)
    pub amount: f64,
    /// 发送方
    pub from: String,
    /// 接收方
    pub to: String,
    /// 时间戳
    pub timestamp: u64,
    /// 状态
    pub status: TxStatus,
    /// 备注
    pub memo: Option<String>,
}

/// 交易类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TransactionType {
    /// GyID 铸造
    Mint,
    /// GY 转账
    Transfer,
    /// 设备关联
    Link,
    /// 奖励
    Reward,
}

impl std::fmt::Display for TransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TransactionType::Mint => write!(f, "铸造"),
            TransactionType::Transfer => write!(f, "转账"),
            TransactionType::Link => write!(f, "关联"),
            TransactionType::Reward => write!(f, "奖励"),
        }
    }
}

/// 交易状态
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TxStatus {
    Pending,
    Confirmed,
    Failed,
}

impl std::fmt::Display for TxStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TxStatus::Pending => write!(f, "待确认"),
            TxStatus::Confirmed => write!(f, "已确认"),
            TxStatus::Failed => write!(f, "失败"),
        }
    }
}

/// 节点状态
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NodeStatus {
    /// 是否在线
    pub online: bool,
    /// 连接的对等节点数
    pub peer_count: usize,
    /// 当前区块高度
    pub block_height: u64,
    /// 同步状态
    pub sync_status: SyncStatus,
    /// 联邦 ID
    pub federation_id: Option<String>,
}

/// 同步状态
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum SyncStatus {
    Syncing,
    Synced,
    #[default]
    Offline,
}


/// 网络类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum NetworkType {
    Mainnet,
    #[default]
    Testnet,
}



impl NetworkType {
    #[allow(dead_code)]
    pub fn prefix(&self) -> &'static str {
        match self {
            NetworkType::Mainnet => "GyID",
            NetworkType::Testnet => "TGyID",
        }
    }
}

/// 应用设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    /// 自动锁定 (秒)
    pub auto_lock_seconds: u32,
    /// 允许自动备份
    pub auto_backup: bool,
    /// 备份路径
    pub backup_path: Option<String>,
    /// 高德 API Key
    pub amap_key: Option<String>,
    /// Polygon RPC 地址
    #[serde(default = "default_polygon_rpc")]
    pub polygon_rpc: String,
    /// 语言
    pub language: String,
    /// 主题
    pub theme: Theme,
}

fn default_polygon_rpc() -> String {
    "https://polygon-rpc.com".to_string()
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            auto_lock_seconds: 300,
            auto_backup: true,
            backup_path: None,
            amap_key: None,
            polygon_rpc: default_polygon_rpc(),
            language: "zh-CN".to_string(),
            theme: Theme::Dark,
        }
    }
}

/// 主题
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub enum Theme {
    #[default]
    Dark,
    Light,
}


impl AppState {
    /// 创建新状态
    pub fn new() -> Self {
        Self::default()
    }

    /// 检查是否有账户
    #[allow(dead_code)]
    pub fn has_account(&self) -> bool {
        self.gy_id.is_some()
    }

    /// 获取 GyID (如果存在)
    #[allow(dead_code)]
    pub fn get_gyid(&self) -> Option<&str> {
        self.gy_id.as_deref()
    }

    /// 获取钱包地址 (如果不存在则生成)
    #[allow(dead_code)]
    pub fn get_or_create_wallet_address(&mut self) -> String {
        if let Some(ref addr) = self.wallet_address {
            addr.clone()
        } else {
            // 基于 GyID 生成地址
            let addr = self.generate_address();
            self.wallet_address = Some(addr.clone());
            addr
        }
    }

    /// 生成地址 (简化版，实际应使用加密哈希)
    fn generate_address(&self) -> String {
        if let Some(ref gyid) = self.gy_id {
            format!("0x{}", &blake3::hash(gyid.as_bytes()).to_hex()[..40])
        } else {
            "0x0000000000000000000000000000000000000000".to_string()
        }
    }

    /// 添加交易记录
    #[allow(dead_code)]
    pub fn add_transaction(&mut self, tx: TransactionRecord) {
        self.transactions.push(tx);
        // 按时间倒序排列
        self.transactions.sort_by_key(|b| std::cmp::Reverse(b.timestamp));
    }

    /// 获取最近交易
    #[allow(dead_code)]
    pub fn get_recent_transactions(&self, limit: usize) -> &[TransactionRecord] {
        &self.transactions[..std::cmp::min(limit, self.transactions.len())]
    }

    /// 更新余额
    #[allow(dead_code)]
    pub fn update_balance(&mut self, delta: f64) {
        self.balance += delta;
        if self.balance < 0.0 {
            self.balance = 0.0;
        }
    }

    /// 保存状态到文件
    pub fn save_to_file(&self, path: &std::path::Path) -> Result<(), std::io::Error> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

        // 确保目录存在
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        std::fs::write(path, json)
    }

    /// 从文件加载状态
    pub fn load_from_file(path: &std::path::Path) -> Result<Self, std::io::Error> {
        let json = std::fs::read_to_string(path)?;
        serde_json::from_str(&json)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    }

    /// 重置状态
    #[allow(dead_code)]
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}


