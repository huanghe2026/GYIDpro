//! storage 命令 - 存储管理

use clap::{Parser, Subcommand};
use gyid_core::storage::LocalStorage;
use std::path::PathBuf;

/// 存储管理命令
#[derive(Parser, Debug)]
#[command(name = "storage")]
pub struct StorageArgs {
    /// 子命令
    #[command(subcommand)]
    pub action: StorageAction,
}

/// 存储操作类型
#[derive(Subcommand, Debug, Clone)]
pub enum StorageAction {
    /// 列出所有存储的 GyID
    List,
    
    /// 导入 GyID（从 JSON 文件）
    Import {
        /// GyID JSON 文件路径
        file: String,
        
        /// 设置为当前 GyID
        #[arg(short, long)]
        set_current: bool,
    },
    
    /// 删除 GyID
    Delete {
        /// GyID 字符串
        gyid: String,
        
        /// 强制删除（不提示确认）
        #[arg(short, long)]
        force: bool,
    },
    
    /// 备份数据库
    Backup {
        /// 备份文件路径
        output: String,
    },
    
    /// 从备份恢复
    Restore {
        /// 备份文件路径
        backup: String,
    },
    
    /// 显示存储统计信息
    Stats,
    
    /// 显示数据库路径
    Path,
}

/// 运行存储管理命令
pub fn run(action: &StorageAction) -> anyhow::Result<()> {
    let storage = LocalStorage::new(None)?;
    
    match action {
        StorageAction::List => run_list(&storage),
        StorageAction::Import { file, set_current } => run_import(&storage, file, *set_current),
        StorageAction::Delete { gyid, force } => run_delete(&storage, gyid, *force),
        StorageAction::Backup { output } => run_backup(&storage, output),
        StorageAction::Restore { backup } => run_restore(&storage, backup),
        StorageAction::Stats => run_stats(&storage),
        StorageAction::Path => run_path(&storage),
    }
}

/// 列出所有 GyID
fn run_list(storage: &LocalStorage) -> anyhow::Result<()> {
    let gyids = storage.list_all_gyids()?;
    
    if gyids.is_empty() {
        println!("📭 数据库中没有保存的 GyID");
        return Ok(());
    }
    
    println!("📋 存储的 GyID 列表 (共 {} 个)\n", gyids.len());
    println!("┌──────────────────────────────────────────────┬──────────────────────────────┬────────────┐");
    println!("│ GyID                                         │ 创建时间                    │ 关联设备  │");
    println!("├──────────────────────────────────────────────┼──────────────────────────────┼────────────┤");
    
    for gyid in &gyids {
        let created_at_ms = (gyid.created_at >> 16) as i64;
        let time_str = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(created_at_ms)
            .map(|t| t.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "未知".to_string());
        
        let display_id = if gyid.id.len() > 42 {
            format!("{}...{}", &gyid.id[..20], &gyid.id[gyid.id.len()-20..])
        } else {
            gyid.id.clone()
        };
        
        println!("│ {:<44} │ {:<26} │ {:>10} │", display_id, time_str, gyid.linked_devices);
    }
    
    println!("└──────────────────────────────────────────────┴──────────────────────────────┴────────────┘");
    
    Ok(())
}

/// 导入 GyID
fn run_import(storage: &LocalStorage, file: &str, set_current: bool) -> anyhow::Result<()> {
    println!("📥 正在导入 GyID...\n");
    
    let content = std::fs::read_to_string(file)?;
    
    // 尝试解析为单个 GyID 或数组
    let gyid = if content.trim().starts_with('[') {
        // 数组格式 - 取第一个
        let gyids: Vec<gyid_core::identity::GyId> = serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("JSON 解析失败: {}", e))?;
        
        gyids.into_iter().next()
            .ok_or_else(|| anyhow::anyhow!("文件中没有 GyID 数据"))?
    } else {
        // 单个对象
        serde_json::from_str(&content)
            .map_err(|e| anyhow::anyhow!("JSON 解析失败: {}", e))?
    };
    
    // 验证格式
    if !gyid.is_valid_format() {
        return Err(anyhow::anyhow!("无效的 GyID 格式"));
    }
    
    // 保存到数据库
    storage.save_gyid(&gyid)?;
    
    if set_current {
        storage.save_config("last_gyid", &gyid.id)?;
        println!("✅ GyID 已导入并设置为当前 GyID");
    } else {
        println!("✅ GyID 已导入");
    }
    
    println!("   ID: {}", gyid.id);
    println!("   Hash: {}...", &gyid.hash[..16.min(gyid.hash.len())]);
    
    Ok(())
}

/// 删除 GyID
fn run_delete(storage: &LocalStorage, gyid: &str, force: bool) -> anyhow::Result<()> {
    if !force {
        println!("⚠️  确认删除 GyID: {} ?", gyid);
        println!("   此操作不可撤销！使用 --force 强制删除");
        return Ok(());
    }
    
    // 检查是否存在
    let existing = storage.get_gyid(gyid)?;
    if existing.is_none() {
        return Err(anyhow::anyhow!("GyID 不存在: {}", gyid));
    }
    
    storage.delete_gyid(gyid)?;
    
    // 如果是 last_gyid，清除配置
    if let Ok(Some(last)) = storage.get_config("last_gyid") {
        if last == gyid {
            let _ = storage.save_config("last_gyid", "");
        }
    }
    
    println!("✅ GyID 已删除: {}", gyid);
    
    Ok(())
}

/// 备份数据库
fn run_backup(storage: &LocalStorage, output: &str) -> anyhow::Result<()> {
    let dest = PathBuf::from(output);
    
    storage.backup(&dest)?;
    
    println!("✅ 数据库已备份到: {}", output);
    
    Ok(())
}

/// 从备份恢复
fn run_restore(storage: &LocalStorage, backup: &str) -> anyhow::Result<()> {
    let src = PathBuf::from(backup);
    
    println!("⚠️  警告：此操作将覆盖当前数据库！");
    println!("   备份文件: {}", backup);
    
    storage.restore(&src)?;
    
    println!("✅ 数据库已从备份恢复");
    
    Ok(())
}

/// 显示统计信息
fn run_stats(storage: &LocalStorage) -> anyhow::Result<()> {
    let gyid_count = storage.count_gyids()?;
    let link_count = storage.count_links()?;
    let db_path = storage.get_db_path();
    
    println!("📊 存储统计\n");
    println!("   GyID 数量: {}", gyid_count);
    println!("   设备关联: {}", link_count);
    println!("   数据库路径: {}", db_path.display());
    
    // 如果存在，获取文件大小
    if db_path.exists() {
        if let Ok(metadata) = std::fs::metadata(&db_path) {
            let size_kb = metadata.len() as f64 / 1024.0;
            println!("   数据库大小: {:.2} KB", size_kb);
        }
    }
    
    Ok(())
}

/// 显示数据库路径
fn run_path(storage: &LocalStorage) -> anyhow::Result<()> {
    let db_path = storage.get_db_path();
    
    if db_path.as_os_str().is_empty() {
        println!("⚠️  无法确定数据库路径");
    } else {
        println!("{}", db_path.display());
    }
    
    Ok(())
}