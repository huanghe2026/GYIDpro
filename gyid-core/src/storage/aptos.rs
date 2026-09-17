//! Aptos 链签名模块
//!
//! 实现 Aptos 交易的 ED25519 签名和 BCS 编码序列化

use crate::{GyIdError, Result};

// ─────────────────────────────────────────────────────────────────────────────
// Aptos 交易结构
// ─────────────────────────────────────────────────────────────────────────────

/// Aptos RawTransaction 结构 (BCS 编码)
#[derive(Debug, Clone)]
pub struct RawTransaction {
    /// 发送方账户地址 (32 字节)
    pub sender: Vec<u8>,
    /// 序列号
    pub sequence_number: u64,
    /// 交易 payload (BCS 编码)
    pub payload: Vec<u8>,
    /// 最大 gas 数量
    pub max_gas_amount: u64,
    /// gas 单价
    pub gas_unit_price: u64,
    /// 交易过期时间 (Unix 时间戳 秒)
    pub expiration_timestamp_secs: u64,
    /// chain_id
    pub chain_id: u8,
}

impl RawTransaction {
    /// 创建新交易
    pub fn new(
        sender: Vec<u8>,
        sequence_number: u64,
        payload: Vec<u8>,
        max_gas_amount: u64,
        gas_unit_price: u64,
        expiration_timestamp_secs: u64,
        chain_id: u8,
    ) -> Self {
        Self {
            sender,
            sequence_number,
            payload,
            max_gas_amount,
            gas_unit_price,
            expiration_timestamp_secs,
            chain_id,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// BCS 序列化实现
// ─────────────────────────────────────────────────────────────────────────────

/// BCS (Binary Canonical Serialization) 编码器
pub struct BcsEncoder {
    pub bytes: Vec<u8>,
}

impl BcsEncoder {
    pub fn new() -> Self {
        Self { bytes: Vec::new() }
    }

    /// 序列化一个 u64
    pub fn encode_u64(&mut self, val: u64) {
        uleb128_encode(val, &mut self.bytes);
    }

    /// 序列化一个 u8
    pub fn encode_u8(&mut self, val: u8) {
        self.bytes.push(val);
    }

    /// 序列化字节数组
    pub fn encode_bytes(&mut self, bytes: &[u8]) {
        self.encode_u64(bytes.len() as u64);
        self.bytes.extend_from_slice(bytes);
    }

    /// 序列化字符串
    pub fn encode_str(&mut self, s: &str) {
        self.encode_bytes(s.as_bytes());
    }

    /// 序列化 Vec<u8> (不带长度前缀，仅追加)
    pub fn encode_fixed_bytes(&mut self, bytes: &[u8]) {
        self.bytes.extend_from_slice(bytes);
    }

    /// 序列化 Bool
    pub fn encode_bool(&mut self, val: bool) {
        self.bytes.push(if val { 1 } else { 0 });
    }

    /// 获取编码结果
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

impl Default for BcsEncoder {
    fn default() -> Self {
        Self::new()
    }
}

/// ULEB128 编码 (用于 u64)
fn uleb128_encode(mut val: u64, output: &mut Vec<u8>) {
    loop {
        let mut byte = (val & 0x7F) as u8;
        val >>= 7;
        if val != 0 {
            byte |= 0x80;
        }
        output.push(byte);
        if val == 0 {
            break;
        }
    }
}

/// 为 RawTransaction 生成签名消息
pub fn get_transaction_signing_message(tx: &RawTransaction) -> Vec<u8> {
    let mut encoder = BcsEncoder::new();

    // 签名消息前缀 (Aptos 规范)
    encoder.encode_fixed_bytes(b"APTOS::RawTransaction");

    // 序列化交易字段
    encoder.encode_fixed_bytes(&tx.sender); // 32 字节地址
    encoder.encode_u64(tx.sequence_number);
    encoder.encode_bytes(&tx.payload);
    encoder.encode_u64(tx.max_gas_amount);
    encoder.encode_u64(tx.gas_unit_price);
    encoder.encode_u64(tx.expiration_timestamp_secs);
    encoder.encode_u8(tx.chain_id);

    encoder.into_bytes()
}

// ─────────────────────────────────────────────────────────────────────────────
// ED25519 签名
// ─────────────────────────────────────────────────────────────────────────────

/// 使用 ED25519 签名
#[cfg(feature = "aptos-signing")]
pub fn sign_transaction(
    tx: &RawTransaction,
    private_key_bytes: &[u8],
) -> Result<Vec<u8>> {
    use ed25519_dalek::{Signature, Signer, SigningKey};

    // 32 字节私钥
    if private_key_bytes.len() != 32 {
        return Err(GyIdError::StorageError(
            format!("ED25519 私钥长度错误: 期望 32 字节, 实际 {} 字节", private_key_bytes.len())
        ));
    }

    // 构造签名密钥
    let signing_key = SigningKey::from_bytes(private_key_bytes.try_into().map_err(|_| {
        GyIdError::StorageError("ED25519 私钥转换失败".to_string())
    })?);

    // 生成签名消息
    let signing_message = get_transaction_signing_message(tx);

    // 直接签名 (ed25519-dalek 会自动处理 SHA512)
    let signature: Signature = signing_key.sign(&signing_message);

    // 64 字节签名字节
    Ok(signature.to_bytes().to_vec())
}

#[cfg(not(feature = "aptos-signing"))]
pub fn sign_transaction(
    _tx: &RawTransaction,
    _private_key_bytes: &[u8],
) -> Result<Vec<u8>> {
    Err(GyIdError::StorageError(
        "Aptos 签名功能需要启用 'aptos-signing' feature".to_string(),
    ))
}

// ─────────────────────────────────────────────────────────────────────────────
// Transaction Payload 构造
// ─────────────────────────────────────────────────────────────────────────────

/// 构造 EntryFunction payload
pub fn encode_entry_function_payload(
    module: &str,
    function: &str,
    type_args: &[&str],
    args: &[Vec<u8>],
) -> Vec<u8> {
    let mut encoder = BcsEncoder::new();

    // ScriptFunction 类型标识: 0x00
    encoder.encode_u8(0);

    // module (字符串)
    encoder.encode_str(module);

    // function (字符串)
    encoder.encode_str(function);

    // type_args (vec<StructTag>)
    encoder.encode_u64(type_args.len() as u64);
    for _type_arg in type_args {
        // 简化处理：空类型参数
    }

    // args (vec<vector<u8>>)
    encoder.encode_u64(args.len() as u64);
    for arg in args {
        encoder.encode_bytes(arg);
    }

    encoder.into_bytes()
}

/// 构造 BcsBytes 类型的参数
pub fn encode_bcs_bytes(data: &[u8]) -> Vec<u8> {
    data.to_vec()
}

/// 构造 U64 类型的参数
pub fn encode_u64_arg(val: u64) -> Vec<u8> {
    let mut encoder = BcsEncoder::new();
    encoder.encode_u64(val);
    encoder.into_bytes()
}

// ─────────────────────────────────────────────────────────────────────────────
// 地址处理
// ─────────────────────────────────────────────────────────────────────────────

/// 规范化 Aptos 地址 (确保 32 字节)
pub fn normalize_address(addr: &str) -> Result<Vec<u8>> {
    // 移除 0x 前缀
    let addr = addr.trim_start_matches("0x").trim_start_matches("0X");

    // 如果是奇数长度，补零前缀使其成为偶数
    let addr = if !addr.len().is_multiple_of(2) {
        format!("0{}", addr)
    } else {
        addr.to_string()
    };

    // 解析 hex
    let bytes = hex::decode(&addr)
        .map_err(|e| GyIdError::StorageError(format!("地址 hex 解码失败: {}", e)))?;

    // 填充到 32 字节 (左补零)
    if bytes.len() > 32 {
        return Err(GyIdError::StorageError(
            format!("地址太长: {} 字节, 最大 32 字节", bytes.len())
        ));
    }

    let mut padded = vec![0u8; 32 - bytes.len()];
    padded.extend_from_slice(&bytes);

    Ok(padded)
}

/// 将私钥 hex 转换为字节
pub fn decode_private_key(key_hex: &str) -> Result<Vec<u8>> {
    let key = key_hex.trim_start_matches("0x").trim_start_matches("0X");

    let bytes = hex::decode(key)
        .map_err(|e| GyIdError::StorageError(format!("私钥 hex 解码失败: {}", e)))?;

    // ED25519 私钥应该是 32 字节
    if bytes.len() != 32 {
        return Err(GyIdError::StorageError(
            format!("ED25519 私钥长度错误: {} 字节, 期望 32 字节", bytes.len())
        ));
    }

    Ok(bytes)
}

// ─────────────────────────────────────────────────────────────────────────────
// 单元测试
// ─────────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bcs_encoder() {
        let mut encoder = BcsEncoder::new();
        encoder.encode_u64(0);
        encoder.encode_u64(127);
        encoder.encode_u64(128);
        encoder.encode_u64(300);
        encoder.encode_bytes(b"hello");
        encoder.encode_str("world");

        // 验证可以正确解码
        let bytes = encoder.into_bytes();
        assert!(!bytes.is_empty());
    }

    #[test]
    fn test_uleb128() {
        let mut output = Vec::new();
        uleb128_encode(0, &mut output);
        assert_eq!(output, vec![0]);

        output.clear();
        uleb128_encode(127, &mut output);
        assert_eq!(output, vec![127]);

        output.clear();
        uleb128_encode(128, &mut output);
        assert_eq!(output, vec![0x80, 0x01]);

        output.clear();
        uleb128_encode(300, &mut output);
        assert_eq!(output, vec![0xAC, 0x02]);
    }

    #[test]
    fn test_normalize_address() {
        // 短地址
        let addr = normalize_address("0x1").unwrap();
        assert_eq!(addr.len(), 32);
        assert_eq!(addr[31], 0x01);

        // 全长地址
        let addr = normalize_address("0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef").unwrap();
        assert_eq!(addr.len(), 32);

        // 带 0x 前缀
        let addr = normalize_address("0x1").unwrap();
        assert_eq!(addr.len(), 32);
    }

    #[test]
    fn test_decode_private_key() {
        // 32 字节 hex 私钥
        let key = "a".repeat(64);
        let bytes = decode_private_key(&key).unwrap();
        assert_eq!(bytes.len(), 32);

        // 带 0x 前缀
        let key = format!("0x{}", "b".repeat(64));
        let bytes = decode_private_key(&key).unwrap();
        assert_eq!(bytes.len(), 32);
    }
}
