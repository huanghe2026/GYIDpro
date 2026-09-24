//! `WS /v1/challenge?attester=<hex32>`：Attester 实时挑战通道。
//!
//! - 连接建立并注册后，服务端先发一条文本 `{"type":"ready",...}`（消除
//!   "通道何时可用"的竞态，RP 应在收到 ready 后再 POST /v1/verify）；
//! - RP 发起验证时，服务端推送二进制帧 = LivenessChallenge CBOR；
//! - Attester 回二进制帧 = LivenessResponse CBOR；验签 + 与挑战严格匹配后，
//!   阻塞跑经典引擎并签发 PoH，回文本 JSON 回执；
//! - 一个连接可串行处理多个挑战；连接断开即注销推送通道。

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::{Query, State};
use axum::response::{IntoResponse, Response};
use futures_util::{SinkExt, StreamExt};
use serde::Deserialize;
use serde_json::json;
use tokio::sync::mpsc;
use tokio::task::spawn_blocking;

use trip_core::{LivenessChallenge, LivenessResponse};

use crate::config::now_unix_secs;
use crate::engine_bridge::{evaluate_identity, issue_poh, EvalResult};
use crate::error::{ServerError, ServerResult};
use crate::handlers::parse_hex32;
use crate::state::{AppState, ChallengeStatus, OutMsg};

#[derive(Debug, Deserialize)]
pub struct WsQuery {
    pub attester: String,
}

pub async fn ws(
    upgrade: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<AppState>,
) -> Response {
    let attester = match parse_hex32(&query.attester, "attester") {
        Ok(k) => k,
        Err(e) => return e.into_response(),
    };
    upgrade.on_upgrade(move |socket| handle_socket(socket, attester, state))
}

async fn handle_socket(socket: WebSocket, attester: [u8; 32], state: AppState) {
    let (mut ws_tx, mut ws_rx) = socket.split();

    // 单一下行通道：挑战二进制帧 / 文本回执统一排队发送。
    let (out_tx, mut out_rx) = mpsc::unbounded_channel::<OutMsg>();
    // 保留一份句柄用于断开时的 same_channel 判定（避免误删重连后的新通道）。
    let own_tx = out_tx.clone();
    state
        .inner
        .ws_senders
        .write()
        .await
        .insert(attester, out_tx);

    let send_task = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            let frame = match msg {
                OutMsg::Challenge(bytes) => Message::Binary(bytes),
                OutMsg::Text(text) => Message::Text(text),
            };
            if ws_tx.send(frame).await.is_err() {
                break;
            }
        }
    });

    // ready 回执：注册完成，RP 可以发起 verify。
    let _ = state
        .push_to_attester(
            &attester,
            OutMsg::Text(json!({"type":"ready","attester": hex::encode(attester)}).to_string()),
        )
        .await;

    // 接收 Attester 响应。
    while let Some(Ok(msg)) = ws_rx.next().await {
        match msg {
            Message::Binary(data) => {
                let ack = process_response(&state, &attester, &data).await;
                let _ = state.push_to_attester(&attester, OutMsg::Text(ack)).await;
            }
            Message::Close(_) => break,
            // Text/Pong：Ping 由 axum 自动应答；其余忽略。
            _ => {}
        }
    }

    // 注销：仅当当前注册通道仍是本连接时才移除（支持断线重连）。
    {
        let mut senders = state.inner.ws_senders.write().await;
        if senders
            .get(&attester)
            .is_some_and(|t| t.same_channel(&own_tx))
        {
            senders.remove(&attester);
        }
    }
    send_task.abort();
}

/// 处理一帧 LivenessResponse，返回 JSON 回执文本。
async fn process_response(state: &AppState, attester: &[u8; 32], data: &[u8]) -> String {
    match verify_and_issue(state, attester, data).await {
        Ok((challenge_id, eval, policy_ok)) => json!({
            "type": "poh_issued",
            "ok": true,
            "challenge_id": hex::encode(challenge_id),
            "alpha": eval.alpha,
            "beta": eval.beta,
            "trust": eval.trust,
            "confidence": eval.confidence,
            "meets_policy": policy_ok,
        })
        .to_string(),
        Err(e) => json!({"ok": false, "error": e.to_string()}).to_string(),
    }
}

