//! 黄金向量测试：逐字节重放 tests/vectors/golden-vectors.json。
//!
//! 这些向量是 TRIP draft-04 跨语言互操作的基准——任何合规实现用同样的
//! 固定输入都必须得到完全一致的 CBOR / 签名 / 哈希输出。

use serde_json::Value;
use trip_core::{Breadcrumb, ChainRules, Epoch, MetaFlags, PohCertificate, ProtocolKey};

const VECTORS: &str = include_str!("vectors/golden-vectors.json");

fn bytes(j: &Value, key: &str) -> Vec<u8> {
    hex::decode(j[key].as_str().expect("hex string")).expect("valid hex")
}

fn opt_bytes(j: &Value, key: &str) -> Option<[u8; 32]> {
    match j[key].as_str() {
        Some(s) => {
            let v = hex::decode(s).unwrap();
            Some(v.try_into().expect("32 bytes"))
        }
        None => None,
    }
}

fn fixed<const N: usize>(v: Vec<u8>) -> [u8; N] {
    v.try_into().expect("fixed length")
}

fn meta(j: &Value) -> MetaFlags {
    let m = &j["meta"];
    MetaFlags {
        exploration: m["exploration"].as_bool().unwrap(),
        wifi_present: m["wifi_present"].as_bool().unwrap(),
        cell_present: m["cell_present"].as_bool().unwrap(),
        imu_present: m["imu_present"].as_bool().unwrap(),
        photo_present: m["photo_present"].as_bool().unwrap(),
    }
}

