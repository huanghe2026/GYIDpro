//! 端到端集成测试：identity → breadcrumb 采集 → chain 续链 → CBOR 流
//! → chain 重解析 → liveness challenge/response → PoH 验证。
//!
//! 不涉及 trip-server；所有协议原语都来自 trip-core。

use gyid_shared::breadcrumb::ContextExtras;
use gyid_shared::liveness::sign_liveness_response;
use gyid_shared::poh::verify_poh;
use gyid_shared::{Chain, Identity};
use trip_core::liveness::LivenessChallenge;
use trip_core::poh::PohCertificate;
use trip_core::ProtocolKey;

fn make_identity() -> Identity {
    Identity::from_seed_hex(&"07".repeat(32)).unwrap()
}

#[test]
fn full_collector_flow() {
    let id = make_identity();

    // 1) 端上采集两条面包屑：探索会话允许 5..15 分钟间隔，跳到不同 H3 cell。
    let mut chain = Chain::new();
    chain
        .collect_and_append(&id, 39.9042, 116.4074, 10, 1_700_000_000, true, None)
        .unwrap();
    chain
        .collect_and_append(&id, 39.9150, 116.4270, 10, 1_700_000_400, true, None)
        .unwrap();

    // 2) 全链自检通过
    chain.verify_self().expect("chain self-verify");

    // 3) CBOR 帧流可往返
    let stream = chain.to_cbor_stream();
    let parsed = Chain::from_cbor_stream(&stream).expect("cbor stream parse");
    assert_eq!(parsed.len(), 2);
    parsed.verify_self().expect("parsed chain self-verify");

    // 4) 链尾 hash 取出
    let head = chain.block_hash_of_last().expect("non-empty chain has head");

    // 5) 模拟 Verifier 下发 liveness challenge，Attester 签名响应
    let challenge = LivenessChallenge::new([0xAB; 16], [0xCD; 16], head, 2, 1_700_000_060);
    let resp = sign_liveness_response(&id, &challenge).unwrap();
    assert!(resp.matches_challenge(&challenge));
    resp.verify_signature(&id.protocol_key().public_bytes())
        .expect("attester signature valid");
}

#[test]
fn poh_verify_closed_loop() {
    let verifier = ProtocolKey::from_seed(&[3u8; 32]);
    let verifier_pk = verifier.public_bytes();

    // 模拟 Verifier 在 PoH 证书里绑定的字段
    let attester_pk = make_identity().protocol_key().public_bytes();
    let nonce = [0xCDu8; 16];
    let chain_head = [0xEFu8; 32];

    let poh = PohCertificate::issue(
        &verifier,
        attester_pk,
        1_700_000_000,
        3,
        0.57,
        1.72,
        42.5,
        0.88,
        0.736,
        64.5,
        50,
        200,
        3600,
        nonce,
        chain_head,
    );
    let bytes = poh.to_cbor();

    // RP 侧验证：所有条件通过
    let info = verify_poh(&bytes, &verifier_pk, &nonce, 1_700_000_100, 0.5, 20.0).unwrap();
    assert!(info.fresh);
    assert!(info.policy_pass);
    assert!(info.is_trusted());
    assert_eq!(info.identity, attester_pk);
    assert_eq!(info.nonce, nonce);
    assert_eq!(info.chain_head, chain_head);
}

#[test]
fn extras_flow() {
    let id = make_identity();
    let extras = ContextExtras {
        wifi_bssid: Some([0xAA; 6]),
        cell_id: Some(vec![0xBB, 0xCC]),
        imu_digest: Some([0x11; 32]),
    };
    let mut chain = Chain::new();
    chain
        .collect_and_append(&id, 39.9042, 116.4074, 10, 1_700_000_000, true, Some(&extras))
        .unwrap();
    let bc = chain.last().unwrap();
    assert!(bc.meta.wifi_present);
    assert!(bc.meta.cell_present);
    assert!(bc.meta.imu_present);
}
