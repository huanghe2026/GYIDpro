//! §12 Active Verification Attester 端响应签名。
//!
//! Attester 收到 Verifier 经实时通道（WebSocket/推送）下发的
//! [`LivenessChallenge`] 后，对
//! `challenge_id || rp_nonce || chain_head || index` 用身份密钥签名，
//! 回送 [`LivenessResponse`]。Verifier 校验通过才签发绑定该 nonce 的 PoH。
//!
//! 本模块只封装签名；WebSocket 传输由各端原生实现（浏览器用原生 WebSocket，
//! CLI 用 tokio-tungstenite，Android 用 OkHttp）。

use trip_core::liveness::{LivenessChallenge, LivenessResponse};

use crate::identity::Identity;
use crate::GyidResult;

/// Attester 用身份密钥对挑战回送签名响应。
///
/// 等价于 `LivenessResponse::for_challenge(challenge)` + `sign(identity)`。
/// 调用方负责把响应经实时通道回传 Verifier（见 trip-server `challenge.rs`）。
pub fn sign_liveness_response(
    identity: &Identity,
    challenge: &LivenessChallenge,
) -> GyidResult<LivenessResponse> {
    let mut resp = LivenessResponse::for_challenge(challenge);
    resp.sign(&identity.protocol_key());
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake_id() -> Identity {
        Identity::from_seed_hex(&"07".repeat(32)).unwrap()
    }

    #[test]
    fn sign_and_verify_roundtrip() {
        let id = fake_id();
        let pk = id.protocol_key().public_bytes();
        let challenge =
            LivenessChallenge::new([0xAB; 16], [0xCD; 16], [0xEF; 32], 7, 1_700_000_060);
        let resp = sign_liveness_response(&id, &challenge).unwrap();

        assert!(resp.matches_challenge(&challenge));
        resp.verify_signature(&pk).unwrap();
    }

    #[test]
    fn wrong_key_rejected() {
        let id = fake_id();
        let other_pk = Identity::from_seed_hex(&"09".repeat(32))
            .unwrap()
            .protocol_key()
            .public_bytes();
        let challenge = LivenessChallenge::new([0; 16], [0; 16], [0; 32], 0, 0);
        let resp = sign_liveness_response(&id, &challenge).unwrap();
        assert!(resp.verify_signature(&other_pk).is_err());
    }
}
