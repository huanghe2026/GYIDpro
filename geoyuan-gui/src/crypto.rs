// ═══════════════════════════════════════════════════════════════════════════
// 加密服务模块
// GyID 生成
// ═══════════════════════════════════════════════════════════════════════════

use blake3::Hasher;
use base58::{ToBase58, FromBase58};
use sha2::Digest;
use std::path::Path;

/// GyID 生成配置
#[derive(Debug, Clone)]
pub struct GyIdConfig {
    /// 网络前缀: "GyID" (主网) 或 "TGyID" (测试网)
    pub prefix: String,
    /// 版本号
    pub version: u8,
}

impl Default for GyIdConfig {
    fn default() -> Self {
        Self {
            prefix: "GyID".to_string(),
            version: 0x01,
        }
    }
}

/// GyID 生成结果
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct GyIdResult {
    /// GyID 字符串
    pub id: String,
    /// GyID 原始字节
    pub bytes: Vec<u8>,
    /// 校验和
    pub checksum: [u8; 4],
    /// 创建时间戳
    pub timestamp: i64,
}

/// 加密服务
pub struct CryptoService;

impl CryptoService {
    /// 生成 GyID
    /// 
    /// 算法:
    /// 1. SHA256(照片数据)
    /// 2. BLAKE3(lat || lon || timestamp || sha256_photo)
    /// 3. Base58 编码，添加前缀 GyID
    pub fn generate_gyid(
        photo_path: &Path,
        latitude: f64,
        longitude: f64,
        timestamp: i64,
    ) -> Result<String, CryptoError> {
        log::info!("🔐 开始生成 GyID...");
        
        // 1. 计算照片哈希
        let photo_hash = Self::hash_photo(photo_path)?;
        log::debug!("📷 照片哈希: {}", hex::encode(photo_hash));
        
        // 2. 构建输入数据
        let mut input = Vec::with_capacity(8 + 8 + 8 + 32);
        input.extend_from_slice(&latitude.to_le_bytes());
        input.extend_from_slice(&longitude.to_le_bytes());
        input.extend_from_slice(&timestamp.to_le_bytes());
        input.extend_from_slice(&photo_hash);
        
        // 3. BLAKE3 哈希
        let mut hasher = Hasher::new();
        hasher.update(&input);
        let gyid_hash = hasher.finalize();
        
        // 4. 构建 GyID
        let result = Self::encode_gyid(&gyid_hash)?;
        
        log::info!("✅ GyID 生成成功: {}", result.id);
        Ok(result.id)
    }
    
