// ─────────────────────────────────────────────────────────────────────────────
// geoyuan identity 子命令
// ─────────────────────────────────────────────────────────────────────────────

use clap::Subcommand;
use anyhow::Result;
use colored::Colorize;

use geoyuan_core::{
    identity::{GyIdGenerator, GeoIdConfig},
    identity::validator::{GyIdValidator, ValidationResult},
    photo,
    photo::GpsCoordinates,
};

#[derive(Subcommand)]
pub enum IdentityCmd {
    /// 从照片生成新的 GyID（需要含 GPS EXIF 数据的 JPEG）
    Generate {
        /// 输入照片路径（JPEG，需含 GPS EXIF）
        #[arg(short, long)]
        photo: Option<String>,

        /// 生成测试网 GyID（TGyID 前缀）
        #[arg(long)]
        testnet: bool,

        /// 输出为 JSON
        #[arg(long)]
        json: bool,
    },

    /// 验证 GyID 格式与校验和
    Verify {
        /// GyID 字符串
        gyid: String,
    },

    /// 显示 GyID 详细信息
    Info {
        /// GyID 字符串
        gyid: String,

        /// 输出为 JSON
        #[arg(long)]
        json: bool,
    },
}

impl IdentityCmd {
    pub async fn run(self) -> Result<()> {
        match self {
            IdentityCmd::Generate { photo, testnet, json } => {
                generate(photo.as_deref(), testnet, json).await
            }
            IdentityCmd::Verify { gyid } => verify(&gyid),
            IdentityCmd::Info { gyid, json } => info(&gyid, json),
        }
    }
}

// ─── generate ────────────────────────────────────────────────────────────────

async fn generate(photo: Option<&str>, testnet: bool, json: bool) -> Result<()> {
    if !json {
        println!("{}", "⚙  正在生成 GyID...".cyan());
        if let Some(p) = photo {
            println!("   照片: {}", p.dimmed());
        } else {
            println!("   模式: {}", "无照片（随机模式）".yellow());
        }
        if testnet {
            println!("   网络: {}", "测试网".yellow());
        }
    }

    let config = GeoIdConfig {
        amap_api_key: std::env::var("AMAP_API_KEY").ok(),
        testnet,
        ..Default::default()
    };
    let generator = GyIdGenerator::new(config);

    let gyid = if let Some(path) = photo {
        // 从照片生成
        let meta = photo::parse_photo(path).await
            .map_err(|e| anyhow::anyhow!("照片解析失败: {}", e))?;

        generator.generate(&meta).await
            .map_err(|e| anyhow::anyhow!("GyID 生成失败: {}", e))?
    } else {
        // 无照片：用随机盐 + 当前时间生成（测试用）
        use geoyuan_core::photo::PhotoMetadata;
        use rand::RngCore;

        let mut salt = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut salt);

        let fake_meta = PhotoMetadata {
            image_hash: hex::encode(salt),
            filename: None,
            file_size: 0,
            dimensions: None,
            gps: Some(GpsCoordinates::new(0.0, 0.0)),
            capture_time: Some(chrono::Utc::now().timestamp()),
            make: None,
            model: None,
        };

        generator.generate(&fake_meta).await
            .map_err(|e| anyhow::anyhow!("GyID 生成失败: {}", e))?
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&gyid)?);
    } else {
        println!();
        println!("{}", "✅ GyID 生成成功".green().bold());
        println!();
        println!("  {} {}", "GyID:".bold(), gyid.id.cyan().bold());
        println!("  {} {}", "哈希:".bold(), gyid.hash.dimmed());
        println!("  {} {:.6}, {:.6}", "坐标:".bold(), gyid.latitude, gyid.longitude);
        println!("  {} {}", "地理哈希:".bold(), gyid.geohash.yellow());
        println!("  {} {}", "版本:".bold(), gyid.version);
        println!("  {} {}", "创建时间:".bold(), gyid.created_at.format("%Y-%m-%d %H:%M:%S UTC"));
        println!();
    }

    Ok(())
}

// ─── verify ──────────────────────────────────────────────────────────────────

fn verify(gyid_str: &str) -> Result<()> {
    print!("🔐 验证 GyID: {} ... ", gyid_str.cyan());

    let validator = GyIdValidator::new_default();
    match validator.validate_format(gyid_str) {
        ValidationResult::Valid => {
            println!("{}", "✓ 有效".green().bold());
            println!("   前缀检查: {}", "通过".green());
            println!("   Base58 格式: {}", "通过".green());
            println!("   长度检查: {}", "通过".green());

            let prefix = if gyid_str.starts_with("TGyID") { "TGyID" } else { "GyID" };
            let raw = gyid_str.trim_start_matches(prefix);
            let preview = &raw[..raw.len().min(8)];
            println!("   原始摘要: {}...", preview.yellow());
        }
        ValidationResult::Invalid(msg) => {
            println!("{}", "✗ 无效".red().bold());
            println!("   原因: {}", msg);
        }
    }

    Ok(())
}

// ─── info ─────────────────────────────────────────────────────────────────────

fn info(gyid_str: &str, json: bool) -> Result<()> {
    let validator = GyIdValidator::new_default();
    let valid = validator.validate_format(gyid_str).is_valid();
    let is_testnet = gyid_str.starts_with("TGyID");
    let network = if is_testnet { "testnet" } else { "mainnet" };
    let prefix = if is_testnet { "TGyID" } else { "GyID" };
    let raw = gyid_str.trim_start_matches(prefix);

    if json {
        let info = serde_json::json!({
            "id": gyid_str,
            "valid": valid,
            "network": network,
            "prefix": prefix,
            "raw_length": raw.len(),
        });
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("{}", "📋 GyID 信息".bold());
        println!();
        println!("  {} {}", "GyID:".bold(), gyid_str.cyan());
        println!("  {} {}", "有效:".bold(), if valid { "是".green() } else { "否".red() });
        println!("  {} {}", "网络:".bold(), if is_testnet { "测试网".yellow() } else { "主网".green() });
        println!("  {} {}", "前缀:".bold(), prefix.dimmed());
        println!("  {} {}", "原始长度:".bold(), raw.len().to_string().dimmed());
    }

    Ok(())
}
