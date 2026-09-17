//! 黄金测试向量生成器。
//!
//! 运行：
//!
//! ```bash
//! cargo run -p trip-core --example gen_vectors > trip-core/tests/vectors/golden-vectors.json
//! ```
//!
//! 向量使用**固定密钥种子**与固定轨迹，任何语言的合规实现重放这些输入时，
//! 都必须得到逐字节相同的确定性 CBOR、Ed25519 签名、块哈希与 Merkle root。
//! 重新生成只应在协议版本升级（draft-05 等）时进行，并需同步更新测试。

use serde_json::{json, Map, Value};
use trip_core::{Breadcrumb, ChainRules, Epoch, MetaFlags, PohCertificate, ProtocolKey};

fn hx(b: impl AsRef<[u8]>) -> String {
    hex::encode(b.as_ref())
}

fn meta_json(m: &MetaFlags) -> Value {
    json!({
        "exploration": m.exploration,
        "wifi_present": m.wifi_present,
        "cell_present": m.cell_present,
        "imu_present": m.imu_present,
        "photo_present": m.photo_present,
    })
}

fn main() {
    // ---- 固定密钥（仅用于测试向量，绝非常见生产密钥） ----
    let identity_seed: [u8; 32] = *b"trip-geoyuan-golden-identity-key";
    let verifier_seed: [u8; 32] = *b"trip-geoyuan-golden-verifier!!00";
    let identity = ProtocolKey::from_seed(&identity_seed);
    let verifier = ProtocolKey::from_seed(&verifier_seed);
    let id_pk = identity.public_bytes();

    // ---- 固定轨迹：3 条面包屑，严格 15 分钟间隔，3 个不同 res10 cell ----
    let t0: u64 = 1_768_000_000;
    let cells: [u64; 3] = [
        0x8a1f_429b_4590_000f,
        0x8a1f_429b_4590_001f,
        0x8a1f_429b_4590_002f,
    ];
    let contexts: [[u8; 32]; 3] = [[0x01; 32], [0x02; 32], [0x03; 32]];
    let metas = [
        MetaFlags {
            exploration: false,
            wifi_present: true,
            cell_present: true,
            imu_present: false,
            photo_present: false,
        },
        MetaFlags {
            exploration: false,
            wifi_present: true,
            cell_present: true,
            imu_present: false,
            photo_present: false,
        },
        MetaFlags {
            exploration: false,
            wifi_present: true,
            cell_present: false,
            imu_present: true,
            photo_present: true, // 覆盖 GeoYuan 扩展标志
        },
    ];

    let mut crumbs: Vec<Breadcrumb> = Vec::new();
    let mut crumb_json: Vec<Value> = Vec::new();
    let mut prev: Option<[u8; 32]> = None;
    for i in 0..3 {
        let mut c = Breadcrumb::new_unsigned(
            i as u64,
            id_pk,
            t0 + i as u64 * 900,
            cells[i],
            10,
            contexts[i],
            prev,
            metas[i],
        );
        c.sign(&identity).expect("sign breadcrumb");
        let signable = c.signable_bytes();
        let full = c.to_cbor();
        let block = c.block_hash();

        crumb_json.push(json!({
            "index": c.index,
            "timestamp": c.timestamp,
            "h3_cell_hex": format!("0x{:016x}", c.h3_cell),
            "h3_cell": c.h3_cell,
            "h3_resolution": c.h3_resolution,
            "context_digest_hex": hx(c.context_digest),
            "prev_hash_hex": c.prev_hash.map(hx),
            "meta": meta_json(&c.meta),
            "signable_cbor_hex": hx(&signable),
            "signature_hex": hx(c.signature),
            "full_cbor_hex": hx(&full),
            "block_hash_hex": hx(block),
        }));

        prev = Some(block);
        crumbs.push(c);
    }

    // 链规则自检（生成的向量必须自身合法）
    ChainRules::default()
        .verify(&crumbs)
        .expect("golden chain must satisfy chain rules");

    // ---- Epoch：覆盖全部 3 条面包屑 ----
    let epoch = Epoch::seal(0, &crumbs, &identity).expect("seal epoch");
    let epoch_signable = epoch.signable_bytes();
    let epoch_full = epoch.to_cbor();
    let epoch_json = json!({
        "number": epoch.number,
        "first_index": epoch.first_index,
        "last_index": epoch.last_index,
        "first_timestamp": epoch.first_timestamp,
        "last_timestamp": epoch.last_timestamp,
        "unique_cells": epoch.unique_cells,
        "merkle_root_hex": hx(epoch.merkle_root),
        "signable_cbor_hex": hx(&epoch_signable),
        "signature_hex": hx(epoch.signature),
        "full_cbor_hex": hx(&epoch_full),
    });

    // ---- PoH Certificate：由独立 Verifier 密钥签发，绑定 nonce 与链头 ----
    let nonce = [0xAB; 16];
    let chain_head = crumbs[2].block_hash();
    let issued_at = t0 + 1_900;
    let cert = PohCertificate::issue(
        &verifier, id_pk, issued_at, 1,     // epoch_count
        0.57,  // alpha（生物区间内）
        1.72,  // beta
        42.5,  // kappa km
        0.88,  // pi
        0.736, // criticality confidence
        64.5,  // trust
        3,     // unique cells
        3,     // breadcrumb count
        3600,  // validity secs
        nonce, chain_head,
    );
    let poh_signable = cert.signable_bytes();
    let poh_full = cert.to_cbor();
    let poh_json = json!({
        "identity_public_key_hex": hx(cert.identity),
        "issued_at": cert.issued_at,
        "epoch_count": cert.epoch_count,
        "alpha": cert.alpha,
        "beta": cert.beta,
        "kappa": cert.kappa,
        "pi": cert.pi,
        "criticality_confidence": cert.criticality_confidence,
        "trust": cert.trust,
        "unique_cells": cert.unique_cells,
        "breadcrumb_count": cert.breadcrumb_count,
        "validity_secs": cert.validity_secs,
        "nonce_hex": hx(cert.nonce),
        "chain_head_hex": hx(cert.chain_head),
        "signable_cbor_hex": hx(&poh_signable),
        "verifier_signature_hex": hx(cert.verifier_signature),
        "full_cbor_hex": hx(&poh_full),
    });

    let mut root = Map::new();
    root.insert("spec".into(), json!("draft-ayerbe-trip-protocol-04"));
    root.insert(
        "generated_by".into(),
        json!("trip-core examples/gen_vectors (deterministic; do not edit by hand)"),
    );
    root.insert(
        "notes".into(),
        json!({
            "encoding": "RFC 8949 core deterministic CBOR; floats are float64",
            "signature": "Ed25519 (RFC 8032) over signable CBOR; block hash = SHA-256(full CBOR)",
            "warning": "seeds are public test fixtures, never use them in production",
        }),
    );
    root.insert("identity_seed_hex".into(), json!(hx(identity_seed)));
    root.insert("identity_public_key_hex".into(), json!(hx(id_pk)));
    root.insert("verifier_seed_hex".into(), json!(hx(verifier_seed)));
    root.insert(
        "verifier_public_key_hex".into(),
        json!(hx(verifier.public_bytes())),
    );
    root.insert("breadcrumbs".into(), Value::Array(crumb_json));
    root.insert("epoch".into(), epoch_json);
    root.insert("poh_certificate".into(), poh_json);

    println!(
        "{}",
        serde_json::to_string_pretty(&Value::Object(root)).unwrap()
    );
}
