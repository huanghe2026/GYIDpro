//! `GET /v1/did/:did`：`did:geoyuan` 解析 → W3C DID Document。
//!
//! DID 完全由 Ed25519 公钥派生（`did:geoyuan:z<base58btc>`），因此"解析"
//! 不依赖任何链上状态或数据库：只要 DID 语法合法即可实时生成文档。
//! `#verifier` / `#tit` 端点来自 `TRIP_PUBLIC_URL`，`#anchor` 来自
//! `TRIP_ANCHOR`（CAIP-2 `eip155:<chain_id>:<registry>`）。
//!
//! - 200：`application/did+json`
//! - 400：DID 不合法（method 前缀 / multibase / 公钥长度）
//!
//! 仅读配置，不持锁、不做 CPU 计算。

use axum::extract::{Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use trip_core::did::{self, DidDocument, DidDocumentConfig};

use crate::error::{ServerError, ServerResult};
use crate::state::AppState;

pub async fn resolve(
    State(state): State<AppState>,
    Path(did_str): Path<String>,
) -> ServerResult<Response> {
    let public_key =
        did::decode(&did_str).map_err(|e| ServerError::bad_request(format!("invalid did: {e}")))?;

    let base = state
        .inner
        .config
        .public_url
        .trim_end_matches('/')
        .to_string();
    let cfg = DidDocumentConfig {
        tit_endpoint: Some(format!("{base}/v1/tit/{}", hex::encode(public_key))),
        verifier_endpoint: Some(base),
        anchor: state.inner.config.anchor.clone(),
        updated: Some(crate::config::now_unix_secs()),
        ..DidDocumentConfig::default()
    };

    let doc = DidDocument::build(&public_key, &cfg);
    let json = doc
        .to_json()
        .map_err(|e| ServerError::Internal(e.to_string()))?;

    Ok((
        [(header::CONTENT_TYPE, "application/did+json; charset=utf-8")],
        json,
    )
        .into_response())
}
