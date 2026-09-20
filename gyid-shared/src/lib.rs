//! # gyid-shared
//!
//! GyID 三端（Web WASM / CLI / Android）共享的业务逻辑层。基于
//! [`trip-core`] 协议库，向上提供：
//!
//! - [`identity`]：身份密钥生成与加密存储（PBKDF2 + AES-GCM，feature `encrypt`）
//! - [`breadcrumb`]：从 (lat,lng) 经 H3 量化并签名一条面包屑
//! - [`chain`]：本地链管理（append、verify、CBOR 帧流拼接）
//! - [`liveness`]：§12 Active Verification Attester 端响应签名
//! - [`poh`]：Proof-of-Humanity 证书解析与策略检查
//! - [`verifier_client`]：异步 Verifier HTTP 客户端（feature `http-client`）
//!
//! 不在本 crate：WebSocket 客户端（CLI 用 tokio-tungstenite，Android 用
//! OkHttp，浏览器用原生 WebSocket，各端原生实现）、UI、传感器采集。

#![forbid(unsafe_code)]

pub mod breadcrumb;
pub mod chain;
pub mod identity;
pub mod liveness;
pub mod poh;

#[cfg(feature = "http-client")]
pub mod verifier_client;

pub use breadcrumb::collect_breadcrumb;
pub use chain::Chain;
pub use identity::Identity;
pub use liveness::sign_liveness_response;
pub use poh::{verify_poh, PohInfo};

#[cfg(feature = "http-client")]
pub use verifier_client::VerifierClient;

/// 统一错误类型。
#[derive(Debug, thiserror::Error)]
pub enum GyidError {
    #[error("trip-core: {0}")]
    Core(#[from] trip_core::TripError),
    #[error("hex decode: {0}")]
    Hex(String),
    #[error("H3 quantize: {0}")]
    H3(String),
    #[error("encrypt/decrypt: {0}")]
    Crypto(String),
    #[cfg(feature = "http-client")]
    #[error("http: {0}")]
    Http(String),
    #[cfg(feature = "http-client")]
    #[error("verifier rejected: status={0} body={1}")]
    Verifier(u16, String),
    #[error("invalid input: {0}")]
    BadInput(String),
}

pub type GyidResult<T> = Result<T, GyidError>;
