//! 设备关联模块
//!
//! 提供主设备-从设备树形关联结构

mod master;
mod linked;

pub use master::MasterDevice;
pub use linked::LinkedDevice;

use serde::{Deserialize, Serialize};

/// 设备类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceType {
    /// 主设备
    Master,
    /// 从设备
    Linked,
}

/// 设备信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    /// 设备 ID
    pub device_id: String,
    /// 设备类型
    pub device_type: DeviceType,
    /// 设备名称
    pub name: Option<String>,
    /// 关联的 GyID
    pub gyid: String,
    /// 创建时间
    pub created_at: u64,
    /// 最后活跃时间
    pub last_active: u64,
}

impl Device {
    /// 创建新设备
    pub fn new(device_id: String, device_type: DeviceType, gyid: String) -> Self {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        
        Self {
            device_id,
            device_type,
            name: None,
            gyid,
            created_at: now,
            last_active: now,
        }
    }

    /// 是否为主设备
    pub fn is_master(&self) -> bool {
        self.device_type == DeviceType::Master
    }
}

/// 设备关联信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceLink {
    /// 关联 ID
    pub link_id: String,
    /// 主设备 ID
    pub master_id: String,
    /// 从设备 ID
    pub linked_id: String,
    /// 授权码
    pub auth_code: String,
    /// 关联时间
    pub linked_at: u64,
    /// 关联状态
    pub active: bool,
}
