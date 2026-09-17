//! Proof-of-Humanity Certificate（draft-04 §10）。
//!
//! Verifier 签发的**唯一**对外证明：只含统计指数与聚合计数，
//! 不含任何原始位置、cell 或时间序列。所有证书必须经 Active Verification
//! 绑定 RP nonce 与链头哈希（§12），因此字段 12/13 为必填。
//!
//! CBOR 字段（key 0..=14）：
//!
//! | key | 含义 | 类型 |
//! |-----|------|------|
//! | 0 | 身份公钥 | bstr(32) |
//! | 1 | 签发时间 Unix 秒 | uint |
//! | 2 | 签发时 epoch 数 | uint |
//! | 3 | PSD 标度指数 α | float |
//! | 4 | Levy β 指数 | float |
//! | 5 | Levy κ 截断距离（km） | float |
//! | 6 | 可预测性 Π | float |
//! | 7 | 临界置信度 | float |
//! | 8 | 信任分 T（[0,100]） | float |
//! | 9 | unique cell 数 | uint |
//! | 10 | 面包屑总数 | uint |
//! | 11 | 有效期（秒） | uint |
//! | 12 | RP nonce（必填） | bstr(16) |
//! | 13 | 签发时链头哈希（必填） | bstr(32) |
//! | 14 | Verifier Ed25519 签名（64） | bstr(64) |

use crate::breadcrumb::{fixed32, fixed64};
use crate::cbor::{parse_all, Writer};
use crate::crypto::{self, ProtocolKey};
use crate::error::{Result, TripError};

// §7.1 生物区间的协议常量以 engine::psd 为唯一事实来源，此处保持原有
// 公开名称作为别名，避免破坏既有使用方。
pub use crate::engine::psd::{
    BIO_ALPHA_MAX as BIOLOGICAL_ALPHA_MAX, BIO_ALPHA_MIN as BIOLOGICAL_ALPHA_MIN,
};

/// PoH 证书签发输入（字段 0..=13，签名前）。
#[derive(Debug, Clone, PartialEq)]
pub struct PohCertificate {
    /// 字段 0：被证明身份的公钥。
    pub identity: [u8; 32],
    /// 字段 1：签发时间。
    pub issued_at: u64,
    /// 字段 2：截至签发时的 epoch 数。
    pub epoch_count: u64,
    /// 字段 3：PSD α。
    pub alpha: f64,
    /// 字段 4：Levy β。
    pub beta: f64,
    /// 字段 5：Levy κ。
    pub kappa: f64,
    /// 字段 6：可预测性 Π。
    pub pi: f64,
    /// 字段 7：临界置信度。
    pub criticality_confidence: f64,
    /// 字段 8：信任分（百分比）。
    pub trust: f64,
    /// 字段 9：unique cell 数。
    pub unique_cells: u64,
    /// 字段 10：面包屑总数。
    pub breadcrumb_count: u64,
    /// 字段 11：有效期秒数。
    pub validity_secs: u64,
    /// 字段 12：RP nonce。
    pub nonce: [u8; 16],
    /// 字段 13：签发时链头块哈希。
    pub chain_head: [u8; 32],
    /// 字段 14：Verifier 签名。
    pub verifier_signature: [u8; 64],
}

impl PohCertificate {
    /// 由 Verifier 签发证书（自动填字段 14）。
    #[allow(clippy::too_many_arguments)] // 参数对应草案 §10 Table 的 15 个证书字段
    pub fn issue(
        verifier: &ProtocolKey,
        identity: [u8; 32],
        issued_at: u64,
        epoch_count: u64,
        alpha: f64,
        beta: f64,
        kappa: f64,
        pi: f64,
        criticality_confidence: f64,
        trust: f64,
        unique_cells: u64,
        breadcrumb_count: u64,
        validity_secs: u64,
        nonce: [u8; 16],
        chain_head: [u8; 32],
    ) -> Self {
        let mut cert = Self {
            identity,
            issued_at,
            epoch_count,
            alpha,
            beta,
            kappa,
            pi,
            criticality_confidence,
            trust,
            unique_cells,
            breadcrumb_count,
            validity_secs,
            nonce,
            chain_head,
            verifier_signature: [0u8; 64],
        };
        cert.verifier_signature = verifier.sign(&cert.signable_bytes());
        cert
    }

    fn write_fields(&self, w: &mut Writer) {
        w.uint(0).bstr(&self.identity);
        w.uint(1).uint(self.issued_at);
        w.uint(2).uint(self.epoch_count);
        w.uint(3).f64(self.alpha);
        w.uint(4).f64(self.beta);
        w.uint(5).f64(self.kappa);
        w.uint(6).f64(self.pi);
        w.uint(7).f64(self.criticality_confidence);
        w.uint(8).f64(self.trust);
        w.uint(9).uint(self.unique_cells);
        w.uint(10).uint(self.breadcrumb_count);
        w.uint(11).uint(self.validity_secs);
        w.uint(12).bstr(&self.nonce);
        w.uint(13).bstr(&self.chain_head);
    }

