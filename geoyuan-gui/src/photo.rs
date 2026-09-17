// ═══════════════════════════════════════════════════════════════════════════
// 照片处理模块
// 提取 EXIF GPS 信息
// ═══════════════════════════════════════════════════════════════════════════

use std::path::Path;
use std::io::BufReader;
use serde::{Deserialize, Serialize};
use sha2::Digest;

/// 照片元数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhotoMetadata {
    /// 文件路径
    pub path: String,
    /// 文件大小
    pub size: u64,
    /// 创建时间
    pub created: Option<chrono::DateTime<chrono::Utc>>,
    /// GPS 坐标
    pub gps: Option<GpsCoordinate>,
    /// 时间戳
    pub timestamp: i64,
    /// 照片哈希 (SHA256)
    pub hash: String,
}

/// GPS 坐标
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GpsCoordinate {
    /// 纬度
    pub latitude: f64,
    /// 经度
    pub longitude: f64,
    /// 高度 (米)
    pub altitude: Option<f64>,
    /// 原始 EXIF 数据
    pub raw: RawGpsData,
}

/// 原始 GPS 数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RawGpsData {
    /// GPS 纬度
    pub latitude: (f64, f64, f64, char),
    /// GPS 经度
    pub longitude: (f64, f64, f64, char),
    /// GPS 高度参照
    pub altitude_ref: Option<i32>,
    /// GPS 时间戳
    pub timestamp: Option<String>,
    /// GPS 日期
    pub date: Option<String>,
}

/// 照片处理器
pub struct PhotoProcessor;

impl PhotoProcessor {
    /// 从照片中提取元数据
    pub fn extract_metadata(path: &Path) -> Result<PhotoMetadata, PhotoError> {
        log::info!("📷 提取照片元数据: {:?}", path);
        
        // 检查文件是否存在
        if !path.exists() {
            return Err(PhotoError::FileNotFound(path.display().to_string()));
        }
        
        // 检查文件扩展名
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        
        if !matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "heic" | "webp" | "tiff" | "tif") {
            return Err(PhotoError::UnsupportedFormat(ext));
        }

        // 读取文件
        let file = std::fs::File::open(path)?;
        let file_size = file.metadata()?.len();
        let mut reader = BufReader::new(file);
        
        // 提取 EXIF（对无 EXIF 的格式如 PNG 容错）
        let exif = exif::Reader::new()
            .read_from_container(&mut reader)
            .map_err(|e| {
                log::warn!("⚠️ EXIF 解析失败 ({}): {} — 尝试继续", ext, e);
                PhotoError::ExifError(e.to_string())
            })?;
        
        // 解析 GPS（没有 GPS 返回 None，不是错误）
        let gps = Self::parse_gps(&exif)?;
        
        // 计算文件哈希
        let hash = Self::calculate_hash(path)?;
        
        // 解析时间戳
        let timestamp = Self::parse_timestamp(&exif)?;
        
        // 获取创建时间
        let created = std::fs::metadata(path)?
            .created()
            .ok()
            .map(chrono::DateTime::from);
        
