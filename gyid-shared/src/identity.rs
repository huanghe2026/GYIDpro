//! 身份密钥管理。
//!
//! - [`Identity`] 包装 32 字节 Ed25519 seed + 标签；
//! - [`Identity::generate`] 真随机生成新 seed（OsRng）；
//! - 在 feature `encrypt` 下，`to_encrypted_json` / `from_encrypted_json`
//!   用 PBKDF2(SHA-256, 250k iters) 从 passphrase 派生 AES-256-GCM key，
//!   对 seed 加密后 base64 落盘 JSON。salt 与 nonce 均随机，同一 passphrase
//!   每次输出不同。

use rand_core::{OsRng, RngCore};
use trip_core::ProtocolKey;

use crate::{GyidError, GyidResult};

/// 一份 GyID 身份（Ed25519 seed + 标签）。
#[derive(Debug, Clone)]
pub struct Identity {
    pub seed: [u8; 32],
    pub label: String,
}

impl Identity {
    /// 真随机生成新身份（OsRng）。
    pub fn generate() -> Self {
        let mut seed = [0u8; 32];
        OsRng.fill_bytes(&mut seed);
        Self {
            seed,
            label: "default".to_string(),
        }
    }

    /// 从 hex seed（64 字符）构造。
    pub fn from_seed_hex(hex_seed: &str) -> GyidResult<Self> {
        let bytes = hex::decode(hex_seed.trim())
            .map_err(|e| GyidError::Hex(format!("seed hex decode: {e}")))?;
        let seed: [u8; 32] = bytes
            .as_slice()
            .try_into()
            .map_err(|_| GyidError::Hex("seed must be exactly 32 bytes".into()))?;
        Ok(Self {
            seed,
            label: "default".to_string(),
        })
    }

    /// 派生协议密钥句柄。
    pub fn protocol_key(&self) -> ProtocolKey {
        ProtocolKey::from_seed(&self.seed)
    }

    /// 公钥 hex（64 字符）。
    pub fn pubkey_hex(&self) -> String {
        hex::encode(self.protocol_key().public_bytes())
    }

    /// seed hex（64 字符）。仅供本地存储/调试，不要外发。
    pub fn seed_hex(&self) -> String {
        hex::encode(self.seed)
    }
}

#[cfg(feature = "encrypt")]
mod encrypted {
    use super::*;
    use aes_gcm::aead::{Aead, KeyInit};
    use aes_gcm::{Aes256Gcm, Key, Nonce};
    use base64::Engine;
    use pbkdf2::pbkdf2_hmac;
    use serde::{Deserialize, Serialize};
    use sha2::Sha256;

    const PBKDF2_ITERS: u32 = 250_000;
    const SALT_LEN: usize = 16;
    const NONCE_LEN: usize = 12;
    const KEY_LEN: usize = 32;

    #[derive(Debug, Serialize, Deserialize)]
    struct EncryptedJson {
        v: u32,
        label: String,
        salt: String,
        nonce: String,
        ciphertext: String,
    }

    impl Identity {
        /// 用 passphrase 加密 seed 后序列化为 JSON 字符串。
        /// 同一 passphrase 每次输出不同（salt 与 nonce 均随机）。
        pub fn to_encrypted_json(&self, passphrase: &str) -> GyidResult<String> {
            let mut salt = [0u8; SALT_LEN];
            let mut nonce = [0u8; NONCE_LEN];
            OsRng.fill_bytes(&mut salt);
            OsRng.fill_bytes(&mut nonce);

            let mut key_bytes = [0u8; KEY_LEN];
            pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), &salt, PBKDF2_ITERS, &mut key_bytes);
            let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
            let cipher = Aes256Gcm::new(key);

            let ciphertext = cipher
                .encrypt(Nonce::from_slice(&nonce), self.seed.as_ref())
                .map_err(|e| GyidError::Crypto(format!("aes-gcm encrypt: {e}")))?;

