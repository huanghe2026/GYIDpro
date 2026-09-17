//! 磁盘序列号采集模块

use crate::{GyIdError, Result};

/// 磁盘序列号采集器
pub struct DiskCollector;

impl DiskCollector {
    /// 采集磁盘序列号
    pub fn collect() -> Result<Option<String>> {
        #[cfg(target_os = "windows")]
        {
            Self::collect_windows()
        }
        
        #[cfg(target_os = "macos")]
        {
            Self::collect_macos()
        }
        
        #[cfg(target_os = "linux")]
        {
            Self::collect_linux()
        }
        
        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            Ok(None)
        }
    }

    #[cfg(target_os = "windows")]
    fn collect_windows() -> Result<Option<String>> {
        use std::process::Command;
        
        // 使用 wmic 获取磁盘序列号
        let output = Command::new("wmic")
            .args(["diskdrive", "get", "SerialNumber"])
            .output();
        
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let lines: Vec<&str> = stdout.lines().collect();
                if lines.len() >= 2 {
                    let serial = lines[1].trim().to_string();
                    if !serial.is_empty() {
                        return Ok(Some(serial));
                    }
                }
                Ok(None)
            }
            Err(e) => Err(GyIdError::PermissionDenied(
                format!("无法执行 wmic: {}", e)
            )),
        }
    }

    #[cfg(target_os = "macos")]
    fn collect_macos() -> Result<Option<String>> {
        use std::process::Command;
        
        let output = Command::new("system_profiler")
            .args(["SPStorageDataType"])
            .output();
        
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                // 解析输出获取 BSD Name 或 Serial Number
                for line in stdout.lines() {
                    if line.contains("BSD Name") || line.contains("Serial Number") {
                        if let Some(pos) = line.find(':') {
                            let value = line[pos + 1..].trim().to_string();
                            if !value.is_empty() && !value.contains("N/A") {
                                return Ok(Some(value));
                            }
                        }
                    }
                }
                Ok(None)
            }
            Err(e) => Err(GyIdError::PermissionDenied(
                format!("无法执行 system_profiler: {}", e)
            )),
        }
    }

    #[cfg(target_os = "linux")]
    fn collect_linux() -> Result<Option<String>> {
        use std::fs;
        
        // 尝试从 sysfs 读取磁盘序列号
        let paths = [
            "/sys/class/block/sda/device/serial",
            "/sys/class/block/sda/device/volume_serial",
        ];
        
        for path in &paths {
            if let Ok(content) = fs::read_to_string(path) {
                let serial = content.trim().to_string();
                if !serial.is_empty() {
                    return Ok(Some(serial));
                }
            }
        }
        
        // 备选：使用 lsblk
        use std::process::Command;
        let output = Command::new("lsblk")
            .args(["-o", "SERIAL", "-n"])
            .output();
        
        if let Ok(out) = output {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let serial = stdout.lines().next().unwrap_or("").trim().to_string();
            if !serial.is_empty() {
                return Ok(Some(serial));
            }
        }
        
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_disk() {
        let disk_serial = DiskCollector::collect();
        println!("磁盘序列号: {:?}", disk_serial);
    }
}
