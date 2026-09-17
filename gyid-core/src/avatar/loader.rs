//! 头像加载模块

use crate::{GyIdError, Result, avatar::Avatar};

/// 头像加载器
pub struct AvatarLoader;

impl AvatarLoader {
    /// 加载头像图片
    /// 
    /// 支持格式: PNG, JPEG, WebP, GIF
    pub fn load(path: &str) -> Result<Avatar> {
        use std::path::Path;
        use image::GenericImageView;
        
        let path_obj = Path::new(path);
        
        // 检查文件是否存在
        if !path_obj.exists() {
            return Err(GyIdError::AvatarError(
                format!("文件不存在: {}", path)
            ));
        }
        
        // 获取文件扩展名
        let ext = path_obj
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.to_lowercase())
            .unwrap_or_default();
        
        // 检查格式
        let supported = ["png", "jpg", "jpeg", "webp", "gif", "bmp"];
        if !supported.contains(&ext.as_str()) {
            return Err(GyIdError::AvatarError(
                format!("不支持的图片格式: {}", ext)
            ));
        }
        
        // 加载图片
        let img = image::open(path)
            .map_err(|e| GyIdError::AvatarError(format!("图片加载失败: {}", e)))?;
        
        let dimensions = Some(img.dimensions());
        
        // 计算哈希
        let hash = crate::avatar::hasher::AvatarHasher::hash_image(&img)?;
        
        Ok(Avatar {
            path: Some(path.to_string()),
            hash,
            dimensions,
            format: Some(ext),
        })
    }
    
    /// 从内存数据加载头像
    pub fn load_from_bytes(data: &[u8], format: &str) -> Result<Avatar> {
        use image::ImageReader;
        use std::io::Cursor;
        
        let cursor = Cursor::new(data);
        let img = ImageReader::new(cursor)
            .with_guessed_format()
            .map_err(|e| GyIdError::AvatarError(format!("格式检测失败: {}", e)))?
            .decode()
            .map_err(|e| GyIdError::AvatarError(format!("图片解码失败: {}", e)))?;
        
        use image::GenericImageView;
        let dimensions = Some(img.dimensions());
        let hash = crate::avatar::hasher::AvatarHasher::hash_image(&img)?;
        
        Ok(Avatar {
            path: None,
            hash,
            dimensions,
            format: Some(format.to_string()),
        })
    }
}

#[cfg(test)]
mod tests {
    #[allow(unused_imports)]
    use super::*;

    #[test]
    fn test_load_sample() {
        // 测试需要一个存在的图片文件
        // let avatar = AvatarLoader::load("test.png");
        // println!("Avatar: {:?}", avatar);
    }
}
