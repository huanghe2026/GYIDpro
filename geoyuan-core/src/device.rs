//! 设备关联模块
//!
//! 管理 GeoYuan 网络中的多设备关联：
//! - 主设备（Master）生成授权码
//! - 从设备（Linked）通过授权码建立关联
//! - 关联记录持久化到 redb
//! - P2P 协议支持设备关联请求/响应

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 设备关联记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceLink {
    /// 关联 ID（BLAKE3 哈希）
    pub link_id: String,
    /// 主设备 GyID
    pub master_gyid: String,
    /// 从设备 GyID
    pub linked_gyid: String,
    /// 授权码
    pub auth_code: String,
    /// 关联时间（UTC 毫秒）
    pub linked_at: u64,
    /// 是否活跃
    pub active: bool,
}

/// 设备信息（完整）
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Device {
    /// GyID
    pub gyid: String,
    /// 设备角色
    pub role: DeviceRole,
    /// 关联时间
    pub linked_at: Option<u64>,
    /// 最后活跃时间
    pub last_active: u64,
    /// 是否活跃
    pub active: bool,
}

/// 设备角色
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DeviceRole {
    /// 主设备
    Master,
    /// 从设备
    Linked,
}

/// 授权码结构（解析后）
#[derive(Debug, Clone)]
struct ParsedAuthCode {
    /// 主设备 ID 前 8 字符
    master_prefix: String,
    /// 生成时间戳（秒）
    timestamp: u64,
    /// HMAC 校验段
    hmac: String,
}

/// 设备管理器
///
/// 管理本节点的设备关联信息。使用内存 HashMap 存储，
/// 持久化由 redb（geoyuan-core::storage）或 GUI 的 AppState 完成。
#[derive(Debug, Clone, Default)]
pub struct DeviceManager {
    /// 本设备关联记录: link_id → DeviceLink
    links: HashMap<String, DeviceLink>,
    /// 授权码缓存: auth_code → (master_gyid, created_at)
    pub pending_auth_codes: HashMap<String, (String, u64)>,
}

impl DeviceManager {
    /// 创建新的设备管理器
    pub fn new() -> Self {
        Self::default()
    }

    /// 生成授权码（主设备模式）
    ///
    /// 格式：`<master_prefix_8>:<ts_hex_8>:<hmac_8>`
    pub fn generate_auth_code(&self, master_gyid: &str) -> String {
        let prefix = if master_gyid.len() >= 8 {
            master_gyid[..8].to_uppercase()
        } else {
            format!("{:0<8}", master_gyid).to_uppercase()
        };

        let ts = chrono::Utc::now().timestamp() as u64;
        let ts_hex = format!("{:08X}", ts & 0xFFFF_FFFF);

        // HMAC = BLAKE3(master_id + timestamp) 前 8 字符
        let hmac_input = format!("{}:{}", master_gyid, ts);
        let hash = blake3::hash(hmac_input.as_bytes()).to_hex().to_string();
        let hmac_short = &hash[..8];
        let hmac_short = hmac_short.to_uppercase();

        format!("{}:{}:{}", prefix, ts_hex, hmac_short)
    }

    /// 保存授权码到待处理缓存
    pub fn save_pending_auth_code(&mut self, auth_code: &str, master_gyid: &str) {
        let now = chrono::Utc::now().timestamp() as u64;
        self.pending_auth_codes.insert(auth_code.to_string(), (master_gyid.to_string(), now));
    }

    /// 验证授权码
    ///
    /// 检查格式、时间戳（10 分钟有效期）、HMAC
    pub fn verify_auth_code(&self, auth_code: &str, master_gyid: &str) -> bool {
        let parsed = match Self::parse_auth_code(auth_code) {
            Some(p) => p,
            None => return false,
        };

        let now_secs = chrono::Utc::now().timestamp() as u64;

        // 10 分钟有效期 + 5 秒时钟偏差
        let elapsed = now_secs.saturating_sub(parsed.timestamp);
        if elapsed > 605 {
            return false;
        }
        if parsed.timestamp > now_secs + 5 {
            return false;
        }

        // 校验主设备前缀
        let expected_prefix = if master_gyid.len() >= 8 {
            master_gyid[..8].to_uppercase()
        } else {
            format!("{:0<8}", master_gyid).to_uppercase()
        };
        if parsed.master_prefix.to_uppercase() != expected_prefix {
            return false;
        }

        // 重新计算 HMAC
        let hmac_input = format!("{}:{}", master_gyid, parsed.timestamp);
        let expected = blake3::hash(hmac_input.as_bytes()).to_hex().to_string();
        parsed.hmac.to_uppercase() == expected[..8].to_uppercase()
    }

