//! 进程内共享状态（MVP 内存存储）。
//!
//! 四张表：
//! - `evidence`：attester 公钥 → 已验证的完整面包屑链；
//! - `challenges`：challenge id → Active Verification 挑战记录；
//! - `poh`：challenge id → 已签发的 PoH 证书 CBOR；
//! - `ws_senders`：attester 公钥 → 其当前 WebSocket 连接的下发通道。
//!
//! 所有表均为 `tokio::RwLock<HashMap>`，**锁内不做 CPU 密集计算、不跨
//! await 持锁**：引擎评估前把面包屑链 clone 出来丢进 `spawn_blocking`。
//! 本存储无持久化，进程重启即清空（见 README）。

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::{mpsc, RwLock};
use trip_core::{Breadcrumb, ProtocolKey};

use crate::config::Config;

/// 挑战生命周期状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChallengeStatus {
    /// 已下发，等待 Attester 应答。
    Pending,
    /// 应答有效，PoH 已签发（RP 可取）。
    Issued,
    /// 超过 deadline 仍未应答。
    Expired,
}

/// 一次 Active Verification 挑战的服务端记录。
#[derive(Debug, Clone)]
pub struct ChallengeRecord {
    pub attester: [u8; 32],
    pub rp_nonce: [u8; 16],
    pub chain_head: [u8; 32],
    pub expected_index: u64,
    pub deadline: u64,
    pub status: ChallengeStatus,
}

/// WebSocket 下行帧（挑战二进制帧 / 文本回执）。
#[derive(Debug, Clone)]
pub enum OutMsg {
    /// LivenessChallenge 的 CBOR 字节。
    Challenge(Vec<u8>),
    /// 处理结果 JSON 文本回执。
    Text(String),
}

/// 不可变共享内部状态（外套 Arc）。
pub struct Shared {
    pub verifier_key: Arc<ProtocolKey>,
    pub config: Config,
    pub evidence: RwLock<HashMap<[u8; 32], Vec<Breadcrumb>>>,
    pub challenges: RwLock<HashMap<[u8; 16], ChallengeRecord>>,
    pub poh: RwLock<HashMap<[u8; 16], Vec<u8>>>,
    pub ws_senders: RwLock<HashMap<[u8; 32], mpsc::UnboundedSender<OutMsg>>>,
}

/// 应用状态句柄（廉价 clone）。
#[derive(Clone)]
pub struct AppState {
    pub inner: Arc<Shared>,
}

impl AppState {
    /// 用显式 Verifier 密钥与配置构造（测试 / main 共用）。
    pub fn new(verifier_key: ProtocolKey, config: Config) -> Self {
        Self {
            inner: Arc::new(Shared {
                verifier_key: Arc::new(verifier_key),
                config,
                evidence: RwLock::new(HashMap::new()),
                challenges: RwLock::new(HashMap::new()),
                poh: RwLock::new(HashMap::new()),
                ws_senders: RwLock::new(HashMap::new()),
            }),
        }
    }

    /// 向某 attester 的当前 WS 连接推送一条下行消息；不在线则返回 false。
    pub async fn push_to_attester(&self, attester: &[u8; 32], msg: OutMsg) -> bool {
        let senders = self.inner.ws_senders.read().await;
        if let Some(tx) = senders.get(attester) {
            tx.send(msg).is_ok()
        } else {
            false
        }
    }
}
