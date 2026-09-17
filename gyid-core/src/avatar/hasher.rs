//! 头像哈希计算模块

use crate::Result;
use image::DynamicImage;

/// 头像哈希器
pub struct AvatarHasher;

impl AvatarHasher {
    /// 计算图片的 BLAKE3 哈希
    /// 
    /// 流程:
    /// 1. 将图片缩放到固定大小 (64x64)
    /// 2. 转换为灰度图
    /// 3. 计算 BLAKE3 哈希
    pub fn hash_image(img: &DynamicImage) -> Result<String> {
        // 缩放到固定大小以保证一致性
        let resized = img.resize_exact(64, 64, image::imageops::FilterType::Triangle);
        
        // 转换为灰度图
        let gray = resized.to_luma8();
        
        // 计算 BLAKE3 哈希
        let hash = blake3::hash(gray.as_raw());
        let hash_str = hash.to_hex().to_string();
        
        Ok(hash_str)
    }
    
    /// 计算字节数据的哈希
    pub fn hash_bytes(data: &[u8]) -> String {
        let hash = blake3::hash(data);
        hash.to_hex().to_string()
    }
    
    /// 计算字符串的哈希
    pub fn hash_string(s: &str) -> String {
        let hash = blake3::hash(s.as_bytes());
        hash.to_hex().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_string() {
        let hash = AvatarHasher::hash_string("test");
        println!("Hash: {}", hash);
        assert_eq!(hash.len(), 64); // BLAKE3 输出 64 字符的十六进制
    }
}