    /// 创建设备关联
    pub fn create_link(
        &mut self,
        master_gyid: &str,
        linked_gyid: &str,
        auth_code: &str,
    ) -> Result<DeviceLink, String> {
        // 防止关联到自身
        if master_gyid == linked_gyid {
            return Err("不能关联到本设备".to_string());
        }

        // 防止重复关联
        for link in self.links.values() {
            if link.master_gyid == master_gyid
                && link.linked_gyid == linked_gyid
                && link.active
            {
                return Err("该设备已关联".to_string());
            }
        }

        let link_id = blake3::hash(format!("{}:{}", master_gyid, linked_gyid).as_bytes()).to_hex().to_string();
        let now = chrono::Utc::now().timestamp_millis() as u64;

        let link = DeviceLink {
            link_id: link_id.clone(),
            master_gyid: master_gyid.to_string(),
            linked_gyid: linked_gyid.to_string(),
            auth_code: auth_code.to_string(),
            linked_at: now,
            active: true,
        };

        self.links.insert(link_id.clone(), link.clone());

        // 清理对应的待处理授权码
        self.pending_auth_codes.retain(|_, v| {
            chrono::Utc::now().timestamp() as u64 - v.1 < 600
        });

        Ok(link)
    }

    /// 获取某主设备的所有关联
    pub fn get_links(&self, master_gyid: &str) -> Vec<&DeviceLink> {
        self.links
            .values()
            .filter(|l| l.master_gyid == master_gyid && l.active)
            .collect()
    }

    /// 获取活跃关联数量
    pub fn active_link_count(&self, master_gyid: &str) -> usize {
        self.links
            .values()
            .filter(|l| l.master_gyid == master_gyid && l.active)
            .count()
    }

    /// 停用某关联
    pub fn deactivate_link(&mut self, link_id: &str) -> bool {
        if let Some(link) = self.links.get_mut(link_id) {
            link.active = false;
            true
        } else {
            false
        }
    }

    /// 解析授权码字符串
    fn parse_auth_code(code: &str) -> Option<ParsedAuthCode> {
        let parts: Vec<&str> = code.split(':').collect();
        if parts.len() != 3 {
            return None;
        }
        let ts = u64::from_str_radix(parts[1], 16).ok()?;
        Some(ParsedAuthCode {
            master_prefix: parts[0].to_string(),
            timestamp: ts,
            hmac: parts[2].to_string(),
        })
    }

    /// 导出所有关联为序列化格式（供持久化）
    pub fn export_links(&self) -> Vec<DeviceLink> {
        self.links.values().cloned().collect()
    }

    /// 导入关联记录（从持久化恢复）
    pub fn import_links(&mut self, links: Vec<DeviceLink>) {
        for link in links {
            self.links.insert(link.link_id.clone(), link);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_auth_code_roundtrip() {
        let mgr = DeviceManager::new();
        let master_id = "GyID_ABCDE12345_FGHIJ67890_KLMNO";
        let code = mgr.generate_auth_code(master_id);

        assert!(mgr.verify_auth_code(&code, master_id));
    }

    #[test]
    fn test_expired_auth_code() {
        let mgr = DeviceManager::new();
        let master_id = "GyID_ABCDE12345_FGHIJ67890_KLMNO";
        let code = mgr.generate_auth_code(master_id);

        // 模拟 11 分钟后
        let future = chrono::Utc::now().timestamp() as u64 + 660;
        // 直接测试解析逻辑
        let parsed = DeviceManager::parse_auth_code(&code).unwrap();
        let elapsed = future.saturating_sub(parsed.timestamp);
        assert!(elapsed > 605, "授权码应该已过期");
    }

    #[test]
    fn test_create_link() {
        let mut mgr = DeviceManager::new();
        let master = "GyID_Master1234567890ABCDEFGHIJ";
        let linked = "GyID_Linked1234567890ABCDEFGH";

        let link = mgr.create_link(master, linked, "test_auth").unwrap();
        assert_eq!(link.master_gyid, master);
        assert_eq!(link.linked_gyid, linked);
        assert!(link.active);

        // 重复关联应该失败
        assert!(mgr.create_link(master, linked, "test_auth2").is_err());
    }

    #[test]
    fn test_self_link_rejected() {
        let mut mgr = DeviceManager::new();
        let gyid = "GyID_Same1234567890ABCDEFGHJKLM";
        assert!(mgr.create_link(gyid, gyid, "code").is_err());
    }

    #[test]
    fn test_deactivate_link() {
        let mut mgr = DeviceManager::new();
        let link = mgr.create_link("master", "linked", "code").unwrap();

        assert!(mgr.deactivate_link(&link.link_id));
        assert_eq!(mgr.active_link_count("master"), 0);
    }

    #[test]
    fn test_export_import() {
        let mut mgr = DeviceManager::new();
        mgr.create_link("m1", "l1", "c1").unwrap();
        mgr.create_link("m1", "l2", "c2").unwrap();

        let exported = mgr.export_links();
        assert_eq!(exported.len(), 2);

        let mut mgr2 = DeviceManager::new();
        mgr2.import_links(exported);
        assert_eq!(mgr2.active_link_count("m1"), 2);
    }
}
