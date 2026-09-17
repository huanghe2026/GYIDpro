// ═══════════════════════════════════════════════════════════════════════════
// GeoYuan GUI - P2P 节点管理器（真实 libp2p 版本）
// 基于 geoyuan-core::p2p::swarm::SwarmHandle 实现
// ═══════════════════════════════════════════════════════════════════════════

use std::sync::{Arc, Mutex};

use geoyuan_core::p2p::{SwarmHandle, SwarmStatus, MessageType, TransferPayload, WalletSyncResponse, start_swarm, pick_listen_port};

// ─────────────────────────────────────────────────────────────────────────────
// 对外暴露的节点状态（与 Slint UI 字段对应）
// ─────────────────────────────────────────────────────────────────────────────

/// 轻量快照，供 GUI 的 slint::Timer 轮询
#[derive(Debug, Clone, Default)]
pub struct NodeStatus {
    pub online: bool,
    pub peer_count: usize,
    pub local_peer_id: Option<String>,
    #[allow(dead_code)]
    pub listen_addrs: Vec<String>,
    /// 本次轮询收到的入站转账（通过 sync_status 返回）
    #[allow(dead_code)]
    pub pending_transfers: Vec<TransferPayload>,
    /// 本次轮询收到的钱包同步响应（通过 sync_status 返回）
    #[allow(dead_code)]
    pub pending_wallet_responses: Vec<WalletSyncResponse>,
}

impl From<&SwarmStatus> for NodeStatus {
    fn from(s: &SwarmStatus) -> Self {
        Self {
            online: s.running,
            peer_count: s.peer_count,
            local_peer_id: if s.local_peer_id.is_empty() {
                None
            } else {
                Some(s.local_peer_id.clone())
            },
            listen_addrs: s.listen_addrs.clone(),
            pending_transfers: Vec::new(), // 不从 SwarmStatus 复制，由 sync_status 单独处理
            pending_wallet_responses: Vec::new(), // 不从 SwarmStatus 复制，由 sync_status 单独处理
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// NodeManager：包装 SwarmHandle，生命周期与 main 绑定
// ─────────────────────────────────────────────────────────────────────────────

/// P2P 节点管理器。
///
/// - `start()` 启动真实 libp2p Swarm（mDNS + Kademlia + RequestResponse）
/// - `status_handle()` 返回供 UI 轮询的 `Arc<Mutex<NodeStatus>>`
/// - `Drop` 时自动停止节点
pub struct NodeManager {
    /// 核心 Swarm 控制句柄（`start()` 后有值）
    handle: Option<SwarmHandle>,
    /// GUI 轮询用的状态镜像（每次轮询从 SwarmHandle 同步）
    ui_status: Arc<Mutex<NodeStatus>>,
}

impl NodeManager {
    /// 创建节点管理器（不自动启动）
    pub fn new() -> Self {
        Self {
            handle: None,
            ui_status: Arc::new(Mutex::new(NodeStatus::default())),
        }
    }

    /// 获取供 Slint Timer 轮询的状态引用
    pub fn status_handle(&self) -> Arc<Mutex<NodeStatus>> {
        self.ui_status.clone()
    }

    /// 启动真实 libp2p Swarm
    ///
    /// 必须在 tokio runtime 上下文中调用。
    pub fn start(&mut self) {
        if self.handle.is_some() {
            log::warn!("节点已在运行，跳过重复启动");
            return;
        }

        let port = pick_listen_port();
        log::info!("🌐 启动 P2P 节点，监听端口 {}", port);

        let swarm_handle = start_swarm(port);

        // 初始化 UI 状态
        {
            let swarm_status = swarm_handle.cached_status();
            let mut s = self.ui_status.lock().unwrap();
            *s = NodeStatus::from(&swarm_status);
            // 标记为"在线"（Swarm 已提交到 tokio，等待 NewListenAddr 事件）
            s.online = true;
            if s.local_peer_id.is_none() || s.local_peer_id.as_deref() == Some("") {
                s.local_peer_id = Some("(初始化中...)".to_string());
            }
        }

        self.handle = Some(swarm_handle);
        log::info!("✅ P2P 节点管理器已启动");
    }

    /// 同步 SwarmHandle 状态到 UI 状态缓存，返回收到的入站转账和钱包同步响应
    pub fn sync_status(&self) -> (Vec<TransferPayload>, Vec<WalletSyncResponse>) {
        let mut pending = Vec::new();
        let mut wallet_responses = Vec::new();
        if let Some(ref handle) = self.handle {
            let swarm_status = handle.cached_status();
            let mut s = self.ui_status.lock().unwrap();
            *s = NodeStatus::from(&swarm_status);
            // 若 Swarm 刚启动还未触发 running=true，保持 online=true
            if self.handle.is_some() && !s.online {
                s.online = true;
            }
            // 取出待处理的入站转账
            pending = handle.drain_pending_transfers();
            // 取出待处理的钱包同步响应
            wallet_responses = handle.drain_pending_wallet_responses();
        }
        (pending, wallet_responses)
    }

    /// 停止节点
    pub fn stop(&mut self) {
        if let Some(ref handle) = self.handle {
            handle.shutdown();
            log::info!("🔴 P2P 节点已停止");
        }
        self.handle = None;

        let mut s = self.ui_status.lock().unwrap();
        s.online = false;
        s.peer_count = 0;
    }

    /// 读取当前 UI 状态快照
    #[allow(dead_code)]
    pub fn snapshot(&self) -> NodeStatus {
        self.ui_status.lock().unwrap().clone()
    }

    /// 是否已启动
    #[allow(dead_code)]
    pub fn is_running(&self) -> bool {
        self.handle.is_some()
    }

    /// 广播消息到所有已连接节点
    pub fn broadcast(&self, msg_type: MessageType, payload: Vec<u8>) {
        if let Some(ref handle) = self.handle {
            handle.broadcast(msg_type, payload);
            log::debug!("📤 NodeManager: 广播 {:?} 消息", msg_type);
        } else {
            log::warn!("⚠️ NodeManager: 节点未启动，无法广播");
        }
    }

    /// 设置本地钱包数据（供其他节点同步查询时回复）
    pub fn set_wallet_data(&self, data: WalletSyncResponse) {
        if let Some(ref handle) = self.handle {
            handle.set_wallet_data(data);
            log::debug!("💰 NodeManager: 钱包同步数据已更新");
        }
    }
}

impl Default for NodeManager {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for NodeManager {
    fn drop(&mut self) {
        self.stop();
    }
}
