//! GyID 验证器

use crate::{identity::GyId, Result, GyIdError};

/// GyID 验证器
pub struct GyIdValidator;

impl GyIdValidator {
    /// 验证 GyID 格式
    pub fn validate_format(gyid: &str) -> Result<bool> {
        // 检查长度
        if gyid.len() < 32 || gyid.len() > 48 {
            return Ok(false);
        }
        
        // 检查前缀
        if !gyid.starts_with("GyID") {
            return Ok(false);
        }
        
        // 检查字符集 (Base58: A-Z, a-z, 0-9, -, _)
        for c in gyid.chars() {
            if !c.is_ascii_alphanumeric() && c != '-' && c != '_' {
                return Ok(false);
            }
        }
        
        Ok(true)
    }
    
    /// 验证 GyID 结构体
    pub fn validate(gyid: &GyId) -> Result<bool> {
        // 验证格式
        if !Self::validate_format(&gyid.id)? {
            return Err(GyIdError::InvalidParam("GyID 格式无效".to_string()));
        }
        
        // 如果有哈希值，则验证哈希匹配
        if !gyid.hash.is_empty() {
            use crate::crypto::Base58Encoder;
            let expected_hash = Base58Encoder::encode_hex(&gyid.hash)
                .map_err(GyIdError::CryptoError)?;
            
            // 确保 expected_hash 长度足够做切片
            if expected_hash.len() > 2 && !gyid.id.contains(&expected_hash[2..]) {
                return Err(GyIdError::InvalidParam("GyID 哈希不匹配".to_string()));
            }
        }
        
        // 验证时间戳
        if gyid.created_at == 0 {
            return Err(GyIdError::InvalidParam("GyID 时间戳无效".to_string()));
        }
        
        Ok(true)
    }
    
    /// 比较两个 GyID 是否相同
    pub fn equals(a: &GyId, b: &GyId) -> bool {
        a.id == b.id && a.hash == b.hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_format() {
        // 有效的 GyID（32字符，GyID前缀，Base58字符集）
        // "GyID" + 28字符 = 32字符
        assert_eq!(GyIdValidator::validate_format("GyID1234567890abcdefghijklmnopqr").unwrap(), true);
        // 太短（< 32字符）
        assert_eq!(GyIdValidator::validate_format("short").unwrap(), false);
        // 无效前缀（32字符，但不以GyID开头）
        assert_eq!(GyIdValidator::validate_format("XXXX1234567890abcdefghijklmnopqr").unwrap(), false);
    }
}