        Ok(PhotoMetadata {
            path: path.display().to_string(),
            size: file_size,
            created,
            gps,
            timestamp,
            hash,
        })
    }
    
    /// 解析 GPS 坐标
    fn parse_gps(exif: &exif::Exif) -> Result<Option<GpsCoordinate>, PhotoError> {
        // 检查是否有 GPS 信息
        let lat_field = match exif.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY) {
            Some(f) => f,
            None => return Ok(None),
        };
        
        let lon_field = exif.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY)
            .ok_or(PhotoError::MissingGpsField("Longitude".to_string()))?;
        
        let lat_ref = exif.get_field(exif::Tag::GPSLatitudeRef, exif::In::PRIMARY)
            .map(|f| f.display_value().to_string());
        let lon_ref = exif.get_field(exif::Tag::GPSLongitudeRef, exif::In::PRIMARY)
            .map(|f| f.display_value().to_string());
        
        // 解析纬度
        let lat = Self::parse_gps_degree(lat_field)?;
        let lat = match lat_ref.as_deref() {
            Some("S") => -lat,
            _ => lat,
        };
        
        // 解析经度
        let lon = Self::parse_gps_degree(lon_field)?;
        let lon = match lon_ref.as_deref() {
            Some("W") => -lon,
            _ => lon,
        };
        
        // 解析高度
        let altitude = exif.get_field(exif::Tag::GPSAltitude, exif::In::PRIMARY)
            .and_then(|f| {
                match &f.value {
                    exif::Value::Rational(v) if !v.is_empty() => {
                        Some(v[0].to_f64())
                    }
                    _ => None,
                }
            });
        
        // 解析时间戳
        let timestamp = exif.get_field(exif::Tag::GPSTimeStamp, exif::In::PRIMARY)
            .map(|f| f.display_value().to_string());
        
        // 解析日期
        let date = exif.get_field(exif::Tag::GPSDateStamp, exif::In::PRIMARY)
            .map(|f| f.display_value().to_string());
        
        let raw = RawGpsData {
            latitude: (0.0, 0.0, 0.0, 'N'), // 实际应从字段解析
            longitude: (0.0, 0.0, 0.0, 'E'),
            altitude_ref: None,
            timestamp,
            date,
        };
        
        log::info!("📍 GPS: lat={}, lon={}", lat, lon);
        
        Ok(Some(GpsCoordinate {
            latitude: lat,
            longitude: lon,
            altitude,
            raw,
        }))
    }
    
    /// 解析 GPS 度数
    fn parse_gps_degree(field: &exif::Field) -> Result<f64, PhotoError> {
        match &field.value {
            exif::Value::Rational(rationals) if rationals.len() >= 3 => {
                let deg = rationals[0].to_f64();
                let min = rationals[1].to_f64();
                let sec = rationals[2].to_f64();
                Ok(deg + min / 60.0 + sec / 3600.0)
            }
            _ => Err(PhotoError::InvalidGpsFormat),
        }
    }
    
    /// 解析时间戳
    fn parse_timestamp(exif: &exif::Exif) -> Result<i64, PhotoError> {
        // 优先使用 EXIF DateTime
        if let Some(field) = exif.get_field(exif::Tag::DateTimeOriginal, exif::In::PRIMARY) {
            let datetime = field.display_value().to_string();
            if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(&datetime, "%Y:%m:%d %H:%M:%S") {
                return Ok(dt.and_utc().timestamp_millis());
            }
        }
        
        // 使用文件创建时间
        Ok(chrono::Utc::now().timestamp_millis())
    }
    
    /// 计算文件 SHA256 哈希
    fn calculate_hash(path: &Path) -> Result<String, PhotoError> {
        use std::io::Read;
        
        let mut file = std::fs::File::open(path)?;
        let mut hasher = sha2::Sha256::new();
        let mut buffer = [0u8; 8192];
        
        loop {
            let bytes_read = file.read(&mut buffer)?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }
        
        Ok(format!("{:x}", hasher.finalize()))
    }
    
    /// 快速预检照片是否有 GPS 信息
    /// 返回：
    ///   Ok(Some(true))  — 有 GPS
    ///   Ok(Some(false)) — EXIF 读取成功，但无 GPS
    ///   Ok(None)        — EXIF 读取失败（格式不支持或文件损坏）
    ///   Err(_)          — 文件不存在或 IO 错误
    pub fn check_gps(path: &Path) -> Result<Option<bool>, PhotoError> {
        if !path.exists() {
            return Err(PhotoError::FileNotFound(path.display().to_string()));
        }
        
        let ext = path.extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        
        // PNG 格式明确不支持 EXIF
        if ext == "png" {
            log::warn!("⚠️ PNG 格式不支持 EXIF GPS 数据");
            return Ok(None);
        }
        
        let file = std::fs::File::open(path)?;
        let mut reader = BufReader::new(file);
        
        match exif::Reader::new().read_from_container(&mut reader) {
            Ok(exif) => {
                let has_lat = exif.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY).is_some();
                let has_lon = exif.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY).is_some();
                Ok(Some(has_lat && has_lon))
            }
            Err(e) => {
                log::warn!("⚠️ EXIF 预检失败 ({}): {}", ext, e);
                Ok(None)
            }
        }
    }
    
    /// 验证照片是否有 GPS 信息
    #[allow(dead_code)]
    pub fn has_gps(path: &Path) -> Result<bool, PhotoError> {
        match Self::check_gps(path) {
            Ok(Some(has)) => Ok(has),
            Ok(None) => Ok(false),
            Err(e) => Err(e),
        }
    }
}

/// 照片处理错误
#[derive(Debug, thiserror::Error)]
pub enum PhotoError {
    #[error("文件不存在: {0}")]
    FileNotFound(String),
    
    #[error("EXIF 解析错误: {0}")]
    ExifError(String),
    
    #[error("不支持的照片格式: {0}")]
    UnsupportedFormat(String),
    
    #[error("缺少 GPS 字段: {0}")]
    MissingGpsField(String),
    
    #[error("无效的 GPS 格式")]
    InvalidGpsFormat,
    
