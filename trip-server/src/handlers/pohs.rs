//! `GET /v1/pohs?attester=<hex32>`：列出某 attester 的 PoH 证书 challenge_id。
//!
//! Query 参数 `attester` 是 32 字节公钥 hex。返回 JSON：
//! `{ "attester": "<hex>", "challenge_ids": ["<hex16>", ...] }`
//!
//! - 200：返回列表（可能为空数组）；
//! - 400：hex 长度/格式错误；
//! - 404：该公钥从未签发过 PoH。
//!
//! 仅读访问，单次锁。

use axum::extract::{Query, State};
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;
use serde_json::json;

use crate::error::{ServerError, ServerResult};
use crate::handlers::parse_hex32;
use crate::state::AppState;

#[derive(Debug, Deserialize)]
pub struct PohsQuery {
    pub attester: String,
}

pub async fn list(
    State(state): State<AppState>,
    Query(q): Query<PohsQuery>,
) -> ServerResult<impl IntoResponse> {
    let pubkey = parse_hex32(&q.attester, "attester")?;

    let ids = {
        let attester_pohs = state.inner.attester_pohs.read().await;
        attester_pohs.get(&pubkey).cloned()
    };

    let Some(ids) = ids else {
        return Err(ServerError::NotFound("no PoHs issued for attester".into()));
    };

    let challenge_ids: Vec<String> = ids.iter().map(hex::encode).collect();
    Ok(Json(json!({
        "attester": hex::encode(pubkey),
        "challenge_ids": challenge_ids,
        "count": challenge_ids.len(),
    })))
}
