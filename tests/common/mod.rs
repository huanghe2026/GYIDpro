//! 测试通用工具模块

use tempfile::TempDir;
use std::path::PathBuf;

/// 创建临时目录用于测试
#[allow(dead_code)]
pub fn temp_dir() -> (TempDir, PathBuf) {
    let temp = TempDir::new().expect("Failed to create temp dir");
    let path = temp.path().to_path_buf();
    (temp, path)
}

