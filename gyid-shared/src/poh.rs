//! PoH 证书解析与策略检查（RP 侧）。
//!
//! [`verify_poh] 把 [`trip_core::PohCertificate`] 的 CBOR 解析 + 签名验证 +
//! 新鲜性（§10 §1/5/6）+ 策略（§10 §2–4）封装成一次调用，返回 [`PohInfo`]。
//!
//! `verifier_pubkey` 是 Verifier Ed25519 公钥（RP 从
//! `GET /.well-known/verifier.json` 拉到，见 [`crate::VerifierClient::fetch_verifier_pubkey_hex`]）。

use trip_core::poh::PohCertificate;

use crate::{GyidError, GyidResult};

/// 解析后的 PoH 信息（镜像 [`PohCertificate`] 字段 + 验证结果）。
#[derive(Debug, Clone)]
pub struct PohInfo {
    pub identity: [u8; 32],
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
    pub nonce: [u8; 16],
    pub chain_head: [u8; 32],
    pub verifier_signature: [u8; 64],
    /// `verify_freshness` 是否通过（签名有效 + 未过期 + nonce 匹配）。
    pub fresh: bool,
    /// `meets_policy` 是否通过（α/置信度/信任分达到门槛）。
    pub policy_pass: bool,
}

impl PohInfo {
    /// 整体是否可信：fresh 且 policy_pass。
    pub fn is_trusted(&self) -> bool {
        self.fresh && self.policy_pass
    }
}

/// 校验 PoH 证书：CBOR 解析 → Verifier 签名 → 新鲜性 → 策略。
///
/// 即使 `fresh=false` 或 `policy_pass=false`，函数仍返回 [`PohInfo`]，
/// 调用方可据此决定拒绝 / 接受 / 轮询续期。仅当 CBOR 无法解析时返回 `Err`。
pub fn verify_poh(
    poh_bytes: &[u8],
    verifier_pubkey: &[u8; 32],
    expected_nonce: &[u8; 16],
    now_unix: u64,
    min_confidence: f64,
    min_trust: f64,
) -> GyidResult<PohInfo> {
    let cert = PohCertificate::from_cbor(poh_bytes).map_err(GyidError::from)?;
    let fresh = cert
        .verify_freshness(verifier_pubkey, expected_nonce, now_unix)
        .is_ok();
    let policy_pass = cert.meets_policy(min_confidence, min_trust);
    let PohCertificate {
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
        verifier_signature,
    } = cert;
    Ok(PohInfo {
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
        verifier_signature,
        fresh,
        policy_pass,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use trip_core::ProtocolKey;

    fn fake_poh(verifier: &ProtocolKey, nonce: [u8; 16], chain_head: [u8; 32]) -> Vec<u8> {
        PohCertificate::issue(
            verifier,
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
            chain_head,
        )
        .to_cbor()
    }

    #[test]
    fn verify_valid_poh() {
        let verifier = ProtocolKey::from_seed(&[3u8; 32]);
        let vk = verifier.public_bytes();
        let nonce = [0xABu8; 16];
        let bytes = fake_poh(&verifier, nonce, [0xCD; 32]);

        let info = verify_poh(&bytes, &vk, &nonce, 1_700_000_100, 0.5, 20.0).unwrap();
        assert!(info.fresh);
        assert!(info.policy_pass);
        assert!(info.is_trusted());
    }

    #[test]
    fn reject_wrong_nonce() {
        let verifier = ProtocolKey::from_seed(&[3u8; 32]);
        let vk = verifier.public_bytes();
        let bytes = fake_poh(&verifier, [0xAB; 16], [0xCD; 32]);

        let info = verify_poh(&bytes, &vk, &[0; 16], 1_700_000_100, 0.5, 20.0).unwrap();
        assert!(!info.fresh);
        assert!(!info.is_trusted());
    }

    #[test]
    fn reject_expired() {
        let verifier = ProtocolKey::from_seed(&[3u8; 32]);
        let vk = verifier.public_bytes();
        let nonce = [0xABu8; 16];
        let bytes = fake_poh(&verifier, nonce, [0xCD; 32]);

        let info = verify_poh(&bytes, &vk, &nonce, 1_700_000_000 + 3601, 0.5, 20.0).unwrap();
        assert!(!info.fresh);
        assert!(!info.is_trusted());
    }

    #[test]
    fn reject_tampered_bytes() {
        let verifier = ProtocolKey::from_seed(&[3u8; 32]);
        let vk = verifier.public_bytes();
        let nonce = [0xABu8; 16];
        let mut bytes = fake_poh(&verifier, nonce, [0xCD; 32]);
        bytes[0] ^= 0x01; // 破坏 CBOR 头
        assert!(verify_poh(&bytes, &vk, &nonce, 1_700_000_100, 0.5, 20.0).is_err());
    }
}
