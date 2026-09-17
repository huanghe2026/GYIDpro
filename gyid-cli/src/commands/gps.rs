//! GPS 相关命令

use anyhow::Result;
use gyid_core::{GpsLocator, GpsAvailability, GyIdGenerator, GeneratorConfig, GeoPrecisionLevel};

/// 检查 GPS 可用性
pub async fn run_check() -> Result<()> {
    println!("正在检测 GPS 可用性（快速检测，1秒超时）...");
    
    let availability = GpsLocator::check_availability().await;
    
    match availability {
        GpsAvailability::Available => {
            println!("✅ GPS 可用");
            println!();
            println!("提示：可以使用 --level exact 启用精确级定位");
        }
        GpsAvailability::NoHardware => {
            println!("⚠️  未检测到 GPS 硬件");
            println!();
            println!("建议方案：");
            println!("  1. 使用 --level district 启用 WiFi BSSID 定位");
            println!("  2. 使用 --level manual 手动输入坐标");
            println!("  3. 使用 --level none 仅使用硬件指纹");
        }
        GpsAvailability::PermissionDenied => {
            println!("⚠️  GPS 权限被拒绝");
            println!();
            println!("解决方案：");
            println!("  1. 前往系统设置 → 隐私 → 位置服务");
            println!("  2. 允许本应用访问位置信息");
            println!("  3. 或者使用其他定位方式");
        }
        GpsAvailability::ServiceDisabled => {
            println!("⚠️  位置服务已禁用");
            println!();
            println!("解决方案：");
            println!("  1. 前往系统设置 → 隐私 → 位置服务");
            println!("  2. 启用位置服务");
            println!("  3. 或者使用 --level city 仅使用 IP 定位");
        }
        GpsAvailability::Timeout => {
            println!("⚠️  GPS 检测超时");
            println!();
            println!("提示：GPS 信号可能较弱，请移至开阔区域重试");
        }
    }
    
    Ok(())
}

/// 手动坐标测试
pub fn run_manual_test(lat: f64, lon: f64) -> Result<()> {
    println!("测试手动坐标输入...");
    println!("输入: lat={}, lon={}", lat, lon);
    
    // 创建生成器
    let config = GeneratorConfig {
        geo_level: GeoPrecisionLevel::Manual,
        ..Default::default()
    };
    let generator = GyIdGenerator::new(config);
    
    // 验证坐标
    match generator.set_manual_geo(lat, lon) {
        Ok(geo) => {
            println!("✅ 坐标验证通过");
            println!("哈希因子: {}", geo.to_hash_factor());
        }
        Err(e) => {
            println!("❌ 坐标验证失败: {}", e);
        }
    }
    
    Ok(())
}