    /// 字段 0..=13 的确定性 CBOR（签名输入）。
    pub fn signable_bytes(&self) -> Vec<u8> {
        let mut w = Writer::with_capacity(220);
        w.map(14);
        self.write_fields(&mut w);
        w.into_bytes()
    }

    /// 字段 0..=14 的完整 CBOR。
    pub fn to_cbor(&self) -> Vec<u8> {
        let mut w = Writer::with_capacity(290);
        w.map(15);
        self.write_fields(&mut w);
        w.uint(14).bstr(&self.verifier_signature);
        w.into_bytes()
    }

    /// 检查 §10 RP 侧的密码学与新鲜性条件：
    ///
    /// 1. Verifier 签名有效；
    /// 5. 当前时间未超过 `issued_at + validity_secs`；
    /// 6. nonce 与 RP 原始挑战一致。
    ///
    /// 统计阈值（α/置信度/T）属于 RP 策略，用
    /// [`PohCertificate::meets_policy`] 单独判定。
    pub fn verify_freshness(
        &self,
        verifier_public_key: &[u8; 32],
        expected_nonce: &[u8; 16],
        now_unix: u64,
    ) -> Result<()> {
        crypto::verify(
            verifier_public_key,
            &self.signable_bytes(),
            &self.verifier_signature,
        )?;
        if &self.nonce != expected_nonce {
            return Err(TripError::InvalidSignature);
        }
        if now_unix > self.issued_at.saturating_add(self.validity_secs) {
            return Err(TripError::InvalidChain("PoH certificate expired".into()));
        }
        Ok(())
    }

    /// RP 策略检查：α 落在生物区间且置信度/信任分达到门槛（§10 步骤 2–4）。
    pub fn meets_policy(&self, min_confidence: f64, min_trust: f64) -> bool {
        (BIOLOGICAL_ALPHA_MIN..=BIOLOGICAL_ALPHA_MAX).contains(&self.alpha)
            && self.criticality_confidence >= min_confidence
            && self.trust >= min_trust
    }

    /// 从完整 CBOR 解析（带规范化自检）。
    pub fn from_cbor(bytes: &[u8]) -> Result<Self> {
        let root = parse_all(bytes)?;
        let cert = Self {
            identity: fixed32(root.field(0)?.as_bstr()?)?,
            issued_at: root.field(1)?.as_uint()?,
            epoch_count: root.field(2)?.as_uint()?,
            alpha: root.field(3)?.as_float()?,
            beta: root.field(4)?.as_float()?,
            kappa: root.field(5)?.as_float()?,
            pi: root.field(6)?.as_float()?,
            criticality_confidence: root.field(7)?.as_float()?,
            trust: root.field(8)?.as_float()?,
            unique_cells: root.field(9)?.as_uint()?,
            breadcrumb_count: root.field(10)?.as_uint()?,
            validity_secs: root.field(11)?.as_uint()?,
            nonce: root
                .field(12)?
                .as_bstr()?
                .try_into()
                .map_err(|_| TripError::Cbor("nonce must be 16 bytes".into()))?,
            chain_head: fixed32(root.field(13)?.as_bstr()?)?,
            verifier_signature: fixed64(root.field(14)?.as_bstr()?)?,
        };
        if cert.to_cbor() != bytes {
            return Err(TripError::Cbor(
                "PoH certificate cbor is not in canonical deterministic form".into(),
            ));
        }
        Ok(cert)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issue_verify_roundtrip_and_policy() {
        let verifier = ProtocolKey::from_seed(&[3u8; 32]);
        let vk = verifier.public_bytes();
        let id = [4u8; 32];
        let nonce = [0xabu8; 16];
        let head = [0xcd; 32];

        let cert = PohCertificate::issue(
            &verifier,
            id,
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
            head,
        );

        cert.verify_freshness(&vk, &nonce, 1_700_000_100).unwrap();
        assert!(cert.meets_policy(0.5, 20.0));

        // nonce 不符 → 拒绝
        assert!(cert.verify_freshness(&vk, &[0; 16], 1_700_000_100).is_err());
        // 过期 → 拒绝
        assert!(cert
            .verify_freshness(&vk, &nonce, 1_700_000_000 + 3601)
            .is_err());
        // α 越界 → 策略不通过（签名本身仍有效）
        let mut bad = cert.clone();
        bad.alpha = 0.10;
        assert!(!bad.meets_policy(0.5, 20.0));
    }

    #[test]
    fn canonical_cbor_roundtrip() {
        let verifier = ProtocolKey::from_seed(&[5u8; 32]);
        let cert = PohCertificate::issue(
            &verifier, [6; 32], 123, 0, 0.55, 1.75, 10.0, 0.9, 0.8, 40.0, 10, 100, 600, [1; 16],
            [2; 32],
        );
        let bytes = cert.to_cbor();
        let parsed = PohCertificate::from_cbor(&bytes).unwrap();
        assert_eq!(parsed, cert);
    }
}
