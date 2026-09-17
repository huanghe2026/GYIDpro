//! 编码模块

use base58::{FromBase58, ToBase58};

/// Base58 编码器
pub struct Base58Encoder;

impl Base58Encoder {
    /// 将字节数组编码为 Base58 字符串
    pub fn encode(data: &[u8]) -> String {
        data.to_base58()
    }
    
    /// 将十六进制字符串编码为 Base58
    pub fn encode_hex(hex: &str) -> Result<String, String> {
        let bytes = hex::decode(hex).map_err(|e| e.to_string())?;
        Ok(Self::encode(&bytes))
    }
    
    /// 解码 Base58 字符串
    pub fn decode(data: &str) -> Result<Vec<u8>, String> {
        data.from_base58().map_err(|e| format!("Base58 解码错误: {:?}", e))
    }
    
    /// 解码为十六进制字符串
    pub fn decode_hex(data: &str) -> Result<String, String> {
        let bytes = Self::decode(data)?;
        Ok(hex::encode(&bytes))
    }
    
    /// 编码并截取指定长度
    pub fn encode_truncated(data: &[u8], len: usize) -> String {
        let encoded = Self::encode(data);
        if encoded.len() > len {
            encoded[..len].to_string()
        } else {
            encoded
        }
    }
}

/// 十六进制编码器
#[allow(dead_code)]
pub struct HexEncoder;

#[allow(dead_code)]
impl HexEncoder {
    /// 编码为十六进制字符串
    pub fn encode(data: &[u8]) -> String {
        hex::encode(data)
    }
    
    /// 解码十六进制字符串
    pub fn decode(hex: &str) -> Result<Vec<u8>, String> {
        hex::decode(hex).map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base58() {
        let data = b"Hello, GyID!";
        let encoded = Base58Encoder::encode(data);
        println!("Encoded: {}", encoded);
        
        let decoded = Base58Encoder::decode(&encoded).unwrap();
        assert_eq!(decoded.as_slice(), data);
    }
    
    #[test]
    fn test_hex() {
        let data = b"Test";
        let encoded = HexEncoder::encode(data);
        println!("Hex: {}", encoded);
        assert_eq!(encoded, "54657374");
    }
}
