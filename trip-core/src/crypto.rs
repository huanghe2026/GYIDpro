//! Ed25519（RFC 8032）身份与 Verifier 密钥封装。
//!
//! TRIP 中同一把 Ed25519 密钥承担：
//! - Attester：面包屑 / epoch / Liveness Response 签名（TIT = 公钥 + 轨迹元数据）；
//! - Verifier：PoH Certificate 签名（draft-04 §2.4）。

use ed25519_dalek::{Signature, Signer, SigningKey, VerifyingKey};

use crate::error::{Result, TripError};

/// Ed25519 签名/验证密钥（身份密钥或 Verifier 密钥）。
///
/// 私钥字节永不离开持有方；trip-core 不提供任何序列化私钥的方法，
/// 助记词/密钥保管属于 attester 层职责。
#[derive(Debug)]
pub struct ProtocolKey(SigningKey);

impl ProtocolKey {
    /// 从 32 字节种子确定性构造（黄金向量与助记词派生使用）。
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        Self(SigningKey::from_bytes(seed))
    }

    /// 使用操作系统 CSPRNG 生成新密钥。
    pub fn generate() -> Self {
        Self(SigningKey::generate(&mut rand_core::OsRng))
    }

    /// 返回 32 字节公钥（进入面包屑字段 1 / PoH 字段 0 / DID）。
    pub fn public_bytes(&self) -> [u8; 32] {
        self.0.verifying_key().to_bytes()
    }

    /// 对任意消息签名，返回 64 字节签名。
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        let sig: Signature = self.0.sign(message);
        sig.to_bytes()
    }
}

/// 用公钥验证一条 Ed25519 签名。
pub fn verify(public_key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> Result<()> {
    let vk = VerifyingKey::from_bytes(public_key).map_err(|_| TripError::InvalidSignature)?;
    let sig = Signature::from_bytes(signature);
    vk.verify_strict(message, &sig)
        .map_err(|_| TripError::InvalidSignature)
}