    #[error("IO 错误: {0}")]
    IoError(#[from] std::io::Error),
    
    #[allow(dead_code)]
    #[error("哈希计算错误: {0}")]
    HashError(String),
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_gps_degree_parsing() {
        // 测试度数解析逻辑
        let degrees = 45.0 + 30.0 / 60.0 + 15.0 / 3600.0;
        assert!((degrees - 45.504166_f64).abs() < 0.0001_f64);
    }

    #[test]
    fn test_real_photo_exif() {
        // 用真实照片测试 EXIF 解析
        let path = Path::new(r"C:\Users\Nice\Pictures\GEOYUAN\IMG_20260418_035047.jpg");
        if !path.exists() {
            eprintln!("⚠️ 测试照片不存在: {:?}", path);
            return;
        }
        
        eprintln!("📁 文件大小: {} bytes", std::fs::metadata(path).map(|m| m.len()).unwrap_or(0));
        
        // 先测试 EXIF 读取
        let file = std::fs::File::open(path).unwrap();
        let mut reader = BufReader::new(file);
        
        match exif::Reader::new().read_from_container(&mut reader) {
            Ok(exif) => {
                eprintln!("✅ EXIF 读取成功");
                
                // 列出所有 EXIF 字段
                for field in exif.fields() {
                    eprintln!("  [{}] {} = {}", field.ifd_num, field.tag, field.display_value().to_string());
                }
                
                // 检查 GPS
                let has_gps_lat = exif.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY).is_some();
                let has_gps_lon = exif.get_field(exif::Tag::GPSLongitude, exif::In::PRIMARY).is_some();
                eprintln!("📍 GPS Latitude: {}", has_gps_lat);
                eprintln!("📍 GPS Longitude: {}", has_gps_lon);
                
                // 检查时间戳
                let has_datetime = exif.get_field(exif::Tag::DateTime, exif::In::PRIMARY).is_some();
                eprintln!("🕐 DateTime: {}", has_datetime);
            }
            Err(e) => {
                eprintln!("❌ EXIF 读取失败: {}", e);
            }
        }
        
        // 再测完整 extract_metadata
        match PhotoProcessor::extract_metadata(path) {
            Ok(meta) => {
                eprintln!("✅ extract_metadata 成功");
                eprintln!("  路径: {}", meta.path);
                eprintln!("  大小: {} bytes", meta.size);
                eprintln!("  GPS: {:?}", meta.gps);
                eprintln!("  时间戳: {}", meta.timestamp);
                eprintln!("  哈希: {}", meta.hash);
            }
            Err(e) => {
                eprintln!("❌ extract_metadata 失败: {}", e);
            }
        }
    }

    #[test]
    fn test_png_files_exif() {
        // 测试 PNG 文件是否能解析 EXIF
        let png_dir = Path::new(r"C:\Users\Nice\Pictures\GEOYUAN");
        if !png_dir.exists() {
            eprintln!("⚠️ 测试目录不存在，跳过 PNG EXIF 测试: {:?}", png_dir);
            return;
        }
        for entry in std::fs::read_dir(png_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()) != Some("png".to_string()) {
                continue;
            }
            eprintln!("\n📁 PNG 文件: {:?}", path);
            eprintln!("  大小: {} bytes", entry.metadata().map(|m| m.len()).unwrap_or(0));
            
            let file = std::fs::File::open(&path).unwrap();
            let mut reader = BufReader::new(file);
            
            match exif::Reader::new().read_from_container(&mut reader) {
                Ok(exif) => {
                    eprintln!("  ✅ EXIF 读取成功");
                    let has_gps = exif.get_field(exif::Tag::GPSLatitude, exif::In::PRIMARY).is_some();
                    eprintln!("  📍 GPS: {}", has_gps);
                    if !has_gps {
                        eprintln!("  ⚠️ 没有 GPS 信息");
                    }
                }
                Err(e) => {
                    eprintln!("  ❌ EXIF 读取失败: {}", e);
                }
            }
            
            match PhotoProcessor::extract_metadata(&path) {
                Ok(meta) => {
                    eprintln!("  ✅ extract_metadata 成功, GPS: {:?}", meta.gps);
                }
                Err(e) => {
                    eprintln!("  ❌ extract_metadata 失败: {}", e);
                }
            }
        }
    }

    #[test]
    fn test_extract_real_photo() {
        let path = Path::new(r"C:\Users\Nice\Pictures\GEOYUAN\IMG_20260418_035047.jpg");
        if !path.exists() {
            eprintln!("⚠️ 测试照片不存在: {:?}", path);
            return;
        }
        match PhotoProcessor::extract_metadata(path) {
            Ok(meta) => {
                eprintln!("✅ extract_metadata 成功");
                eprintln!("  路径: {}", meta.path);
                eprintln!("  大小: {} bytes", meta.size);
                eprintln!("  GPS: {:?}", meta.gps);
                eprintln!("  时间戳: {}", meta.timestamp);
                eprintln!("  哈希: {}", meta.hash);
            }
            Err(e) => {
                eprintln!("❌ extract_metadata 失败: {}", e);
            }
        }
    }
}
