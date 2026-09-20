//! TIT / DID 黄金向量（跨实现互操作基准）。
//!
//! 用固定身份 seed 与固定统计字段锁定 TIT 的：
//! - 签名输入 CBOR（字段 0..=8）
//! - 完整 CBOR（字段 0..=9）
//! - Base64url 文本
//! - 绑定出的 `did:geoyuan`
//!
//! 任何后端/其它语言实现若产生不同字节，即为不兼容。字段编号见
//! `src/tit.rs` 顶部说明（-04 未规定 TIT 编号，本表为 GYIP-0003 约定，
//! 待 -05 规范后需重新 pin 并更新本文件）。

use trip_core::did;
use trip_core::tit::{Tit, TitClaims};
use trip_core::ProtocolKey;

const IDENTITY_SEED: [u8; 32] = [0x11; 32];

const EXPECT_PUBKEY_HEX: &str = "d04ab232742bb4ab3a1368bd4615e4e6d0224ab71a016baf8520a332c9778737";
const EXPECT_SIGNABLE_HEX: &str = "a90001015820d04ab232742bb4ab3a1368bd4615e4e6d0224ab71a016baf8520a332c977873702030319012c04188905fb404f400000000000061a6553f10007190e100801";
const EXPECT_CBOR_HEX: &str = "aa0001015820d04ab232742bb4ab3a1368bd4615e4e6d0224ab71a016baf8520a332c977873702030319012c04188905fb404f400000000000061a6553f10007190e10080109584017b651818f05d07f69c19660e10d641c2ba72185db7d46794b48af55909c52a9f2ae277f4dbf45d57a020a04764540dd53fbfd00cebd3dcb070fa35c9346a50a";
const EXPECT_BASE64URL: &str = "qgABAVgg0EqyMnQrtKs6E2i9RhXk5tAiSrcaAWuvhSCjMsl3hzcCAwMZASwEGIkF-0BPQAAAAAAABhplU_EABxkOEAgBCVhAF7ZRgY8F0H9pwZZg4Q1kHCunIYXbfUZ5S0ivVZCcUqnyrid_Tb9F1XoCCgR2RUDdU_v9AM69PcsHD6Nck0alCg";
const EXPECT_DID: &str = "did:geoyuan:zF25s3DdjXdCxYBhh2z8FBusVEMT4b9bGNFVKJi3wFoF4";

/// 用固定密钥与固定字段构造身份自签 TIT。
fn fixed_tit() -> Tit {
    let key = ProtocolKey::from_seed(&IDENTITY_SEED);
    Tit::issue_identity_signed(
        &key,
        TitClaims {
            identity: [0u8; 32], // 由签发函数覆盖为 key 的公钥
            epochs: 3,
            breadcrumbs: 300,
            unique_cells: 137,
            trust: 62.5,
            issued_at: 1_700_000_000,
            validity_secs: 3600,
        },
    )
}

#[test]
fn tit_bytes_replay_exactly() {
    let tit = fixed_tit();

    assert_eq!(hex::encode(tit.claims.identity), EXPECT_PUBKEY_HEX);
    assert_eq!(hex::encode(tit.signable_bytes()), EXPECT_SIGNABLE_HEX);
    assert_eq!(hex::encode(tit.to_cbor()), EXPECT_CBOR_HEX);
    assert_eq!(tit.to_base64url(), EXPECT_BASE64URL);
    assert_eq!(tit.did(), EXPECT_DID);

    // 解析回同样内容，且签名可验
    let parsed = Tit::from_cbor(&hex::decode(EXPECT_CBOR_HEX).unwrap()).unwrap();
    assert_eq!(parsed, tit);
    assert_eq!(Tit::from_base64url(EXPECT_BASE64URL).unwrap(), tit);
    tit.verify(None, 1_700_000_100).unwrap();
}

#[test]
fn did_vectors_replay_exactly() {
    // base58btc：32 个零字节 → 32 个 '1'
    assert_eq!(
        did::encode(&[0u8; 32]),
        format!("did:geoyuan:z{}", "1".repeat(32))
    );
    // 本向量用例的身份公钥
    assert_eq!(
        did::encode(&hex::decode(EXPECT_PUBKEY_HEX).unwrap().try_into().unwrap()),
        EXPECT_DID
    );
    // 再取一个独立公钥，锁定 base58 实现
    assert_eq!(
        did::encode(&[0x9au8; 32]),
        "did:geoyuan:zBQWX3Nia4vjtUwXAB1EVoorUbTPzinjH3qe3yXuG4Jfs"
    );
    assert_eq!(
        hex::encode(did::decode(EXPECT_DID).unwrap()),
        EXPECT_PUBKEY_HEX
    );
}
