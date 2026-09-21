//! # gyid-wasm
//!
//! 浏览器 WASM 桥，把 [`gyid_shared`] 的关键 API 暴露给 JS。
//! 所有 Rust struct 通过 `tsify` 自动生成 `.d.ts`，前端开发体验 = 原生 TS。
//!
//! ## 设计原则
//! - **字节一律 hex 字符串**：`[u8; N]` / `Vec<u8>` 在 JS 侧统一用 `string` 表达，
//!   避免 `Uint8Array` 在 IndexedDB / JSON 之间反复转换的痛点；
//! - **状态在 JS 侧**：本 crate 不维护任何长生命周期状态（Identity / Chain 都
//!   由前端 store 持有），WASM 函数都是纯函数；
//! - **错误一律 `Result<T, JsValue>`**：失败时返回 `Error(string)` 给 JS catch；
//! - **传输用 CBOR hex**：与 `trip-server` 端的 CBOR body 对齐，签名走确定性 CBOR。
//!
//! ## 不在本 crate
//! - WebSocket / fetch 传输：浏览器原生 WebSocket + fetch 比 wasm-bindgen-futures
//!   包一层更快，前端 `src/lib/verifier.ts` 直接用原生 API；
//! - IndexedDB 持久化：前端 `src/lib/storage.ts` 用 `idb` 库；
//! - UI：SolidJS 在 `gyid-web/` 里写。

#![forbid(unsafe_code)]
// tsify 0.5 的 into_wasm_abi/from_wasm_abi 标记 deprecated（tsify#65，新 API 在
// 更新大版本才有）；本 crate 已在浏览器端到端验证，暂不迁移，统一放行。
#![allow(deprecated)]

use gyid_shared::{
    chain::Chain,
    collect_breadcrumb as shared_collect_breadcrumb,
    identity::Identity,
    liveness::sign_liveness_response as shared_sign_liveness,
    poh::{verify_poh as shared_verify_poh, PohInfo},
};
use serde::{Deserialize, Serialize};
use trip_core::breadcrumb::Breadcrumb;
use trip_core::liveness::{LivenessChallenge, LivenessResponse};
use wasm_bindgen::prelude::*;

// ============================================================================
// JS-facing 类型（tsify 自动生成 .d.ts）
// ============================================================================

/// 身份密钥对（seed 用于本地加密存储，pubkey 公开上链）。
#[derive(Serialize, Deserialize, tsify::Tsify)]
#[tsify(from_wasm_abi, into_wasm_abi)]
pub struct KeypairJs {
    /// 32 字节 Ed25519 seed 的 hex（64 字符）。**仅供本地存储/调试，不要外发**。
    pub seed_hex: String,
    /// 32 字节 Ed25519 公钥的 hex（64 字符）。
    pub pubkey_hex: String,
}

/// 一条已签名面包屑（draft-04 §3）。所有字节字段都是 hex 字符串。
#[derive(Serialize, Deserialize, tsify::Tsify)]
#[tsify(from_wasm_abi, into_wasm_abi)]
pub struct BreadcrumbJs {
    /// 链内序号（从 0 起连续）。
    pub index: u64,
    /// Attester 公钥 hex。
    pub pubkey_hex: String,
    /// Unix 秒时间戳。
    pub timestamp: u64,
    /// H3 cell 的 hex（16 字符）。**不用 number**：res 7..=10 的 cell 值
    /// 约 6×10^17，超过 JS Number.MAX_SAFE_INTEGER（2^53），会丢精度。
    pub h3_cell_hex: String,
    /// H3 分辨率（7..=10）。
    pub h3_resolution: u8,
    /// §2.2 context digest hex（32 字节）。
    pub context_digest_hex: String,
    /// 上一条面包屑的 block_hash hex；创世为 null。
    pub prev_hash_hex: Option<String>,
    /// 当前块哈希 hex（= SHA-256(to_cbor)）。
    pub block_hash_hex: String,
    /// §4.2 探索会话标志。
    pub exploration: bool,
    /// Wi-Fi 分量是否在 context digest 中。
    pub wifi_present: bool,
    /// 基站分量是否在 context digest 中。
    pub cell_present: bool,
    /// IMU 分量是否在 context digest 中。
    pub imu_present: bool,
    /// Attester Ed25519 签名 hex（64 字节）。
    pub signature_hex: String,
}

