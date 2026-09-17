//! `POST /v1/verify`：RP 发起 Active Verification。
//!
//! 请求 JSON：`{ "attester": "<hex32 公钥>", "rp_nonce": "<hex16>" }`。
//! Verifier 取该身份当前链头与下一个 index，生成 LivenessChallenge 落库，
//! 并经 Attester 已建立的 WebSocket 推送。返回 challenge_id 与过期时间；
//! `delivered=false` 表示 Attester 当前不在线（挑战仍保留至 deadline）。

use axum::extract::State;
use axum::Json;
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::config::now_unix_secs;
use crate::error::{ServerError, ServerResult};
use crate::handlers::{parse_hex16, parse_hex32};
use crate::state::{AppState, ChallengeRecord, ChallengeStatus, OutMsg};
use trip_core::LivenessChallenge;

#[derive(Debug, Deserialize)]
pub struct VerifyRequest {
    pub attester: String,
    pub rp_nonce: String,
}

pub async fn request(
    State(state): State<AppState>,
    Json(req): Json<VerifyRequest>,
) -> ServerResult<Json<Value>> {
    let attester = parse_hex32(&req.attester, "attester")?;
    let rp_nonce = parse_hex16(&req.rp_nonce, "rp_nonce")?;

    // 当前链头与期望的下一面包屑 index。
    let (chain_head, expected_index) = {
        let evidence = state.inner.evidence.read().await;
        let crumbs = evidence
            .get(&attester)
            .ok_or_else(|| ServerError::NotFound("no evidence for attester".into()))?;
        let last = crumbs.last().expect("evidence chain is never empty");
        (last.block_hash(), last.index + 1)
    };

    let challenge_id = *Uuid::new_v4().as_bytes();
    let now = now_unix_secs();
    let deadline = now + state.inner.config.challenge_ttl_secs;

    let challenge =
        LivenessChallenge::new(challenge_id, rp_nonce, chain_head, expected_index, deadline);

    state.inner.challenges.write().await.insert(
        challenge_id,
        ChallengeRecord {
            attester,
            rp_nonce,
            chain_head,
            expected_index,
            deadline,
            status: ChallengeStatus::Pending,
        },
    );

    let delivered = state
        .push_to_attester(&attester, OutMsg::Challenge(challenge.to_cbor()))
        .await;
    if !delivered {
        tracing::info!(
            attester = %hex::encode(attester),
            "challenge created but attester has no active WS connection"
        );
    }

    Ok(Json(json!({
        "challenge_id": hex::encode(challenge_id),
        "expires_at": deadline,
        "delivered": delivered,
    })))
}
