//! 原生平台特定功能
//!
//! Windows/macOS/Linux 特有功能

/// 获取本地数据库路径
pub fn default_db_path() -> std::path::PathBuf {
    let mut path = dirs::data_local_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    path.push("gyid");
    path.push("gyid.db");
    path
}

/// 检查当前进程是否有管理员权限
pub fn has_admin_privilege() -> bool {
    #[cfg(target_os = "windows")]
    {
        // Windows：尝试执行需要管理员的操作来检测
        std::process::Command::new("net")
            .args(["session"])
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }
    #[cfg(unix)]
    {
        // Unix：检查 UID == 0
        unsafe { libc::getuid() == 0 }
    }
    #[cfg(not(any(target_os = "windows", unix)))]
    {
        false
    }
}

