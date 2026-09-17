//! 主板序列号采集模块

use crate::{GyIdError, Result};

/// 主板序列号采集器
pub struct BoardCollector;

impl BoardCollector {
    /// 采集主板序列号
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
        
        // 使用 wmic 获取主板序列号
        let output = Command::new("wmic")
            .args(["baseboard", "get", "SerialNumber"])
            .output();
        
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                let lines: Vec<&str> = stdout.lines().collect();
                if lines.len() >= 2 {
                    let serial = lines[1].trim().to_string();
                    // 过滤掉默认无效值
                    if !serial.is_empty() 
                        && serial != "To be filled by O.E.M."
                        && serial != "Serial Number" {
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
        
        let output = Command::new("ioreg")
            .args(["-rd1", "-c", "IOPlatformExpertDevice"])
            .output();
        
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    if line.contains("IOPlatformSerialNumber") {
                        // 格式: "IOPlatformSerialNumber" = "XXXXXXXXXXXX"
                        if let Some(start) = line.find("= \"") {
                            let rest = &line[start + 3..];
                            if let Some(end) = rest.find('"') {
                                let serial = &rest[..end];
                                if !serial.is_empty() && serial != "000000000000" {
                                    return Ok(Some(serial.to_string()));
                                }
                            }
                        }
                    }
                }
                Ok(None)
            }
            Err(e) => Err(GyIdError::PermissionDenied(
                format!("无法执行 ioreg: {}", e)
            )),
        }
    }

    #[cfg(target_os = "linux")]
    fn collect_linux() -> Result<Option<String>> {
        use std::fs;
        
        // 尝试从 DMI 读取主板序列号
        let paths = [
            "/sys/class/dmi/id/board_serial",
            "/sys/class/dmi/id/product_serial",
        ];
        
        for path in &paths {
            if let Ok(content) = fs::read_to_string(path) {
                let serial = content.trim().to_string();
                if !serial.is_empty() && serial != "0" && !serial.contains("Not") {
                    return Ok(Some(serial));
                }
            }
        }
        
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_board() {
        let board_serial = BoardCollector::collect();
        println!("主板序列号: {:?}", board_serial);
    }
}
