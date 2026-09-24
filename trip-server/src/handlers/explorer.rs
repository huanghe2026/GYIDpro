//! `GET /v1/explorer`：公开身份 / PoH 浏览器聚合端点（W6 Phase 10）。
//!
//! 遍历 evidence 表列出全部已上传过面包屑的 attester 及其链统计，
//! 供 gyid-web Explorer 页展示。返回 JSON：
//!
//! ```json
//! {
//!   "total_identities": 2,
//!   "total_breadcrumbs": 1026,
//!   "identities": [
//!     { "attester": "<hex32>", "breadcrumb_count": 513, "unique_cells": 513,
//!       "chain_head": "<hex32>", "last_ts": 1700000000, "poh_count": 3 }
//!   ]
//! }
//! ```
//!
//! - 200：空表也返回 200 + 空数组（浏览页空态是正常业务，不报 404）；
//! - 按 `last_ts` 降序（最新活跃在前）。
//!
//! 锁策略：整体 clone 出 evidence + attester_pohs 两张表后立即释放锁，
//! 统计在锁外计算（遵循 state.rs "锁内不做 CPU 密集计算" 约定）。

use axum::extract::State;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use std::collections::HashSet;

use crate::error::ServerResult;
use crate::state::AppState;

pub async fn overview(State(state): State<AppState>) -> ServerResult<impl IntoResponse> {
    // 一次读锁 clone 全部证据链，立刻释放；统计在锁外做。
    let evidence = state.inner.evidence.read().await.clone();
    let poh_counts: std::collections::HashMap<[u8; 32], usize> = {
        let attester_pohs = state.inner.attester_pohs.read().await;
        attester_pohs.iter().map(|(k, v)| (*k, v.len())).collect()
    };

    let mut identities: Vec<serde_json::Value> = Vec::with_capacity(evidence.len());
    let mut total_breadcrumbs: u64 = 0;
    for (pubkey, crumbs) in &evidence {
        let mut unique_cells = HashSet::new();
        for c in crumbs {
            unique_cells.insert(c.h3_cell);
        }
        // 空链不展示（与 /v1/identity 对空链 404 的语义一致）
        let Some(last) = crumbs.last() else {
            continue;
        };
        let count = crumbs.len() as u64;
        total_breadcrumbs += count;
        identities.push(json!({
            "attester": hex::encode(pubkey),
            "breadcrumb_count": count,
            "unique_cells": unique_cells.len(),
            "chain_head": hex::encode(last.block_hash()),
            "last_ts": last.timestamp,
            "poh_count": poh_counts.get(pubkey).copied().unwrap_or(0),
        }));
    }
    // 最新活跃在前
    identities.sort_by(|a, b| {
        let ta = a["last_ts"].as_u64().unwrap_or(0);
        let tb = b["last_ts"].as_u64().unwrap_or(0);
        tb.cmp(&ta)
    });

    let total_identities = identities.len();
    Ok(Json(json!({
        "total_identities": total_identities,
        "total_breadcrumbs": total_breadcrumbs,
        "identities": identities,
    })))
}
