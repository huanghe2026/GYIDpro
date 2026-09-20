//! W8 验收门：`did:geoyuan` 解析 + TIT（Verifier 背书）签发。
//!
//! 用真实 TCP + axum 跑通：
//! 1. `GET /v1/did/:did` → W3C DID Document（`application/did+json`），且文档
//!    里的公钥/端点与配置一致，非法 DID 返回 400；
//! 2. `POST /v1/evidence`（分页）→ `GET /v1/tit/:hex` → 得到 Verifier 签名的
//!    TIT，用 `/.well-known/verifier.json` 的公钥验签通过、错误公钥被拒。

use std::time::{SystemTime, UNIX_EPOCH};

use axum::serve;
use rand::rngs::StdRng;
use rand::SeedableRng;
use tokio::net::TcpListener;

use trip_core::did::{
    self, AnchorReference, DidDocument, SERVICE_ANCHOR, SERVICE_TIT, SERVICE_VERIFIER,
};
use trip_core::engine::sim::{trip_walk_path, SimConfig, TripConfig};
use trip_core::tit::Tit;
use trip_core::{Breadcrumb, MetaFlags, ProtocolKey};
use trip_server::{build_router, AppState, Config};

const VERIFIER_SEED: [u8; 32] = [43u8; 32];
const ATTESTER_SEED: [u8; 32] = [42u8; 32];

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// 先绑端口再建配置，使 `public_url` 与真实监听地址一致（DID Document 要用）。
async fn spawn_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let base = format!("http://127.0.0.1:{port}");

    let config = Config {
        public_url: base.clone(),
        anchor: Some(AnchorReference::new(8453, "0xGeoTITRegistry")),
        ..Config::default()
    };
    let state = AppState::new(ProtocolKey::from_seed(&VERIFIER_SEED), config);
    let app = build_router(state);
    tokio::spawn(async move {
        serve(listener, app).await.unwrap();
    });
    base
}

/// 用 trip_walk 生成 513 个合法 res10 cell 并签成面包屑链（与三方流程测试同源）。
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

/// 把 CBOR 帧流切成 ≤300 条的批次（服务端单批上限）。
fn batches(body: &[u8], per_batch: usize) -> Vec<Vec<u8>> {
    let mut frames: Vec<Vec<u8>> = Vec::new();
    let mut rest: &[u8] = body;
    while !rest.is_empty() {
        let (one, tail) = trip_core::cbor::split_value(rest).unwrap();
        frames.push(one.to_vec());
        rest = tail;
    }
    frames.chunks(per_batch).map(|c| c.concat()).collect()
}

#[tokio::test]
async fn did_resolution_returns_w3c_document() {
    let base = spawn_server().await;
    let key = ProtocolKey::from_seed(&ATTESTER_SEED);
    let pubkey = key.public_bytes();
    let did_str = did::encode(&pubkey);
    let client = reqwest::Client::new();

    let resp = client
        .get(format!("{base}/v1/did/{did_str}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let ctype = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or_default()
        .to_string();
    assert!(
        ctype.starts_with("application/did+json"),
        "content-type = {ctype}"
    );

    let json = resp.text().await.unwrap();
    // 文档必须自洽（id ↔ verificationMethod ↔ publicKeyMultibase）
    let doc = DidDocument::from_json(&json).unwrap();
    assert_eq!(doc.id, did_str);
    assert_eq!(doc.public_key().unwrap(), pubkey);
    assert_eq!(doc.service_endpoint(SERVICE_VERIFIER), Some(base.as_str()));
    let expect_tit = format!("{base}/v1/tit/{}", hex::encode(pubkey));
    assert_eq!(doc.service_endpoint(SERVICE_TIT), Some(expect_tit.as_str()));
    assert_eq!(
        doc.service_endpoint(SERVICE_ANCHOR),
        Some("eip155:8453:0xGeoTITRegistry")
    );

    // 非法 DID → 400
    for bad in ["did:geoyuan:zzz", "did:key:z6Mk", "not-a-did"] {
        let resp = client
            .get(format!("{base}/v1/did/{bad}"))
            .send()
            .await
            .unwrap();
        assert_eq!(resp.status(), 400, "did {bad} should be rejected");
    }
}

#[tokio::test]
async fn tit_issuance_and_verification() {
    let base = spawn_server().await;
    let (key, body) = sim_signed_chain();
    let pubkey = key.public_bytes();
    let pubkey_hex = hex::encode(pubkey);
    let client = reqwest::Client::new();

    // ---- 上传 evidence（513 条 → 300 + 213）----
    for batch in batches(&body, 300) {
        let resp = client
            .post(format!("{base}/v1/evidence"))
            .header("content-type", "application/cbor")
            .body(batch)
            .send()
            .await
            .unwrap();
        assert!(
            resp.status().is_success(),
            "evidence upload {}",
            resp.status()
        );
    }

    // ---- GET /v1/tit/:hex ----
    let resp = client
        .get(format!("{base}/v1/tit/{pubkey_hex}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let v: serde_json::Value = resp.json().await.unwrap();

    assert_eq!(v["issuer"], "verifier");
    assert_eq!(v["did"], did::encode(&pubkey));
    assert_eq!(v["breadcrumbs"], 513);
    assert_eq!(v["epochs"], 5);
    assert_eq!(
        v["did_document_url"].as_str().unwrap(),
        format!("/v1/did/{}", did::encode(&pubkey))
    );

    let tit = Tit::from_base64url(v["tit_base64url"].as_str().unwrap()).unwrap();
    assert_eq!(tit.did(), did::encode(&pubkey));
    assert_eq!(tit.claims.breadcrumbs, 513);
    assert!(tit.meets_handle_threshold(), "trust = {}", tit.claims.trust);

    // ---- 用 /.well-known 的 Verifier 公钥验签 ----
    let meta: serde_json::Value = client
        .get(format!("{base}/.well-known/verifier.json"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let vk: [u8; 32] = hex::decode(meta["verifier_pubkey"].as_str().unwrap())
        .unwrap()
        .try_into()
        .unwrap();
    assert_eq!(vk, ProtocolKey::from_seed(&VERIFIER_SEED).public_bytes());
    tit.verify(Some(&vk), now_unix()).unwrap();
    // 错误 Verifier 公钥 → 拒绝
    assert!(tit.verify(Some(&[0u8; 32]), now_unix()).is_err());
    // 过期 → 拒绝
    assert!(tit
        .verify(
            Some(&vk),
            tit.claims.issued_at + tit.claims.validity_secs + 1
        )
        .is_err());

    // ---- 未知 attester → 404 ----
    let resp = client
        .get(format!("{base}/v1/tit/{}", "11".repeat(32)))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 404);

    // ---- 非法 hex → 400 ----
    let resp = client
        .get(format!("{base}/v1/tit/xyz"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
}
