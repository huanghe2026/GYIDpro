// ═══════════════════════════════════════════════════════════════════════════
// CLI 命令定义与处理
// ═══════════════════════════════════════════════════════════════════════════

mod identity;
mod wallet;
mod consensus;
mod storage;
mod p2p;

pub use root::Cli;

mod root {
    use clap::{Parser, Subcommand};
    use anyhow::Result;

    use super::{identity, wallet, consensus, storage, p2p};

    /// GeoYuan CLI - 地理元宇宙身份与经济系统命令行工具
    #[derive(Parser)]
    #[command(
        name = "geoyuan",
        version = env!("CARGO_PKG_VERSION"),
        about = "GeoYuan CLI — photo-based decentralized identity",
        long_about = None,
        arg_required_else_help = true,
    )]
    pub struct Cli {
        #[command(subcommand)]
        pub command: Commands,
    }

    #[derive(Subcommand)]
    pub enum Commands {
        /// GyID 身份操作
        #[command(subcommand)]
        Identity(identity::IdentityCmd),

        /// 钱包与 Geoyuan 操作
        #[command(subcommand)]
        Wallet(wallet::WalletCmd),

        /// PoI 共识状态
        #[command(subcommand)]
        Consensus(consensus::ConsensusCmd),

        /// 本地存储操作
        #[command(subcommand)]
        Storage(storage::StorageCmd),

        /// P2P 节点管理（GeoCast 地理广播）
        #[command(subcommand)]
        P2p(p2p::P2pCmd),
    }

    impl Cli {
        pub async fn run(self) -> Result<()> {
            match self.command {
                Commands::Identity(cmd) => cmd.run().await,
                Commands::Wallet(cmd) => cmd.run().await,
                Commands::Consensus(cmd) => cmd.run().await,
                Commands::Storage(cmd) => cmd.run().await,
                Commands::P2p(cmd) => cmd.run().await,
            }
        }
    }
}
