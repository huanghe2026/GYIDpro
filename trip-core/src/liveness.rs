//! Active Verification 实时性消息（draft-04 §12）。
//!
//! Active Verification 是 TRIP 唯一主推的模式：RP 生成 16 字节 nonce，
//! Verifier 通过实时通道（WebSocket/推送）向 Attester 下发挑战，Attester
//! 对 `challenge_id || rp_nonce || chain_head || index` 签名后回送，
//! Verifier 校验通过才签发绑定该 nonce 的 PoH 证书。
//!
//! 本模块只定义线上协议消息的确定性 CBOR 编码与签名/验签，与具体传输
//!（HTTP/WebSocket）无关；传输绑定由 trip-server 负责。
//!
//! ## LivenessChallenge（Verifier → Attester）
//!
//! CBOR map，5 个字段（无签名侧；其真实性由承载通道与后续 PoH 绑定保证）：
//!
//! | key | 含义 | 类型 |
//! |-----|------|------|
//! | 0 | challenge id（UUID v4 字节） | bstr(16) |
//! | 1 | RP nonce | bstr(16) |
//! | 2 | Verifier 已知的 attester 链头 | bstr(32) |
//! | 3 | 期望响应绑定的面包屑 index | uint |
//! | 4 | 挑战截止时间 Unix 秒 | uint |
//!
//! ## LivenessResponse（Attester → Verifier）
//!
//! CBOR map，5 个字段：
//!
//! | key | 含义 | 类型 |
//! |-----|------|------|
//! | 0 | challenge id（回拷，路由绑定） | bstr(16) |
//! | 1 | RP nonce（回拷） | bstr(16) |
//! | 2 | 链头块哈希（回拷） | bstr(32) |
//! | 3 | 响应绑定的面包屑 index | uint |
//! | 4 | Attester Ed25519 签名（64） | bstr(64) |
//!
//! 签名覆盖字段 0..=3 的确定性 CBOR（map 头计 4 个字段）。把 `challenge_id`
//! 纳入签名域可防止把某一挑战的响应重放到另一挑战。

use crate::breadcrumb::{fixed32, fixed64};
use crate::cbor::{parse_all, Writer};
use crate::crypto::{self, ProtocolKey};
use crate::error::{Result, TripError};

/// Verifier 经实时通道下发给 Attester 的活体挑战。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LivenessChallenge {
    /// 字段 0：挑战 id。
    pub challenge_id: [u8; 16],
    /// 字段 1：RP 提供的 nonce。
    pub rp_nonce: [u8; 16],
    /// 字段 2：签发时 Verifier 已知的链头块哈希。
    pub chain_head: [u8; 32],
    /// 字段 3：期望 Attester 响应绑定的面包屑 index。
    pub expected_index: u64,
    /// 字段 4：挑战截止时间（Unix 秒）。
    pub deadline: u64,
}

impl LivenessChallenge {
    /// 构造挑战。
    #[allow(clippy::too_many_arguments)] // 字段对应草案 §12 的挑战载荷
    pub fn new(
        challenge_id: [u8; 16],
        rp_nonce: [u8; 16],
        chain_head: [u8; 32],
        expected_index: u64,
        deadline: u64,
    ) -> Self {
        Self {
            challenge_id,
            rp_nonce,
            chain_head,
            expected_index,
            deadline,
        }
    }

    /// 挑战在给定 Unix 秒是否已过期。
    pub fn is_expired(&self, now_unix: u64) -> bool {
        now_unix > self.deadline
    }

    /// 完整确定性 CBOR。
    pub fn to_cbor(&self) -> Vec<u8> {
        let mut w = Writer::with_capacity(96);
        w.map(5);
        w.uint(0).bstr(&self.challenge_id);
        w.uint(1).bstr(&self.rp_nonce);
        w.uint(2).bstr(&self.chain_head);
        w.uint(3).uint(self.expected_index);
        w.uint(4).uint(self.deadline);
        w.into_bytes()
    }

