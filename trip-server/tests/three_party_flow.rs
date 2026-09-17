//! W5 验收门：Active Verification 三方（RP ↔ Verifier ↔ Attester）联机集成测试。
//!
//! 用真实 TCP + axum + WebSocket，跑通：
//! evidence 上传 → verify 建挑战 → WS 下发 → Attester 签名应答 → 引擎评估
//! → PoH 签发 → RP 取回并验签/验策略。

use std::time::{SystemTime, UNIX_EPOCH};

use axum::serve;
use futures_util::{SinkExt, StreamExt};
use rand::rngs::StdRng;
use rand::SeedableRng;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message as WsMessage;

use trip_core::engine::sim::{trip_walk_path, SimConfig, TripConfig};
use trip_core::{
    Breadcrumb, ChainRules, LivenessChallenge, LivenessResponse, MetaFlags, PohCertificate,
    ProtocolKey,
};
use trip_server::{build_router, AppState, Config};

const VERIFIER_SEED: [u8; 32] = [43u8; 32];
const ATTESTER_SEED: [u8; 32] = [42u8; 32];

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// 绑定随机端口启动服务器，返回 (http base, ws base)。
async fn spawn_server() -> (String, String) {
    let state = AppState::new(ProtocolKey::from_seed(&VERIFIER_SEED), Config::default());
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let app = build_router(state);
    tokio::spawn(async move {
        serve(listener, app).await.unwrap();
    });
    (
        format!("http://127.0.0.1:{port}"),
        format!("ws://127.0.0.1:{port}"),
    )
}

/// 用 trip_walk 生成 513 个合法 res10 cell 并签成面包屑链。
fn sim_signed_chain() -> (ProtocolKey, Vec<u8>) {
    let mut rng = StdRng::seed_from_u64(0x4707);
    let cfg = SimConfig {
        raw_step_cap: 20_000,
        ..SimConfig::default()
    };
    let path = trip_walk_path(&mut rng, 1.75, 5.0, 512, &cfg, &TripConfig::default());
    assert_eq!(path.cells.len(), 513);

    let key = ProtocolKey::from_seed(&ATTESTER_SEED);
    let id_pub = key.public_bytes();
    let mut body = Vec::new();
    let mut prev_hash: Option<[u8; 32]> = None;
    for (i, &cell) in path.cells.iter().enumerate() {
        let mut bc = Breadcrumb::new_unsigned(
            i as u64,
            id_pub,
            1_700_000_000 + (i as u64) * 900,
            cell,
            10,
            [0u8; 32],
            prev_hash,
            MetaFlags::new(),
        );
        bc.sign(&key).unwrap();
        let hash = bc.block_hash();
        body.extend_from_slice(&bc.to_cbor());
        prev_hash = Some(hash);
    }
    (key, body)
}

/// 造 n 条任意（非法 H3 但不参与地理运算）cell 的合法签名小链，返回 CBOR 帧流。
fn small_signed_chain(key: &ProtocolKey, n: u64, tamper_at: Option<u64>) -> Vec<u8> {
    let id_pub = key.public_bytes();
    let mut body = Vec::new();
    let mut prev_hash: Option<[u8; 32]> = None;
    for i in 0..n {
        let mut bc = Breadcrumb::new_unsigned(
            i,
            id_pub,
            1_700_000_000 + i * 900,
            1000 + i, // 相邻不同
            10,
            [0u8; 32],
            prev_hash,
            MetaFlags::new(),
        );
        bc.sign(key).unwrap();
        if Some(i) == tamper_at {
            bc.signature[0] ^= 0x01;
        }
        let hash = bc.block_hash();
        body.extend_from_slice(&bc.to_cbor());
        prev_hash = Some(hash);
    }
    body
}

