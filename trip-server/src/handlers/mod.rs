//! HTTP/WS 路由与处理器。
//!
//! | 方法/协议 | 路径 | 角色 | 说明 |
//! |---|---|---|---|
//! | POST | `/v1/evidence` | Attester | 批量上传面包屑（CBOR 帧流，单批 ≤ 300） |
//! | POST | `/v1/verify` | RP | 发起 Active Verification，返回 challenge_id |
//! | WS   | `/v1/challenge?attester=<hex32>` | Attester | 接 LivenessChallenge，回 LivenessResponse |
//! | POST | `/v1/poh` | RP | 凭 challenge_id 取 PoH 证书 CBOR |
//! | GET  | `/.well-known/verifier.json` | 任意 | Verifier 公钥与策略 |

pub mod challenge;
pub mod evidence;
pub mod poh;
pub mod verify;
pub mod well_known;

use axum::routing::{get, post};
use axum::Router;

use crate::error::{ServerError, ServerResult};
use crate::state::AppState;

/// 解码 32 字节 hex（Ed25519 公钥 / 链头）。
pub fn parse_hex32(s: &str, what: &str) -> ServerResult<[u8; 32]> {
    let bytes =
        hex::decode(s.trim()).map_err(|e| ServerError::bad_request(format!("{what}: {e}")))?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| ServerError::bad_request(format!("{what} must be 32 bytes")))
}

/// 解码 16 字节 hex（nonce / challenge id）。
pub fn parse_hex16(s: &str, what: &str) -> ServerResult<[u8; 16]> {
    let bytes =
        hex::decode(s.trim()).map_err(|e| ServerError::bad_request(format!("{what}: {e}")))?;
    bytes
        .as_slice()
        .try_into()
        .map_err(|_| ServerError::bad_request(format!("{what} must be 16 bytes")))
}

/// 构建完整路由器。
pub fn build_router(state: AppState) -> Router {
    Router::new()
        .route("/v1/evidence", post(evidence::upload))
        .route("/v1/verify", post(verify::request))
        .route("/v1/challenge", get(challenge::ws))
        .route("/v1/poh", post(poh::fetch))
        .route("/.well-known/verifier.json", get(well_known::meta))
        .with_state(state)
}
