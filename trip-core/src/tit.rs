//! TIT（Trajectory Identity Token，轨迹身份令牌）—— GYIP-0003 §5.4。
//!
//! TIT 是身份的**公开自描述令牌**：把"这条轨迹积累到什么程度"压缩成可
//! 离线核验的短令牌，直接绑定 [`crate::did`] 的同一把 Ed25519 密钥。
//! 它是 PoH 证书的轻量补充——PoH 绑定一次 RP nonce（一次性、带统计指数），
//! TIT 则长期有效、可放进二维码/名片/DID Document，用于展示与发现。
//!
//! ## CBOR 字段（key 0..=9）
//!
//! | key | 含义 | 类型 |
//! |-----|------|------|
//! | 0 | 版本（当前 1） | uint |
//! | 1 | 身份公钥（= DID 公钥，同一密钥） | bstr(32) |
//! | 2 | epoch 数 | uint |
//! | 3 | 面包屑总数 | uint |
//! | 4 | unique H3 cell 数 | uint |
//! | 5 | 信任分 T（[0,100]） | float64 |
//! | 6 | 签发时间（Unix 秒） | uint |
//! | 7 | 有效期（秒） | uint |
//! | 8 | 签发者（0=unsigned / 1=identity / 2=verifier） | uint |
//! | 9 | 签发者 Ed25519 签名（覆盖 0..=8；unsigned 时全零） | bstr(64) |
//!
//! ## 关于字段编号（重要）
//!
//! `draft-ayerbe-trip-protocol-04` 只给出 TIT 的**构成要素**，未规定字段编号；
//! 上表是 GYIP-0003 的内部约定，与其它协议结构（Breadcrumb/Epoch/PoH）保持
//! 同样的"小整数 key + 升序 + 确定性 CBOR"风格。等 -05 明确 TIT 编码后需
//! 重新 pin 并更新黄金向量（见 `tests/tit_golden.rs`）。
//!
//! ## 两种签发者
//!
//! - **identity-signed**（`issuer = 1`）：身份自证，签名者 = 字段 1 公钥；
//!   任何人都能验签，用于展示/导出（信任分属于"自我声明"）。
//! - **verifier-signed**（`issuer = 2`）：Verifier 依据已验证面包屑链核算
//!   统计量后签发，是对"该身份确实积累了这些轨迹"的**外部背书**；
//!   验签公钥由 RP 从 `/.well-known/verifier.json` 取得（与 PoH 同源）。
//!
//! 无论哪种签发者，**TIT 都不含任何原始位置、cell 或时间序列**。

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine as _;

use crate::cbor::{parse_all, Writer};
use crate::crypto::{self, ProtocolKey};
use crate::did;
use crate::error::{Result, TripError};

/// 当前 TIT 版本。
pub const TIT_VERSION: u8 = 1;
/// 信任分上限。
pub const TRUST_MAX: f64 = 100.0;

/// TIT 签发者。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TitIssuer {
    /// 未签名（仅统计载荷，可校验但不可信）。
    Unsigned,
    /// 身份自签（签名者 = 字段 1 公钥）。
    Identity,
    /// Verifier 背书签发。
    Verifier,
}

impl TitIssuer {
    /// 编码值。
    pub fn as_u64(self) -> u64 {
        match self {
            TitIssuer::Unsigned => 0,
            TitIssuer::Identity => 1,
            TitIssuer::Verifier => 2,
        }
    }

    /// 从编码值解析。
    pub fn from_u64(v: u64) -> Result<Self> {
        match v {
            0 => Ok(TitIssuer::Unsigned),
            1 => Ok(TitIssuer::Identity),
            2 => Ok(TitIssuer::Verifier),
            other => Err(TripError::InvalidTit(format!(
                "unknown issuer value {other}"
            ))),
        }
    }

    /// 可读名（CLI / JSON 输出）。
    pub fn as_str(self) -> &'static str {
        match self {
            TitIssuer::Unsigned => "unsigned",
            TitIssuer::Identity => "identity",
            TitIssuer::Verifier => "verifier",
        }
    }
}

/// TIT 的统计载荷（字段 0..=8，签名前的全部内容）。
#[derive(Debug, Clone, PartialEq)]
pub struct TitClaims {
    /// 身份公钥（= DID 公钥）。
    pub identity: [u8; 32],
    /// epoch 数。
    pub epochs: u64,
    /// 面包屑总数。
    pub breadcrumbs: u64,
    /// unique H3 cell 数。
    pub unique_cells: u64,
    /// 信任分 T（[0,100]）。
    pub trust: f64,
    /// 签发时间（Unix 秒）。
    pub issued_at: u64,
    /// 有效期（秒）。
    pub validity_secs: u64,
}

