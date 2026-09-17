//! `POST /v1/evidence`：Attester 批量上传面包屑。
//!
//! 请求体为**多条完整面包屑 CBOR map 的顺序拼接**（CBOR 自分隔，用
//! [`trip_core::cbor::split_value`] 切帧），便于增量分页上传。
//! 服务端把本批与已存链按 index 合并、去重，对**完整链**跑
//! [`trip_core::ChainRules`]（index 连续 / 时间间隔 / 哈希链 / Ed25519 验签 /
//! cell 去重与上限），全部通过才落库。验签为 CPU 密集，放 `spawn_blocking`。

use std::collections::HashSet;

use axum::body::Bytes;
use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};
use tokio::task::spawn_blocking;
use trip_core::cbor::split_value;
use trip_core::{Breadcrumb, ChainRules};

use crate::error::{ServerError, ServerResult};
use crate::state::AppState;

/// 单批最大面包屑数（对齐草案 256 高风险窗口并留余量；超出请分页）。
pub const MAX_EVIDENCE_BATCH: usize = 300;

pub async fn upload(State(state): State<AppState>, body: Bytes) -> ServerResult<Json<Value>> {
    // ---- 切帧（纯字节算术，轻量）----
    let mut frames: Vec<Vec<u8>> = Vec::new();
    let mut rest: &[u8] = &body;
    while !rest.is_empty() {
        let (one, tail) = split_value(rest).map_err(ServerError::from)?;
        frames.push(one.to_vec());
        rest = tail;
        if frames.len() > MAX_EVIDENCE_BATCH {
            return Err(ServerError::PayloadTooLarge(format!(
                "batch exceeds {MAX_EVIDENCE_BATCH} breadcrumbs; upload in pages"
            )));
        }
    }
    if frames.is_empty() {
        return Err(ServerError::bad_request("empty evidence batch"));
    }

    // ---- CBOR 解析（阻塞：规范化自检）----
    let parsed = spawn_blocking(move || -> Result<Vec<Breadcrumb>, trip_core::TripError> {
        frames
            .iter()
            .map(|f| Breadcrumb::from_cbor(f))
            .collect::<Result<Vec<_>, _>>()
    })
    .await
    .map_err(|e| ServerError::Internal(e.to_string()))?
    .map_err(ServerError::from)?;

    let identity = parsed[0].identity;

    // ---- 与已存链合并 + 全链验证（阻塞：逐条 Ed25519 + 链规则）----
    let existing = state.inner.evidence.read().await.get(&identity).cloned();

    let merged = spawn_blocking(move || -> Result<Vec<Breadcrumb>, trip_core::TripError> {
        let mut chain = existing.unwrap_or_default();
        for bc in parsed {
            match chain.binary_search_by_key(&bc.index, |c| c.index) {
                Ok(pos) => {
                    // 同 index 重传：内容必须逐字节一致，否则视为冲突。
                    if chain[pos].block_hash() != bc.block_hash() {
                        return Err(trip_core::TripError::InvalidChain(format!(
                            "conflicting breadcrumb at index {}",
                            bc.index
                        )));
                    }
                }
                Err(pos) => chain.insert(pos, bc),
            }
        }
        ChainRules::default().verify(&chain)?;
        Ok(chain)
    })
    .await
    .map_err(|e| ServerError::Internal(e.to_string()))?
    .map_err(ServerError::from)?;

    let stored = merged.len();
    let unique_cells = merged
        .iter()
        .map(|c| c.h3_cell)
        .collect::<HashSet<_>>()
        .len();
    let chain_head = merged.last().expect("non-empty after verify").block_hash();

    state.inner.evidence.write().await.insert(identity, merged);

    Ok(Json(json!({
        "identity": hex::encode(identity),
        "stored": stored,
        "unique_cells": unique_cells,
        "chain_head": hex::encode(chain_head),
    })))
}