/// Verifier 下发的活体挑战（draft-04 §12）。
#[derive(Serialize, Deserialize, tsify::Tsify)]
#[tsify(from_wasm_abi, into_wasm_abi)]
pub struct LivenessChallengeJs {
    pub challenge_id_hex: String,
    pub rp_nonce_hex: String,
    pub chain_head_hex: String,
    pub expected_index: u64,
    pub deadline: u64,
}

/// Attester 回送的活体响应（draft-04 §12）。
#[derive(Serialize, Deserialize, tsify::Tsify)]
#[tsify(from_wasm_abi, into_wasm_abi)]
pub struct LivenessResponseJs {
    pub challenge_id_hex: String,
    pub rp_nonce_hex: String,
    pub chain_head_hex: String,
    pub index: u64,
    pub attester_signature_hex: String,
}

/// PoH 证书解析结果（RP 侧）。
#[derive(Serialize, Deserialize, tsify::Tsify)]
#[tsify(from_wasm_abi, into_wasm_abi)]
pub struct PohInfoJs {
    pub identity_hex: String,
    pub issued_at: u64,
    pub epoch_count: u64,
    pub alpha: f64,
    pub beta: f64,
    pub kappa: f64,
    pub pi: f64,
    pub criticality_confidence: f64,
    pub trust: f64,
    pub unique_cells: u64,
    pub breadcrumb_count: u64,
    pub validity_secs: u64,
    pub nonce_hex: String,
    pub chain_head_hex: String,
    pub verifier_signature_hex: String,
    /// 签名有效 + 未过期 + nonce 匹配。
    pub fresh: bool,
    /// α / 置信度 / 信任分达到门槛。
    pub policy_pass: bool,
    /// fresh && policy_pass。
    pub is_trusted: bool,
}

// ============================================================================
// 内部转换工具
// ============================================================================

fn err_to_js<E: std::fmt::Display>(e: E) -> JsValue {
    JsValue::from_str(&format!("{e}"))
}

fn hex32(bytes: &[u8; 32]) -> String {
    hex::encode(bytes)
}

fn hex64(bytes: &[u8; 64]) -> String {
    hex::encode(bytes)
}

fn hex16(bytes: &[u8; 16]) -> String {
    hex::encode(bytes)
}

