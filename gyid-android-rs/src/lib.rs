//! # gyid-android-rs
//!
//! GyID 核心到 Kotlin/Android 的 UniFFI 桥。接口定义见 [`gyid.udl`]，
//! 实现全部委托 [`gyid_shared`] 与 [`trip_core`]，本 crate 只做
//! hex ↔ bytes、错误类型映射等 FFI 适配。
//!
//! 与 gyid-wasm 的设计约定保持一致：
//! - **字节一律 hex 字符串**（seed/pubkey/CBOR 帧/cell）
//! - **无状态纯函数**：Identity / Chain 状态由 Kotlin 侧持有
//! - **传输用 CBOR hex**：与 trip-server 端点对齐

// UniFFI 生成代码（gyid.uniffi.rs）带空行的 doc comment 触发新 lint，此处统一放行
#![allow(clippy::empty_line_after_doc_comments)]

use gyid_shared::chain::Chain;
use gyid_shared::{collect_breadcrumb, verify_poh as shared_verify_poh, Identity};
use trip_core::liveness::{LivenessChallenge, LivenessResponse};

uniffi::include_scaffolding!("gyid");

// ── UDL 暴露的数据类型（字段名与 gyid.udl 一致）──

/// 身份密钥对（镜像 UDL `Keypair`）。
pub struct Keypair {
    /// 32 字节 Ed25519 seed hex（64 字符），仅供本地加密存储。
    pub seed_hex: String,
    /// 32 字节公钥 hex（64 字符），公开。
    pub pubkey_hex: String,
}

/// PoH 校验结果（镜像 UDL `PohInfo`）。
pub struct PohInfo {
    /// 签名有效 + 未过期 + nonce 匹配。
    pub fresh: bool,
    /// α / 置信度 / 信任分达到 RP 门槛。
    pub policy: bool,
    pub alpha: f64,
    pub trust: f64,
    pub unique_cells: u64,
    pub breadcrumb_count: u64,
}

/// FFI 错误（UDL `[Error] interface`，命名字段变体）。
#[derive(Debug, thiserror::Error)]
pub enum GyidError {
    /// 参数 / 编码错误（hex、CBOR 解析失败等）。
    #[error("bad request: {detail}")]
    BadRequest { detail: String },
    /// 资源不存在。
    #[error("not found: {detail}")]
    NotFound { detail: String },
    /// 证书 / 挑战过期。
    #[error("expired: {detail}")]
    Expired { detail: String },
    /// 核心引擎 / 内部错误。
    #[error("internal: {detail}")]
    Internal { detail: String },
}

impl From<gyid_shared::GyidError> for GyidError {
    fn from(e: gyid_shared::GyidError) -> Self {
        use gyid_shared::GyidError::*;
        match e {
            Hex(msg) | BadInput(msg) | H3(msg) => GyidError::BadRequest {
                detail: msg.to_string(),
            },
            Core(msg) => GyidError::Internal {
                detail: msg.to_string(),
            },
            Crypto(msg) => GyidError::Internal { detail: msg },
            // gyid-shared 固定启用 http-client feature（见 Cargo.toml）
            Http(msg) => GyidError::Internal { detail: msg },
            Verifier(code, body) => match code {
                404 => GyidError::NotFound { detail: body },
                410 => GyidError::Expired { detail: body },
                400..=499 => GyidError::BadRequest {
                    detail: format!("HTTP {code}: {body}"),
                },
                _ => GyidError::Internal {
                    detail: format!("HTTP {code}: {body}"),
                },
            },
        }
    }
}

// ── hex 工具 ──

fn decode_hex(hex_str: &str, expected: usize, name: &str) -> Result<Vec<u8>, GyidError> {
    let bytes = hex::decode(hex_str.trim()).map_err(|e| GyidError::BadRequest {
        detail: format!("{name} hex decode: {e}"),
    })?;
    if bytes.len() != expected {
        return Err(GyidError::BadRequest {
            detail: format!("{name} must be {expected} bytes, got {}", bytes.len()),
        });
    }
    Ok(bytes)
}

// ═══════════════════════════════════════════════════════════════════════
// UDL 函数实现
// ═══════════════════════════════════════════════════════════════════════

/// 生成新 Ed25519 身份。
fn generate_keypair() -> Result<Keypair, GyidError> {
    let id = Identity::generate();
    Ok(Keypair {
        seed_hex: id.seed_hex(),
        pubkey_hex: id.pubkey_hex(),
    })
}

/// 从 seed hex 推导公钥 hex。
fn pubkey_from_seed(seed_hex: String) -> Result<String, GyidError> {
    Ok(Identity::from_seed_hex(&seed_hex)?.pubkey_hex())
}