    /// 从 CBOR 解析（带规范化自检）。
    pub fn from_cbor(bytes: &[u8]) -> Result<Self> {
        let root = parse_all(bytes)?;
        let challenge_id = root
            .field(0)?
            .as_bstr()?
            .try_into()
            .map_err(|_| TripError::Cbor("challenge_id must be 16 bytes".into()))?;
        let rp_nonce = root
            .field(1)?
            .as_bstr()?
            .try_into()
            .map_err(|_| TripError::Cbor("rp_nonce must be 16 bytes".into()))?;
        let chain_head = fixed32(root.field(2)?.as_bstr()?)?;
        let v = Self {
            challenge_id,
            rp_nonce,
            chain_head,
            expected_index: root.field(3)?.as_uint()?,
            deadline: root.field(4)?.as_uint()?,
        };
        if v.to_cbor() != bytes {
            return Err(TripError::Cbor(
                "liveness challenge cbor is not in canonical deterministic form".into(),
            ));
        }
        Ok(v)
    }
}

/// Attester 回送 Verifier 的活体响应（对挑战的签名应答）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LivenessResponse {
    /// 字段 0：回拷的 challenge id。
    pub challenge_id: [u8; 16],
    /// 字段 1：回拷的 RP nonce。
    pub rp_nonce: [u8; 16],
    /// 字段 2：回拷的链头块哈希。
    pub chain_head: [u8; 32],
    /// 字段 3：响应绑定的面包屑 index。
    pub index: u64,
    /// 字段 4：Attester 对字段 0..=3 的 Ed25519 签名。
    pub attester_signature: [u8; 64],
}

impl LivenessResponse {
    /// 构造尚未签名的响应（字段 4 置零）。
    pub fn new_unsigned(
        challenge_id: [u8; 16],
        rp_nonce: [u8; 16],
        chain_head: [u8; 32],
        index: u64,
    ) -> Self {
        Self {
            challenge_id,
            rp_nonce,
            chain_head,
            index,
            attester_signature: [0u8; 64],
        }
    }

    /// 由挑战构造待签名响应（index 取挑战的 expected_index）。
    pub fn for_challenge(challenge: &LivenessChallenge) -> Self {
        Self::new_unsigned(
            challenge.challenge_id,
            challenge.rp_nonce,
            challenge.chain_head,
            challenge.expected_index,
        )
    }

    /// 字段 0..=3 的确定性 CBOR（签名输入，map 头计 4 个字段）。
    pub fn signable_bytes(&self) -> Vec<u8> {
        let mut w = Writer::with_capacity(90);
        w.map(4);
        w.uint(0).bstr(&self.challenge_id);
        w.uint(1).bstr(&self.rp_nonce);
        w.uint(2).bstr(&self.chain_head);
        w.uint(3).uint(self.index);
        w.into_bytes()
    }

    /// 用 Attester 身份密钥签名字段 0..=3。
    pub fn sign(&mut self, attester_key: &ProtocolKey) {
        self.attester_signature = attester_key.sign(&self.signable_bytes());
    }

    /// 完整确定性 CBOR（含字段 4 签名）。
    pub fn to_cbor(&self) -> Vec<u8> {
        let mut w = Writer::with_capacity(160);
        w.map(5);
        w.uint(0).bstr(&self.challenge_id);
        w.uint(1).bstr(&self.rp_nonce);
        w.uint(2).bstr(&self.chain_head);
        w.uint(3).uint(self.index);
        w.uint(4).bstr(&self.attester_signature);
        w.into_bytes()
    }

    /// 用 Attester 公钥验证签名（不校验业务字段是否匹配某挑战——
    /// 业务匹配由 Verifier 另行比对 challenge_id/index/chain_head）。
    pub fn verify_signature(&self, attester_public_key: &[u8; 32]) -> Result<()> {
        crypto::verify(
            attester_public_key,
            &self.signable_bytes(),
            &self.attester_signature,
        )
    }

    /// 校验响应与挑战严格匹配（id/nonce/链头/index 四元组全等）。
    pub fn matches_challenge(&self, challenge: &LivenessChallenge) -> bool {
        self.challenge_id == challenge.challenge_id
            && self.rp_nonce == challenge.rp_nonce
            && self.chain_head == challenge.chain_head
            && self.index == challenge.expected_index
    }