#[tokio::test]
async fn active_verification_three_party_flow() {
    let (http, ws_base) = spawn_server().await;
    let verifier_key = ProtocolKey::from_seed(&VERIFIER_SEED);
    let verifier_pub = verifier_key.public_bytes();
    let (attester_key, evidence_body) = sim_signed_chain();
    let attester_hex = hex::encode(attester_key.public_bytes());
    let rp_nonce = [0xA1u8; 16];
    let client = reqwest::Client::new();

    // ---- Attester 先建立 WS 挑战通道，等待 ready 回执 ----
    let (mut ws, _) =
        tokio_tungstenite::connect_async(format!("{ws_base}/v1/challenge?attester={attester_hex}"))
            .await
            .expect("ws connect");
    let ready = ws.next().await.unwrap().unwrap();
    assert!(ready.to_text().unwrap().contains("\"ready\""));

    // ---- 1. POST /v1/evidence（513 条，分页 300 + 213，验证增量合并）----
    let mut frames: Vec<Vec<u8>> = Vec::new();
    {
        let mut rest: &[u8] = &evidence_body;
        while !rest.is_empty() {
            let (one, tail) = trip_core::cbor::split_value(rest).unwrap();
            frames.push(one.to_vec());
            rest = tail;
        }
    }
    assert_eq!(frames.len(), 513);
    let batch1: Vec<u8> = frames[..300]
        .iter()
        .flat_map(|f| f.iter().copied())
        .collect();
    let batch2: Vec<u8> = frames[300..]
        .iter()
        .flat_map(|f| f.iter().copied())
        .collect();

    let resp = client
        .post(format!("{http}/v1/evidence"))
        .header("content-type", "application/cbor")
        .body(batch1)
        .send()
        .await
        .unwrap();
    assert!(
        resp.status().is_success(),
        "evidence batch1 {}",
        resp.status()
    );
    let ev1: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(ev1["stored"], 300);

    let resp = client
        .post(format!("{http}/v1/evidence"))
        .header("content-type", "application/cbor")
        .body(batch2)
        .send()
        .await
        .unwrap();
    assert!(
        resp.status().is_success(),
        "evidence batch2 {}",
        resp.status()
    );
    let ev: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(ev["stored"], 513);
    let chain_head_hex = ev["chain_head"].as_str().unwrap().to_string();

    // ---- 2. POST /v1/verify ----
    let resp = client
        .post(format!("{http}/v1/verify"))
        .json(&serde_json::json!({
            "attester": attester_hex,
            "rp_nonce": hex::encode(rp_nonce),
        }))
        .send()
        .await
        .unwrap();
    assert!(resp.status().is_success());
    let verify: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(verify["delivered"], true);
    let challenge_id_hex = verify["challenge_id"].as_str().unwrap().to_string();

    // ---- 3. WS 收到 LivenessChallenge ----
    let frame = ws.next().await.unwrap().unwrap();
    let challenge = LivenessChallenge::from_cbor(&frame.into_data()).unwrap();
    assert_eq!(challenge.rp_nonce, rp_nonce);
    assert_eq!(challenge.expected_index, 513);
    assert_eq!(hex::encode(challenge.chain_head), chain_head_hex);

    // ---- 4. Attester 签 LivenessResponse 回送 ----
    let mut liveness = LivenessResponse::for_challenge(&challenge);
    liveness.sign(&attester_key);
    ws.send(WsMessage::Binary(liveness.to_cbor()))
        .await
        .unwrap();

    // 收到签发回执
    let ack_frame = ws.next().await.unwrap().unwrap();
    let ack: serde_json::Value = serde_json::from_str(ack_frame.to_text().unwrap()).unwrap();
    assert_eq!(ack["ok"], true, "ack = {ack}");
    assert_eq!(ack["meets_policy"], true);

    // ---- 5. RP POST /v1/poh 取回证书 ----
    let resp = client
        .post(format!("{http}/v1/poh"))
        .json(&serde_json::json!({ "challenge_id": challenge_id_hex }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(
        resp.headers()["content-type"],
        "application/cbor",
        "poh content type"
    );
    let cert_cbor = resp.bytes().await.unwrap();
    let cert = PohCertificate::from_cbor(&cert_cbor).expect("parse PoH");

    // RP 侧密码学 + 新鲜性 + nonce 绑定
    cert.verify_freshness(&verifier_pub, &rp_nonce, now_unix())
        .expect("PoH freshness + signature");
    assert!(cert.meets_policy(0.1, 20.0));
    assert_eq!(cert.identity, attester_key.public_bytes());
    assert_eq!(cert.breadcrumb_count, 513);
    assert_eq!(cert.nonce, rp_nonce);

    // 统计指数落在生物区间且信任分达标（W3/W4 校准结果）
    assert!(
        (0.30..=0.80).contains(&cert.alpha),
        "alpha = {} outside biological range",
        cert.alpha
    );
    assert!(cert.trust >= 20.0, "trust = {}", cert.trust);
}

#[tokio::test]
async fn poh_before_response_returns_202() {
    let (http, _ws_base) = spawn_server().await;
    let key = ProtocolKey::from_seed(&[77u8; 32]);
    let body = small_signed_chain(&key, 3, None);
    let attester_hex = hex::encode(key.public_bytes());
    let client = reqwest::Client::new();

    // 不建立 WS（delivered=false），挑战仍会创建并保留至 deadline。
    client
        .post(format!("{http}/v1/evidence"))
        .header("content-type", "application/cbor")
        .body(body)
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap();

    let verify: serde_json::Value = client
        .post(format!("{http}/v1/verify"))
        .json(&serde_json::json!({
            "attester": attester_hex,
            "rp_nonce": hex::encode([0xBBu8; 16]),
        }))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let challenge_id = verify["challenge_id"].as_str().unwrap();

    // 不应答直接取 → 202。
    let resp = client
        .post(format!("{http}/v1/poh"))
        .json(&serde_json::json!({ "challenge_id": challenge_id }))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 202);
}

#[tokio::test]
async fn tampered_evidence_is_rejected() {
    let (http, _ws_base) = spawn_server().await;
    let key = ProtocolKey::from_seed(&[88u8; 32]);
    let body = small_signed_chain(&key, 4, Some(2)); // 第 3 条签名被篡改

    // 与 trip-core 链规则结论一致：篡改必被拒。
    assert!(ChainRules::default().verify(&parse_chain(&body)).is_err());

    let resp = reqwest::Client::new()
        .post(format!("{http}/v1/evidence"))
        .header("content-type", "application/cbor")
        .body(body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}

#[tokio::test]
async fn oversize_batch_is_rejected() {
    let (http, _ws_base) = spawn_server().await;
    let key = ProtocolKey::from_seed(&[99u8; 32]);
    // 301 条：超过 MAX_EVIDENCE_BATCH(300)，直接 413（在链验证之前）。
    let body = small_signed_chain(&key, 301, None);
    let resp = reqwest::Client::new()
        .post(format!("{http}/v1/evidence"))
        .header("content-type", "application/cbor")
        .body(body)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 413);
}

/// 测试辅助：把 CBOR 帧流解析回面包屑（仅用于本文件的链规则断言）。
fn parse_chain(body: &[u8]) -> Vec<Breadcrumb> {
    let mut out = Vec::new();
    let mut rest = body;
    while !rest.is_empty() {
        let (one, tail) = trip_core::cbor::split_value(rest).unwrap();
        out.push(Breadcrumb::from_cbor(one).unwrap());
        rest = tail;
    }
    out
}