            let b64 = base64::engine::general_purpose::STANDARD;
            let enc = EncryptedJson {
                v: 1,
                label: self.label.clone(),
                salt: b64.encode(salt),
                nonce: b64.encode(nonce),
                ciphertext: b64.encode(&ciphertext),
            };
            serde_json::to_string(&enc)
                .map_err(|e| GyidError::Crypto(format!("json serialize: {e}")))
        }

        /// 用 passphrase 解密之前 `to_encrypted_json` 输出的 JSON。
        pub fn from_encrypted_json(json: &str, passphrase: &str) -> GyidResult<Self> {
            let enc: EncryptedJson = serde_json::from_str(json)
                .map_err(|e| GyidError::Crypto(format!("json parse: {e}")))?;
            if enc.v != 1 {
                return Err(GyidError::Crypto(format!("unsupported version {}", enc.v)));
            }
            let b64 = base64::engine::general_purpose::STANDARD;
            let salt = b64
                .decode(&enc.salt)
                .map_err(|e| GyidError::Crypto(format!("salt b64: {e}")))?;
            let nonce = b64
                .decode(&enc.nonce)
                .map_err(|e| GyidError::Crypto(format!("nonce b64: {e}")))?;
            let ciphertext = b64
                .decode(&enc.ciphertext)
                .map_err(|e| GyidError::Crypto(format!("ct b64: {e}")))?;

            let mut key_bytes = [0u8; KEY_LEN];
            pbkdf2_hmac::<Sha256>(passphrase.as_bytes(), &salt, PBKDF2_ITERS, &mut key_bytes);
            let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
            let cipher = Aes256Gcm::new(key);

            let plain = cipher
                .decrypt(Nonce::from_slice(&nonce), ciphertext.as_ref())
                .map_err(|e| GyidError::Crypto(format!("aes-gcm decrypt: {e}")))?;
            if plain.len() != 32 {
                return Err(GyidError::Crypto(format!(
                    "decrypted seed length {} != 32",
                    plain.len()
                )));
            }
            let mut seed = [0u8; 32];
            seed.copy_from_slice(&plain);
            Ok(Self {
                seed,
                label: enc.label,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generate_and_roundtrip() {
        let id = Identity::generate();
        let key = id.protocol_key();
        let id2 = Identity::from_seed_hex(&id.seed_hex()).unwrap();
        assert_eq!(key.public_bytes(), id2.protocol_key().public_bytes());
    }

    #[test]
    fn from_seed_hex_known() {
        let id = Identity::from_seed_hex("07".repeat(32).as_str()).unwrap();
        assert_eq!(id.seed[0], 0x07);
        assert_eq!(id.pubkey_hex().len(), 64);
    }

    #[test]
    fn from_seed_hex_bad_length() {
        assert!(Identity::from_seed_hex("abcd").is_err());
        assert!(Identity::from_seed_hex(&"07".repeat(31)).is_err());
        assert!(Identity::from_seed_hex(&"07".repeat(33)).is_err());
    }

    #[cfg(feature = "encrypt")]
    #[test]
    fn encrypt_decrypt_roundtrip() {
        let id = Identity::generate();
        let enc = id.to_encrypted_json("hunter2hunter2").unwrap();
        let dec = Identity::from_encrypted_json(&enc, "hunter2hunter2").unwrap();
        assert_eq!(id.seed, dec.seed);
        assert_eq!(id.label, dec.label);
    }

    #[cfg(feature = "encrypt")]
    #[test]
    fn encrypt_wrong_passphrase_rejected() {
        let id = Identity::generate();
        let enc = id.to_encrypted_json("correct-pass").unwrap();
        assert!(Identity::from_encrypted_json(&enc, "wrong-pass").is_err());
    }

    #[cfg(feature = "encrypt")]
    #[test]
    fn encrypt_salt_nonce_random_per_call() {
        let id = Identity::generate();
        let e1 = id.to_encrypted_json("p").unwrap();
        let e2 = id.to_encrypted_json("p").unwrap();
        assert_ne!(e1, e2, "salt/nonce must differ per call");
    }
}
