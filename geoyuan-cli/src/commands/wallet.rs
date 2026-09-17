// ─────────────────────────────────────────────────────────────────────────────
// geoyuan wallet 子命令
// ─────────────────────────────────────────────────────────────────────────────

use clap::Subcommand;
use anyhow::Result;
use colored::Colorize;

use geoyuan_core::{
    wallet::{Wallet, CoinMinter},
    identity::GyId,
    storage::{RedbStorage, serialize, deserialize, CF_ACCOUNTS},
    photo,
};
use std::path::PathBuf;

fn default_db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("geoyuan")
        .join("wallet.redb")
}

#[derive(Subcommand)]
pub enum WalletCmd {
    /// 显示钱包信息和余额
    Show {
        /// 查询的 GyID（不填则显示所有）
        #[arg(long)]
        gyid: Option<String>,

        /// 输出为 JSON
        #[arg(long)]
        json: bool,
    },

    /// 从照片铸造新的 Geoyuan（需要 GPS EXIF 且已生成 GyID）
    Mint {
        /// 照片路径
        #[arg(short, long)]
        photo: String,

        /// 钱包归属的 GyID（照片生成的 GyID）
        #[arg(long)]
        gyid: String,

        /// 输出为 JSON
        #[arg(long)]
        json: bool,
    },

    /// 列出所有 Geoyuan 代币
    List {
        /// 归属的 GyID
        #[arg(long)]
        gyid: String,

        /// 输出为 JSON
        #[arg(long)]
        json: bool,
    },
}

impl WalletCmd {
    pub async fn run(self) -> Result<()> {
        match self {
            WalletCmd::Show { gyid, json } => show(gyid.as_deref(), json),
            WalletCmd::Mint { photo, gyid, json } => mint(&photo, &gyid, json).await,
            WalletCmd::List { gyid, json } => list(&gyid, json),
        }
    }
}

// ─── show ────────────────────────────────────────────────────────────────────

fn show(gyid: Option<&str>, json: bool) -> Result<()> {
    let db_path = default_db_path();

    if !db_path.exists() {
        if json {
            println!("[]");
        } else {
            println!("{}", "⚠  钱包数据库不存在".yellow());
            println!("   使用 `geoyuan wallet mint --photo <file> --gyid <id>` 铸造第一枚 Geoyuan");
        }
        return Ok(());
    }

    let db = RedbStorage::open(&db_path)?;
    let mut wallets: Vec<Wallet> = Vec::new();

    if let Some(id) = gyid {
        if let Some(raw) = db.get(CF_ACCOUNTS, id.as_bytes())? {
            let w: Wallet = deserialize(&raw)?;
            wallets.push(w);
        }
    } else {
        db.iterate(CF_ACCOUNTS, |_k, v| {
            if let Ok(w) = deserialize::<Wallet>(v) {
                wallets.push(w);
            }
            true
        })?;
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&wallets)?);
        return Ok(());
    }

    if wallets.is_empty() {
        println!("{}", "（空钱包）".dimmed());
        return Ok(());
    }

    for w in &wallets {
        println!();
        println!("{}", "💰 钱包".bold());
        println!("  {} {}", "GyID:".bold(), w.gy_id.cyan().bold());
        println!("  {} {} GY", "余额:".bold(), w.balance.to_string().yellow());
        println!("  {} {}", "代币数:".bold(), w.coins.len());
        println!("  {} {}", "创建时间:".bold(), w.created_at.format("%Y-%m-%d %H:%M UTC"));
    }
    println!();

    Ok(())
}

// ─── mint ────────────────────────────────────────────────────────────────────

async fn mint(photo_path: &str, gyid_str: &str, json: bool) -> Result<()> {
    if !std::path::Path::new(photo_path).exists() {
        anyhow::bail!("照片文件不存在: {}", photo_path);
    }

    if !json {
        println!("{}", "🪙 正在铸造 Geoyuan...".cyan());
        println!("   照片: {}", photo_path.dimmed());
        println!("   归属: {}", gyid_str.dimmed());
    }

    // 解析照片
    let meta = photo::parse_photo(photo_path).await
        .map_err(|e| anyhow::anyhow!("照片解析失败: {}", e))?;

    // 构造 GyId（坐标来自照片 EXIF）
    let gps = meta.gps.as_ref()
        .ok_or_else(|| anyhow::anyhow!("照片缺少 GPS 信息，无法铸造"))?;

    let mut gyid = GyId::new(gyid_str)
        .ok_or_else(|| anyhow::anyhow!("无效的 GyID: {}", gyid_str))?;
    gyid.latitude = gps.latitude;
    gyid.longitude = gps.longitude;
    gyid.photo_hash = meta.image_hash.clone();

    // 铸造
    let coin = CoinMinter::mint(&gyid, &meta.image_hash)
        .map_err(|e| anyhow::anyhow!("铸造失败: {}", e))?;

    // 存入钱包
    let db_path = default_db_path();
    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let db = RedbStorage::open(&db_path)?;

    let mut wallet = if let Some(raw) = db.get(CF_ACCOUNTS, gyid_str.as_bytes())? {
        deserialize::<Wallet>(&raw)?
    } else {
        Wallet::new(gyid_str.to_string(), "0".repeat(64))
    };

    wallet.add_coin(coin.clone());
    let bytes = serialize(&wallet)?;
    db.put(CF_ACCOUNTS, gyid_str.as_bytes(), &bytes)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&coin)?);
    } else {
        println!();
        println!("{}", "✅ Geoyuan 铸造成功".green().bold());
        println!("  {} {}", "代币 ID:".bold(), coin.id.cyan());
        println!("  {} {}...", "照片哈希:".bold(), &meta.image_hash[..16].dimmed());
        println!("  {} {:.6}, {:.6}", "坐标:".bold(), coin.latitude, coin.longitude);
        println!("  {} {} GY", "当前余额:".bold(), wallet.balance.to_string().yellow());
        println!();
    }

    Ok(())
}

// ─── list ────────────────────────────────────────────────────────────────────

fn list(gyid: &str, json: bool) -> Result<()> {
    let db_path = default_db_path();
    if !db_path.exists() {
        if json { println!("[]"); } else { println!("{}", "（空钱包）".dimmed()); }
        return Ok(());
    }

    let db = RedbStorage::open(&db_path)?;

    let wallet = match db.get(CF_ACCOUNTS, gyid.as_bytes())? {
        Some(raw) => deserialize::<Wallet>(&raw)?,
        None => {
            if json {
                println!("[]");
            } else {
                println!("{}", format!("⚠  未找到 GyID {} 的钱包", gyid).yellow());
            }
            return Ok(());
        }
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&wallet.coins)?);
        return Ok(());
    }

    if wallet.coins.is_empty() {
        println!("{}", "当前钱包无代币".dimmed());
        return Ok(());
    }

    println!("{}", format!("🗃  {} 的 Geoyuan 列表", gyid).bold());
    println!();
    for (i, coin) in wallet.coins.iter().enumerate() {
        println!("  {}. {}", i + 1, coin.id.cyan());
        println!("     坐标: {:.4}, {:.4}", coin.latitude, coin.longitude);
        println!("     地理哈希: {}", coin.geohash.yellow());
        println!("     铸造时间: {}", coin.minted_at.format("%Y-%m-%d %H:%M UTC"));
        println!();
    }

    Ok(())
}
