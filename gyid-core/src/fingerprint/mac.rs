//! MAC 地址采集模块

use crate::{GyIdError, Result};

/// MAC 地址采集器
pub struct MacCollector;

impl MacCollector {
    /// 采集 MAC 地址
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
        
        let output = Command::new("getmac")
            .args(["/v", "/fo", "csv", "/nh"])
            .output();
        
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                // 解析 CSV 输出获取第一个物理地址
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 3 {
                        let addr = parts[2].trim_matches('"');
                        if !addr.is_empty() && !addr.contains("N/A") {
                            return Ok(Some(addr.to_uppercase().replace("-", ":")));
                        }
                    }
                }
                Ok(None)
            }
            Err(e) => Err(GyIdError::PermissionDenied(
                format!("无法执行 getmac: {}", e)
            )),
        }
    }

    #[cfg(target_os = "macos")]
    fn collect_macos() -> Result<Option<String>> {
        use std::process::Command;
        
        let output = Command::new("ifconfig")
            .args(["en0"])
            .output();
        
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    if line.contains("ether") {
                        let addr = line.split_whitespace().nth(1).unwrap_or("");
                        if !addr.is_empty() {
                            return Ok(Some(addr.to_string()));
                        }
                    }
                }
                Ok(None)
            }
            Err(e) => Err(GyIdError::PermissionDenied(
                format!("无法执行 ifconfig: {}", e)
            )),
        }
    }

    #[cfg(target_os = "linux")]
    fn collect_linux() -> Result<Option<String>> {
        use std::process::Command;
        
        let output = Command::new("cat")
            .args(["/sys/class/net/eth0/address"])
            .output();
        
        match output {
            Ok(out) => {
                let addr = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !addr.is_empty() {
                    Ok(Some(addr.to_uppercase().replace("-", ":")))
                } else {
                    Ok(None)
                }
            }
            Err(e) => Err(GyIdError::PermissionDenied(
                format!("无法读取 MAC 地址: {}", e)
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_mac() {
        let mac = MacCollector::collect();
        println!("MAC 地址: {:?}", mac);
    }
}
