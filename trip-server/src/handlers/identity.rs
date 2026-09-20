//! `GET /v1/identity/:hex`：查询某 attester 公钥的身份信息。
//!
//! 路径参数 `:hex` 是 attester 的 32 字节公钥 hex（64 个字符）。
//! 返回 JSON：`{ "attester": "<hex>", "breadcrumb_count": u64,
//!   "unique_cells": u64, "chain_head": "<hex32>", "last_ts": u64 }`。
//!
//! - 200：身份存在（即使面包屑链为空也返回 count=0）；
//! - 400：hex 长度/格式错误；
//! - 404：该公钥未上传过任何面包屑。
//!
//! 仅做读访问，不持锁跨 await。

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;
use std::collections::HashSet;

use crate::error::{ServerError, ServerResult};
use crate::handlers::parse_hex32;
use crate::state::AppState;

pub async fn get(
    State(state): State<AppState>,
    Path(hex): Path<String>,
) -> ServerResult<impl IntoResponse> {
    let pubkey = parse_hex32(&hex, "attester")?;

    // 读锁内 clone 出链，不在锁内做长计算。
    let crumbs = {
        let evidence = state.inner.evidence.read().await;
        evidence.get(&pubkey).cloned()
    };

    let Some(crumbs) = crumbs else {
        return Err(ServerError::NotFound("no evidence for attester".into()));
    };
    if crumbs.is_empty() {
        return Err(ServerError::NotFound("empty evidence chain".into()));
    }

    // 统计：面包屑数、唯一 H3 cell 数、链头 hash、最后时间戳。
    let breadcrumb_count = crumbs.len() as u64;
    let unique_cells: u64 = {
        let mut set = HashSet::new();
        for c in &crumbs {
            set.insert(c.h3_cell);
        }
        set.len() as u64
    };
    let last = crumbs.last().expect("non-empty crumbs checked above");
    let chain_head = hex::encode(last.block_hash());
    let last_ts = last.timestamp;

    Ok(Json(json!({
        "attester": hex::encode(pubkey),
        "breadcrumb_count": breadcrumb_count,
        "unique_cells": unique_cells,
        "chain_head": chain_head,
        "last_ts": last_ts,
    })))
}
