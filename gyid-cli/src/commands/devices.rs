//! devices 命令

use gyid_core::storage::LocalStorage;

pub async fn run(gyid: Option<&str>) -> anyhow::Result<()> {
    println!("📱 设备列表\n");
    
    // 加载本地存储
    let storage = LocalStorage::new(None)?;
    
    // 获取 GyID
    let gyid_str = if let Some(id) = gyid {
        id.to_string()
    } else {
        storage.get_config("last_gyid")?
            .ok_or_else(|| anyhow::anyhow!("未找到已保存的 GyID"))?
    };
    
    // 获取关联设备
    let links = storage.get_links(&gyid_str)?;
    
    if links.is_empty() {
        println!("暂无关联设备");
        println!("\n💡 使用以下命令关联新设备:");
        println!("   gyid link --master {}", gyid_str);
    } else {
        println!("┌──────────────────────────────────────────────┐");
        println!("│ 关联设备 ({})                                   │", links.len());
        println!("└──────────────────────────────────────────────┘\n");
        
        for (i, link) in links.iter().enumerate() {
            println!("{}. 设备 ID: {}", i + 1, link.linked_id);
            println!("   关联时间: {}", chrono::DateTime::<chrono::Utc>::from_timestamp_millis(link.linked_at as i64).unwrap_or_default());
            println!("   状态: {}", if link.active { "活跃" } else { "已停用" });
            println!();
        }
    }
    
    Ok(())
}