/// 一个轨迹身份令牌。
#[derive(Debug, Clone, PartialEq)]
pub struct Tit {
    /// 字段 0：版本。
    pub version: u8,
    /// 字段 1..=8：统计载荷。
    pub claims: TitClaims,
    /// 字段 8：签发者。
    pub issuer: TitIssuer,
    /// 字段 9：签名（unsigned 时全零）。
    pub signature: [u8; 64],
}

impl Tit {
    /// 未签名 TIT（只有统计载荷）。
    pub fn unsigned(claims: TitClaims) -> Self {
        Self {
            version: TIT_VERSION,
            claims,
            issuer: TitIssuer::Unsigned,
            signature: [0u8; 64],
        }
    }

    /// 身份自签 TIT（签名者 = `key`，并作为字段 1 的身份公钥）。
    pub fn issue_identity_signed(key: &ProtocolKey, mut claims: TitClaims) -> Self {
        claims.identity = key.public_bytes();
        let mut tit = Self {
            version: TIT_VERSION,
            claims,
            issuer: TitIssuer::Identity,
            signature: [0u8; 64],
        };
        tit.signature = key.sign(&tit.signable_bytes());
        tit
    }

    /// Verifier 背书签发 TIT（身份公钥由 `claims.identity` 指定）。
    pub fn issue_verifier_signed(key: &ProtocolKey, claims: TitClaims) -> Self {
        let mut tit = Self {
            version: TIT_VERSION,
            claims,
            issuer: TitIssuer::Verifier,
            signature: [0u8; 64],
        };
        tit.signature = key.sign(&tit.signable_bytes());
        tit
    }

    /* ---------------- 编码 ---------------- */

    fn write_fields(&self, w: &mut Writer) {
        w.uint(0).uint(self.version as u64);
        w.uint(1).bstr(&self.claims.identity);
        w.uint(2).uint(self.claims.epochs);
        w.uint(3).uint(self.claims.breadcrumbs);
        w.uint(4).uint(self.claims.unique_cells);
        w.uint(5).f64(self.claims.trust);
        w.uint(6).uint(self.claims.issued_at);
        w.uint(7).uint(self.claims.validity_secs);
        w.uint(8).uint(self.issuer.as_u64());
    }

    /// 字段 0..=8 的确定性 CBOR（签名输入）。
    pub fn signable_bytes(&self) -> Vec<u8> {
        let mut w = Writer::with_capacity(120);
        w.map(9);
        self.write_fields(&mut w);
        w.into_bytes()
    }

    /// 字段 0..=9 的完整确定性 CBOR。
    pub fn to_cbor(&self) -> Vec<u8> {
        let mut w = Writer::with_capacity(150);
        w.map(10);
        self.write_fields(&mut w);
        w.uint(9).bstr(&self.signature);
        w.into_bytes()
    }

    /// 从完整 CBOR 解析（带规范化与字段范围自检）。
    pub fn from_cbor(bytes: &[u8]) -> Result<Self> {
        let root = parse_all(bytes)?;
        let version = u8::try_from(root.field(0)?.as_uint()?)
            .map_err(|_| TripError::InvalidTit("version out of range".into()))?;
        if version != TIT_VERSION {
            return Err(TripError::InvalidTit(format!(
                "unsupported version {version}, expected {TIT_VERSION}"
            )));
        }
        let identity: [u8; 32] = root
            .field(1)?
            .as_bstr()?
            .try_into()
            .map_err(|_| TripError::InvalidTit("identity must be 32 bytes".into()))?;
        let claims = TitClaims {
            identity,
            epochs: root.field(2)?.as_uint()?,
            breadcrumbs: root.field(3)?.as_uint()?,
            unique_cells: root.field(4)?.as_uint()?,
            trust: root.field(5)?.as_float()?,
            issued_at: root.field(6)?.as_uint()?,
            validity_secs: root.field(7)?.as_uint()?,
        };
        let issuer = TitIssuer::from_u64(root.field(8)?.as_uint()?)?;
        let signature: [u8; 64] = root
            .field(9)?
            .as_bstr()?
            .try_into()
            .map_err(|_| TripError::InvalidTit("signature must be 64 bytes".into()))?;

        let tit = Self {
            version,
            claims,
            issuer,
            signature,
        };
        tit.validate()?;
        if tit.to_cbor() != bytes {
            return Err(TripError::Cbor(
                "TIT cbor is not in canonical deterministic form".into(),
            ));
        }
        Ok(tit)
    }

