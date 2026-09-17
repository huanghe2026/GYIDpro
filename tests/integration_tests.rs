//! GyID 集成测试
//!
//! 测试完整流程：生成 GyID → 验证 → 存储 → 导出

mod common;

use anyhow::Result;
use gyid_core::{
    GeneratorConfig, GyIdGenerator, GyIdValidator,
    crypto::GyIdWeights,
    geo::GeoPrecisionLevel,
    storage::LocalStorage,
};
use tempfile::TempDir;

#[tokio::test]
async fn test_full_generation_flow() -> Result<()> {
    // 1. 使用默认配置生成 GyID
    let config = GeneratorConfig::default();
    let generator = GyIdGenerator::new(config);
    
    let gyid = generator.generate(None).await?;
    
    // 2. 验证格式
    assert!(GyIdValidator::validate_format(&gyid.id)?);
    
    // 3. 验证长度
    assert!(gyid.id.starts_with("GyID"));
    assert!(gyid.id.len() >= 32);
    
    // 4. 验证哈希格式
    assert_eq!(gyid.hash.len(), 64); // 32字节 hex 编码
    
    Ok(())
}

#[tokio::test]
async fn test_custom_geo_precision() -> Result<()> {
    // 只测试不需要网络的精度等级
    let offline_cases = vec![
        GeoPrecisionLevel::None,
    ];
    
    for level in offline_cases {
        let config = GeneratorConfig {
            geo_level: level,
            ..Default::default()
        };
        let generator = GyIdGenerator::new(config);
        let gyid = generator.generate(None).await?;
        
        // 所有精度等级都应该生成有效的 GyID
        assert!(GyIdValidator::validate_format(&gyid.id)?);
    }
    
    // 网络相关等级（City/District/Exact）：在离线时自动降级，不应 panic
    // 注意：如果网络不可用，生成器会返回 Err，这是设计行为。
    // 这里只验证它们不会 panic（即使 Err 也是可接受的）。
    let network_levels = vec![
        GeoPrecisionLevel::City,
        GeoPrecisionLevel::District,
        GeoPrecisionLevel::Exact,
    ];
    for level in network_levels {
        let config = GeneratorConfig {
            geo_level: level,
            ..Default::default()
        };
        let generator = GyIdGenerator::new(config);
        // 允许网络失败（Err），但不能 panic
        let _result = generator.generate(None).await;
        // 只要不 panic 就通过
    }
    
    Ok(())
}

#[tokio::test]
async fn test_custom_weights() -> Result<()> {
    let weights = GyIdWeights {
        hardware: 0.40,
        geo: 0.30,
        timestamp: 0.10,
        avatar: 0.20,
    };
    
    let config = GeneratorConfig {
        geo_level: GeoPrecisionLevel::City,
        weights,
        ..Default::default()
    };
    
    let generator = GyIdGenerator::new(config);
    let gyid = generator.generate(None).await?;
    
    assert!(GyIdValidator::validate_format(&gyid.id)?);
    Ok(())
}

#[tokio::test]
async fn test_gyid_uniqueness() -> Result<()> {
    // 生成多个 GyID，验证哈希不同
    let mut hashes = Vec::new();
    
    for _ in 0..5 {
        let gyid = GyIdGenerator::default()
            .generate(None)
            .await?;
        
        // 哈希应该各不相同
        assert!(!hashes.contains(&gyid.hash));
        hashes.push(gyid.hash);
    }
    
    Ok(())
}

#[tokio::test]
async fn test_storage_roundtrip() -> Result<()> {
    let temp_dir = TempDir::new()?;
    // 传入完整的数据库文件路径，而非目录路径
    let db_path = temp_dir.path().join("gyid.db");
    let storage = LocalStorage::new(Some(db_path))?;
    
    // 生成 GyID
    let gyid = GyIdGenerator::default()
        .generate(None)
        .await?;
    
    // 保存到存储
    storage.save_gyid(&gyid)?;
    storage.save_config("last_gyid", &gyid.id)?;
    
    // 从存储读取
    let saved_id = storage.get_config("last_gyid")?
        .ok_or_else(|| anyhow::anyhow!("Missing last_gyid"))?;
    assert_eq!(saved_id, gyid.id);
    
    let saved_gyid = storage.get_gyid(&gyid.id)?
        .ok_or_else(|| anyhow::anyhow!("Missing gyid"))?;
    
    assert_eq!(saved_gyid.id, gyid.id);
    assert_eq!(saved_gyid.hash, gyid.hash);
    
    Ok(())
}

#[tokio::test]
async fn test_validator_edge_cases() -> Result<()> {
    // 有效格式
    assert!(GyIdValidator::validate_format("GyID1234567890abcdefghijklmnopqr")?);
    
    // 无效格式 - 太短
    assert!(!GyIdValidator::validate_format("GyID1234567890")?);
    
    // 无效格式 - 错误前缀
    assert!(!GyIdValidator::validate_format("XXXX1234567890abcdefghijklmnopqr")?);
    
    // 无效格式 - 包含非法字符
    assert!(!GyIdValidator::validate_format("GyID1234567890abcdefghijklmno!@#")?);
    
    Ok(())
}

#[tokio::test]
async fn test_weights_validation() -> Result<()> {
    // 有效权重
    let weights = GyIdWeights {
        hardware: 0.35,
        geo: 0.25,
        timestamp: 0.15,
        avatar: 0.25,
    };
    assert!(weights.is_valid());
    
    // 无效权重 - 总和不为1
    let bad_weights = GyIdWeights {
        hardware: 0.40,
        geo: 0.20,
        timestamp: 0.15,
        avatar: 0.20,
    };
    assert!(!bad_weights.is_valid());
    
    Ok(())
}

#[tokio::test]
async fn test_config_defaults() {
    let config = GeneratorConfig::default();
    
    assert_eq!(config.geo_level, GeoPrecisionLevel::City);
    assert!(config.with_geo);
    assert!(!config.with_avatar);
    
    let weights = config.weights;
    assert!((weights.hardware + weights.geo + weights.timestamp + weights.avatar - 1.0).abs() < 0.01);
}
