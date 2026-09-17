//! CPU ID 采集模块

use crate::{GyIdError, Result};

/// CPU ID 采集器
pub struct CpuCollector;

impl CpuCollector {
    /// 采集 CPU ID
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
        
        // 使用 wmic 获取 CPU ID
        let output = Command::new("wmic")
            .args(["cpu", "get", "ProcessorId"])
            .output();
        
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                // 跳过第一行标题，读取第二行
                let lines: Vec<&str> = stdout.lines().collect();
                if lines.len() >= 2 {
                    let id = lines[1].trim();
                    if !id.is_empty() {
                        return Ok(Some(id.to_string()));
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
        
        let output = Command::new("sysctl")
            .args(["-a"])
            .output();
        
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    if line.contains("machdep.cpu.feature") {
                        // macOS 没有直接的 CPU ID，使用 CPU 特征哈希
                        let features: String = stdout.lines()
                            .filter(|l| l.starts_with("machdep.cpu.features"))
                            .take(1)
                            .collect();
                        if !features.is_empty() {
                            return Ok(Some(format!("{:x}", Self::hash_string(&features))));
                        }
                    }
                }
                // 备选：使用 CPU 型号
                let model = Command::new("sysctl")
                    .args(["-n", "machdep.cpu.brand_string"])
                    .output();
                if let Ok(m) = model {
                    let brand = String::from_utf8_lossy(&m.stdout).trim().to_string();
                    if !brand.is_empty() {
                        return Ok(Some(format!("{:x}", Self::hash_string(&brand))));
                    }
                }
                Ok(None)
            }
            Err(e) => Err(GyIdError::PermissionDenied(
                format!("无法获取 CPU 信息: {}", e)
            )),
        }
    }

    #[cfg(target_os = "linux")]
    fn collect_linux() -> Result<Option<String>> {
        use std::process::Command;
        
        let output = Command::new("cat")
            .args(["/proc/cpuinfo"])
            .output();
        
        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    if line.starts_with("processor") && line.contains(':') {
                        // 尝试获取 vendor_id 或 model name 作为替代
                        continue;
                    }
                    if line.starts_with("vendor_id") || line.starts_with("model name") {
                        let parts: Vec<&str> = line.split(':').collect();
                        if parts.len() >= 2 {
                            let value = parts[1].trim();
                            if !value.is_empty() {
                                return Ok(Some(format!("{:x}", Self::hash_string(value))));
                            }
                        }
                    }
                }
                Ok(None)
            }
            Err(e) => Err(GyIdError::PermissionDenied(
                format!("无法读取 /proc/cpuinfo: {}", e)
            )),
        }
    }

/// 对字符串进行简单哈希
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    fn hash_string(s: &str) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        s.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collect_cpu() {
        let cpu_id = CpuCollector::collect();
        println!("CPU ID: {:?}", cpu_id);
    }
}