async fn verify_and_issue(
    state: &AppState,
    attester: &[u8; 32],
    data: &[u8],
) -> ServerResult<([u8; 16], EvalResult, bool)> {
    // 1) 解析 + Attester 签名验证（阻塞）。
    let att = *attester;
    let data_owned = data.to_vec();
    let response = spawn_blocking(move || -> ServerResult<LivenessResponse> {
        let r = LivenessResponse::from_cbor(&data_owned)?;
        r.verify_signature(&att)?;
        Ok(r)
    })
    .await
    .map_err(|e| ServerError::Internal(e.to_string()))??;

    // 2) 查挑战记录并做业务匹配。
    let record = {
        let challenges = state.inner.challenges.read().await;
        challenges
            .get(&response.challenge_id)
            .cloned()
            .ok_or_else(|| ServerError::NotFound("unknown challenge_id".into()))?
    };
    if record.attester != *attester {
        return Err(ServerError::bad_request(
            "challenge belongs to another attester",
        ));
    }
    if record.status != ChallengeStatus::Pending {
        return Err(ServerError::Expired("challenge already consumed".into()));
    }
    let now = now_unix_secs();
    if now > record.deadline {
        state
            .inner
            .challenges
            .write()
            .await
            .entry(response.challenge_id)
            .and_modify(|r| r.status = ChallengeStatus::Expired);
        return Err(ServerError::Expired("challenge deadline passed".into()));
    }
    let challenge = LivenessChallenge::new(
        response.challenge_id,
        record.rp_nonce,
        record.chain_head,
        record.expected_index,
        record.deadline,
    );
    if !response.matches_challenge(&challenge) {
        return Err(ServerError::bad_request(
            "response does not match challenge (id/nonce/head/index)",
        ));
    }

    // 3) 取证据链；当前链头必须仍是挑战时的链头（挑战后不得偷偷追加）。
    let crumbs = {
        let evidence = state.inner.evidence.read().await;
        evidence
            .get(attester)
            .cloned()
            .ok_or_else(|| ServerError::NotFound("no evidence for attester".into()))?
    };
    let current_head = crumbs
        .last()
        .map(|c| c.block_hash())
        .ok_or_else(|| ServerError::NotFound("empty evidence chain".into()))?;
    if current_head != record.chain_head {
        return Err(ServerError::Expired(
            "chain advanced after challenge was issued".into(),
        ));
    }

    // 4) 引擎评估 + PoH 签发（阻塞）。
    let verifier = state.inner.verifier_key.clone();
    let validity_secs = state.inner.config.validity_secs;
    let min_confidence = state.inner.config.min_confidence;
    let min_trust = state.inner.config.min_trust;
    let rp_nonce = record.rp_nonce;
    let issued = now;
    let (eval, cert_cbor, policy_ok) =
        spawn_blocking(move || -> ServerResult<(EvalResult, Vec<u8>, bool)> {
            let eval = evaluate_identity(&crumbs)?;
            let cert = issue_poh(&verifier, att, &eval, rp_nonce, issued, validity_secs);
            let policy_ok = cert.meets_policy(min_confidence, min_trust);
            Ok((eval, cert.to_cbor(), policy_ok))
        })
        .await
        .map_err(|e| ServerError::Internal(e.to_string()))??;

    // 5) 落库：PoH 待 RP 取，挑战置 Issued；同时记入 attester 的 PoH 列表。
    {
        let mut poh = state.inner.poh.write().await;
        poh.insert(response.challenge_id, cert_cbor);
        // 不在锁内跨 await：先解锁 poh，再写 challenges 和 attester_pohs。
    }
    {
        let mut challenges = state.inner.challenges.write().await;
        challenges
            .entry(response.challenge_id)
            .and_modify(|r| r.status = ChallengeStatus::Issued);
    }
    {
        let mut attester_pohs = state.inner.attester_pohs.write().await;
        attester_pohs
            .entry(*attester)
            .or_default()
            .push(response.challenge_id);
    }

    // 链上中继（启用时）：首次主动验证成功后登记身份（幂等，actor 先查链）。
    state.on_identity_verified(attester);

    Ok((response.challenge_id, eval, policy_ok))
}
