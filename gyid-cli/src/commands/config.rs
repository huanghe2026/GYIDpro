//! config 命令

use crate::ConfigAction;
use gyid_core::{identity::GyIdConfig, storage::LocalStorage};

pub fn run(action: &ConfigAction) -> anyhow::Result<()> {
    let storage = LocalStorage::new(None)?;
    
    match action {
        ConfigAction::Show => {
            println!("⚙️  GyID 配置\n");
            
            let config_json = storage.get_config("gyid_config")?
                .unwrap_or_else(|| GyIdConfig::default().to_json().unwrap());
            
            let config: GyIdConfig = serde_json::from_str(&config_json)
                .unwrap_or_default();
            
            println!("┌──────────────────────────────────────────────┐");
            println!("│ 配置项                                        │");
            println!("└──────────────────────────────────────────────┘");
            println!();
            println!("   地理位置精度: {}", config.geo_level.name());
            println!("   精度描述: {}", config.geo_level.description());
            println!();
            println!("   权重配置:");
            println!("   - 硬件指纹: {:.0}%", config.weights.hardware * 100.0);
            println!("   - 地理位置: {:.0}%", config.weights.geo * 100.0);
            println!("   - 时间戳: {:.0}%", config.weights.timestamp * 100.0);
            println!("   - 头像: {:.0}%", config.weights.avatar * 100.0);
            println!();
            println!("   启用头像: {}", config.enable_avatar);
            println!("   启用地理位置: {}", config.enable_geo);
            println!("   版本: {}", config.version);
        }
        
        ConfigAction::Reset => {
            let default_config = GyIdConfig::default();
            let json = default_config.to_json()?;
            storage.save_config("gyid_config", &json)?;
            println!("✅ 配置已重置为默认值");
        }
        
        ConfigAction::Set { key, value } => {
            let mut config_json = storage.get_config("gyid_config")?
                .unwrap_or_else(|| GyIdConfig::default().to_json().unwrap());
            
            let mut config: GyIdConfig = serde_json::from_str(&config_json)
                .unwrap_or_default();
            
            match key.as_str() {
                "geo_level" => {
                    config.geo_level = match value.as_str() {
                        "city" => gyid_core::geo::GeoPrecisionLevel::City,
                        "district" => gyid_core::geo::GeoPrecisionLevel::District,
                        "exact" => gyid_core::geo::GeoPrecisionLevel::Exact,
                        _ => {
                            eprintln!("❌ 无效的精度等级: city/district/exact");
                            std::process::exit(1);
                        }
                    };
                }
                "enable_avatar" => {
                    config.enable_avatar = value == "true" || value == "1";
                }
                "enable_geo" => {
                    config.enable_geo = value == "true" || value == "1";
                }
                _ => {
                    eprintln!("❌ 不支持的配置项: {}", key);
                    std::process::exit(1);
                }
            }
            
            config_json = config.to_json()?;
            storage.save_config("gyid_config", &config_json)?;
            println!("✅ 配置已更新");
        }
    }
    
    Ok(())
}
