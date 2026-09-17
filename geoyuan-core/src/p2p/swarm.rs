//! GeoYuan libp2p Swarm 事件循环
//!
//! 基于 libp2p 0.53，实现：
//! - TCP 传输 + Noise 加密 + Yamux 多路复用
//! - mDNS 本地节点自动发现
//! - Kademlia DHT 全局路由
//! - 自定义 `/geoyuan/1.0` 请求-响应协议（CBOR 编码）
//! - `SwarmHandle`：线程安全的外部控制句柄

use std::sync::{Arc, Mutex};
use std::time::Duration;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc, oneshot};

use crate::p2p::protocol::{MessageType, Envelope, TransferPayload, WalletSyncPayload, WalletSyncResponse};
use crate::geo::h3grid::{self, H3Cell};

// ─────────────────────────────────────────────────────────────────────────────
// 公共状态结构（GUI 读取用）
// ─────────────────────────────────────────────────────────────────────────────

/// Swarm 对外暴露的轻量级状态快照
#[derive(Debug, Clone, Default)]
pub struct SwarmStatus {
    /// 本地 PeerID（Base58）
    pub local_peer_id: String,
    /// 监听地址列表
    pub listen_addrs: Vec<String>,
    /// 已连接节点数
    pub peer_count: usize,
    /// Swarm 是否正在运行
    pub running: bool,
    /// 收到的转账消息缓冲区
    pub pending_transfers: Vec<TransferPayload>,
    /// 本地钱包信息（供 WalletResponse 回复使用）
    pub wallet_sync_data: Option<WalletSyncResponse>,
    /// 收到的钱包同步响应缓冲区
    pub pending_wallet_responses: Vec<WalletSyncResponse>,
    /// 本机 H3 cell（GeoCast 路由用）
    pub local_h3_cell: Option<u64>,
}

// ─────────────────────────────────────────────────────────────────────────────
// 控制命令（主线程 → Swarm 任务）
// ─────────────────────────────────────────────────────────────────────────────

/// 发送到 Swarm 后台任务的控制指令
pub enum SwarmCommand {
    /// 广播一条消息给所有已连接节点（source_h3_cell=None 为全网广播）
    Broadcast {
        msg_type: MessageType,
        payload: Vec<u8>,
        /// 发送者 H3 cell（Some 时启用 GeoCast 地理广播）
        source_h3_cell: Option<u64>,
    },
    /// 请求停止 Swarm
    Shutdown,
    /// 查询当前状态（回调通道）
    QueryStatus(oneshot::Sender<SwarmStatus>),
    /// 拨号连接到指定地址
    #[allow(dead_code)]
    Dial(String),
    /// 设置本地钱包信息（GUI 在状态变化时调用）
    SetWalletData(WalletSyncResponse),
    /// 设置本机 H3 cell（GeoCast 地理路由定位）
    AnnounceLocation(u64),
}

// ─────────────────────────────────────────────────────────────────────────────
// SwarmHandle：对外 API
// ─────────────────────────────────────────────────────────────────────────────

/// 线程安全的 Swarm 控制句柄。
///
/// Clone 后可以在任意线程中使用，内部通过 `mpsc` 与后台任务通信。
#[derive(Clone)]
pub struct SwarmHandle {
    cmd_tx: mpsc::UnboundedSender<SwarmCommand>,
    /// 共享状态快照（定期由后台任务写入）
    pub status: Arc<Mutex<SwarmStatus>>,
}

impl SwarmHandle {
    /// 广播消息（全网广播）
    #[allow(dead_code)]
    pub fn broadcast(&self, msg_type: MessageType, payload: Vec<u8>) {
        let _ = self.cmd_tx.send(SwarmCommand::Broadcast { msg_type, payload, source_h3_cell: None });
    }

    /// GeoCast 地理广播（仅邻近 H3 环内节点接收）
    #[allow(dead_code)]
    pub fn geocast_broadcast(&self, msg_type: MessageType, payload: Vec<u8>, h3_cell: u64) {
        let _ = self.cmd_tx.send(SwarmCommand::Broadcast { msg_type, payload, source_h3_cell: Some(h3_cell) });
    }

