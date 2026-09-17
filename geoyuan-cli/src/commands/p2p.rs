// ─────────────────────────────────────────────────────────────────────────────
// geoyuan p2p 子命令
// ─────────────────────────────────────────────────────────────────────────────

use clap::Subcommand;
use anyhow::Result;
use colored::Colorize;

use geoyuan_core::p2p::swarm::{start_swarm, pick_listen_port};
use geoyuan_core::geo::h3grid;

#[derive(Subcommand)]
pub enum P2pCmd {
    /// 启动 P2P 节点（监听指定端口）
    Start {
        /// 监听端口（默认自动选择 4001-4999）
        #[arg(short, long, default_value_t = 0)]
        port: u16,
    },

    /// 显示 P2P 节点状态
    Status,

    /// 列出已连接的对等节点
    Peers,

    /// 列出地理邻近节点（基于 H3 cell）
    PeersNearby {
        /// H3 cell 索引（16进制）
        h3_cell: String,
    },
}

impl P2pCmd {
    pub async fn run(self) -> Result<()> {
        match self {
            P2pCmd::Start { port } => cmd_start(port).await,
            P2pCmd::Status => cmd_status().await,
            P2pCmd::Peers => cmd_peers().await,
            P2pCmd::PeersNearby { h3_cell } => cmd_peers_nearby(&h3_cell).await,
        }
    }
}

async fn cmd_start(port: u16) -> Result<()> {
    let listen_port = if port == 0 { pick_listen_port() } else { port };
    println!("{} GeoYuan P2P 节点启动中...", "🌐".bright_cyan());
    println!("   监听端口: {}", listen_port.to_string().bright_yellow());

    let handle = start_swarm(listen_port);
    let status = handle.cached_status();

    println!("{} 本机 PeerID: {}", "  •".bright_cyan(), status.local_peer_id.bright_green());
    println!("{} 按 Ctrl+C 停止节点", "  •".bright_cyan());

    // 等待 Ctrl+C
    tokio::signal::ctrl_c().await?;
    println!("\n{} 正在停止节点...", "🛑".bright_red());
    handle.shutdown();
    println!("{} 节点已停止", "✅".bright_green());
    Ok(())
}

async fn cmd_status() -> Result<()> {
    // 启动临时节点查询状态
    let handle = start_swarm(pick_listen_port());
    let status = handle.cached_status();

    println!("{} GeoYuan P2P 节点状态", "📊".bright_cyan());
    println!("{}", "─".repeat(50));
    println!("  {}  运行状态: {}",
        if status.running { "🟢" } else { "🔴" },
        if status.running { "在线".bright_green() } else { "离线".bright_red() }
    );
    println!("  {}  PeerID:    {}", "•", status.local_peer_id.bright_yellow());
    println!("  {}  连接数:    {}", "•", status.peer_count.to_string().bright_cyan());
    println!("  {}  监听地址:  {}", "•", status.listen_addrs.join(", ").bright_cyan());

    if let Some(h3) = status.local_h3_cell {
        println!("  {}  H3 Cell:   {:x}", "•", h3);
    }

    handle.shutdown();
    Ok(())
}

async fn cmd_peers() -> Result<()> {
    let handle = start_swarm(pick_listen_port());
    let status = handle.cached_status();

    if status.peer_count == 0 {
        println!("{} 当前无已连接节点", "ℹ️".bright_yellow());
        handle.shutdown();
        return Ok(());
    }

    println!("{} 已连接节点 ({} 个)", "📋".bright_cyan(), status.peer_count);
    println!("{}", "─".repeat(50));

    // 显示连接信息
    for (i, addr) in status.listen_addrs.iter().enumerate() {
        println!("  {}. {}", i + 1, addr.bright_green());
    }

    handle.shutdown();
    Ok(())
}

async fn cmd_peers_nearby(h3_hex: &str) -> Result<()> {
    let src_cell = match h3grid::cell_from_string(h3_hex) {
        Some(c) => c,
        None => {
            println!("{} 无效的 H3 cell 格式（需要 16 进制字符串）", "❌".bright_red());
            return Ok(());
        }
    };

    let handle = start_swarm(pick_listen_port());
    let status = handle.cached_status();

    if status.peer_count == 0 {
        println!("{} 当前无已连接节点", "ℹ️".bright_yellow());
        handle.shutdown();
        return Ok(());
    }

    // 显示本机 H3 cell（如果已设置）
    if let Some(local) = status.local_h3_cell {
        let local_cell = h3grid::H3Cell::from_u64(local);
        let within = h3grid::is_within_ring(src_cell, local_cell, h3grid::GEOCAST_NEIGHBOR_RINGS);
        if within {
            println!("  ✓ 本机在源 H3 cell {} 环内", h3grid::GEOCAST_NEIGHBOR_RINGS);
        } else {
            println!("  ✗ 本机不在源 H3 cell {} 环内", h3grid::GEOCAST_NEIGHBOR_RINGS);
        }
    } else {
        println!("  ℹ️ 本机未设置 H3 cell（未定位）");
    }

    println!("  {}  源 H3 cell: {:x}", "•".bright_cyan(), src_cell.to_u64());
    println!("  {}  当前连接: {} 个节点", "•".bright_cyan(), status.peer_count);

    handle.shutdown();
    Ok(())
}