    /// Base64url（无 padding）编码，可直接放进网址/二维码。
    pub fn to_base64url(&self) -> String {
        URL_SAFE_NO_PAD.encode(self.to_cbor())
    }

    /// 从 Base64url 解析。
    pub fn from_base64url(s: &str) -> Result<Self> {
        let bytes = URL_SAFE_NO_PAD
            .decode(s.trim())
            .map_err(|e| TripError::InvalidTit(format!("base64url decode: {e}")))?;
        Self::from_cbor(&bytes)
    }

    /* ---------------- 校验 ---------------- */

    /// 字段自洽性检查（不验签）。
    pub fn validate(&self) -> Result<()> {
        if self.version != TIT_VERSION {
            return Err(TripError::InvalidTit(format!(
                "unsupported version {}, expected {TIT_VERSION}",
                self.version
            )));
        }
        if !self.claims.trust.is_finite() || !(0.0..=TRUST_MAX).contains(&self.claims.trust) {
            return Err(TripError::InvalidTit(format!(
                "trust {} out of range [0, {TRUST_MAX}]",
                self.claims.trust
            )));
        }
        if self.claims.unique_cells > self.claims.breadcrumbs {
            return Err(TripError::InvalidTit(format!(
                "unique_cells {} exceeds breadcrumbs {}",
                self.claims.unique_cells, self.claims.breadcrumbs
            )));
        }
        if self.issuer == TitIssuer::Unsigned && self.signature != [0u8; 64] {
            return Err(TripError::InvalidTit(
                "unsigned TIT must carry an all-zero signature".into(),
            ));
        }
        Ok(())
    }

    /// 校验签名：
    /// - `Identity` → 用字段 1 公钥验签；
    /// - `Verifier` → 必须提供 `verifier_pubkey`；
    /// - `Unsigned` → 直接报错（无签发者背书）。
    pub fn verify_signature(&self, verifier_pubkey: Option<&[u8; 32]>) -> Result<()> {
        match self.issuer {
            TitIssuer::Unsigned => Err(TripError::InvalidTit(
                "unsigned TIT has no issuer signature to verify".into(),
            )),
            TitIssuer::Identity => crypto::verify(
                &self.claims.identity,
                &self.signable_bytes(),
                &self.signature,
            ),
            TitIssuer::Verifier => {
                let vk = verifier_pubkey.ok_or_else(|| {
                    TripError::InvalidTit(
                        "verifier-signed TIT requires the verifier public key".into(),
                    )
                })?;
                crypto::verify(vk, &self.signable_bytes(), &self.signature)
            }
        }
    }

    /// 新鲜性：`now <= issued_at + validity_secs`。
    pub fn verify_freshness(&self, now_unix: u64) -> Result<()> {
        if now_unix
            > self
                .claims
                .issued_at
                .saturating_add(self.claims.validity_secs)
        {
            return Err(TripError::InvalidTit("TIT expired".into()));
        }
        Ok(())
    }

    /// 签名 + 新鲜性一步校验。
    pub fn verify(&self, verifier_pubkey: Option<&[u8; 32]>, now_unix: u64) -> Result<()> {
        self.validate()?;
        self.verify_signature(verifier_pubkey)?;
        self.verify_freshness(now_unix)
    }

    /* ---------------- 便捷 ---------------- */

    /// 绑定的 `did:geoyuan`（由字段 1 公钥派生）。
    pub fn did(&self) -> String {
        did::encode(&self.claims.identity)
    }