    /// 停止 Swarm
    pub fn shutdown(&self) {
        let _ = self.cmd_tx.send(SwarmCommand::Shutdown);
    }

    /// 异步查询状态快照
    #[allow(dead_code)]
    pub async fn query_status(&self) -> SwarmStatus {
        let (tx, rx) = oneshot::channel();
        if self.cmd_tx.send(SwarmCommand::QueryStatus(tx)).is_ok() {
            rx.await.unwrap_or_default()
        } else {
            SwarmStatus::default()
        }
    }

    /// 同步读取缓存状态（不阻塞）
    pub fn cached_status(&self) -> SwarmStatus {
        self.status.lock().unwrap().clone()
    }

    /// 取出并清空待处理的入站转账消息（GUI 每 5 秒调用）
    pub fn drain_pending_transfers(&self) -> Vec<TransferPayload> {
        let mut s = self.status.lock().unwrap();
        std::mem::take(&mut s.pending_transfers)
    }

    /// 取出并清空待处理的钱包同步响应（GUI 每 5 秒调用）
    pub fn drain_pending_wallet_responses(&self) -> Vec<WalletSyncResponse> {
        let mut s = self.status.lock().unwrap();
        std::mem::take(&mut s.pending_wallet_responses)
    }

    /// 拨号
    #[allow(dead_code)]
    pub fn dial(&self, addr: String) {
        let _ = self.cmd_tx.send(SwarmCommand::Dial(addr));
    }

    /// 设置本地钱包数据（供 WalletResponse 回复时使用）
    pub fn set_wallet_data(&self, data: WalletSyncResponse) {
        let _ = self.cmd_tx.send(SwarmCommand::SetWalletData(data));
    }