#[test]
fn golden_vectors_replay_exactly() {
    let v: Value = serde_json::from_str(VECTORS).expect("parse golden json");
    assert_eq!(v["spec"], "draft-ayerbe-trip-protocol-04");

    // ---- 固定密钥派生出的公钥必须与向量一致 ----
    let identity_seed = fixed::<32>(bytes(&v, "identity_seed_hex"));
    let verifier_seed = fixed::<32>(bytes(&v, "verifier_seed_hex"));
    let identity = ProtocolKey::from_seed(&identity_seed);
    let verifier = ProtocolKey::from_seed(&verifier_seed);
    assert_eq!(
        hex::encode(identity.public_bytes()),
        v["identity_public_key_hex"].as_str().unwrap()
    );
    assert_eq!(
        hex::encode(verifier.public_bytes()),
        v["verifier_public_key_hex"].as_str().unwrap()
    );
    let id_pk = identity.public_bytes();

    // ---- 逐条重放面包屑 ----
    let mut crumbs: Vec<Breadcrumb> = Vec::new();
    for j in v["breadcrumbs"].as_array().unwrap() {
        let mut c = Breadcrumb::new_unsigned(
            j["index"].as_u64().unwrap(),
            id_pk,
            j["timestamp"].as_u64().unwrap(),
            j["h3_cell"].as_u64().unwrap(),
            j["h3_resolution"].as_u64().unwrap() as u8,
            fixed::<32>(bytes(j, "context_digest_hex")),
            opt_bytes(j, "prev_hash_hex"),
            meta(j),
        );
        c.sign(&identity).unwrap();

        assert_eq!(
            hex::encode(c.signable_bytes()),
            j["signable_cbor_hex"].as_str().unwrap(),
            "signable cbor mismatch at index {}",
            c.index
        );
        assert_eq!(
            hex::encode(c.signature),
            j["signature_hex"].as_str().unwrap(),
            "signature mismatch at index {}",
            c.index
        );
        assert_eq!(
            hex::encode(c.to_cbor()),
            j["full_cbor_hex"].as_str().unwrap(),
            "full cbor mismatch at index {}",
            c.index
        );
        assert_eq!(
            hex::encode(c.block_hash()),
            j["block_hash_hex"].as_str().unwrap(),
            "block hash mismatch at index {}",
            c.index
        );

        // 解析回来必须结构相等、签名有效
        let parsed = Breadcrumb::from_cbor(&bytes(j, "full_cbor_hex")).unwrap();
        assert_eq!(parsed, c);
        parsed.verify_signature().unwrap();

        crumbs.push(c);
    }

    // ---- 整条链满足 §4 规则 ----
    ChainRules::default().verify(&crumbs).unwrap();

    // ---- Epoch ----
    let ej = &v["epoch"];
    let epoch = Epoch::seal(ej["number"].as_u64().unwrap(), &crumbs, &identity).unwrap();
    assert_eq!(
        hex::encode(epoch.merkle_root),
        ej["merkle_root_hex"].as_str().unwrap()
    );
    assert_eq!(
        hex::encode(epoch.signable_bytes()),
        ej["signable_cbor_hex"].as_str().unwrap()
    );
    assert_eq!(
        hex::encode(epoch.signature),
        ej["signature_hex"].as_str().unwrap()
    );
    assert_eq!(
        hex::encode(epoch.to_cbor()),
        ej["full_cbor_hex"].as_str().unwrap()
    );
    assert_eq!(epoch.unique_cells, 3);

    let parsed_epoch = Epoch::from_cbor(&bytes(ej, "full_cbor_hex")).unwrap();
    assert_eq!(parsed_epoch, epoch);
    parsed_epoch.verify_signature().unwrap();
    parsed_epoch.verify_coverage(&crumbs).unwrap();

    // ---- PoH Certificate ----
    let pj = &v["poh_certificate"];
    let nonce = fixed::<16>(bytes(pj, "nonce_hex"));
    let chain_head = fixed::<32>(bytes(pj, "chain_head_hex"));
    let cert = PohCertificate::issue(
        &verifier,
        id_pk,
        pj["issued_at"].as_u64().unwrap(),
        pj["epoch_count"].as_u64().unwrap(),
        pj["alpha"].as_f64().unwrap(),
        pj["beta"].as_f64().unwrap(),
        pj["kappa"].as_f64().unwrap(),
        pj["pi"].as_f64().unwrap(),
        pj["criticality_confidence"].as_f64().unwrap(),
        pj["trust"].as_f64().unwrap(),
        pj["unique_cells"].as_u64().unwrap(),
        pj["breadcrumb_count"].as_u64().unwrap(),
        pj["validity_secs"].as_u64().unwrap(),
        nonce,
        chain_head,
    );
    assert_eq!(
        hex::encode(cert.signable_bytes()),
        pj["signable_cbor_hex"].as_str().unwrap()
    );
    assert_eq!(
        hex::encode(cert.verifier_signature),
        pj["verifier_signature_hex"].as_str().unwrap()
    );
    assert_eq!(
        hex::encode(cert.to_cbor()),
        pj["full_cbor_hex"].as_str().unwrap()
    );

    let parsed_cert = PohCertificate::from_cbor(&bytes(pj, "full_cbor_hex")).unwrap();
    assert_eq!(parsed_cert, cert);

    let vk = verifier.public_bytes();
    let now = pj["issued_at"].as_u64().unwrap() + 100;
    parsed_cert.verify_freshness(&vk, &nonce, now).unwrap();
    assert!(parsed_cert.meets_policy(0.5, 20.0));

    // 用身份公钥（不是 Verifier）验签必须失败
    assert!(parsed_cert.verify_freshness(&id_pk, &nonce, now).is_err());
}

#[test]
fn tampered_signature_is_rejected() {
    let v: Value = serde_json::from_str(VECTORS).unwrap();
    let c0 = &v["breadcrumbs"][0];
    let mut raw = bytes(c0, "full_cbor_hex");

    // 翻转签名字节（CBOR 尾部 64 字节中的第一个）
    let pos = raw.len() - 64;
    raw[pos] ^= 0x01;

    let parsed = Breadcrumb::from_cbor(&raw).unwrap();
    assert!(parsed.verify_signature().is_err());
}

#[test]
fn non_canonical_integer_encoding_is_rejected() {
    let v: Value = serde_json::from_str(VECTORS).unwrap();
    let c0_full = v["breadcrumbs"][0]["full_cbor_hex"].as_str().unwrap();

    // 时间戳最小宽度编码是 02 1a 69618a00（4 字节）；
    // 改写成 8 字节补零形式（02 1b 0000000069618a00）违反 core deterministic 要求。
    let non_canonical = c0_full.replace("021a69618a00", "021b0000000069618a00");
    assert_ne!(non_canonical, c0_full);
    let raw = hex::decode(non_canonical).unwrap();
    assert!(Breadcrumb::from_cbor(&raw).is_err());
}