/// (lat,lng) → H3 cell hex（16 字符）。
fn h3_to_cell(lat: f64, lng: f64, res: u8) -> Result<String, GyidError> {
    let resolution = h3o::Resolution::try_from(res).map_err(|e| GyidError::BadRequest {
        detail: format!("h3 resolution {res}: {e}"),
    })?;
    let latlng = h3o::LatLng::new(lat, lng).map_err(|e| GyidError::BadRequest {
        detail: format!("latlng ({lat},{lng}): {e}"),
    })?;
    Ok(format!("{:016x}", u64::from(latlng.to_cell(resolution))))
}

/// 签名一条面包屑，返回 CBOR hex。
///
/// - `wifi_bssid_hex`：12 hex（6 字节 MAC，取信号最强 AP，BSSID 按信号排序后取首）；
/// - `imu_digest_hex`：64 hex（加速度+陀螺仪向量串的 SHA-256）；
///   两者传 null 表示该分量缺失。
#[allow(clippy::too_many_arguments)] // FFI 边界，参数平铺便于 Kotlin 调用
fn sign_breadcrumb(
    seed_hex: String,
    index: u64,
    ts: u64,
    lat: f64,
    lng: f64,
    res: u8,
    prev_hex: Option<String>,
    exploration: bool,
    wifi_bssid_hex: Option<String>,
    imu_digest_hex: Option<String>,
) -> Result<String, GyidError> {
    let identity = Identity::from_seed_hex(&seed_hex)?;
    let prev = match prev_hex {
        None => None,
        Some(h) if h.is_empty() => None,
        Some(h) => {
            let bytes = decode_hex(&h, 32, "prev_hash")?;
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            Some(arr)
        }
    };

    // 环境分量（best-effort；端上没有传感器数据则缺省）
    let wifi_bssid = match wifi_bssid_hex {
        None => None,
        Some(h) if h.is_empty() => None,
        Some(h) => {
            let bytes = decode_hex(&h, 6, "wifi_bssid")?;
            let mut arr = [0u8; 6];
            arr.copy_from_slice(&bytes);
            Some(arr)
        }
    };
    let imu_digest = match imu_digest_hex {
        None => None,
        Some(h) if h.is_empty() => None,
        Some(h) => {
            let bytes = decode_hex(&h, 32, "imu_digest")?;
            let mut arr = [0u8; 32];
            arr.copy_from_slice(&bytes);
            Some(arr)
        }
    };
    let extras = if wifi_bssid.is_none() && imu_digest.is_none() {
        None
    } else {
        Some(gyid_shared::breadcrumb::ContextExtras {
            wifi_bssid,
            cell_id: None,
            imu_digest,
        })
    };

    let bc = collect_breadcrumb(
        &identity,
        lat,
        lng,
        res,
        ts,
        index,
        prev,
        exploration,
        extras.as_ref(),
    )?;
    Ok(hex::encode(bc.to_cbor()))
}

/// 单条面包屑 CBOR hex → block_hash（64 hex），供端上续链。
fn breadcrumb_block_hash(crumb_hex: String) -> Result<String, GyidError> {
    let bytes = decode_hex(&crumb_hex, crumb_hex.len() / 2, "crumb")?;
    let bc = trip_core::Breadcrumb::from_cbor(&bytes).map_err(|e| GyidError::BadRequest {
        detail: format!("parse breadcrumb: {e}"),
    })?;
    Ok(hex::encode(bc.block_hash()))
}

/// 校验 CBOR 帧流（hex）面包屑链。
fn verify_chain(hex_crumbs: String) -> Result<bool, GyidError> {
    let bytes = decode_hex(&hex_crumbs, hex_crumbs.len() / 2, "chain")?;
    let chain = Chain::from_cbor_stream(&bytes)?;
    chain.verify_self()?;
    Ok(true)
}

/// Attester 对 LivenessChallenge 签名，返回 LivenessResponse CBOR hex。
fn sign_liveness_response(seed_hex: String, challenge_hex: String) -> Result<String, GyidError> {
    let identity = Identity::from_seed_hex(&seed_hex)?;
    let challenge_bytes = decode_hex(&challenge_hex, challenge_hex.len() / 2, "challenge")?;
    let challenge =
        LivenessChallenge::from_cbor(&challenge_bytes).map_err(|e| GyidError::BadRequest {
            detail: format!("parse challenge: {e}"),
        })?;
    let resp: LivenessResponse = gyid_shared::sign_liveness_response(&identity, &challenge)?;
    Ok(hex::encode(resp.to_cbor()))
}

