// ─────────────────────────────────────────────────────────────────────────────
// geoyuan storage 子命令
// ─────────────────────────────────────────────────────────────────────────────

use clap::Subcommand;
use anyhow::Result;
use colored::Colorize;

use geoyuan_core::storage::{
    RedbStorage, CF_ACCOUNTS, CF_BLOCKS, CF_METADATA, CF_TRANSACTIONS,
};
use std::path::PathBuf;

fn default_db_path() -> PathBuf {
    dirs::data_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("geoyuan")
        .join("wallet.redb")
}

#[derive(Subcommand)]
pub enum StorageCmd {
    /// 显示存储统计信息
    Stats,

    /// 列出指定列族的所有键
    List {
        /// 列族名称 (accounts/blocks/metadata/transactions)
        #[arg(short, long, default_value = "accounts")]
        cf: String,

        /// 最多显示条目数
        #[arg(short, long, default_value = "20")]
        limit: usize,
    },

    /// 初始化存储目录
    Init,
}

impl StorageCmd {
    pub async fn run(self) -> Result<()> {
        match self {
            StorageCmd::Stats => stats(),
            StorageCmd::List { cf, limit } => list(&cf, limit),
            StorageCmd::Init => init(),
        }
    }
}

// ─── stats ───────────────────────────────────────────────────────────────────

fn stats() -> Result<()> {
    let db_path = default_db_path();

    println!("{}", "🗄  GeoYuan 存储统计".bold());
    println!();
    println!("  {} {}", "数据库路径:".bold(), db_path.display().to_string().dimmed());

    if !db_path.exists() {
        println!("  {}", "⚠  数据库文件不存在".yellow());
        println!();
        println!("  使用 `geoyuan storage init` 初始化数据库");
        return Ok(());
    }

    let db = RedbStorage::open(&db_path)?;

    // 统计各列族的条目数
    let cfs = [
        (CF_ACCOUNTS, "账户"),
        (CF_BLOCKS, "区块"),
        (CF_METADATA, "元数据"),
        (CF_TRANSACTIONS, "交易"),
    ];

    println!();
    println!("{:<20} {:>10}", "列族", "条目数");
    println!("{}", "─".repeat(32));

    let mut total = 0usize;
    for (cf, name) in &cfs {
        let mut count = 0usize;
        db.iterate(cf, |_k, _v| {
            count += 1;
            true
        })?;
        total += count;
        println!("{:<20} {:>10}", name.bold(), count.to_string().cyan());
    }

    println!("{}", "─".repeat(32));
    println!("{:<20} {:>10}", "合计".bold(), total.to_string().yellow().bold());
    println!();

    // 数据库文件大小
    if let Ok(meta) = std::fs::metadata(&db_path) {
        let size_kb = meta.len() / 1024;
        println!("  {} {} KB", "文件大小:".bold(), size_kb.to_string().green());
    }

    Ok(())
}

// ─── list ────────────────────────────────────────────────────────────────────

fn list(cf_name: &str, limit: usize) -> Result<()> {
    let cf = match cf_name.to_lowercase().as_str() {
        "accounts" => CF_ACCOUNTS,
        "blocks" => CF_BLOCKS,
        "metadata" => CF_METADATA,
        "transactions" => CF_TRANSACTIONS,
        other => anyhow::bail!("未知列族: '{}' (可选: accounts/blocks/metadata/transactions)", other),
    };

    let db_path = default_db_path();
    if !db_path.exists() {
        println!("{}", "⚠  数据库不存在".yellow());
        return Ok(());
    }

    let db = RedbStorage::open(&db_path)?;

    println!("{}", format!("📋 列族 '{}' 的键列表 (最多 {})", cf_name, limit).bold());
    println!();

    let mut count = 0usize;
    db.iterate(cf, |k, v| {
        if count >= limit {
            return false;
        }
        let key_str = String::from_utf8_lossy(k);
        let val_preview = if v.len() > 40 {
            format!("[{} 字节]", v.len())
        } else {
            String::from_utf8_lossy(v).to_string()
        };
        println!("  {} → {}", key_str.cyan(), val_preview.dimmed());
        count += 1;
        true
    })?;

    if count == 0 {
        println!("  {}", "（空）".dimmed());
    }

    Ok(())
}

// ─── init ────────────────────────────────────────────────────────────────────

fn init() -> Result<()> {
    let db_path = default_db_path();

    if let Some(parent) = db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    if db_path.exists() {
        println!("{}", "⚠  数据库已存在，跳过初始化".yellow());
        println!("   路径: {}", db_path.display().to_string().dimmed());
        return Ok(());
    }

    // 创建数据库（会初始化 main 表）
    let _db = RedbStorage::open(&db_path)?;

    println!("{}", "✅ 数据库初始化成功".green().bold());
    println!("   路径: {}", db_path.display().to_string().cyan());

    Ok(())
}