    /// 是否满足 handle 声明门槛（n ≥ 100 且 T ≥ 20，§10）。
    pub fn meets_handle_threshold(&self) -> bool {
        self.claims.breadcrumbs >= crate::engine::trust::HANDLE_MIN_BREADCRUMBS as u64
            && self.claims.trust >= crate::engine::trust::HANDLE_MIN_TRUST
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDENTITY_SEED: [u8; 32] = [0x11; 32];
    const VERIFIER_SEED: [u8; 32] = [0x22; 32];

    fn claims() -> TitClaims {
        TitClaims {
            identity: [0u8; 32], // 由签发函数填充
            epochs: 3,
            breadcrumbs: 300,
            unique_cells: 137,
            trust: 62.5,
            issued_at: 1_700_000_000,
            validity_secs: 3600,
        }
    }

    #[test]
    fn identity_signed_roundtrip_and_did_binding() {
        let key = ProtocolKey::from_seed(&IDENTITY_SEED);
        let tit = Tit::issue_identity_signed(&key, claims());

        assert_eq!(tit.claims.identity, key.public_bytes());
        assert_eq!(tit.did(), did::encode(&key.public_bytes()));
        tit.verify(None, 1_700_000_100).unwrap();

        // CBOR / Base64url 双向往返
        let cbor = tit.to_cbor();
        assert_eq!(Tit::from_cbor(&cbor).unwrap(), tit);
        let b64 = tit.to_base64url();
        assert!(!b64.contains('=') && !b64.contains('+') && !b64.contains('/'));
        assert_eq!(Tit::from_base64url(&b64).unwrap(), tit);
    }

    #[test]
    fn verifier_signed_requires_verifier_key() {
        let attester = ProtocolKey::from_seed(&IDENTITY_SEED);
        let verifier = ProtocolKey::from_seed(&VERIFIER_SEED);
        let vk = verifier.public_bytes();

        let mut c = claims();
        c.identity = attester.public_bytes();
        let tit = Tit::issue_verifier_signed(&verifier, c);

        tit.verify(Some(&vk), 1_700_000_100).unwrap();
        // 缺 verifier 公钥 → 拒绝
        assert!(tit.verify_signature(None).is_err());
        // 错误的 verifier 公钥 → 签名校验失败
        assert!(tit.verify_signature(Some(&[0x33u8; 32])).is_err());
    }

    #[test]
    fn tampering_is_detected() {
        let key = ProtocolKey::from_seed(&IDENTITY_SEED);
        let mut tit = Tit::issue_identity_signed(&key, claims());
        tit.claims.trust = 99.0;
        assert!(tit.verify(None, 1_700_000_100).is_err());
    }

    #[test]
    fn expiry_is_enforced() {
        let key = ProtocolKey::from_seed(&IDENTITY_SEED);
        let tit = Tit::issue_identity_signed(&key, claims());
        tit.verify(None, 1_700_000_000 + 3600).unwrap(); // 边界内
        assert!(tit.verify_freshness(1_700_000_000 + 3601).is_err());
    }

    #[test]
    fn unsigned_cannot_be_verified() {
        let tit = Tit::unsigned(claims());
        assert_eq!(tit.issuer, TitIssuer::Unsigned);
        assert!(tit.verify_signature(None).is_err());
        // 但编码/解析仍然可用
        assert_eq!(Tit::from_cbor(&tit.to_cbor()).unwrap(), tit);
        assert_eq!(Tit::from_base64url(&tit.to_base64url()).unwrap(), tit);
    }

    #[test]
    fn field_range_validation() {
        let key = ProtocolKey::from_seed(&IDENTITY_SEED);

        let mut too_many_cells = claims();
        too_many_cells.unique_cells = too_many_cells.breadcrumbs + 1;
        assert!(Tit::from_cbor(&Tit::unsigned(too_many_cells).to_cbor()).is_err());

        let mut bad_trust = claims();
        bad_trust.trust = 100.5;
        assert!(Tit::from_cbor(&Tit::unsigned(bad_trust).to_cbor()).is_err());

        // 版本不符
        let mut tit = Tit::issue_identity_signed(&key, claims());
        tit.version = 2;
        assert!(Tit::from_cbor(&tit.to_cbor()).is_err());
    }

    #[test]
    fn canonical_form_is_enforced() {
        let key = ProtocolKey::from_seed(&IDENTITY_SEED);
        let tit = Tit::issue_identity_signed(&key, claims());
        let mut bytes = tit.to_cbor();
        bytes.push(0x00); // 尾部多余字节
        assert!(Tit::from_cbor(&bytes).is_err());
    }

    #[test]
    fn handle_threshold() {
        let key = ProtocolKey::from_seed(&IDENTITY_SEED);
        let mut c = claims();
        c.breadcrumbs = 100;
        c.unique_cells = 50;
        c.trust = 20.0;
        assert!(Tit::issue_identity_signed(&key, c).meets_handle_threshold());

        let mut below = claims();
        below.breadcrumbs = 99;
        below.unique_cells = 50;
        below.trust = 20.0;
        assert!(!Tit::issue_identity_signed(&key, below).meets_handle_threshold());
    }

    #[test]
    fn encoding_is_deterministic() {
        let key = ProtocolKey::from_seed(&IDENTITY_SEED);
        let a = Tit::issue_identity_signed(&key, claims());
        let b = Tit::issue_identity_signed(&key, claims());
        assert_eq!(a.to_cbor(), b.to_cbor());
    }
}