/// RP 校验 PoH 证书。
#[allow(clippy::too_many_arguments)] // FFI 边界，参数平铺便于 Kotlin 调用
fn verify_poh(
    poh_hex: String,
    verifier_pubkey_hex: String,
    nonce_hex: String,
    now: u64,
    min_confidence: f64,
    min_trust: f64,
) -> Result<PohInfo, GyidError> {
    let poh_bytes = decode_hex(&poh_hex, poh_hex.len() / 2, "poh")?;
    let vk_bytes = decode_hex(&verifier_pubkey_hex, 32, "verifier_pubkey")?;
    let nonce_bytes = decode_hex(&nonce_hex, 16, "nonce")?;
    let mut vk = [0u8; 32];
    vk.copy_from_slice(&vk_bytes);
    let mut nonce = [0u8; 16];
    nonce.copy_from_slice(&nonce_bytes);

    let info = shared_verify_poh(&poh_bytes, &vk, &nonce, now, min_confidence, min_trust)?;
    Ok(PohInfo {
        fresh: info.fresh,
        policy: info.policy_pass,
        alpha: info.alpha,
        trust: info.trust,
        unique_cells: info.unique_cells,
        breadcrumb_count: info.breadcrumb_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use trip_core::liveness::LivenessChallenge;
    use trip_core::poh::PohCertificate;
    use trip_core::ProtocolKey;

    const SEED: &str = "0707070707070707070707070707070707070707070707070707070707070707";

    #[test]
    fn keypair_is_64_hex_each() {
        let kp = generate_keypair().unwrap();
        assert_eq!(kp.seed_hex.len(), 64);
        assert_eq!(kp.pubkey_hex.len(), 64);
        assert_ne!(kp.seed_hex, kp.pubkey_hex);
        // seed → pubkey 推导一致
        assert_eq!(pubkey_from_seed(kp.seed_hex).unwrap(), kp.pubkey_hex);
    }

    #[test]
    fn h3_cell_is_16_hex() {
        let cell = h3_to_cell(39.9042, 116.4074, 10).unwrap();
        assert_eq!(cell.len(), 16);
        u64::from_str_radix(&cell, 16).unwrap();
    }

    #[test]
    fn sign_and_verify_chain_roundtrip() {
        // 创世块携带 wifi + imu extras（meta 标志与 context_digest 由 shared 保证）
        let bc0 = sign_breadcrumb(
            SEED.into(),
            0,
            1_700_000_000,
            39.9042,
            116.4074,
            10,
            None,
            false,
            Some("deadbeef0001".into()),
            Some("11".repeat(32)),
        )
        .unwrap();
        // FFI 取 block_hash 续链（间隔 900s、不同坐标）
        let prev = breadcrumb_block_hash(bc0.clone()).unwrap();
        assert_eq!(prev.len(), 64);
        let bc1 = sign_breadcrumb(
            SEED.into(),
            1,
            1_700_000_900,
            39.9150,
            116.4270,
            10,
            Some(prev),
            false,
            None,
            None,
        )
        .unwrap();
        let stream = format!("{bc0}{bc1}");
        assert!(verify_chain(stream).unwrap());
    }

    #[test]
    fn sign_rejects_bad_extras_hex() {
        // wifi 必须 6 字节（12 hex）
        assert!(sign_breadcrumb(
            SEED.into(),
            0,
            1_700_000_000,
            39.9,
            116.4,
            10,
            None,
            false,
            Some("aabb".into()),
            None,
        )
        .is_err());
        // imu 必须 32 字节（64 hex）
        assert!(sign_breadcrumb(
            SEED.into(),
            0,
            1_700_000_000,
            39.9,
            116.4,
            10,
            None,
            false,
            None,
            Some("ab".into()),
        )
        .is_err());
    }

    #[test]
    fn liveness_roundtrip() {
        let identity = Identity::from_seed_hex(SEED).unwrap();
        let challenge =
            LivenessChallenge::new([0xAB; 16], [0xCD; 16], [0xEF; 32], 7, 1_700_000_060);
        let ch_hex = hex::encode(challenge.to_cbor());
        let resp_hex = sign_liveness_response(SEED.into(), ch_hex).unwrap();
        let resp =
            trip_core::liveness::LivenessResponse::from_cbor(&hex::decode(&resp_hex).unwrap())
                .unwrap();
        resp.verify_signature(&identity.protocol_key().public_bytes())
            .unwrap();
    }

    #[test]
    fn verify_poh_fresh_and_policy() {
        let verifier = ProtocolKey::from_seed(&[3u8; 32]);
        let nonce = [0xAB; 16];
        let cert = PohCertificate::issue(
            &verifier,
            [4u8; 32],
            1_700_000_000,
            1,
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
            [0xCD; 32],
        )
        .to_cbor();
        let info = verify_poh(
            hex::encode(&cert),
            hex::encode(verifier.public_bytes()),
            hex::encode(nonce),
            1_700_000_100,
            0.1,
            20.0,
        )
        .unwrap();
        assert!(info.fresh);
        assert!(info.policy);
        assert!((info.alpha - 0.57).abs() < 1e-9);
        assert_eq!(info.breadcrumb_count, 200);
    }

    #[test]
    fn bad_hex_returns_bad_request() {
        // 正常调用对照
        assert_eq!(h3_to_cell(39.9, 116.4, 10).unwrap().len(), 16);
        match verify_chain("zz".into()) {
            Err(GyidError::BadRequest { .. }) => {}
            other => panic!("expected BadRequest, got {other:?}"),
        }
    }
}
