//! export 命令

use crate::ExportFormat;
use gyid_core::storage::LocalStorage;

pub async fn run(format: &ExportFormat, output: Option<&str>, all: bool) -> anyhow::Result<()> {
    println!("📤 正在导出 GyID...\n");
    
    // 加载本地存储
    let storage = LocalStorage::new(None)?;
    
    if all {
        // 批量导出所有 GyID
        let gyids = storage.list_all_gyids()?;
        
        if gyids.is_empty() {
            println!("⚠️  数据库中没有保存的 GyID");
            return Ok(());
        }
        
        println!("📋 共找到 {} 个 GyID\n", gyids.len());
        
        let content = match format {
            ExportFormat::Json => {
                serde_json::to_string_pretty(&gyids)?
            }
            ExportFormat::Text => {
                gyids.iter().enumerate().map(|(i, g)| {
                    let created_at_ms = (g.created_at >> 16) as i64;
                    let time_str = chrono::DateTime::<chrono::Utc>::from_timestamp_millis(created_at_ms)
                        .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
                        .unwrap_or_else(|| "未知".to_string());
                    format!(
                        "{}. {}\n   Hash: {}...\n   Created: {}\n   Linked: {}\n",
                        i + 1,
                        g.id,
                        &g.hash[..16.min(g.hash.len())],
                        time_str,
                        g.linked_devices
                    )
                }).collect()
            }
        };
        
        if let Some(path) = output {
            std::fs::write(path, &content)?;
            println!("✅ 已导出 {} 个 GyID 到: {}", gyids.len(), path);
        } else {
            println!("{}", content);
        }
    } else {
        // 导出单个（last_gyid）
        let gyid_str = storage.get_config("last_gyid")?
            .ok_or_else(|| anyhow::anyhow!("未找到已保存的 GyID"))?;
        
        let gyid_obj = storage.get_gyid(&gyid_str)?
            .ok_or_else(|| anyhow::anyhow!("GyID 数据不完整"))?;
        
        let content = match format {
            ExportFormat::Json => {
                serde_json::to_string_pretty(&gyid_obj)?
            }
            ExportFormat::Text => {
                format!(
                    "GyID: {}\nHash: {}\nCreated: {}\nLinked Devices: {}",
                    gyid_obj.id,
                    gyid_obj.hash,
                    chrono::DateTime::<chrono::Utc>::from_timestamp_millis(gyid_obj.created_at as i64).unwrap_or_default(),
                    gyid_obj.linked_devices
                )
            }
        };
        
        // 输出
        if let Some(path) = output {
            std::fs::write(path, &content)?;
            println!("✅ 已导出到: {}", path);
        } else {
            println!("{}", content);
        }
    }
    
    Ok(())
}
