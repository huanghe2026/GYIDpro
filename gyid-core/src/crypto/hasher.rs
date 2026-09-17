//! 哈希运算模块

use blake3::Hasher;

/// GyID 专用哈希器
pub struct GyIdHasher {
    hasher: Hasher,
}

impl GyIdHasher {
    /// 创建新的 GyIdHasher
    pub fn new() -> Self {
        Self {
            hasher: Hasher::new(),
        }
    }
    
    /// 添加数据到哈希
    pub fn update(&mut self, data: &[u8]) -> &mut Self {
        self.hasher.update(data);
        self
    }
    
    /// 添加字符串数据
    pub fn update_str(&mut self, s: &str) -> &mut Self {
        self.hasher.update(s.as_bytes());
        self
    }
    
    /// 最终化并返回十六进制字符串
    pub fn finalize_hex(&self) -> String {
        self.hasher.finalize().to_hex().to_string()
    }
    
    /// 最终化并返回原始字节
    pub fn finalize(&self) -> [u8; 32] {
        *self.hasher.finalize().as_bytes()
    }
}

impl Default for GyIdHasher {
    fn default() -> Self {
        Self::new()
    }
}

impl GyIdHasher {
    /// 便捷方法：计算多个字符串的组合哈希
    pub fn hash_strings(strings: &[&str]) -> String {
        let mut hasher = GyIdHasher::new();
        for s in strings {
            hasher.update_str(s);
        }
        hasher.finalize_hex()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_hash() {
        let mut hasher = GyIdHasher::new();
        hasher.update_str("hello");
        hasher.update_str("world");
        let hash = hasher.finalize_hex();
        println!("Hash: {}", hash);
        assert_eq!(hash.len(), 64);
    }
    
    #[test]
    fn test_hash_strings() {
        let hash = GyIdHasher::hash_strings(&["a", "b", "c"]);
        println!("Combined hash: {}", hash);
    }
}
