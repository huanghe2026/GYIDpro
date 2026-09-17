//! GyID CLI
//!
//! 去中心化账号系统的命令行工具

mod commands;

use clap::{Parser, Subcommand};
use tracing_subscriber::{fmt, EnvFilter};

/// GyID CLI 主程序
#[derive(Parser)]
#[command(
    name = "gyid",
    about = "GyID - 去中心化身份账号生成器",
    version,
    author
)]
struct Cli {
    /// 详细输出
    #[arg(short, long)]
    verbose: bool,
    
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// 生成新的 GyID
    Generate {
        /// 头像图片路径
        #[arg(short, long)]
        avatar: Option<String>,

        /// 地理位置精度等级
        #[arg(short, long, value_enum, default_value = "city")]
        level: GeoLevel,

        /// 手动输入纬度 (配合 --level manual 使用)
        #[arg(long, requires = "level", conflicts_with = "lon")]
        lat: Option<f64>,

        /// 手动输入经度 (配合 --level manual 使用)
        #[arg(long, requires = "level", conflicts_with = "lat")]
        lon: Option<f64>,
    },

    /// 验证 GyID
    Verify {
        /// GyID 字符串
        #[arg(required = true)]
        gyid: String,
    },

    /// 关联设备
    ///
    /// 主设备：使用 --auth-code gen 生成授权码
    /// 从设备：使用 --auth-code <CODE> --master <MASTER_GYID> 完成关联
    Link {
        /// 授权码（从设备输入收到的码；主设备填 gen 生成授权码）
        #[arg(short, long)]
        auth_code: String,

        /// 主设备 GyID（从设备模式下指定主设备 ID）
        #[arg(short, long)]
        master: Option<String>,
    },

    /// 列出关联设备
    Devices {
        /// GyID (默认使用本地存储)
        #[arg(short, long)]
        gyid: Option<String>,
    },

    /// 配置管理
    Config {
        /// 操作类型
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// 导出 GyID
    Export {
        /// 导出格式
        #[arg(short, long, default_value = "json")]
        format: ExportFormat,

        /// 输出文件 (默认 stdout)
        #[arg(short, long)]
        output: Option<String>,

        /// 导出所有 GyID（批量导出）
        #[arg(short, long)]
        all: bool,
    },

    /// 链上锚定管理
    Chain {
        /// 链上操作
        #[command(subcommand)]
        action: commands::chain::ChainAction,
    },

    /// H3 六边形网格工具
    ///
    /// 编码/解码 H3 单元格，查询邻居等
    H3 {
        /// H3 操作
        #[command(subcommand)]
        action: commands::h3::H3SubCommand,
    },

    /// 存储管理（列出/导入/删除/备份 GyID）
    Storage {
        /// 存储操作
        #[command(subcommand)]
        action: commands::storage::StorageAction,
    },

    /// 检查 GPS 可用性
    ///
    /// 快速检测 GPS 状态，不会长时间等待
    GpsCheck,

    /// 手动输入坐标测试
    GeoManual {
        /// 纬度
        #[arg(required = true)]
        lat: f64,
        /// 经度
        #[arg(required = true)]
        lon: f64,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum GeoLevel {
    /// 城市级 (IP 定位)
    City,
    /// 区域级 (WiFi BSSID)
    District,
    /// 精确级 (GPS + WiFi)
    Exact,
    /// 手动输入坐标
    Manual,
    /// 不使用位置信息
    None,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// 显示当前配置
    Show,
    /// 重置为默认配置
    Reset,
    /// 设置配置项
    Set {
        /// 配置键
        key: String,
        /// 配置值
        value: String,
    },
}

#[derive(clap::ValueEnum, Clone)]
enum ExportFormat {
    /// JSON 格式
    Json,
    /// 纯文本格式
    Text,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // 初始化日志
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info"));
    
    fmt()
        .with_env_filter(filter)
        .with_target(false)
        .with_level(true)
        .init();
    
    let cli = Cli::parse();
    
    // 根据命令执行
    match &cli.command {
        Commands::Generate { avatar, level, lat, lon } => {
            commands::generate::run(avatar.as_deref(), level, lat.as_ref(), lon.as_ref()).await?;
        }
        Commands::Verify { gyid } => {
            commands::verify::run(gyid)?;
        }
        Commands::Link { auth_code, master } => {
            commands::link::run(auth_code, master.as_deref()).await?;
        }
        Commands::Devices { gyid } => {
            commands::devices::run(gyid.as_deref()).await?;
        }
        Commands::Config { action } => {
            commands::config::run(action)?;
        }
        Commands::Export { format, output, all } => {
            commands::export::run(format, output.as_deref(), *all).await?;
        }
        Commands::Chain { action } => {
            commands::chain::run(action).await?;
        }
        Commands::H3 { action } => {
            let cmd = commands::h3::H3Command { subcommand: action.clone() };
            cmd.run().await?;
        }
        Commands::Storage { action } => {
            commands::storage::run(action)?;
        }
        Commands::GpsCheck => {
            commands::gps::run_check().await?;
        }
        Commands::GeoManual { lat, lon } => {
            commands::gps::run_manual_test(*lat, *lon)?;
        }
    }
    
    Ok(())
}
