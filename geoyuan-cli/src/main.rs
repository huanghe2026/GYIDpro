// ═══════════════════════════════════════════════════════════════════════════
// GeoYuan CLI - 命令行工具入口
// 
// 命令结构:
//   geoyuan identity generate [--photo <path>] [--testnet]
//   geoyuan identity verify <gyid>
//   geoyuan identity info <gyid>
//   geoyuan wallet show [--gyid <gyid>]
//   geoyuan wallet mint --photo <path>
//   geoyuan consensus status
//   geoyuan node start [--port <port>]
//   geoyuan storage stats
// ═══════════════════════════════════════════════════════════════════════════

mod commands;

use clap::Parser;
use commands::Cli;

fn main() {
    env_logger::init_from_env(env_logger::Env::default().default_filter_or("warn"));

    let cli = Cli::parse();

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("Failed to build tokio runtime");

    if let Err(e) = rt.block_on(cli.run()) {
        eprintln!("{} {}", colored::Colorize::red("Error:"), e);
        std::process::exit(1);
    }
}
