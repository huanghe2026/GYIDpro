//! 运行配置与 Verifier 长密钥加载。
//!
//! MVP 阶段：
//! - Verifier Ed25519 密钥从环境变量 `TRIP_VERIFIER_SEED`（64 个 hex 字符 =
//!   32 字节种子）读取；未设置时使用固定开发种子并打 WARN（仅限本地开发，
//!   生产必须显式提供）。
//! - 其余参数有内置默认值，可用环境变量覆盖。

use std::env;

use trip_core::crypto::ProtocolKey;
use trip_core::did::AnchorReference;

/// Verifier 服务配置。
#[derive(Debug, Clone)]
pub struct Config {
    /// HTTP 监听地址。
    pub listen: String,
    /// PoH 证书有效期（秒）。
    pub validity_secs: u64,
    /// Active Verification 挑战有效期（秒）。
    pub challenge_ttl_secs: u64,
    /// RP 策略：最小临界置信度。
    pub min_confidence: f64,
    /// RP 策略：最小信任分。
    pub min_trust: f64,
    /// CORS 允许的 Origin 列表。`["*"]` 表示允许任意 Origin（仅开发环境）。
    pub cors_origins: Vec<String>,
    /// 对外可访问的 base URL（写入 DID Document 的 `#verifier` / `#tit` 端点）。
    pub public_url: String,
    /// EVM 链上锚定指针（写入 DID Document 的 `#anchor`）。
    pub anchor: Option<AnchorReference>,
    /// GeoTITRegistry 只读/广播 RPC 端点（`TRIP_EVM_RPC_URL`）。
    /// 缺省时按 chain id 选 Base 官方公共 RPC。
    pub evm_rpc_url: Option<String>,
    /// 每个链上 epoch 覆盖的面包屑数（`TRIP_EPOCH_SIZE`，默认 100）。
    pub epoch_size: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            listen: "127.0.0.1:8080".to_string(),
            validity_secs: 3600,
            challenge_ttl_secs: 60,
            min_confidence: 0.1,
            min_trust: 20.0,
            cors_origins: vec!["*".to_string()],
            public_url: "http://127.0.0.1:8080".to_string(),
            anchor: None,
            evm_rpc_url: None,
            epoch_size: 100,
        }
    }
}

/// 固定开发用种子（仅当 `TRIP_VERIFIER_SEED` 未设置时使用）。
pub const DEV_VERIFIER_SEED: [u8; 32] = [7u8; 32];

impl Config {
    /// 从环境变量加载配置（缺失项取默认值）。
    pub fn from_env() -> Self {
        let mut cfg = Config::default();
        if let Ok(v) = env::var("TRIP_LISTEN") {
            cfg.listen = v;
        }
        if let Ok(v) = env::var("TRIP_VALIDITY_SECS") {
            if let Ok(n) = v.parse() {
                cfg.validity_secs = n;
            }
        }
        if let Ok(v) = env::var("TRIP_CHALLENGE_TTL_SECS") {
            if let Ok(n) = v.parse() {
                cfg.challenge_ttl_secs = n;
            }
        }
        if let Ok(v) = env::var("TRIP_MIN_CONFIDENCE") {
            if let Ok(n) = v.parse() {
                cfg.min_confidence = n;
            }
        }
        if let Ok(v) = env::var("TRIP_MIN_TRUST") {
            if let Ok(n) = v.parse() {
                cfg.min_trust = n;
            }
        }
        if let Ok(v) = env::var("TRIP_CORS_ORIGINS") {
            let origins: Vec<String> = v
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            if !origins.is_empty() {
                cfg.cors_origins = origins;
            }
        }
        if let Ok(v) = env::var("TRIP_PUBLIC_URL") {
            let trimmed = v.trim().trim_end_matches('/');
            if !trimmed.is_empty() {
                cfg.public_url = trimmed.to_string();
            }
        }
        if let Ok(v) = env::var("TRIP_ANCHOR") {
            match AnchorReference::parse(v.trim()) {
                Some(anchor) => cfg.anchor = Some(anchor),
                None => tracing::warn!(
                    value = %v,
                    "TRIP_ANCHOR must be CAIP-2 eip155:<chain_id>:<registry>; ignored"
                ),
            }
        }
        if let Ok(v) = env::var("TRIP_EVM_RPC_URL") {
            let trimmed = v.trim().trim_end_matches('/').to_string();
            if !trimmed.is_empty() {
                cfg.evm_rpc_url = Some(trimmed);
            }
        }
        if let Ok(v) = env::var("TRIP_EPOCH_SIZE") {
            match v.parse::<usize>() {
                Ok(n) if n > 0 => cfg.epoch_size = n,
                _ => tracing::warn!(
                    value = %v,
                    "TRIP_EPOCH_SIZE must be a positive integer; using default 100"
                ),
            }
        }
        cfg
    }
}

/// 加载 Verifier 长密钥。
///
/// 读取 `TRIP_VERIFIER_SEED`（32 字节的 hex 编码）；未设置时回退固定开发
/// 种子并记录 WARN。hex 解析失败直接 panic（配置错误应启动即失败）。
pub fn load_verifier_key() -> ProtocolKey {
    match env::var("TRIP_VERIFIER_SEED") {
        Ok(hex_seed) => {
            let bytes = hex::decode(hex_seed.trim()).expect("TRIP_VERIFIER_SEED must be valid hex");
            let seed: [u8; 32] = bytes
                .as_slice()
                .try_into()
                .expect("TRIP_VERIFIER_SEED must decode to exactly 32 bytes");
            tracing::info!("verifier key loaded from TRIP_VERIFIER_SEED");
            ProtocolKey::from_seed(&seed)
        }
        Err(_) => {
            tracing::warn!(
                "TRIP_VERIFIER_SEED not set; using built-in DEV seed — \
                 do NOT use in production"
            );
            ProtocolKey::from_seed(&DEV_VERIFIER_SEED)
        }
    }
}

/// 当前 Unix 秒。
pub fn now_unix_secs() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before unix epoch")
        .as_secs()
}