fn parse32(hex_str: &str) -> Result<[u8; 32], JsValue> {
    let bytes = hex::decode(hex_str.trim()).map_err(err_to_js)?;
    if bytes.len() != 32 {
        return Err(JsValue::from_str(&format!(
            "expected 32 bytes, got {}",
            bytes.len()
        )));
    }
    let mut arr = [0u8; 32];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

fn parse16(hex_str: &str) -> Result<[u8; 16], JsValue> {
    let bytes = hex::decode(hex_str.trim()).map_err(err_to_js)?;
    if bytes.len() != 16 {
        return Err(JsValue::from_str(&format!(
            "expected 16 bytes, got {}",
            bytes.len()
        )));
    }
    let mut arr = [0u8; 16];
    arr.copy_from_slice(&bytes);
    Ok(arr)
}

fn crumb_to_js(bc: &Breadcrumb) -> BreadcrumbJs {
    BreadcrumbJs {
        index: bc.index,
        pubkey_hex: hex32(&bc.identity),
        timestamp: bc.timestamp,
        h3_cell_hex: format!("{:016x}", bc.h3_cell),
        h3_resolution: bc.h3_resolution,
        context_digest_hex: hex32(&bc.context_digest),
        prev_hash_hex: bc.prev_hash.map(|h| hex32(&h)),
        block_hash_hex: hex32(&bc.block_hash()),
        exploration: bc.meta.exploration,
        wifi_present: bc.meta.wifi_present,
        cell_present: bc.meta.cell_present,
        imu_present: bc.meta.imu_present,
        signature_hex: hex64(&bc.signature),
    }
}

fn crumb_from_js(js: &BreadcrumbJs) -> Result<Breadcrumb, JsValue> {
    let identity = parse32(&js.pubkey_hex)?;
    let context_digest = parse32(&js.context_digest_hex)?;
    let h3_cell = u64::from_str_radix(js.h3_cell_hex.trim(), 16)
        .map_err(|e| JsValue::from_str(&format!("h3_cell hex decode: {e}")))?;
    let prev_hash = match &js.prev_hash_hex {
        Some(h) => Some(parse32(h)?),
        None => None,
    };
    let signature = {
        let bytes = hex::decode(js.signature_hex.trim()).map_err(err_to_js)?;
        if bytes.len() != 64 {
            return Err(JsValue::from_str(&format!(
                "signature must be 64 bytes, got {}",
                bytes.len()
            )));
        }
        let mut arr = [0u8; 64];
        arr.copy_from_slice(&bytes);
        arr
    };
    let mut meta = trip_core::breadcrumb::MetaFlags::new();
    meta.exploration = js.exploration;
    meta.wifi_present = js.wifi_present;
    meta.cell_present = js.cell_present;
    meta.imu_present = js.imu_present;

    Ok(Breadcrumb {
        index: js.index,
        identity,
        timestamp: js.timestamp,
        h3_cell,
        h3_resolution: js.h3_resolution,
        context_digest,
        prev_hash,
        signature,
        meta,
    })
}

// ============================================================================
// Identity API
// ============================================================================

/// 生成新的 Ed25519 身份（OsRng 真随机）。
/// 返回 seed_hex（**仅供本地加密存储，不要外发**）+ pubkey_hex。
#[wasm_bindgen]
pub fn generate_keypair() -> Result<KeypairJs, JsValue> {
    let id = Identity::generate();
    Ok(KeypairJs {
        seed_hex: id.seed_hex(),
        pubkey_hex: id.pubkey_hex(),
    })
}

/// 从 hex seed（64 字符）派生公钥 hex。用于解锁后显示。
#[wasm_bindgen]
pub fn derive_pubkey_hex(seed_hex: &str) -> Result<String, JsValue> {
    let id = Identity::from_seed_hex(seed_hex).map_err(err_to_js)?;
    Ok(id.pubkey_hex())
}

/// 用 passphrase 加密 seed，返回可落盘的 JSON 字符串。
/// PBKDF2(SHA-256, 250k iters) → AES-256-GCM。同一 passphrase 每次输出不同。
#[wasm_bindgen]
pub fn encrypt_identity(seed_hex: &str, passphrase: &str) -> Result<String, JsValue> {
    let id = Identity::from_seed_hex(seed_hex).map_err(err_to_js)?;
    id.to_encrypted_json(passphrase).map_err(err_to_js)
}

/// 用 passphrase 解密之前 `encrypt_identity` 输出的 JSON。
/// 成功返回 KeypairJs（含 seed_hex + pubkey_hex）。
#[wasm_bindgen]
pub fn decrypt_identity(json: &str, passphrase: &str) -> Result<KeypairJs, JsValue> {
    let id = Identity::from_encrypted_json(json, passphrase).map_err(err_to_js)?;
    Ok(KeypairJs {
        seed_hex: id.seed_hex(),
        pubkey_hex: id.pubkey_hex(),
    })
}

// ============================================================================
// H3 + Breadcrumb API
// ============================================================================

/// 把 (lat, lng) 量化为 H3 cell（u64），返回 hex 字符串。
/// 协议要求分辨率 7..=10。
#[wasm_bindgen]
pub fn h3_to_cell_hex(lat: f64, lng: f64, resolution: u8) -> Result<String, JsValue> {
    let res = h3o::Resolution::try_from(resolution)
        .map_err(|e| JsValue::from_str(&format!("resolution: {e}")))?;
    if res < h3o::Resolution::Seven || res > h3o::Resolution::Ten {
        return Err(JsValue::from_str(&format!(
            "H3 resolution must be 7..=10, got {resolution}"
        )));
    }
    let cell = h3o::LatLng::new(lat, lng)
        .map_err(|e| JsValue::from_str(&format!("lat/lng: {e}")))?
        .to_cell(res);
    Ok(format!("{:016x}", u64::from(cell)))
}

/// 采集一条面包屑并签名。`prev_index_opt` / `prev_block_hash_hex_opt` 为 `None`
/// 表示创世（第一条）；否则取链尾的 index + block_hash。
///
/// 返回的 `BreadcrumbJs.block_hash_hex` 即下一条的 `prev_block_hash_hex`。
#[allow(clippy::too_many_arguments)] // JS API 边界，参数平铺便于调用
#[wasm_bindgen]
pub fn collect_breadcrumb(
    seed_hex: &str,
    lat: f64,
    lng: f64,
    h3_resolution: u8,
    timestamp: u64,
    prev_index_opt: Option<u64>,
    prev_block_hash_hex_opt: Option<String>,
    exploration: bool,
) -> Result<BreadcrumbJs, JsValue> {
    let id = Identity::from_seed_hex(seed_hex).map_err(err_to_js)?;
    let prev_hash = match prev_block_hash_hex_opt {
        Some(h) => Some(parse32(&h)?),
        None => None,
    };
    let index = match (prev_index_opt, prev_hash) {
        (Some(i), Some(_)) => i,
        (None, None) => 0,
        _ => {
            return Err(JsValue::from_str(
                "prev_index and prev_block_hash must both be Some or both None",
            ));
        }
    };
    let bc = shared_collect_breadcrumb(
        &id,
        lat,
        lng,
        h3_resolution,
        timestamp,
        index,
        prev_hash,
        exploration,
        None, // 浏览器侧不采 Wi-Fi/Cell/IMU（无权限/无硬件）
    )
    .map_err(err_to_js)?;
    Ok(crumb_to_js(&bc))
}

/// 单条面包屑 → CBOR hex（用于调试或自定义拼接）。
#[wasm_bindgen]
pub fn breadcrumb_to_cbor_hex(crumb: BreadcrumbJs) -> Result<String, JsValue> {
    let bc = crumb_from_js(&crumb)?;
    Ok(hex::encode(bc.to_cbor()))
}

/// CBOR hex → 单条面包屑。
#[wasm_bindgen]
pub fn breadcrumb_from_cbor_hex(cbor_hex: &str) -> Result<BreadcrumbJs, JsValue> {
    let bytes = hex::decode(cbor_hex.trim()).map_err(err_to_js)?;
    let bc = Breadcrumb::from_cbor(&bytes).map_err(err_to_js)?;
    Ok(crumb_to_js(&bc))
}

// ============================================================================
// Chain API
// ============================================================================

/// 把多条面包屑拼成 CBOR 帧流 hex，直接作为 `POST /v1/evidence` body。
///
/// 入参可以是**任意连续切片**（续传时只传后缀，如 [#100..#399]）：
/// 服务端按 index 二分合并，不要求批次从创世 #0 开始。因此这里只做序列化，
/// 不用 [`Chain::append`]（那个要求本批自 index=0 起，会拒掉合法后缀批次）。
#[wasm_bindgen]
pub fn chain_to_cbor_hex(crumbs: Vec<BreadcrumbJs>) -> Result<String, JsValue> {
    let mut out = Vec::new();
    for js in &crumbs {
        let bc = crumb_from_js(js)?;
        out.extend_from_slice(&bc.to_cbor());
    }
    Ok(hex::encode(out))
}

/// 从 CBOR 帧流 hex 解析出多条面包屑（`GET /v1/evidence` 响应或本地存储读回）。
#[wasm_bindgen]
pub fn chain_from_cbor_hex(cbor_hex: &str) -> Result<Vec<BreadcrumbJs>, JsValue> {
    let bytes = hex::decode(cbor_hex.trim()).map_err(err_to_js)?;
    let chain = Chain::from_cbor_stream(&bytes).map_err(err_to_js)?;
    Ok(chain.crumbs.iter().map(crumb_to_js).collect())
}

/// 用 `ChainRules::default()` 跑全链自检（§4.1/§4.2/§4.3）。
/// 返回 `true` 表示通过，`false` 表示违规（具体原因看 console error）。
#[wasm_bindgen]
pub fn verify_chain(crumbs: Vec<BreadcrumbJs>) -> Result<bool, JsValue> {
    let mut chain = Chain::new();
    for js in &crumbs {
        let bc = crumb_from_js(js)?;
        chain.append(bc).map_err(err_to_js)?;
    }
    match chain.verify_self() {
        Ok(()) => Ok(true),
        Err(e) => {
            web_sys_console_warn(&format!("chain verify failed: {e}"));
            Ok(false)
        }
    }
}

// ============================================================================
// Liveness API
// ============================================================================

/// 解析 Verifier 下发的 LivenessChallenge CBOR hex（用于 Verify 页调试显示）。
#[wasm_bindgen]
pub fn parse_liveness_challenge(cbor_hex: &str) -> Result<LivenessChallengeJs, JsValue> {
    let bytes = hex::decode(cbor_hex.trim()).map_err(err_to_js)?;
    let c = LivenessChallenge::from_cbor(&bytes).map_err(err_to_js)?;
    Ok(LivenessChallengeJs {
        challenge_id_hex: hex16(&c.challenge_id),
        rp_nonce_hex: hex16(&c.rp_nonce),
        chain_head_hex: hex32(&c.chain_head),
        expected_index: c.expected_index,
        deadline: c.deadline,
    })
}

/// Attester 用身份密钥对挑战签名，返回 LivenessResponse CBOR hex。
/// 直接通过实时 WS 通道回送 Verifier。
#[wasm_bindgen]
pub fn sign_liveness_response(seed_hex: &str, challenge_cbor_hex: &str) -> Result<String, JsValue> {
    let id = Identity::from_seed_hex(seed_hex).map_err(err_to_js)?;
    let bytes = hex::decode(challenge_cbor_hex.trim()).map_err(err_to_js)?;
    let challenge = LivenessChallenge::from_cbor(&bytes).map_err(err_to_js)?;
    let resp = shared_sign_liveness(&id, &challenge).map_err(err_to_js)?;
    Ok(hex::encode(resp.to_cbor()))
}

/// 解析 LivenessResponse CBOR hex（调试用）。
#[wasm_bindgen]
pub fn parse_liveness_response(cbor_hex: &str) -> Result<LivenessResponseJs, JsValue> {
    let bytes = hex::decode(cbor_hex.trim()).map_err(err_to_js)?;
    let r = LivenessResponse::from_cbor(&bytes).map_err(err_to_js)?;
    Ok(LivenessResponseJs {
        challenge_id_hex: hex16(&r.challenge_id),
        rp_nonce_hex: hex16(&r.rp_nonce),
        chain_head_hex: hex32(&r.chain_head),
        index: r.index,
        attester_signature_hex: hex64(&r.attester_signature),
    })
}

// ============================================================================
// PoH API
// ============================================================================

/// 校验 PoH 证书：CBOR 解析 → Verifier 签名 → 新鲜性 → 策略。
///
/// - `poh_hex`：PoH 证书 CBOR hex（从 `GET /v1/poh` 拿到）；
/// - `verifier_pubkey_hex`：Verifier 公钥 hex（从 `GET /.well-known/verifier.json` 拿到）；
/// - `expected_nonce_hex`：RP 自己生成的 16 字节 nonce hex；
/// - `now_unix`：当前 Unix 秒；
/// - `min_confidence` / `min_trust`：策略门槛（draft-04 §10 §2–4）。
#[wasm_bindgen]
pub fn verify_poh(
    poh_hex: &str,
    verifier_pubkey_hex: &str,
    expected_nonce_hex: &str,
    now_unix: u64,
    min_confidence: f64,
    min_trust: f64,
) -> Result<PohInfoJs, JsValue> {
    let poh_bytes = hex::decode(poh_hex.trim()).map_err(err_to_js)?;
    let verifier_pubkey = parse32(verifier_pubkey_hex)?;
    let expected_nonce = parse16(expected_nonce_hex)?;
    let info: PohInfo = shared_verify_poh(
        &poh_bytes,
        &verifier_pubkey,
        &expected_nonce,
        now_unix,
        min_confidence,
        min_trust,
    )
    .map_err(err_to_js)?;
    Ok(PohInfoJs {
        identity_hex: hex32(&info.identity),
        issued_at: info.issued_at,
        epoch_count: info.epoch_count,
        alpha: info.alpha,
        beta: info.beta,
        kappa: info.kappa,
        pi: info.pi,
        criticality_confidence: info.criticality_confidence,
        trust: info.trust,
        unique_cells: info.unique_cells,
        breadcrumb_count: info.breadcrumb_count,
        validity_secs: info.validity_secs,
        nonce_hex: hex16(&info.nonce),
        chain_head_hex: hex32(&info.chain_head),
        verifier_signature_hex: hex64(&info.verifier_signature),
        fresh: info.fresh,
        policy_pass: info.policy_pass,
        is_trusted: info.is_trusted(),
    })
}

// ============================================================================
// 内部工具：web-sys console（避免依赖 web-sys）
// ============================================================================

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn warn(s: &str);
}

fn web_sys_console_warn(s: &str) {
    warn(s);
}

// ============================================================================
// WASM 启动钩子
// ============================================================================

/// 启动钩子（可选）。前端 `initWasm()` 调用一次以设置 panic hook。
#[wasm_bindgen(start)]
pub fn _start() {
    // 让 panic 输出走 console.error，而不是 silently abort。
    console_error_panic_hook::set_once();
}
