//! `GET /v1/tit/:hex`：Verifier 依据已上传面包屑链核算统计量并签发 TIT。
//!
//! 与 `/v1/poh` 的分工：PoH 必须经 Active Verification 绑定一次性 RP nonce，
//! 携带完整统计指数；TIT 是**长期有效**的轻量令牌（统计字段 + Verifier 签名），
//! 可放进二维码 / DID Document，供展示与发现。
//!
//! 统计量来源与 PoH 完全一致（[`crate::engine_bridge::evaluate_identity`]）：
//! 先跑 `ChainRules` 自检（链必须已通过校验），再算 PSD/Levy/信任分。
//!
//! - 200：JSON（含 `tit_base64url` / `tit_cbor_hex` / `did` / 统计明细）
//! - 400：attester hex 不合法
//! - 404：该公钥无 evidence
//! - 500：链自检失败或引擎评估失败（脏数据不应存在）
//!
//! 引擎是 CPU 密集（DFT / Levy 网格 MLE），一律 `spawn_blocking`。

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use trip_core::ChainRules;

use crate::engine_bridge;
use crate::error::{ServerError, ServerResult};
use crate::handlers::parse_hex32;
use crate::state::AppState;

pub async fn issue(
    State(state): State<AppState>,
    Path(hex): Path<String>,
) -> ServerResult<impl IntoResponse> {
    let attester = parse_hex32(&hex, "attester")?;

    // 读锁内 clone 出链，不在锁内做长计算。
    let crumbs = {
        let evidence = state.inner.evidence.read().await;
        evidence.get(&attester).cloned()
    };
    let Some(crumbs) = crumbs else {
        return Err(ServerError::NotFound("no evidence for attester".into()));
    };
    if crumbs.is_empty() {
        return Err(ServerError::NotFound("empty evidence chain".into()));
    }

    let verifier_key = state.inner.verifier_key.clone();
    let validity_secs = state.inner.config.validity_secs;
    let issued_at = crate::config::now_unix_secs();

    let (tit, eval) = tokio::task::spawn_blocking(move || {
        if ChainRules::default().verify(&crumbs).is_err() {
            return Err("stored evidence chain failed ChainRules verification".to_string());
        }
        let eval = engine_bridge::evaluate_identity(&crumbs)
            .map_err(|e| format!("engine evaluation failed: {e}"))?;
        let tit =
            engine_bridge::issue_tit(&verifier_key, attester, &eval, issued_at, validity_secs);
        Ok::<_, String>((tit, eval))
    })
    .await
    .map_err(|e| ServerError::Internal(format!("engine task failed: {e}")))?
    .map_err(ServerError::Internal)?;

    let did = tit.did();
    Ok(Json(json!({
        "attester": hex::encode(attester),
        "did": did,
        "issuer": tit.issuer.as_str(),
        "epochs": tit.claims.epochs,
        "breadcrumbs": tit.claims.breadcrumbs,
        "unique_cells": tit.claims.unique_cells,
        "trust": tit.claims.trust,
        "alpha": eval.alpha,
        "confidence": eval.confidence,
        "issued_at": tit.claims.issued_at,
        "validity_secs": tit.claims.validity_secs,
        "handle_ok": tit.meets_handle_threshold(),
        "chain_head": hex::encode(eval.chain_head),
        "tit_cbor_hex": hex::encode(tit.to_cbor()),
        "tit_base64url": tit.to_base64url(),
        "did_document_url": format!("/v1/did/{did}"),
    })))
}