    /// 从 CBOR 解析（带规范化自检）。
    pub fn from_cbor(bytes: &[u8]) -> Result<Self> {
        let root = parse_all(bytes)?;
        let v = Self {
            challenge_id: root
                .field(0)?
                .as_bstr()?
                .try_into()
                .map_err(|_| TripError::Cbor("challenge_id must be 16 bytes".into()))?,
            rp_nonce: root
                .field(1)?
                .as_bstr()?
                .try_into()
                .map_err(|_| TripError::Cbor("rp_nonce must be 16 bytes".into()))?,
            chain_head: fixed32(root.field(2)?.as_bstr()?)?,
            index: root.field(3)?.as_uint()?,
            attester_signature: fixed64(root.field(4)?.as_bstr()?)?,
        };
        if v.to_cbor() != bytes {
            return Err(TripError::Cbor(
                "liveness response cbor is not in canonical deterministic form".into(),
            ));
        }
        Ok(v)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn challenge_cbor_roundtrip() {
        let c = LivenessChallenge::new([1u8; 16], [2u8; 16], [3u8; 32], 512, 1_700_000_060);
        let bytes = c.to_cbor();
        assert_eq!(LivenessChallenge::from_cbor(&bytes).unwrap(), c);
    }

    #[test]
    fn challenge_deadline() {
        let c = LivenessChallenge::new([0; 16], [0; 16], [0; 32], 0, 1000);
        assert!(!c.is_expired(1000));
        assert!(c.is_expired(1001));
    }

    #[test]
    fn response_sign_verify_and_match() {
        let attester = ProtocolKey::from_seed(&[9u8; 32]);
        let pk = attester.public_bytes();
        let challenge =
            LivenessChallenge::new([0xAB; 16], [0xCD; 16], [0xEF; 32], 512, 1_700_000_060);

        let mut resp = LivenessResponse::for_challenge(&challenge);
        resp.sign(&attester);
        assert!(resp.matches_challenge(&challenge));
        resp.verify_signature(&pk).unwrap();

        // CBOR roundtrip 后签名仍有效。
        let parsed = LivenessResponse::from_cbor(&resp.to_cbor()).unwrap();
        assert_eq!(parsed, resp);
        parsed.verify_signature(&pk).unwrap();
    }

    #[test]
    fn response_wrong_key_rejected() {
        let attester = ProtocolKey::from_seed(&[9u8; 32]);
        let other_pk = ProtocolKey::from_seed(&[10u8; 32]).public_bytes();
        let challenge = LivenessChallenge::new([0; 16], [0; 16], [0; 32], 1, 1000);
        let mut resp = LivenessResponse::for_challenge(&challenge);
        resp.sign(&attester);
        assert!(resp.verify_signature(&other_pk).is_err());
    }

    #[test]
    fn response_cross_challenge_rejected() {
        let attester = ProtocolKey::from_seed(&[9u8; 32]);
        let c1 = LivenessChallenge::new([1; 16], [2; 16], [3; 32], 10, 1000);
        let c2 = LivenessChallenge::new([4; 16], [2; 16], [3; 32], 10, 1000); // 不同 challenge_id
        let mut resp = LivenessResponse::for_challenge(&c1);
        resp.sign(&attester);
        // 签名对 c1 有效，但不得匹配 c2（防跨挑战重放）。
        assert!(resp.matches_challenge(&c1));
        assert!(!resp.matches_challenge(&c2));
    }

    #[test]
    fn tampered_response_rejected() {
        let attester = ProtocolKey::from_seed(&[9u8; 32]);
        let pk = attester.public_bytes();
        let challenge = LivenessChallenge::new([0; 16], [0; 16], [0; 32], 1, 1000);
        let mut resp = LivenessResponse::for_challenge(&challenge);
        resp.sign(&attester);
        resp.index = 2; // 改了 index 但没重签
        assert!(resp.verify_signature(&pk).is_err());
    }
}
