//! W6 Phase 10 验收：`GET /v1/explorer` 全网身份聚合浏览端点。
//!
//! 用真实 TCP + axum 跑通：
//! 1. 空表 → 200 + 空数组（浏览页空态）；
//! 2. 上传一个 attester 的 513 条面包屑链 → 聚合统计正确（count/unique/head/
//!    last_ts/poh_count=0）；
//! 3. `/v1/identity/:hex` 与 `/v1/explorer` 的统计互相一致。

use std::time::{SystemTime, UNIX_EPOCH};

use axum::serve;
use rand::rngs::StdRng;
use rand::SeedableRng;
use tokio::net::TcpListener;

use trip_core::engine::sim::{trip_walk_path, SimConfig, TripConfig};
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

async fn spawn_server() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let base = format!("http://127.0.0.1:{port}");
    let config = Config::default();
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
async fn explorer_empty_state_is_200() {
    let base = spawn_server().await;
    let client = reqwest::Client::new();
    let resp = client
        .get(format!("{base}/v1/explorer"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let v: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(v["total_identities"], 0);
    assert_eq!(v["total_breadcrumbs"], 0);
    assert_eq!(v["identities"].as_array().unwrap().len(), 0);
}

#[tokio::test]
async fn explorer_lists_uploaded_identity() {
    let base = spawn_server().await;
    let (key, body) = sim_signed_chain();
    let pubkey_hex = hex::encode(key.public_bytes());
    let client = reqwest::Client::new();

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

    // ---- /v1/explorer 聚合统计 ----
    let resp = client
        .get(format!("{base}/v1/explorer"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let v: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(v["total_identities"], 1);
    assert_eq!(v["total_breadcrumbs"], 513);

    let list = v["identities"].as_array().unwrap();
    assert_eq!(list.len(), 1);
    let entry = &list[0];
    assert_eq!(entry["attester"], pubkey_hex);
    assert_eq!(entry["breadcrumb_count"], 513);
    // 轨迹可能重访 cell，unique_cells ≤ 513（与 /v1/identity 一致即可）
    let unique_cells = entry["unique_cells"].as_u64().unwrap();
    assert!(unique_cells > 0 && unique_cells <= 513);
    assert_eq!(entry["poh_count"], 0);
    assert_eq!(entry["last_ts"], 1_700_000_000 + 512 * 900);
    // chain_head 是 64 字符 hex
    let head = entry["chain_head"].as_str().unwrap();
    assert_eq!(head.len(), 64);

    // ---- 与 /v1/identity/:hex 单身份端点一致性 ----
    let ident: serde_json::Value = client
        .get(format!("{base}/v1/identity/{pubkey_hex}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(ident["breadcrumb_count"], entry["breadcrumb_count"]);
    assert_eq!(ident["unique_cells"], unique_cells);
    assert_eq!(ident["chain_head"], entry["chain_head"]);
    assert_eq!(ident["last_ts"], entry["last_ts"]);

    // last_ts 降序生效（单身份无序可言，但字段可排序即可）
    assert!(entry["last_ts"].as_u64().unwrap() <= now_unix());
}