    /// 设置本机 H3 cell（GeoCast 地理路由用）
    #[allow(dead_code)]
    pub fn announce_location(&self, h3_cell: u64) {
        let _ = self.cmd_tx.send(SwarmCommand::AnnounceLocation(h3_cell));
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 自定义 Behaviour（仅 native feature 下编译）
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(feature = "native")]
/// GeoYuan 请求-响应消息（CBOR 字节）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeoyuanMessage(pub Vec<u8>);

/// `#[derive(NetworkBehaviour)]` 宏会自动生成 `GeoyuanBehaviourEvent` 枚举
#[cfg(feature = "native")]
#[derive(libp2p::swarm::NetworkBehaviour)]
pub struct GeoyuanBehaviour {
    pub mdns: libp2p::mdns::tokio::Behaviour,
    pub kad: libp2p::kad::Behaviour<libp2p::kad::store::MemoryStore>,
    pub rr: libp2p::request_response::cbor::Behaviour<GeoyuanMessage, GeoyuanMessage>,
}

// ─────────────────────────────────────────────────────────────────────────────
// 消息处理回调
// ─────────────────────────────────────────────────────────────────────────────

/// 处理收到的 CoinTransferred 消息
/// 将转账信息写入 SwarmStatus.pending_transfers 供 GUI 轮询
fn handle_coin_transferred(envelope: &Envelope, status: &Arc<Mutex<SwarmStatus>>) {
    match serde_json::from_slice::<TransferPayload>(&envelope.payload) {
        Ok(tx) => {
            tracing::info!(
                "💸 转账广播: tx={}, {} → {}, amount={} GY, pubkey={}...",
                &tx.tx_id[..20.min(tx.tx_id.len())],
                &tx.from_gyid[..16.min(tx.from_gyid.len())],
                &tx.to_gyid[..16.min(tx.to_gyid.len())],
                tx.amount,
                &tx.sender_pubkey[..16.min(tx.sender_pubkey.len())],
            );
            // 写入共享状态供 GUI 轮询读取
            if let Ok(mut s) = status.lock() {
                s.pending_transfers.push(tx);
                // 限制缓冲区大小，避免内存无限增长
                if s.pending_transfers.len() > 100 {
                    s.pending_transfers.drain(..50);
                }
            }
        }
        Err(e) => {
            tracing::warn!("⚠️ CoinTransferred payload 解析失败: {}", e);
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Swarm 启动器
// ─────────────────────────────────────────────────────────────────────────────

/// 启动 libp2p Swarm，返回控制句柄。
///
/// 必须在已有 tokio runtime 的上下文中调用。
#[cfg(feature = "native")]
pub fn start_swarm(listen_port: u16) -> SwarmHandle {
    use libp2p::{
        PeerId,
        identity,
        swarm::SwarmEvent,
        request_response::ProtocolSupport,
        StreamProtocol,
        core::upgrade::Version,
        futures::StreamExt,
    };

    let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<SwarmCommand>();
    let status = Arc::new(Mutex::new(SwarmStatus::default()));
    let status_bg = status.clone();

    tokio::spawn(async move {
        // ── 生成身份密钥 ──────────────────────────────────────
        let local_key = identity::Keypair::generate_ed25519();
        let local_peer_id = PeerId::from(local_key.public());

        tracing::info!("🌐 本地 PeerID: {}", local_peer_id);

        // 更新共享状态
        {
            let mut s = status_bg.lock().unwrap();
            s.local_peer_id = local_peer_id.to_base58();
            s.running = true;
        }

        // ── 构建 Transport ─────────────────────────────────────
        use libp2p::Transport as _;
        let transport = libp2p::tcp::tokio::Transport::default()
            .upgrade(Version::V1)
            .authenticate(libp2p::noise::Config::new(&local_key).expect("noise config"))
            .multiplex(libp2p::yamux::Config::default())
            .boxed();

        // ── 构建 mDNS Behaviour ─────────────────────────────────
        let mdns_cfg = libp2p::mdns::Config {
            ttl: Duration::from_secs(60),
            query_interval: Duration::from_secs(5),
            enable_ipv6: false,
        };
        let mdns = libp2p::mdns::tokio::Behaviour::new(mdns_cfg, local_peer_id)
            .expect("mDNS 创建失败");

        // ── 构建 Kademlia Behaviour ─────────────────────────────
        let store = libp2p::kad::store::MemoryStore::new(local_peer_id);
        let kad = libp2p::kad::Behaviour::new(local_peer_id, store);

        // ── 构建 RequestResponse Behaviour ─────────────────────
        let rr_proto = [(
            StreamProtocol::new("/geoyuan/1.0"),
            ProtocolSupport::Full,
        )];
        let rr = libp2p::request_response::cbor::Behaviour::<GeoyuanMessage, GeoyuanMessage>::new(
            rr_proto,
            libp2p::request_response::Config::default(),
        );

        let behaviour = GeoyuanBehaviour { mdns, kad, rr };

        // ── 构建 Swarm ─────────────────────────────────────────
        let mut swarm = libp2p::SwarmBuilder::with_existing_identity(local_key)
            .with_tokio()
            .with_other_transport(|_| Ok(transport))
            .expect("transport 构建失败")
            .with_behaviour(|_| behaviour)
            .expect("behaviour 构建失败")
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        // 监听
        let listen_addr: libp2p::Multiaddr = format!("/ip4/0.0.0.0/tcp/{}", listen_port)
            .parse()
            .unwrap();
        if let Err(e) = swarm.listen_on(listen_addr) {
            tracing::error!("❌ Swarm 监听失败: {}", e);
            let mut s = status_bg.lock().unwrap();
            s.running = false;
            return;
        }

        tracing::info!("📡 GeoYuan P2P 节点启动，监听端口 {}", listen_port);

        // ── 事件循环 ───────────────────────────────────────────
        loop {
            tokio::select! {
                // 处理控制命令
                cmd = cmd_rx.recv() => {
                    match cmd {
                        Some(SwarmCommand::Shutdown) | None => {
                            tracing::info!("🔴 Swarm 收到关闭指令");
                            break;
                        }
                        Some(SwarmCommand::QueryStatus(reply)) => {
                            let s = status_bg.lock().unwrap().clone();
                            let _ = reply.send(s);
                        }
                        Some(SwarmCommand::Dial(addr_str)) => {
                            if let Ok(addr) = addr_str.parse::<libp2p::Multiaddr>() {
                                match swarm.dial(addr.clone()) {
                                    Ok(_) => tracing::info!("🔗 拨号: {}", addr),
                                    Err(e) => tracing::warn!("⚠️ 拨号失败 {}: {}", addr, e),
                                }
                            }
                        }
                        Some(SwarmCommand::Broadcast { msg_type, payload, .. }) => {
                            // GeoCast: 发送端不做过滤，接收端按 H3 邻近性自行决定是否处理
                            let peers: Vec<PeerId> = swarm.connected_peers().cloned().collect();
                            let peer_count = peers.len();
                            let msg = GeoyuanMessage(payload);
                            for peer_id in peers {
                                swarm.behaviour_mut().rr.send_request(&peer_id, msg.clone());
                            }
                            tracing::debug!("📤 广播 {:?} 到 {} 个节点", msg_type, peer_count);
                        }
                        Some(SwarmCommand::SetWalletData(data)) => {
                            let mut s = status_bg.lock().unwrap();
                            s.wallet_sync_data = Some(data);
                            tracing::debug!("💰 本地钱包数据已更新");
                        }
                        Some(SwarmCommand::AnnounceLocation(h3_cell)) => {
                            let mut s = status_bg.lock().unwrap();
                            s.local_h3_cell = Some(h3_cell);
                            tracing::info!("📍 本机 H3 cell 已设置: {:x}", h3_cell);
                        }
                    }
                }

                // 处理 Swarm 事件
                event = swarm.select_next_some() => {
                    match event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            tracing::info!("🎧 监听地址: {}", address);
                            let mut s = status_bg.lock().unwrap();
                            s.listen_addrs.push(address.to_string());
                        }

                        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                            let peer_count = swarm.connected_peers().count();
                            tracing::info!("🤝 节点连接: {} (连接数={})", peer_id, peer_count);
                            let mut s = status_bg.lock().unwrap();
                            s.peer_count = peer_count;
                        }

                        SwarmEvent::ConnectionClosed { peer_id, .. } => {
                            let peer_count = swarm.connected_peers().count();
                            tracing::info!("👋 节点断开: {} (连接数={})", peer_id, peer_count);
                            let mut s = status_bg.lock().unwrap();
                            s.peer_count = peer_count;
                        }

                        SwarmEvent::Behaviour(GeoyuanBehaviourEvent::Mdns(mdns_event)) => {
                            match mdns_event {
                                libp2p::mdns::Event::Discovered(peers) => {
                                    for (peer_id, addr) in peers {
                                        tracing::info!("🔍 mDNS 发现: {} @ {}", peer_id, addr);
                                        swarm.behaviour_mut().kad.add_address(&peer_id, addr.clone());
                                        if let Err(e) = swarm.dial(addr) {
                                            tracing::debug!("mDNS 自动拨号（已连接或失败）: {}", e);
                                        }
                                    }
                                }
                                libp2p::mdns::Event::Expired(peers) => {
                                    for (peer_id, _) in peers {
                                        tracing::debug!("⏱️ mDNS 节点过期: {}", peer_id);
                                    }
                                }
                            }
                        }

                        SwarmEvent::Behaviour(GeoyuanBehaviourEvent::Kad(kad_event)) => {
                            match kad_event {
                                libp2p::kad::Event::RoutingUpdated { peer, .. } => {
                                    tracing::debug!("📚 Kad 路由更新: {}", peer);
                                }
                                libp2p::kad::Event::OutboundQueryProgressed { result, .. } => {
                                    tracing::debug!("📡 Kad 查询进度: {:?}", result);
                                }
                                _ => {}
                            }
                        }

                        SwarmEvent::Behaviour(GeoyuanBehaviourEvent::Rr(rr_event)) => {
                            match rr_event {
                                libp2p::request_response::Event::Message { peer, message } => {
                                    match message {
                                libp2p::request_response::Message::Request { request, channel, .. } => {
                                    // 尝试反序列化 Envelope（CBOR 格式）
                                    match Envelope::deserialize_cbor(&request.0) {
                                        Ok(envelope) => {
                                            tracing::info!(
                                                "📨 收到 {:?} from {}, payload={} bytes, sig={}",
                                                envelope.msg_type, peer, envelope.payload.len(),
                                                if envelope.signature.is_some() { "有" } else { "无" }
                                            );

                                            // 按消息类型处理
                                            match envelope.msg_type {
                                                MessageType::CoinTransferred => {
                                                    handle_coin_transferred(&envelope, &status_bg);
                                                }
                                                MessageType::CoinMinted => {
                                                    tracing::info!("🪙 收到铸造广播 from {}", envelope.sender);
                                                }
                                                MessageType::Ping => {
                                                    // 自动回复 Pong
                                                    let pong = Envelope::new(
                                                        MessageType::Pong,
                                                        &local_peer_id.to_base58(),
                                                        vec![],
                                                    );
                                                    if let Ok(bytes) = pong.serialize_cbor() {
                                                        let _ = swarm.behaviour_mut().rr.send_response(channel, GeoyuanMessage(bytes));
                                                    }
                                                }
                                                // GeoCast 消息：接收端按 H3 邻近性过滤
                                                MessageType::GeoCast => {
                                                    let local_cell = status_bg.lock().unwrap().local_h3_cell;
                                                    if let Some(local) = local_cell {
                                                        // 解析 GeoCast 载荷中的源 cell
                                                        // 载荷格式: 8 bytes H3 cell (小端) + 原始消息
                                                        if envelope.payload.len() >= 8 {
                                                            let src_bytes: [u8; 8] = envelope.payload[..8].try_into().unwrap_or([0u8; 8]);
                                                            let src_cell = u64::from_le_bytes(src_bytes);
                                                            let source = H3Cell::from_u64(src_cell);
                                                            let local_h3 = H3Cell::from_u64(local);
                                                            if h3grid::is_within_ring(source, local_h3, h3grid::GEOCAST_NEIGHBOR_RINGS) {
                                                                tracing::info!("🌐 GeoCast: 在范围内，处理消息 from {}", envelope.sender);
                                                                // 去掉前 8 字节的 cell 头部，处理剩余载荷
                                                                let inner_payload = envelope.payload[8..].to_vec();
                                                                // 尝试解析为转账或其他类型
                                                                if let Ok(tx) = serde_json::from_slice::<TransferPayload>(&inner_payload) {
                                                                    if let Ok(mut s) = status_bg.lock() {
                                                                        s.pending_transfers.push(tx);
                                                                        if s.pending_transfers.len() > 100 {
                                                                            s.pending_transfers.drain(..50);
                                                                        }
                                                                    }
                                                                }
                                                                // 回复 GeoCastResponse
                                                                let response = Envelope::new(
                                                                    MessageType::GeoCastResponse,
                                                                    &local_peer_id.to_base58(),
                                                                    local.to_le_bytes().to_vec(),
                                                                );
                                                                if let Ok(bytes) = response.serialize_cbor() {
                                                                    let _ = swarm.behaviour_mut().rr.send_response(channel, GeoyuanMessage(bytes));
                                                                }
                                                            } else {
                                                                tracing::debug!("🗺️ GeoCast: 筛掉（{} 环外）", h3grid::GEOCAST_NEIGHBOR_RINGS);
                                                            }
                                                        }
                                                    } else {
                                                        tracing::debug!("🗺️ GeoCast: 本机未定位，忽略消息");
                                                    }
                                                }
                                                MessageType::GeoCastResponse => {
                                                    tracing::debug!("🌐 收到 GeoCast 响应 from {}", envelope.sender);
                                                    let _ = swarm.behaviour_mut().rr.send_response(channel, GeoyuanMessage(vec![]));
                                                }
                                                MessageType::WalletRequest | MessageType::GeoIdRequest => {
                                                    // 处理钱包同步请求
                                                    if envelope.msg_type == MessageType::WalletRequest {
                                                        // 解析请求
                                                        if let Ok(_req) = serde_json::from_slice::<WalletSyncPayload>(&envelope.payload) {
                                                            // 从 SwarmStatus 读取本地钱包数据
                                                            let response_payload = {
                                                                let s = status_bg.lock().unwrap();
                                                                s.wallet_sync_data.clone()
                                                            };
                                                            let response = match response_payload {
                                                                Some(data) => {
                                                                    tracing::info!("💰 回复钱包同步: balance={}, tx_count={}", data.balance, data.tx_count);
                                                                    Envelope::new(
                                                                        MessageType::WalletResponse,
                                                                        &local_peer_id.to_base58(),
                                                                        serde_json::to_vec(&data).unwrap_or_default(),
                                                                    )
                                                                }
                                                                None => {
                                                                    // 没有钱包数据，回复空响应
                                                    tracing::info!("💰 收到钱包同步请求，但本地无钱包数据");
                                                    Envelope::new(
                                                        MessageType::WalletResponse,
                                                        &local_peer_id.to_base58(),
                                                        serde_json::to_vec(&WalletSyncResponse {
                                                            responder_gyid: String::new(),
                                                            balance: 0.0,
                                                            tx_count: 0,
                                                            recent_txs: Vec::new(),
                                                            timestamp: chrono::Utc::now().timestamp(),
                                                        }).unwrap_or_default(),
                                                    )
                                                                }
                                                            };
                                                            if let Ok(bytes) = response.serialize_cbor() {
                                                                let _ = swarm.behaviour_mut().rr.send_response(channel, GeoyuanMessage(bytes));
                                                            }
                                                        } else {
                                                            tracing::warn!("⚠️ WalletRequest payload 解析失败");
                                                        }
                                                    } else {
                                                        tracing::debug!("📋 收到 GeoIdRequest（暂未实现同步）");
                                                    }
                                                }
                                                _ => {
                                                    tracing::debug!("📨 收到其他消息类型: {:?}", envelope.msg_type);
                                                }
                                            }
                                        }
                                        Err(e) => {
                                            tracing::warn!("⚠️ 收到无法解析的消息 from {}: {} ({} bytes)", peer, e, request.0.len());
                                        }
                                    }
                                }
                                        libp2p::request_response::Message::Response { response, .. } => {
                                            // 尝试解析响应内容
                                            match Envelope::deserialize_cbor(&response.0) {
                                                Ok(env) => {
                                                    match env.msg_type {
                                                        MessageType::WalletResponse => {
                                                            if let Ok(data) = serde_json::from_slice::<WalletSyncResponse>(&env.payload) {
                                                                tracing::info!(
                                                                    "📥 收到钱包同步响应 from {}: balance={}, tx_count={}",
                                                                    peer, data.balance, data.tx_count
                                                                );
                                                                // 写入共享状态供 GUI 轮询读取
                                                                if let Ok(mut s) = status_bg.lock() {
                                                                    s.pending_wallet_responses.push(data);
                                                                    if s.pending_wallet_responses.len() > 50 {
                                                                        s.pending_wallet_responses.drain(..25);
                                                                    }
                                                                }
                                                            }
                                                        }
                                                        MessageType::Pong => {
                                                            tracing::debug!("🏓 收到 Pong from {}", peer);
                                                        }
                                                        _ => {
                                                            tracing::debug!("📩 收到 {:?} 响应 from {}: {} bytes", env.msg_type, peer, env.payload.len());
                                                        }
                                                    }
                                                }
                                                Err(_) => {
                                                    tracing::debug!("📩 收到非 Envelope 响应 from {}: {} bytes", peer, response.0.len());
                                                }
                                            }
                                        }
                                    }
                                }
                                libp2p::request_response::Event::OutboundFailure { peer, error, .. } => {
                                    tracing::warn!("⚠️ 发送失败 to {}: {}", peer, error);
                                }
                                libp2p::request_response::Event::InboundFailure { peer, error, .. } => {
                                    tracing::warn!("⚠️ 接收失败 from {}: {}", peer, error);
                                }
                                _ => {}
                            }
                        }

                        SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                            tracing::warn!("⚠️ 连接失败 {:?}: {}", peer_id, error);
                        }

                        _ => {}
                    }
                }
            }
        }

        let mut s = status_bg.lock().unwrap();
        s.running = false;
        s.peer_count = 0;
        tracing::info!("✅ Swarm 已停止");
    });

