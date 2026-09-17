//! `GET /.well-known/verifier.json`：Verifier 元数据。
//!
//! 公布 Verifier 公钥（RP 用它验证 PoH 签名）、证书有效期、RP 策略阈值、
//! 证据保留方式与支持的扩展。MVP 阶段 storage 明确标注内存非持久化。

use axum::extract::State;
use axum::Json;
use serde_json::{json, Value};

use crate::state::AppState;

pub async fn meta(State(state): State<AppState>) -> Json<Value> {
    let cfg = &state.inner.config;
    Json(json!({
        "verifier": "trip-server",
        "protocol": "draft-ayerbe-trip-protocol-04",
        "verifier_pubkey": hex::encode(state.inner.verifier_key.public_bytes()),
        "validity_secs": cfg.validity_secs,
        "challenge_ttl_secs": cfg.challenge_ttl_secs,
        "policy": {
            "alpha_bio_range": [0.30, 0.80],
            "min_confidence": cfg.min_confidence,
            "min_trust": cfg.min_trust
        },
        "evidence": {
            "storage": "in-memory",
            "persistent": false,
            "note": "MVP: all evidence and challenges are lost on process restart"
        },
        "extensions": []
    }))
}
