//! # trip-server
//!
//! IETF TRIP（`draft-ayerbe-trip-protocol-04`）§12 Active Verification 的
//! Verifier 服务（GYIP-0003 §5.2）。
//!
//! 实现 RP ↔ Verifier ↔ Attester 三方流程：
//!
//! 1. Attester `POST /v1/evidence` 批量上传签名面包屑；
//! 2. RP `POST /v1/verify` 发起验证，得 challenge_id；
//! 3. Verifier 经 `WS /v1/challenge` 下发 LivenessChallenge；
//! 4. Attester 回签名 LivenessResponse，Verifier 跑经典引擎并签发 PoH；
//! 5. RP `POST /v1/poh` 取回绑定其 nonce 的 PoH 证书 CBOR。
//!
//! MVP 为单进程内存存储（无 Postgres/Redis），引擎直接复用
//! [`trip_core`]；CPU 密集计算一律 `spawn_blocking`。

pub mod chain;
pub mod config;
pub mod engine_bridge;
pub mod error;
pub mod handlers;
pub mod state;

pub use config::Config;
pub use handlers::build_router;
pub use state::AppState;