    SwarmHandle { cmd_tx, status }
}

/// Stub：非 native 特性下返回空句柄
#[cfg(not(feature = "native"))]
pub fn start_swarm(_listen_port: u16) -> SwarmHandle {
    let (cmd_tx, _) = mpsc::unbounded_channel::<SwarmCommand>();
    SwarmHandle {
        cmd_tx,
        status: Arc::new(Mutex::new(SwarmStatus::default())),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// 生成唯一端口（基于系统时间哈希，避免冲突）
// ─────────────────────────────────────────────────────────────────────────────

/// 在 4001-4999 范围内生成相对唯一的监听端口
pub fn pick_listen_port() -> u16 {
    let mut h = DefaultHasher::new();
    std::time::SystemTime::now().hash(&mut h);
    std::process::id().hash(&mut h);
    4001 + (h.finish() % 999) as u16
}

// ─────────────────────────────────────────────────────────────────────────────
// 单元测试
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // ── SwarmStatus 默认值 ────────────────────────────────────────────────

    #[test]
    fn test_swarm_status_default() {
        let s = SwarmStatus::default();
        assert!(s.local_peer_id.is_empty());
        assert!(s.listen_addrs.is_empty());
        assert_eq!(s.peer_count, 0);
        assert!(!s.running);
    }

    #[test]
    fn test_swarm_status_clone() {
        let s = SwarmStatus {
            local_peer_id: "12D3KooWtest".to_string(),
            listen_addrs: vec!["/ip4/127.0.0.1/tcp/4001".to_string()],
            peer_count: 3,
            running: true,
            pending_transfers: Vec::new(),
            wallet_sync_data: None,
            pending_wallet_responses: Vec::new(),
            local_h3_cell: None,
        };
        let cloned = s.clone();
        assert_eq!(cloned.local_peer_id, s.local_peer_id);
        assert_eq!(cloned.peer_count, s.peer_count);
        assert!(cloned.running);
    }

    // ── pick_listen_port ──────────────────────────────────────────────────

    #[test]
    fn test_pick_listen_port_range() {
        for _ in 0..20 {
            let port = pick_listen_port();
            assert!(port >= 4001 && port <= 4999,
                "端口应在 4001-4999 范围内，实际: {}", port);
        }
    }

    #[test]
    fn test_pick_listen_port_variability() {
        // 快速连续调用，端口可能相同（同一毫秒），但不应崩溃
        let p1 = pick_listen_port();
        let p2 = pick_listen_port();
        // 两次都在合法范围内即可
        assert!(p1 >= 4001 && p1 <= 4999);
        assert!(p2 >= 4001 && p2 <= 4999);
    }

    // ── SwarmHandle（需要 tokio runtime）────────────────────────────────

    #[tokio::test]
    async fn test_swarm_handle_stub_cached_status() {
        // start_swarm Stub 版本（非 native feature）返回空句柄
        let handle = start_swarm(4500);
        let status = handle.cached_status();
        // Stub 模式：running=false, peer_count=0
        assert!(!status.running);
        assert_eq!(status.peer_count, 0);
    }

    #[tokio::test]
    async fn test_swarm_handle_shutdown_no_panic() {
        // shutdown 发送到已关闭通道也不应 panic
        let handle = start_swarm(4501);
        handle.shutdown();
    }

    #[tokio::test]
    async fn test_swarm_handle_clone() {
        let handle = start_swarm(4502);
        let h2 = handle.clone();
        // 克隆后两者 cached_status 共享同一 Arc
        let s1 = handle.cached_status();
        let s2 = h2.cached_status();
        assert_eq!(s1.peer_count, s2.peer_count);
    }

    // ── SwarmCommand 构造 ─────────────────────────────────────────────────

    #[test]
    fn test_swarm_command_broadcast() {
        use crate::p2p::protocol::MessageType;
        let cmd = SwarmCommand::Broadcast {
            msg_type: MessageType::WalletRequest,
            payload: b"test payload".to_vec(),
            source_h3_cell: None,
        };
        // 只要能构造就通过（枚举变体存在）
        if let SwarmCommand::Broadcast { payload, .. } = cmd {
            assert_eq!(payload, b"test payload");
        } else {
            panic!("wrong variant");
        }
    }

    #[test]
    fn test_swarm_command_shutdown() {
        let cmd = SwarmCommand::Shutdown;
        assert!(matches!(cmd, SwarmCommand::Shutdown));
    }

    #[test]
    fn test_swarm_command_dial() {
        let cmd = SwarmCommand::Dial("/ip4/10.0.0.1/tcp/4001".to_string());
        if let SwarmCommand::Dial(addr) = cmd {
            assert!(addr.starts_with("/ip4/"));
        } else {
            panic!("wrong variant");
        }
    }
}
