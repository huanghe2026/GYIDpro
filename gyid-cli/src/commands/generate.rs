//! generate 命令

use crate::GeoLevel;
use gyid_core::{
    identity::{GyIdGenerator, GeneratorConfig},
    geo::GeoPrecisionLevel,
    storage::LocalStorage,
};

pub async fn run(
    avatar_path: Option<&str>, 
    level: &GeoLevel,
    lat: Option<&f64>,
    lon: Option<&f64>,
) -> anyhow::Result<()> {
    println!("🎯 正在生成 GyID...\n");

    // 转换精度等级
    let geo_level = match level {
        GeoLevel::City => GeoPrecisionLevel::City,
        GeoLevel::District => GeoPrecisionLevel::District,
        GeoLevel::Exact => GeoPrecisionLevel::Exact,
        GeoLevel::Manual => GeoPrecisionLevel::Manual,
        GeoLevel::None => GeoPrecisionLevel::None,
    };

    // 验证手动坐标参数
    if geo_level == GeoPrecisionLevel::Manual && (lat.is_none() || lon.is_none()) {
        eprintln!("❌ 使用 --level manual 时需要同时提供 --lat 和 --lon 参数");
        eprintln!("示例: gyid generate --level manual --lat 39.9042 --lon 116.4074");
        std::process::exit(1);
    }

    // 创建生成器
    let config = GeneratorConfig {
        geo_level,
        with_avatar: avatar_path.is_some(),
        with_geo: true,
        ..Default::default()
    };

    let generator = GyIdGenerator::new(config);

    // 生成 GyID
    let gyid = if geo_level == GeoPrecisionLevel::Manual {
        // 手动坐标模式
        let lat = *lat.unwrap();
        let lon = *lon.unwrap();
        println!("📍 使用手动坐标: lat={}, lon={}", lat, lon);
        generator.generate_with_manual_geo(lat, lon, avatar_path).await?
    } else if geo_level == GeoPrecisionLevel::None {
        // 无位置模式
        println!("📍 模式: 不采集位置信息（仅使用硬件指纹）");
        generator.generate(avatar_path).await?
    } else {
        // 常规模式
        match geo_level {
            GeoPrecisionLevel::City => println!("📍 精度: 城市级 (IP 定位)"),
            GeoPrecisionLevel::District => println!("📍 精度: 区域级 (WiFi BSSID 定位)"),
            GeoPrecisionLevel::Exact => println!("📍 精度: 精确级 (GPS + WiFi)"),
            _ => {}
        }
        generator.generate(avatar_path).await?
    };

    println!("✅ GyID 生成成功!\n");
    println!("┌─────────────────────────────────────────┐");
    println!("│  {}", gyid.id);
    println!("└─────────────────────────────────────────┘");
    println!();
    println!("📊 详细信息:");
    println!("   - 哈希: {}...", &gyid.hash[..16]);
    // created_at 格式为 (毫秒时间戳 << 16 | 16bit随机数)，还原时需右移 16 位
    let created_at_ms = (gyid.created_at >> 16) as i64;
    println!("   - 创建时间: {}", chrono::DateTime::<chrono::Utc>::from_timestamp_millis(created_at_ms)
        .map(|t| t.format("%Y-%m-%d %H:%M:%S UTC").to_string())
        .unwrap_or_else(|| "未知".to_string()));
    println!("   - 关联设备: {}", gyid.linked_devices);
    println!();

    // 持久化到本地存储
    match LocalStorage::new(None) {
        Ok(storage) => {
            match storage.save_gyid(&gyid) {
                Ok(_) => {
                    // 同时保存为 last_gyid
                    let _ = storage.save_config("last_gyid", &gyid.id);
                    println!("💾 GyID 已自动保存到本地存储");
                }
                Err(e) => {
                    eprintln!("⚠️  保存失败: {}", e);
                }
            }
        }
        Err(e) => {
            eprintln!("⚠️  存储初始化失败: {}", e);
        }
    }

    Ok(())
}
