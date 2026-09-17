//! verify 命令

use gyid_core::GyIdValidator;

pub fn run(gyid_str: &str) -> anyhow::Result<()> {
    println!("🔍 正在验证 GyID...\n");
    
    // 验证 GyID 格式
    match GyIdValidator::validate_format(gyid_str) {
        Ok(true) => {
            println!("✅ GyID 格式验证通过!");
            println!();
            println!("📋 验证详情:");
            println!("   - 格式: 正确");
            println!("   - 前缀: {}", if gyid_str.starts_with("GyID") { "GyID" } else { "无效" });
            println!("   - 长度: {} 字符", gyid_str.len());
            println!("   - 字符集: Base58 ✓");
        }
        Ok(false) => {
            println!("❌ GyID 格式无效");
            println!();
            println!("📋 问题分析:");
            if gyid_str.len() < 32 {
                println!("   - 长度不足: {} < 32 字符", gyid_str.len());
            }
            if !gyid_str.starts_with("GyID") {
                println!("   - 缺少 'GyID' 前缀");
            }
            std::process::exit(1);
        }
        Err(e) => {
            println!("❌ 验证失败: {}", e);
            std::process::exit(1);
        }
    }
    
    Ok(())
}
