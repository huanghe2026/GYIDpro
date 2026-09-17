//! 硬件指纹采集模块
//!
//! 提供 MAC 地址、CPU ID、主板序列号、磁盘序列号等硬件信息采集

mod mac;
mod cpu;
mod board;
mod disk;

pub use mac::MacCollector;
pub use cpu::CpuCollector;
pub use board::BoardCollector;
pub use disk::DiskCollector;

use serde::{Deserialize, Serialize};

/// 硬件指纹数据
#[derive(Debug, Clone, Serialize, Deserialize)]
#[derive(Default)]
pub struct HardwareFingerprint {
    /// MAC 地址
    pub mac: Option<String>,
    /// CPU ID
    pub cpu_id: Option<String>,
    /// 主板序列号
    pub board_serial: Option<String>,
    /// 磁盘序列号
    pub disk_serial: Option<String>,
}


impl HardwareFingerprint {
    /// 计算有效因子数量
    pub fn valid_factor_count(&self) -> u8 {
        let mut count = 0u8;
        if self.mac.is_some() { count += 1; }
        if self.cpu_id.is_some() { count += 1; }
        if self.board_serial.is_some() { count += 1; }
        if self.disk_serial.is_some() { count += 1; }
        count
    }

    /// 是否包含足够生成 GyID 的数据
    pub fn is_valid(&self) -> bool {
        self.valid_factor_count() >= 2
    }
}