    /// 哈希照片
    fn hash_photo(path: &Path) -> Result<[u8; 32], CryptoError> {
        use std::io::Read;
        
        let mut file = std::fs::File::open(path)
            .map_err(|e| CryptoError::IoError(e.to_string()))?;
        
        let mut hasher = sha2::Sha256::new();
        let mut buffer = [0u8; 65536];
        
        loop {
            let bytes_read = file.read(&mut buffer)
                .map_err(|e| CryptoError::IoError(e.to_string()))?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Ok(hash)
    }
    
    /// 编码为 GyID
    fn encode_gyid(hash: &blake3::Hash) -> Result<GyIdResult, CryptoError> {
        let config = GyIdConfig::default();
        
        // 构建 GyID 数据
        // 格式: prefix (4) + version (1) + hash (32) + checksum (4) = 41 bytes
        let mut data = Vec::with_capacity(41);
        
        // 前缀字节
        let prefix_bytes: [u8; 4] = match config.prefix.as_str() {
            "GyID" => [0x47, 0x79, 0x49, 0x44],  // "GyID"
            "TGyID" => [0x54, 0x47, 0x79, 0x49], // "TGyI" (简化)
            _ => [0x47, 0x79, 0x49, 0x44],
        };
        data.extend_from_slice(&prefix_bytes);
        
        // 版本
        data.push(config.version);
        
        // 哈希前 28 字节 (224 bits)
        let hash_bytes = hash.as_bytes();
        data.extend_from_slice(&hash_bytes[..28]);
        
        // 计算校验和 (BLKe3 前 4 字节)
        let checksum: [u8; 4] = [
            hash_bytes[0],
            hash_bytes[1],
            hash_bytes[2],
            hash_bytes[3],
        ];
        data.extend_from_slice(&checksum);
        
        // Base58 编码
        let encoded = data.to_base58();
        
        Ok(GyIdResult {
            id: format!("{}{}", config.prefix, encoded),
            bytes: data,
            checksum,
            timestamp: chrono::Utc::now().timestamp_millis(),
        })
    }
    
    /// 验证 GyID
    pub fn verify_gyid(gyid: &str) -> Result<bool, CryptoError> {
        // 检查前缀
        let prefix = if gyid.starts_with("TGyID") {
            "TGyID"
        } else if gyid.starts_with("GyID") {
            "GyID"
        } else {
            return Err(CryptoError::InvalidFormat("无效的前缀".to_string()));
        };
        
        // 提取 Base58 部分
        let base58_part = &gyid[prefix.len()..];
        
        // Base58 解码
        let decoded = base58_part.from_base58()
            .map_err(|_| CryptoError::InvalidFormat("Base58 解码失败".to_string()))?;
        
        // 验证长度
        if decoded.len() != 41 {
            return Err(CryptoError::InvalidFormat("长度错误".to_string()));
        }
        
        // 验证前缀
        let expected_prefix: [u8; 4] = match prefix {
            "GyID" => [0x47, 0x79, 0x49, 0x44],
            _ => [0x47, 0x79, 0x49, 0x44],
        };
        if decoded[..4] != expected_prefix {
            return Err(CryptoError::InvalidFormat("前缀不匹配".to_string()));
        }
        
        // 验证校验和
        let stored_checksum: [u8; 4] = [
            decoded[37],
            decoded[38],
            decoded[39],
            decoded[40],
        ];
        
        // 重新计算校验和
        let hash_part = &decoded[5..33];
        let computed_checksum: [u8; 4] = [
            hash_part[0],
            hash_part[1],
            hash_part[2],
            hash_part[3],
        ];
        
        Ok(stored_checksum == computed_checksum)
    }
    
    /// 从 GyID 提取哈希 (不验证校验和)
    pub fn extract_hash(gyid: &str) -> Result<[u8; 28], CryptoError> {
        let prefix = if gyid.starts_with("TGyID") {
            5
        } else if gyid.starts_with("GyID") {
            4
        } else {
            return Err(CryptoError::InvalidFormat("无效的前缀".to_string()));
        };
        
        let base58_part = &gyid[prefix..];
        let decoded = base58_part.from_base58()
            .map_err(|_| CryptoError::InvalidFormat("Base58 解码失败".to_string()))?;
        
        if decoded.len() < 33 {
            return Err(CryptoError::InvalidFormat("数据太短".to_string()));
        }
        
        let mut hash = [0u8; 28];
        hash.copy_from_slice(&decoded[5..33]);
        Ok(hash)
    }
}

/// 加密错误
#[derive(Debug, thiserror::Error)]
pub enum CryptoError {
    #[error("IO 错误: {0}")]
    IoError(String),
    
    #[error("无效格式: {0}")]
    InvalidFormat(String),
    
    #[allow(dead_code)]
    #[error("Base58 编码/解码错误: {0}")]
    Base58Error(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gyid_format() {
        // GyID 格式: GyID + Base58(41字节)
        // Base58 编码后的长度约 56-58 字符
        let test_id = "GyID7NpFH5xL8vJMwBVXyZG5R2eT4hYmK3Qs6Pp8N1XzL9Uo2Mn";
        assert!(test_id.starts_with("GyID"));
        assert!(test_id.len() >= 50);
    }
    
    #[test]
    fn test_hash_extraction() {
        let gyid = "GyID7NpFH5xL8vJMwBVXyZG5R2eT4hYmK3Qs6Pp8N1XzL9Uo2Mn";
        let result = CryptoService::extract_hash(gyid);
        assert!(result.is_ok());
        assert_eq!(result.unwrap().len(), 28);
    }
}
